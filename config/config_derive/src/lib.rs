#![allow(unused)]
use std::borrow::Cow;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, Attribute, Data, DeriveInput, Field, Fields, Type};

type Result<T, E = ConfigError> = std::result::Result<T, E>;

#[derive(Debug)]
struct ConfigError {
    reason: Cow<'static, str>,
    span: Span,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for ConfigError {}

struct ConfigFieldMetadata {
    is_password: bool,
    is_text_area: bool,
}

struct ConfigField {
    name: String,
    ty: ConfigFieldType,
    metadata: ConfigFieldMetadata,
}

#[derive(Clone, Copy)]
enum ConfigFieldType {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    String,
    Bool,
}

impl ConfigFieldType {
    fn into_tokens(self) -> proc_macro2::TokenStream {
        match self {
            Self::I8 => quote! { config::FieldType::I8 },
            Self::I16 => quote! { config::FieldType::I16 },
            Self::I32 => quote! { config::FieldType::I32 },
            Self::I64 => quote! { config::FieldType::I64 },
            Self::U8 => quote! { config::FieldType::U8 },
            Self::U16 => quote! { config::FieldType::U16 },
            Self::U32 => quote! { config::FieldType::U32 },
            Self::U64 => quote! { config::FieldType::U64 },
            Self::String => quote! { config::FieldType::String },
            Self::Bool => quote! { config::FieldType::Bool },
        }
    }
}

fn build_config_field_metadata(field_attributes: &[Attribute]) -> Result<ConfigFieldMetadata> {
    let mut is_password = false;
    let mut is_text_area = false;
    match field_attributes {
        [attr] if attr.path().is_ident("input") => {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("password") {
                    is_password = true;
                    return Ok(());
                };
                if meta.path.is_ident("text_area") {
                    is_text_area = true;
                    return Ok(());
                };
                Ok(())
            })
            .unwrap();
        }
        [attr] => {
            let span = attr.span();
            Err(ConfigError {
                span: attr.span(),
                reason: format!("unsupported attrtibute: {:?}", attr.path()).into(),
            })?
        }
        _ => (),
    };
    Ok(ConfigFieldMetadata {
        is_password,
        is_text_area,
    })
}

fn get_field_type(field: &Field) -> Result<ConfigFieldType> {
    let path = match &field.ty {
        Type::Path(p) => p,
        ty => Err(ConfigError {
            span: ty.span(),
            reason: format!("unsupported field type: {ty:?}").into(),
        })?,
    };
    let ty = match path
        .path
        .get_ident()
        .map(|ident| ident.to_string())
        .as_deref()
    {
        Some("i8") => ConfigFieldType::I8,
        Some("i16") => ConfigFieldType::I16,
        Some("i32") => ConfigFieldType::I32,
        Some("i64") => ConfigFieldType::I64,
        Some("u8") => ConfigFieldType::U8,
        Some("u16") => ConfigFieldType::U16,
        Some("u32") => ConfigFieldType::U32,
        Some("u64") => ConfigFieldType::U64,
        Some("String") => ConfigFieldType::String,
        Some("bool") => ConfigFieldType::Bool,
        Some(other) => Err(ConfigError {
            span: path.span(),
            reason: format!("Unsupported field type: {other}").into(),
        })?,
        None => Err(ConfigError {
            span: path.span(),
            reason: "unexpected empty type".into(),
        })?,
    };
    Ok(ty)
}

fn build_config_field(field: &Field) -> Result<ConfigField> {
    let metadata = build_config_field_metadata(field.attrs.as_slice())?;
    let field_type = get_field_type(field)?;
    Ok(ConfigField {
        name: field.ident.as_ref().unwrap().to_string(),
        ty: field_type,
        metadata,
    })
}

fn parse_config(input: TokenStream) -> Result<TokenStream> {
    let input: DeriveInput = syn::parse(input).unwrap();
    if !input.generics.params.is_empty() {
        Err(ConfigError {
            reason: "generics are not supported".into(),
            span: input.span(),
        })?;
    }
    let ident = &input.ident;
    let strct = match &input.data {
        Data::Union(_) => Err(ConfigError {
            span: input.span(),
            reason: "unions are not supported".into(),
        })?,
        Data::Enum(_) => Err(ConfigError {
            span: input.span(),
            reason: "enums are not supported (yet)".into(),
        })?,
        Data::Struct(s) => s,
    };
    let fields = match strct.fields {
        Fields::Named(ref fields) => fields,
        Fields::Unnamed(_) | Fields::Unit => Err(ConfigError {
            span: strct.fields.span(),
            reason: "unit structs are not supported".into(),
        })?,
    };
    let config_fields = fields
        .named
        .iter()
        .map(build_config_field)
        .map(|res| match res {
            Ok(ConfigField { name, ty, metadata }) => {
                let ty = ty.into_tokens();
                let ConfigFieldMetadata {
                    is_password,
                    is_text_area,
                } = metadata;
                Ok(quote! {
                    config::Field{
                        name: #name,
                        ty: #ty,
                        metadata: config::Metadata {
                            is_password: #is_password,
                            is_text_area: #is_text_area,
                        }
                    }
                })
            }
            Err(e) => Err(e),
        })
        .collect::<Result<Vec<proc_macro2::TokenStream>>>()?;

    let tokens = quote! {
        impl config::Config for #ident {
            fn fields(&self) -> Vec<config::Field> {
                vec![
                    #(#config_fields),*
                ]
            }
        }
    };
    Ok(tokens.into())
}

#[proc_macro_derive(SectionConfig, attributes(input))]
pub fn config(input: TokenStream) -> TokenStream {
    match parse_config(input) {
        Ok(tokens) => tokens,
        Err(ConfigError { reason, span }) => {
            quote_spanned! { span => compile_error!(#reason); }.into()
        }
    }
}
