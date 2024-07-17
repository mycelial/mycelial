#![allow(unused)]
use std::borrow::Cow;

use proc_macro::{Ident, TokenStream};
use proc_macro2::{Span, TokenTree};
use quote::{quote, quote_spanned};
use std::error::Error;
use syn::{
    spanned::Spanned, AngleBracketedGenericArguments, Attribute, Data, DeriveInput, Field, Fields,
    GenericArgument, Meta, PathArguments, PathSegment, Type, TypePath,
};

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

impl Error for ConfigError {}

struct ConfigFieldMetadata {
    is_password: bool,
    is_text_area: bool,
    is_read_only: bool,
}

struct ConfigFieldValidate {}
struct FieldAttributes {
    metadata: ConfigFieldMetadata,
    validate: ConfigFieldValidate,
}

struct ConfigField<'a> {
    name: String,
    ty: ConfigFieldType,
    metadata: ConfigFieldMetadata,
    name_ident: &'a proc_macro2::Ident,
}

#[derive(Clone)]
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
    //    Vec(Box<ConfigFieldType>),
}

#[derive(Debug, Clone, Copy)]
enum SectionIO {
    None,
    Bin,
    DataFrame,
}

impl From<SectionIO> for proc_macro2::TokenStream {
    fn from(val: SectionIO) -> Self {
        match val {
            SectionIO::None => quote! { config::SectionIO::None },
            SectionIO::Bin => quote! { config::SectionIO::Bin },
            SectionIO::DataFrame => quote! { config::SectionIO::DataFrame },
        }
    }
}

impl From<&ConfigFieldType> for proc_macro2::TokenStream {
    fn from(val: &ConfigFieldType) -> Self {
        match val {
            ConfigFieldType::I8 => quote! { config::FieldType::I8 },
            ConfigFieldType::I16 => quote! { config::FieldType::I16 },
            ConfigFieldType::I32 => quote! { config::FieldType::I32 },
            ConfigFieldType::I64 => quote! { config::FieldType::I64 },
            ConfigFieldType::U8 => quote! { config::FieldType::U8 },
            ConfigFieldType::U16 => quote! { config::FieldType::U16 },
            ConfigFieldType::U32 => quote! { config::FieldType::U32 },
            ConfigFieldType::U64 => quote! { config::FieldType::U64 },
            ConfigFieldType::String => quote! { config::FieldType::String },
            ConfigFieldType::Bool => quote! { config::FieldType::Bool },
            //     ConfigFieldType::Vec(ty) => {
            //         let tokens: proc_macro2::TokenStream = (&**ty).into();
            //         quote! { config::FieldType::Vec(Box::new(#tokens)) }
            //     }
        }
    }
}

fn parse_section_attr(
    attrs: &[Attribute],
) -> Result<(proc_macro2::TokenStream, proc_macro2::TokenStream)> {
    let (i, o) = attrs
        .iter()
        .filter(|attr| attr.path().is_ident("section"))
        .try_fold(
            (SectionIO::None, SectionIO::None),
            |(mut i, mut o), attr| {
                let tokens = match &attr.meta {
                    Meta::List(list) => &list.tokens,
                    other => {
                        return Err(ConfigError {
                            span: other.span(),
                            reason: "expected section attributes as list".into(),
                        })
                    }
                };
                let mut iter = tokens.clone().into_iter().peekable();
                loop {
                    match iter.peek() {
                        None => break,
                        Some(TokenTree::Punct(_)) => {
                            iter.next();
                            continue;
                        }
                        Some(TokenTree::Ident(ident)) => {
                            let (io, eq, value) = match (iter.next(), iter.next(), iter.next()) {
                                (Some(io), Some(eq), Some(value)) => (io, eq, value),
                                _ => {
                                    return Err(ConfigError {
                                        span: attr.span(),
                                        reason: "malformed parameters".into(),
                                    })
                                }
                            };
                            match (
                                io.to_string().as_str(),
                                eq.to_string().as_str(),
                                value.to_string().as_str(),
                            ) {
                                ("input", "=", "bin") => {
                                    i = SectionIO::Bin;
                                }
                                ("input", "=", "dataframe") => i = SectionIO::DataFrame,
                                ("output", "=", "bin") => o = SectionIO::Bin,
                                ("output", "=", "dataframe") => o = SectionIO::DataFrame,
                                _ => {
                                    return Err(ConfigError {
                                        span: attr.span(),
                                        reason: "malformed parameters".into(),
                                    })
                                }
                            }
                        }
                        _ => {
                            return Err(ConfigError {
                                span: attr.span(),
                                reason: "malformed parameters".into(),
                            })
                        }
                    }
                }
                Ok((i, o))
            },
        )?;
    Ok((i.into(), o.into()))
}

fn parse_field_attributes(field_attributes: &[Attribute]) -> Result<ConfigFieldMetadata> {
    let mut is_password = false;
    let mut is_text_area = false;
    let mut is_read_only = false;
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
                if meta.path.is_ident("read_only") {
                    is_read_only = true;
                    return Ok(());
                };
                Ok(())
            })
            .unwrap();
        }
        [attr] if attr.path().is_ident("validate") => {}
        [attr] => {
            let span = attr.span();
            Err(ConfigError {
                span: attr.span(),
                reason: format!("unsupported attribute: {:?}", attr.path()).into(),
            })?
        }
        _ => (),
    };
    Ok(ConfigFieldMetadata {
        is_password,
        is_text_area,
        is_read_only,
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
        other => Err(ConfigError {
            span: path.span(),
            reason: format!("Unsupported field type: {other:?}").into(),
        })?,
        //None => get_field_complex_type(path)?,
    };
    Ok(ty)
}

// FIXME: unused for now
//fn get_field_complex_type(path: &TypePath) -> Result<ConfigFieldType> {
//    match path.path.segments.iter().collect::<Vec<_>>().as_slice() {
//        &[PathSegment {
//            ident,
//            arguments: PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }),
//        }] if ident == "Vec" => match args.iter().collect::<Vec<_>>().as_slice() {
//            &[GenericArgument::Type(Type::Path(path))] => {
//                let ty = match path
//                    .path
//                    .get_ident()
//                    .map(|ident| ident.to_string())
//                    .as_deref()
//                {
//                    Some("i8") => ConfigFieldType::I8,
//                    Some("i16") => ConfigFieldType::I16,
//                    Some("i32") => ConfigFieldType::I32,
//                    Some("i64") => ConfigFieldType::I64,
//                    Some("u8") => ConfigFieldType::U8,
//                    Some("u16") => ConfigFieldType::U16,
//                    Some("u32") => ConfigFieldType::U32,
//                    Some("u64") => ConfigFieldType::U64,
//                    Some("String") => ConfigFieldType::String,
//                    Some("bool") => ConfigFieldType::Bool,
//                    Some(other) => Err(ConfigError {
//                        span: path.span(),
//                        reason: format!("Unsupported field type: {other}").into(),
//                    })?,
//                    None => get_field_complex_type(path)?,
//                };
//                Ok(ConfigFieldType::Vec(Box::new(ty)))
//            }
//            _ => Err(ConfigError {
//                span: args.span(),
//                reason: "unexpected Vec arguments".into(),
//            })?,
//        },
//        _ => Err(ConfigError {
//            span: path.span(),
//            reason: "unexpected complex type".into(),
//        })?,
//    }
//}

fn build_config_field(field: &Field) -> Result<ConfigField<'_>> {
    let metadata = parse_field_attributes(field.attrs.as_slice())?;
    let field_type = get_field_type(field)?;
    Ok(ConfigField {
        name: field.ident.as_ref().unwrap().to_string(),
        ty: field_type,
        metadata,
        name_ident: field.ident.as_ref().unwrap(),
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
        .collect::<Result<Vec<ConfigField>>>()?;

    let fields_impl = config_fields
        .iter()
        .map(
            |ConfigField {
                 name,
                 ty,
                 metadata,
                 name_ident,
             }| {
                let ty: proc_macro2::TokenStream = ty.into();
                let ConfigFieldMetadata {
                    is_password,
                    is_text_area,
                    is_read_only,
                } = metadata;
                let name_tokens = quote! { #name };
                quote! {
                    config::Field{
                        name: #name,
                        ty: #ty,
                        metadata: config::Metadata {
                            is_password: #is_password,
                            is_text_area: #is_text_area,
                            is_read_only: #is_read_only,
                        },
                        value: (&self.#name_ident).into(),
                    }
                }
            },
        )
        .collect::<Vec<proc_macro2::TokenStream>>();

    // field value is injected in method implementation
    let get_field_value_arms = config_fields
        .iter()
        .map(
            |ConfigField {
                 name, name_ident, ..
             }| {
                quote! {
                    #name => { Ok((&self.#name_ident).into()) }
                }
            },
        )
        .collect::<Vec<proc_macro2::TokenStream>>();

    // field value is injected in method implementation
    let set_field_value_arms = config_fields
        .iter()
        .map(
            |ConfigField {
                 name, name_ident, ..
             }| {
                quote! {
                    #name => { self.#name_ident = value.try_into()?; }
                }
            },
        )
        .collect::<Vec<proc_macro2::TokenStream>>();

    let strip_secrets_impl = config_fields
        .iter()
        .filter(|ConfigField { metadata, .. }| metadata.is_password)
        .map(
            |ConfigField {
                 name,
                 name_ident,
                 ty,
                 ..
             }| {
                let value: proc_macro2::TokenStream = match ty {
                    ConfigFieldType::Bool => quote! { false },
                    ConfigFieldType::String => quote! { String::new() },
                    _ => quote! { 0 },
                };
                quote! {
                    self.#name_ident = #value;
                }
            },
        )
        .collect::<Vec<proc_macro2::TokenStream>>();

    let name = ident.to_string();
    let tokens = quote! {
        impl config::Config for #ident {
            fn name(&self) -> &str {
                #name
            }

            fn input(&self) -> config::SectionIO {
                #section_input
            }

            fn output(&self) -> config::SectionIO {
                #section_output
            }

            fn fields(&self) -> Vec<config::Field> {
                vec![
                    #(#fields_impl),*
                ]
            }

            fn get_field_value(&self, name: &str)  -> Result<FieldValue<'_>, Box<dyn std::error::Error + Send + Sync + 'static>> {
                match name {
                    #(#get_field_value_arms),*
                    _ => Err(format!("no field with name '{name}' exist"))?,
                }
            }

            fn set_field_value(&mut self, name: &str, value: FieldValue<'_>) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
                match name {
                    #(#set_field_value_arms),*
                    _ => Err(format!("no field with name '{name}' exist"))?,
                };
                Ok(())
            }

            fn strip_secrets(&mut self) {
                #(#strip_secrets_impl)*
            }
        }
    };
    Ok(tokens.into())
}

#[proc_macro_derive(Config, attributes(section, field_type, validate))]
pub fn config(input: TokenStream) -> TokenStream {
    match parse_config(input) {
        Ok(tokens) => tokens,
        Err(ConfigError { reason, span }) => {
            quote_spanned! { span => compile_error!(#reason); }.into()
        }
    }
}
