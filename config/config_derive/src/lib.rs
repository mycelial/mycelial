#![allow(unused)]
use std::borrow::Cow;

use proc_macro::{Ident, TokenStream};
use proc_macro2::{TokenTree, Span};
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, Attribute, Data, DeriveInput, Field, Fields, Meta, Type};

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

#[derive(Debug, Clone, Copy)]
enum SectionIO {
    None,
    Bin,
    DataFrame,
}

impl Into<proc_macro2::TokenStream> for  SectionIO {
    fn into(self) -> proc_macro2::TokenStream {
        match self {
            Self::None => quote!{ config::SectionIO::None },
            Self::Bin => quote!{ config::SectionIO::Bin },
            Self::DataFrame => quote!{ config::SectionIO::DataFrame },
        }
    }
}


impl Into<proc_macro2::TokenStream> for ConfigFieldType {
    fn into(self) -> proc_macro2::TokenStream {
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

fn parse_section_attr(attrs: &[Attribute]) -> Result<(proc_macro2::TokenStream, proc_macro2::TokenStream)> {
    let (i, o) = attrs
        .iter()
        .filter(|attr| attr.path().is_ident("section"))
        .try_fold((SectionIO::None, SectionIO::None), |(mut i, mut o), attr| {
            let tokens = match &attr.meta {
                Meta::List(list) => &list.tokens,
                other => return Err(ConfigError{ span: other.span(), reason: "expected section attributes as list".into()}),
            };
            let mut iter = tokens.clone().into_iter().peekable();
            loop {
                match iter.peek() {
                    None => break,
                    Some(TokenTree::Punct(_)) => {
                        iter.next();
                        continue
                    },
                    Some(TokenTree::Ident(ident)) => {
                        let (io, eq, value) = match (iter.next(), iter.next(), iter.next()) {
                            (Some(io), Some(eq), Some(value)) => (io, eq, value),
                            _ => return Err(ConfigError{span: attr.span(), reason: "malformed parameters".into()}),
                        };
                        match (io.to_string().as_str(), eq.to_string().as_str(), value.to_string().as_str()) {
                            ("input", "=", "bin") => {i = SectionIO::Bin;},
                            ("input", "=", "dataframe") => {i = SectionIO::DataFrame},
                            ("output", "=", "bin") => {o = SectionIO::Bin},
                            ("output", "=", "dataframe") => {o = SectionIO::DataFrame},
                            _ => return Err(ConfigError{span: attr.span(), reason: "malformed parameters".into()}),
                        }
                    },
                    _ => return Err(ConfigError{span: attr.span(), reason: "malformed parameters".into()}),
                }
            }
            Ok((i, o))
        })?;
    Ok((i.into(), o.into()))
}

fn build_config_field_metadata(field_attributes: &[Attribute]) -> Result<ConfigFieldMetadata> {
    let mut is_password = false;
    let mut is_text_area = false;
    match field_attributes {
        [attr] if attr.path().is_ident("field_type") => {
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
    let (section_input, section_output) = parse_section_attr(&input.attrs)?;
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
                let ty: proc_macro2::TokenStream = ty.into();
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
            fn input(&self) -> config::SectionIO {
                #section_input
            }
            
            fn output(&self) -> config::SectionIO {
                #section_output
            }

            fn fields(&self) -> Vec<config::Field> {
                vec![
                    #(#config_fields),*
                ]
            }
        }
    };
    Ok(tokens.into())
}

#[proc_macro_derive(
    Config,
    attributes(section, field_type)
)]
pub fn config(input: TokenStream) -> TokenStream {
    match parse_config(input) {
        Ok(tokens) => tokens,
        Err(ConfigError { reason, span }) => {
            quote_spanned! { span => compile_error!(#reason); }.into()
        }
    }
}
