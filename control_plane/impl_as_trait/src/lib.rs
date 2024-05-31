use proc_macro::TokenStream;
use quote::quote;
use syn::{ImplItem, Signature};


#[proc_macro_attribute]
pub fn derive_trait(attrs: TokenStream, input: TokenStream) -> TokenStream {
    let attrs: proc_macro2::TokenStream = attrs.into();
    let input: syn::Item = syn::parse(input).unwrap();

    match input {
        syn::Item::Impl(ref i) => {
            let mut func_sigs: Vec<&Signature> = vec![];
            let trait_name = &i.trait_.as_ref().unwrap().1;
            for item in i.items.as_slice() {
                match item {
                    ImplItem::Fn(f) => func_sigs.push(&f.sig),
                    _ => (),
                }
            }
            quote! {
                pub trait #trait_name: #attrs{ 
                    #(#func_sigs;)*
                }
                
                #input
            }
        },
        _ => quote!{
            compile_error!("works only for 'impl Trait for Something'");
        }
    }.into()
}