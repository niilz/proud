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
    if !path.ends_with(".proto") {
        panic!("path is not a proto-file")
    }

    let proto_data = std::fs::read_to_string(path).expect("Could not read proto-file");
    let proto_type = parse_proto(&proto_data);
    /*
     * given:
     *
     * message Person {
     *   optional string name = 1;
     * }
     *
     * becomes:
     *
     * struct Person {
     * String name;
     * }
     */
    proto_type
        .parse()
        .expect("Could not parse the proto-data into a rust type")
}

fn parse_proto(proto_data: &str) -> String {
    let rustified = proto_data
        .trim()
        .replace("message", "struct")
        .replace("optional ", "")
        .replace("string", "String");

    rustified
        .lines()
        .map(|line| {
            if line.ends_with(";") {
                println!("{line}");
                let (field, number) = line
                    .split_once('=')
                    .expect("proto-fields must have a number assigned");
                let (typ, ident) = field
                    .trim_end()
                    .rsplit_once(" ")
                    .expect("field must have a type and a ident");
                format!("{ident}: {typ},")
            } else {
                line.trim().to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod test {
    use crate::parse_proto;

    #[test]
    fn parse_single_string() {
        let expected = "struct Person {
  String name;
}";

        let proto_person = "message Person {
  optional string name = 1;
}";

        assert_eq!(parse_proto(proto_person), expected);
    }
}
