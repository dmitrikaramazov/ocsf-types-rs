use heck::{ToPascalCase, ToSnakeCase};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Deserialize, Debug)]
struct OcsfSchema {
    #[serde(default)]
    classes: BTreeMap<String, ClassDef>,
    #[serde(default)]
    objects: BTreeMap<String, ClassDef>,
}

#[derive(Deserialize, Debug)]
struct ClassDef {
    #[serde(default)]
    caption: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    attributes: BTreeMap<String, AttributeDef>,
    #[serde(default)]
    description: String,
    #[serde(default)]
    uid: Option<i64>,
    #[serde(default)]
    category: String,
    #[serde(default, rename = "profiles")]
    _profiles: Option<Vec<String>>,
    #[serde(default, rename = "associations")]
    _associations: Option<BTreeMap<String, Vec<String>>>,
    #[serde(default)]
    constraints: Option<BTreeMap<String, Vec<String>>>,
    #[serde(rename = "@deprecated")]
    #[serde(default)]
    deprecated: Option<DeprecatedInfo>,
}

#[derive(Deserialize, Debug)]
struct DeprecatedInfo {
    message: String,
    since: String,
}

#[derive(Deserialize, Debug)]
struct AttributeDef {
    #[serde(rename = "type")]
    type_name: String,
    #[serde(default)]
    caption: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    requirement: String,
    #[serde(default)]
    is_array: bool,
}

fn main() {
    let content =
        fs::read_to_string("src/resolved.json").expect("Failed to find resolved.json file");
    let schema: OcsfSchema = serde_json::from_str(&content)
        .expect("Failed to parse resolved.json into OcsfSchema types");

    let mut generated_code = Vec::new();

    for (name, def) in &schema.classes {
        generated_code.push(generate_struct(name, def));
    }

    for (name, def) in &schema.objects {
        generated_code.push(generate_struct(name, def));
    }

    let final_code = quote! {
        #![allow(deprecated)]
        #![allow(unused_imports)]
        use serde::{Serialize, Deserialize};
        use serde_json::Value;
        #(#generated_code)*
    };
    let dest_path = Path::new("src/ocsf_generated.rs");
    //fs::write(&dest_path, final_code.to_string()).expect("failed to write genreated code");
    fs::write(&dest_path, final_code.to_string()).unwrap();
    let status = std::process::Command::new("rustfmt").arg(&dest_path).status();
    match status {
        Ok(s) if s.success() => println!("src/ocsf_generated.rs formatted successfully"),
        _ => println!("cargo:warning=failed to format src/ocsf_generated.rs"),
    }
}

fn generate_struct(name: &str, def: &ClassDef) -> TokenStream {
    let struct_name = format_ident!("{}", name.to_pascal_case());

    let deprecation_attribute = if let Some(info) = &def.deprecated {
        let msg = format!("{} (Since {})", info.message, info.since);
        quote! {#[deprecated(note = #msg)]}
    } else {
        quote! {}
    };
    let uid_doc = def.uid.map(|u| format!("UID:{}", u)).unwrap_or_default();
    let meta_doc = format!("Category: {} | Name: {}", def.category, def.name);
    let constraint_doc = if let Some(map) = &def.constraints {
        let mut doc = String::from("\n\n**Constraints:**\n");
        for (rule, fields) in map {
            let field_list = fields.join("`,`");
            doc.push_str(&format!("* {}: `[{}]`\n", rule, field_list));
        }
        doc
    } else {
        String::new()
    };

    let doc_str = format!(
        "{}\n\n{}\n\n[{}] {}{}",
        def.caption, def.description, uid_doc, meta_doc, constraint_doc
    );
    let fields = def.attributes.iter().map(|(attr_name, attr)| {
        let safe_name = sanitize_name(attr_name);
        let field_ident = format_ident!("{}", safe_name);

        let raw_type = map_ocsf_type(&attr.type_name);

        let is_primitive = matches!(
            attr.type_name.as_str(),
            // String types
            "string_t" | "string" | "bytestring_t" | "datetime_t" | "email_t" | 
                "file_hash_t" | "file_name_t" | "file_path_t" | "hostname_t" | 
                "ip_t" | "mac_t" | "subnet_t" | "url_t" | "username_t" | "uuid_t" |
                "process_name_t" | "reg_key_path_t" | "resource_uid_t" |
                // Integer types
                "integer_t" | "integer" | "long_t" | "port_t" | "timestamp_t" |
                // Float types
                "float_t" |
                // Boolean types
                "boolean_t" |
                // JSON/Object types
                "json_t" | "object_t" | "object"
        );

        let type_container = if attr.is_array {
            quote! {Vec<#raw_type>}
        } else if !is_primitive {
            quote! { Box<#raw_type> } // may be recursive non primitive
        } else {
            raw_type
        };

        let final_type = if attr.requirement != "required" {
            quote! {Option<#type_container>}
        } else {
            type_container
        };

        let serde_skip = if attr.requirement != "required" {
            quote! {#[serde(skip_serializing_if = "Option::is_none")]}
        } else {
            quote! {}
        };

        let type_token = final_type;

        let attr_doc = format!("{}\n\n{}", attr.caption, attr.description);
        quote! {
            #[doc = #attr_doc]
            #[serde(rename = #attr_name)]
            #serde_skip
            pub #field_ident: #type_token
        }
    });
    quote! {
        #[doc = #doc_str]
        #deprecation_attribute
        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default )]
        #[non_exhaustive]
        pub struct #struct_name {
            #(#fields),*
        }
    }
}

fn map_ocsf_type(t: &str) -> TokenStream {
    match t {
        "string_t" | "string" | "bytestring_t" | "datetime_t" | "email_t" | "file_hash_t"
        | "file_name_t" | "file_path_t" | "hostname_t" | "ip_t" | "mac_t" | "subnet_t"
        | "url_t" | "username_t" | "uuid_t" | "process_name_t" | "reg_key_path_t"
        | "resource_uid_t" => quote! { String },
        "integer_t" | "integer" | "long_t" | "port_t" | "timestamp_t" => quote! { i64 },
        "float_t" => quote! { f64 },
        "boolean_t" => quote! { bool },
        "json_t" | "object_t" | "object" => quote! { serde_json::Value },
        other => {
            let type_name = format_ident!("{}", other.to_pascal_case());
            quote! { #type_name }
        }
    }
}

// Tell rust to treat keywords as raw identifiers
fn sanitize_name(name: &str) -> String {
    let name = name.to_snake_case();
    match name.as_str() {
        "type" | "ref" | "match" | "enum" | "const" | "struct" | "self" | "use" | "extern"
        | "crate" | "super" | "trait" | "impl" | "async" | "await" | "dyn" | "abstract"
        | "yield" | "box" | "final" | "let" | "loop" | "pub" | "return" | "unsafe" | "where"
        | "while" | "for" | "if" | "else" | "false" | "true" | "mod" | "move" | "mut" => {
            format!("r#{}", name)
        }
        _ => name,
    }
}
