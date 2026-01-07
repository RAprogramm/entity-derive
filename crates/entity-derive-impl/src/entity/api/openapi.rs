// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! OpenAPI struct generation.
//!
//! Generates a struct that implements `utoipa::OpenApi` for Swagger UI
//! integration.
//!
//! # Generated Code
//!
//! For `User` entity with handlers and commands:
//!
//! ```rust,ignore
//! /// OpenAPI documentation for User entity endpoints.
//! #[derive(utoipa::OpenApi)]
//! #[openapi(
//!     paths(
//!         create_user, get_user, update_user, delete_user, list_user,
//!         register_user, update_email_user
//!     ),
//!     components(schemas(
//!         User, UserResponse, CreateUserRequest, UpdateUserRequest,
//!         RegisterUser, UpdateEmailUser
//!     )),
//!     tags((name = "Users", description = "User management"))
//! )]
//! pub struct UserApi;
//! ```

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::entity::parse::{CommandDef, EntityDef};

/// Generate the OpenAPI struct.
pub fn generate(entity: &EntityDef) -> TokenStream {
    let has_crud = entity.api_config().has_handlers();
    let has_commands = !entity.command_defs().is_empty();

    if !has_crud && !has_commands {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let entity_name = entity.name();
    let api_config = entity.api_config();

    let api_struct = format_ident!("{}Api", entity_name);

    let tag = api_config.tag_or_default(&entity.name_str());
    let tag_description = api_config
        .tag_description
        .clone()
        .or_else(|| entity.doc().map(String::from))
        .unwrap_or_else(|| format!("{} management", entity_name));

    let handler_names = generate_all_handler_names(entity);
    let schema_types = generate_all_schema_types(entity);
    let security_schemes = generate_security_schemes(api_config.security.as_deref());

    let doc = format!(
        "OpenAPI documentation for {} entity endpoints.\n\n\
         # Usage\n\n\
         ```rust,ignore\n\
         use utoipa::OpenApi;\n\
         let openapi = {}::openapi();\n\
         ```",
        entity_name, api_struct
    );

    if security_schemes.is_empty() {
        quote! {
            #[doc = #doc]
            #[derive(utoipa::OpenApi)]
            #[openapi(
                paths(#handler_names),
                components(schemas(#schema_types)),
                tags((name = #tag, description = #tag_description))
            )]
            #vis struct #api_struct;
        }
    } else {
        quote! {
            #[doc = #doc]
            #[derive(utoipa::OpenApi)]
            #[openapi(
                paths(#handler_names),
                components(
                    schemas(#schema_types),
                    #security_schemes
                ),
                tags((name = #tag, description = #tag_description))
            )]
            #vis struct #api_struct;
        }
    }
}

/// Generate all handler names (CRUD + commands).
fn generate_all_handler_names(entity: &EntityDef) -> TokenStream {
    let mut names: Vec<syn::Ident> = Vec::new();

    // CRUD handlers
    if entity.api_config().has_handlers() {
        let snake = entity.name_str().to_case(Case::Snake);
        names.push(format_ident!("create_{}", snake));
        names.push(format_ident!("get_{}", snake));
        names.push(format_ident!("update_{}", snake));
        names.push(format_ident!("delete_{}", snake));
        names.push(format_ident!("list_{}", snake));
    }

    // Command handlers
    for cmd in entity.command_defs() {
        names.push(command_handler_name(entity, cmd));
    }

    quote! { #(#names),* }
}

/// Generate all schema types (entity, DTOs, commands).
fn generate_all_schema_types(entity: &EntityDef) -> TokenStream {
    let entity_name = entity.name();
    let entity_name_str = entity.name_str();
    let mut types: Vec<syn::Ident> = vec![entity_name.clone()];

    // CRUD DTOs
    if entity.api_config().has_handlers() {
        types.push(entity.ident_with("", "Response"));
        types.push(entity.ident_with("Create", "Request"));
        types.push(entity.ident_with("Update", "Request"));
    }

    // Command structs
    for cmd in entity.command_defs() {
        types.push(cmd.struct_name(&entity_name_str));
    }

    quote! { #(#types),* }
}

/// Generate security schemes if configured.
fn generate_security_schemes(security: Option<&str>) -> TokenStream {
    match security {
        Some("bearer") => {
            quote! {
                security_schemes(
                    ("bearer_auth" = (
                        ty = Http,
                        scheme = "bearer",
                        bearer_format = "JWT"
                    ))
                )
            }
        }
        Some("api_key") => {
            quote! {
                security_schemes(
                    ("api_key" = (
                        ty = ApiKey,
                        in = "header",
                        name = "X-API-Key"
                    ))
                )
            }
        }
        _ => TokenStream::new()
    }
}

/// Get command handler function name.
fn command_handler_name(entity: &EntityDef, cmd: &CommandDef) -> syn::Ident {
    let entity_snake = entity.name_str().to_case(Case::Snake);
    let cmd_snake = cmd.name.to_string().to_case(Case::Snake);
    format_ident!("{}_{}", cmd_snake, entity_snake)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_crud_only() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", api(tag = "Users", handlers))]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                pub name: String,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let tokens = generate(&entity);
        let output = tokens.to_string();
        assert!(output.contains("UserApi"));
        assert!(output.contains("create_user"));
        assert!(output.contains("get_user"));
        assert!(output.contains("UserResponse"));
        assert!(output.contains("CreateUserRequest"));
    }

    #[test]
    fn generate_with_security() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", api(tag = "Users", security = "bearer", handlers))]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let tokens = generate(&entity);
        let output = tokens.to_string();
        assert!(output.contains("bearer_auth"));
    }

    #[test]
    fn no_api_when_disabled() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let tokens = generate(&entity);
        assert!(tokens.is_empty());
    }
}
