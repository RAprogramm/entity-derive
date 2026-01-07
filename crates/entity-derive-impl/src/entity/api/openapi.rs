// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! OpenAPI struct generation.
//!
//! Generates a struct that implements `utoipa::OpenApi` for Swagger UI
//! integration.
//!
//! # Generated Code
//!
//! For `User` entity:
//!
//! ```rust,ignore
//! /// OpenAPI documentation for User entity endpoints.
//! #[derive(utoipa::OpenApi)]
//! #[openapi(
//!     paths(register_user, update_email_user),
//!     components(schemas(User, RegisterUser, UpdateEmailUser)),
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
    let commands = entity.command_defs();
    if commands.is_empty() {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let entity_name = entity.name();
    let api_config = entity.api_config();

    // OpenApi struct name: UserApi
    let api_struct = format_ident!("{}Api", entity_name);

    // Tag for OpenAPI grouping
    let tag = api_config.tag_or_default(&entity.name_str());

    // Tag description: explicit > entity doc comment > default
    let tag_description = api_config
        .tag_description
        .clone()
        .or_else(|| entity.doc().map(String::from))
        .unwrap_or_else(|| format!("{} management", entity_name));

    // Handler function names for paths
    let handler_names = generate_handler_names(entity, commands);

    // Schema types (entity + all command structs)
    let schema_types = generate_schema_types(entity, commands);

    // Security schemes
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

/// Generate comma-separated handler function names.
fn generate_handler_names(entity: &EntityDef, commands: &[CommandDef]) -> TokenStream {
    let names: Vec<syn::Ident> = commands
        .iter()
        .map(|cmd| handler_function_name(entity, cmd))
        .collect();

    quote! { #(#names),* }
}

/// Generate comma-separated schema types.
fn generate_schema_types(entity: &EntityDef, commands: &[CommandDef]) -> TokenStream {
    let entity_name = entity.name();
    let entity_name_str = entity.name_str();

    let command_structs: Vec<syn::Ident> = commands
        .iter()
        .map(|cmd| cmd.struct_name(&entity_name_str))
        .collect();

    quote! { #entity_name, #(#command_structs),* }
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

/// Get the handler function name.
fn handler_function_name(entity: &EntityDef, cmd: &CommandDef) -> syn::Ident {
    let entity_snake = entity.name_str().to_case(Case::Snake);
    let cmd_snake = cmd.name.to_string().to_case(Case::Snake);
    format_ident!("{}_{}", cmd_snake, entity_snake)
}
