use proc_macro::TokenStream;
use syn::{Attribute, Data, DeriveInput, Field, Fields, FieldsUnnamed, Item};
use quote::{quote, ToTokens};



#[proc_macro_derive(Config, attributes(input))]
pub fn config(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();
    println!("input: {input:#?}");
    if !input.generics.params.is_empty() {
        return quote! {
            compile_error!("generics are not supported");
        }.into();
    }
    let strct = match &input.data {
        Data::Union(_) => {
            return quote! {
                compile_error!("unions are not supported");
            }.into();
        }
        Data::Enum(_) => {
            return quote! {
                compile_error!("enums are not supported (yet)");
            }.into();
        }
        Data::Struct(s) => s,
    };
    let fields = match strct.fields {
        Fields::Named(ref fields) => fields,
        Fields::Unnamed(_) | Fields::Unit => {
            return quote! {
                compile_error!("unit structs are not supported");
            }.into();
        },
    };
    println!("fields: {fields:#?}");
    TokenStream::new()
}