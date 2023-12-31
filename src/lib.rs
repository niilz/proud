extern crate proc_macro;

use std::option;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, GenericArgument, Lit, PathArguments, Type};

#[derive(Debug)]
struct ProtoField {
    name: String,
    typ: String,
    optional: bool,
}

#[proc_macro_derive(ProtoBuf)]
pub fn derive_proto_buf(item: TokenStream) -> proc_macro::TokenStream {
    let proto_buf_struct: syn::DeriveInput = syn::parse(item).unwrap();
    let ident = proto_buf_struct.ident;
    let data = proto_buf_struct.data;
    let fields = match data {
        Data::Struct(s) => s.fields,
        _ => panic!("only structs with fields supported"),
    };
    let proto_type_meta_values: Vec<_> = fields
        .iter()
        .map(|field| {
            (
                field.clone().ident.expect("Field has no ident"),
                field.clone().ty,
            )
        })
        .map(|(field_ident, ty)| {
            let (typ, optional) = extract_ident(&ty, false);
            ProtoField {
                name: field_ident.to_string(),
                typ,
                optional,
            }
        })
        .collect();
    let mut proto_string = format!("message {ident} {{\n");
    for (idx, field) in proto_type_meta_values.iter().enumerate() {
        let proto_type = to_proto_type(&field.typ, field.optional);
        proto_string.push_str(&format!("  {} {} = {};\n", proto_type, field.name, idx + 1));
    }
    proto_string.push('}');
    let ts = quote! {
        impl #ident {
            pub fn to_proto(&self) -> String {
                #proto_string.to_string()
            }

        }
    };
    TokenStream::from(ts)
}

fn extract_ident(ty: &Type, optional: bool) -> (String, bool) {
    let tp = match ty {
        Type::Path(tp) => tp,
        _ => panic!("only typed-path is supported"),
    };
    let last_element = tp.path.segments.last().expect("must have a last element");
    if last_element.ident == "Option" {
        let PathArguments::AngleBracketed(generics) = &last_element.arguments else {
            panic!("Option must have an arg");
        };
        let GenericArgument::Type(inner_type) = &generics.args[0] else {
            panic!("Must have inner type");
        };
        extract_ident(inner_type, true)
    } else {
        (last_element.ident.to_string(), optional)
    }
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
    let trimmed = proto_data.trim();
    let rustified = trimmed;

    let mut lines = rustified
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.trim().is_empty())
        .filter(|line| !line.trim().starts_with("//"))
        .filter(|line| !line.trim().starts_with("/*"))
        .filter(|line| !line.trim().starts_with("*"))
        .filter(|line| !line.trim().starts_with("*/"));
    if lines.next().unwrap() != "syntax = \"proto3\";" {
        panic!("only proto3 is supported");
    }
    lines
        .map(|line| {
            if line.contains("message") {
                // Must be message start
                if !line.ends_with("{") {
                    panic!("messages must declare a block with open {{");
                }
                return line.replace("message", "struct");
            }
            if line == "}" {
                // end of message declaration
                return line.to_string();
            }

            // we have a field (which must end with a semicolon)
            if !line.ends_with(";") {
                panic!("Field declarations must end with semicolon")
            }

            let (field, number) = line
                .split_once('=')
                .expect("proto-fields must have a number assigned");
            let (typ, ident) = field
                .trim_end()
                .rsplit_once(" ")
                .expect("field must have a type and a ident");
            let rust_typ = to_rust_type(typ);
            format!("{}: {},", ident.trim(), rust_typ.trim())
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn to_rust_type(proto_type: &str) -> String {
    let is_optional = proto_type.contains("optional");
    let typ = if is_optional {
        proto_type.replace("optional", "")
    } else {
        proto_type.to_string()
    };
    let trimmed_typ = typ.trim();

    let rust_typ = match trimmed_typ {
        "double" => "f64",
        "float" => "f32",
        "int32" => "i32",
        "int64" => "i64",
        "uint32" => "u32",
        "uint64" => "u64",
        "sint32" => "i32",
        "sint64" => "i64",
        "fixed32" => "u32",
        "fixed64" => "u64",
        "sfixed32" => "i32",
        "sfixed64" => "i64",
        "bool" => "bool",
        "string" => "String",
        "bytes" => "Vec<u8>",
        _ => panic!("unsupported typ: '{typ}'"),
    };

    if is_optional {
        format!("Option<{}>", rust_typ)
    } else {
        rust_typ.to_string()
    }
}

fn to_proto_type(rust_type: &str, is_optional: bool) -> String {
    let proto_type = match rust_type {
        "f64" => "double",
        "f32" => "float",
        "i32" => "int32",
        "i64" => "int64",
        "u32" => "uint32",
        "u64" => "uint64",
        "bool" => "bool",
        "String" => "string",
        "Vec<u8>" => "bytes",
        _ => panic!("unsupported typ: '{rust_type}'"),
    };

    if is_optional {
        format!("optional {}", proto_type)
    } else {
        proto_type.to_string()
    }
}

#[cfg(test)]
mod test {
    use crate::parse_proto;

    const EXPECTED: &str = "struct Person {
name: String,
age: u32,
role: Option<String>,
is_coder: bool,
}";

    const PROTO_PERSON: &str = "syntax = \"proto3\";
message Person {
  string name = 1;
  uint32 age = 2;
  optional string role = 3;
  bool is_coder = 4;
}";
    #[test]
    fn parse_single_string() {
        assert_eq!(parse_proto(PROTO_PERSON), EXPECTED);
    }
}
