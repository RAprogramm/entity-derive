// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! OpenAPI struct generation for utoipa 5.x.
//!
//! Generates a struct that implements `utoipa::OpenApi` for Swagger UI
//! integration, with security schemes and paths added via the `Modify` trait.
//!
//! # Generated Code
//!
//! For `User` entity with handlers and security:
//!
//! ```rust,ignore
//! /// OpenAPI modifier for User entity.
//! struct UserApiModifier;
//!
//! impl utoipa::Modify for UserApiModifier {
//!     fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
//!         // Add security schemes
//!         // Add CRUD paths with documentation
//!     }
//! }
//!
//! /// OpenAPI documentation for User entity endpoints.
//! #[derive(utoipa::OpenApi)]
//! #[openapi(
//!     components(schemas(UserResponse, CreateUserRequest, UpdateUserRequest)),
//!     modifiers(&UserApiModifier),
//!     tags((name = "Users", description = "User management"))
//! )]
//! pub struct UserApi;
//! ```

mod info;
mod paths;
mod schemas;
mod security;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

#[cfg(test)]
pub use self::paths::{build_collection_path, build_item_path};
pub use self::{
    info::generate_info_code,
    paths::generate_paths_code,
    schemas::{generate_all_schema_types, generate_common_schemas_code},
    security::generate_security_code
};
use crate::entity::parse::EntityDef;

/// Generate the OpenAPI struct with modifier.
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
    let modifier_struct = format_ident!("{}ApiModifier", entity_name);

    let tag = api_config.tag_or_default(&entity.name_str());
    let tag_description = api_config
        .tag_description
        .clone()
        .or_else(|| entity.doc().map(String::from))
        .unwrap_or_else(|| format!("{} management", entity_name));

    let schema_types = generate_all_schema_types(entity);
    let modifier_impl = generate_modifier(entity, &modifier_struct);

    let doc = format!(
        "OpenAPI documentation for {} entity endpoints.\n\n\
         # Usage\n\n\
         ```rust,ignore\n\
         use utoipa::OpenApi;\n\
         let openapi = {}::openapi();\n\
         ```",
        entity_name, api_struct
    );

    quote! {
        #modifier_impl

        #[doc = #doc]
        #[derive(utoipa::OpenApi)]
        #[openapi(
            components(schemas(#schema_types)),
            modifiers(&#modifier_struct),
            tags((name = #tag, description = #tag_description))
        )]
        #vis struct #api_struct;
    }
}

/// Generate the modifier struct with Modify implementation.
///
/// This adds security schemes, common schemas, CRUD paths, and info to the
/// OpenAPI spec.
fn generate_modifier(entity: &EntityDef, modifier_name: &syn::Ident) -> TokenStream {
    let entity_name = entity.name();
    let api_config = entity.api_config();

    let info_code = generate_info_code(entity);
    let security_code = generate_security_code(api_config.security.as_deref());
    let common_schemas_code = if api_config.has_handlers() {
        generate_common_schemas_code()
    } else {
        TokenStream::new()
    };
    let paths_code = if api_config.has_handlers() {
        generate_paths_code(entity)
    } else {
        TokenStream::new()
    };

    let doc = format!("OpenAPI modifier for {} entity.", entity_name);

    quote! {
        #[doc = #doc]
        struct #modifier_name;

        impl utoipa::Modify for #modifier_name {
            fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
                use utoipa::openapi::*;

                #info_code
                #security_code
                #common_schemas_code
                #paths_code
            }
        }
    }
}

#[cfg(test)]
mod tests;
