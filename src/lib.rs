extern crate proc_macro;
use proc_macro::{Ident, TokenStream};
use proc_macro2::Span;
use quote::quote;
use syn::{Lit, LitStr};

#[proc_macro_derive(ProtoBuf)]
pub fn derive_proto_buf(item: TokenStream) -> proc_macro::TokenStream {
    let proto_buf_struct: syn::DeriveInput = syn::parse(item).unwrap();
    let ident = proto_buf_struct.ident;
    let ts = quote! {
        impl #ident {
            pub fn to_proto_buf(&self) -> String {
                "foo".to_string()
            }
        }
    };
    TokenStream::from(ts)
}

#[proc_macro]
pub fn generate_structs(path: TokenStream) -> TokenStream {
    let path_to_protobuf: Lit = syn::parse(path).unwrap();
    let path = match path_to_protobuf {
        Lit::Str(lit_str) => lit_str.value(),
        _ => panic!("path must be a string literal"),
    };
    let tokens = LitStr::new(&path, Span::call_site());
    quote! { #tokens }.into()
}
