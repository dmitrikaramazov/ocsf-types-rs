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
    #[serde(default, rename="enum")]
    r#enum: Option<BTreeMap<String, EnumValueInfo>>,
}

#[derive(Deserialize, Debug)]
struct EnumValueInfo {
    caption: String,
    description: Option<String>,
    source: Option<String>,
    deprecated: Option<DeprecatedInfo>,
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
    let struct_name_str = name.to_pascal_case();
    let mut enum_defs = Vec::new();
    let mut helper_methods = Vec::new();

    // Struct docs
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

    // Struct fields
    let fields = def.attributes.iter().map(|(attr_name, attr)| {
        let safe_name = sanitize_name(attr_name);
        let field_ident = format_ident!("{}", safe_name);
        let raw_type = map_ocsf_type(&attr.type_name);
        let is_string_field = matches!(
            attr.type_name.as_str(),
                "string_t" | "string" | "bytestring_t" | "datetime_t" | "email_t" | 
                "file_hash_t" | "file_name_t" | "file_path_t" | "hostname_t" | 
                "ip_t" | "mac_t" | "subnet_t" | "url_t" | "username_t" | "uuid_t" |
                "process_name_t" | "reg_key_path_t" | "resource_uid_t" 
        );

        // Enum generation
        if let Some(enum_values) = &attr.r#enum {
            let enum_name = format!("{}{}", struct_name_str, attr_name.to_pascal_case());
            let enum_ident = format_ident!("{}",  sanitize_name(&enum_name).to_pascal_case());
            let method_ident = format_ident!("{}_enum", safe_name);

            enum_defs.push(generate_enum(&enum_name, enum_values));

            let mut used_variant_names = std::collections::HashSet::new();

            let mut string_key_offset:i64 = 1000;
            if attr.is_array {
                if is_string_field {
                    // string vec
                    let match_arms = enum_values.iter()
                    .map(|(key,value)|{
                        let mut variant_name = sanitize_name(&value.caption).to_pascal_case();
                        let id = get_enum_key_id(key, &mut string_key_offset);
                        if used_variant_names.contains(&variant_name) {
                            variant_name = format!("{}{}", variant_name, id);
                        }
                        used_variant_names.insert(variant_name.clone());
                        let variant_ident = format_ident!("{}", variant_name);
                        let key_str = key.as_str();
                        quote!{#key_str => Some(#enum_ident::#variant_ident)}
                    });
                    helper_methods.push(
                        quote!{
                            pub fn #method_ident(&self) -> Option<Vec<#enum_ident>> {
                                self.#field_ident.as_ref().map(|vec|  {
                                    vec.iter().filter_map(|v| match v.as_str() {
                                        #(#match_arms),*,
                                        _ => None
                                    }).collect()
                                })
                            }
                    });
                } else {
                    // i64 vec
                    let match_arms = enum_values.iter()
                    .map(|(key,value)|{
                        let mut variant_name = sanitize_name(&value.caption).to_pascal_case();
                        let id = get_enum_key_id(key, &mut string_key_offset);
                        if used_variant_names.contains(&variant_name) {
                            variant_name = format!("{}{}", variant_name, id);
                        }
                        used_variant_names.insert(variant_name.clone());
                        let variant_ident = format_ident!("{}", variant_name);
                        quote!{#id => Some(#enum_ident::#variant_ident)}
                    });
                    helper_methods.push(
                        quote!{
                            pub fn #method_ident(&self) -> Option<Vec<#enum_ident>> {
                                self.#field_ident.as_ref().map(|vec|  {
                                    vec.iter().filter_map(|v| match *v {
                                        #(#match_arms),*,
                                        _ => None
                                    }).collect()
                                })
                            }
                    });
                }
            } else {
                if is_string_field {
                    // string fields
                    let match_arms = enum_values.iter()
                    .map(|(key,value)|
                        {
                        let mut variant_name = sanitize_name(&value.caption).to_pascal_case();
                        let id = get_enum_key_id(key, &mut string_key_offset);
                        if used_variant_names.contains(&variant_name) {
                            variant_name = format!("{}{}", variant_name, id);
                        }
                        used_variant_names.insert(variant_name.clone());
                        let variant_ident = format_ident!("{}", variant_name);
                        let key_str = key.as_str();
                        quote!{#key_str => Some(#enum_ident::#variant_ident)}
                    });
                    helper_methods.push(
                        quote!{
                            pub fn #method_ident(&self) -> Option<#enum_ident> {
                                self.#field_ident.as_deref().and_then(|v| match v {
                                    #(#match_arms),*,
                                    _ => None
                                })
                            }
                    });
                } else {
                    // i64 fields
                    let match_arms = enum_values.iter()
                    .map(|(key,value)|{
                        let mut variant_name = sanitize_name(&value.caption).to_pascal_case();
                        let id = get_enum_key_id(key, &mut string_key_offset);
                        if used_variant_names.contains(&variant_name) {
                            variant_name = format!("{}{}", variant_name, id);
                        }
                        used_variant_names.insert(variant_name.clone());
                        let variant_ident = format_ident!("{}", variant_name);
                        quote!{#id => Some(#enum_ident::#variant_ident)}
                    });
                    helper_methods.push(
                        quote!{
                            pub fn #method_ident(&self) -> Option<#enum_ident> {
                                self.#field_ident.and_then(|v| match v {
                                    #(#match_arms),*,
                                    _ => None
                                })
                            }
                        }
                    );
                }
            }
        }

        //

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
        // No longer handling requirement
        // Always wrap in Option<T>
        // If a user needs to see if a field is required, 
        // they should check the requirement field in the attribute definition
        let final_type = quote! {Option<#type_container>};

        // May result in required fields being skipped if object is improperly created
        let serde_skip = quote! {#[serde(skip_serializing_if = "Option::is_none")]};

        let type_token = final_type;
        let attr_doc = format!("{}\n\n{}\n\n{}", attr.caption, attr.description, attr.requirement);
        quote! {
            #[doc = #attr_doc]
            #[serde(rename = #attr_name)]
            #serde_skip
            pub #field_ident: #type_token
        }
    });

    // Struct definition
    quote! {
        #[doc = #doc_str]
        #deprecation_attribute
        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default )]
        #[serde(default)]
        #[non_exhaustive]
        pub struct #struct_name {
            #(#fields),*
        }

        impl #struct_name {
            #(#helper_methods)*
        }

        #(#enum_defs)*
    }
}


fn get_enum_key_id(key: &str, string_key_offset: &mut i64) -> i64 {
    key.parse::<i64>().unwrap_or_else(|_| {
        let id = *string_key_offset;
        *string_key_offset += 1;
        id
    })
}

fn generate_enum(name: &str, def: &BTreeMap<String, EnumValueInfo>) -> TokenStream {
    let enum_ident = format_ident!("{}", sanitize_name(name).to_pascal_case());
    let mut string_key_offset: i64 = 1000; // Start at 1000 to avoid conflicts with numeric variants

    let variants_with_ids: Vec<_> = def.iter()
        .map(|(key, value)|{
            let id = get_enum_key_id(key, &mut string_key_offset);
            (id,key.clone(),value)
    }).collect();

    let variants = variants_with_ids.iter()
        .map(|(id,_key,value)| {
            let safe_caption = sanitize_name(&value.caption).to_pascal_case();
            let safe_description = value.description.as_deref().unwrap_or("");
            let variant_ident = format_ident!("{}", safe_caption);
            let deprecation_attribute = if let Some(info) = &value.deprecated {
                let msg = format!("{} (Since {})", info.message, info.since);
                quote!{#[deprecated(note=#msg)]}
            } else {
                quote!{}
            };
            let source_doc = value.source.as_ref()
            .map(|s| format!("\n\nSource: {}", s))
            .unwrap_or_default();
            let doc = format!("{}\n\n{}\n\n{}", safe_caption, safe_description, source_doc);
            quote!{
                #[doc = #doc]
                #deprecation_attribute
                #variant_ident = #id
            }
        });

    quote!{
        #[derive(Debug,Clone,Copy,PartialEq,Eq,Serialize,Deserialize)]
        #[repr(i64)]
        pub enum #enum_ident {
            #(#variants),*
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