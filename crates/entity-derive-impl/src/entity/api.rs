// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! HTTP API generation with OpenAPI documentation.
//!
//! This module generates axum handlers with utoipa annotations for entities
//! with `#[entity(api(...))]` enabled.
//!
//! # Architecture
//!
//! ```text
//! api/
//! ├── mod.rs         — Orchestrator (this file)
//! ├── crud.rs        — CRUD handler functions (create, get, update, delete, list)
//! ├── handlers.rs    — Command handler functions with #[utoipa::path]
//! ├── router.rs      — Router factory function
//! └── openapi.rs     — OpenApi struct for Swagger UI
//! ```
//!
//! # Generated Code
//!
//! For an entity like:
//!
//! ```rust,ignore
//! #[derive(Entity)]
//! #[entity(
//!     table = "users",
//!     commands,
//!     api(
//!         tag = "Users",
//!         path_prefix = "/api/v1",
//!         security = "bearer",
//!         public = [Register]
//!     )
//! )]
//! #[command(Register)]
//! #[command(UpdateEmail: email)]
//! pub struct User { ... }
//! ```
//!
//! The macro generates:
//!
//! | Type | Purpose |
//! |------|---------|
//! | `register_user` | Handler for POST /api/v1/users/register |
//! | `update_email_user` | Handler for PUT /api/v1/users/{id}/update-email |
//! | `user_router` | Router factory function |
//! | `UserApi` | OpenApi struct for Swagger UI |
//!
//! # Usage
//!
//! ```rust,ignore
//! // In your main.rs or router setup:
//! let app = Router::new()
//!     .merge(user_router::<MyHandler>())
//!     .layer(Extension(handler));
//!
//! // For OpenAPI:
//! let openapi = UserApi::openapi();
//! ```

mod crud;
mod handlers;
mod openapi;
mod router;

use proc_macro2::TokenStream;
use quote::quote;

use super::parse::EntityDef;

/// Main entry point for API code generation.
///
/// Returns empty `TokenStream` if `api(...)` is not configured.
/// Generates CRUD handlers if `handlers` is enabled, and command handlers
/// if commands are defined.
pub fn generate(entity: &EntityDef) -> TokenStream {
    if !entity.has_api() {
        return TokenStream::new();
    }

    let has_crud = entity.api_config().has_handlers();
    let has_commands = entity.has_commands() && !entity.command_defs().is_empty();

    // Need at least one type of handler to generate API
    if !has_crud && !has_commands {
        return TokenStream::new();
    }

    let crud_handlers = crud::generate(entity);
    let command_handlers = handlers::generate(entity);
    let router = router::generate(entity);
    let openapi = openapi::generate(entity);

    quote! {
        #crud_handlers
        #command_handlers
        #router
        #openapi
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::parse::EntityDef;

    #[test]
    fn generate_no_api_returns_empty() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let output = generate(&entity);
        assert!(output.is_empty());
    }

    #[test]
    fn generate_api_no_handlers_no_commands() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", api(tag = "Users"))]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let output = generate(&entity);
        assert!(output.is_empty());
    }

    #[test]
    fn generate_with_handlers() {
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
        let output = generate(&entity);
        assert!(!output.is_empty());
        let output_str = output.to_string();
        assert!(output_str.contains("user_router"));
        assert!(output_str.contains("UserApi"));
    }

    #[test]
    fn generate_with_commands() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", commands, api(tag = "Users"))]
            #[command(Register)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                pub name: String,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let output = generate(&entity);
        assert!(!output.is_empty());
    }

    #[test]
    fn generate_with_both_handlers_and_commands() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "users", commands, api(tag = "Users", handlers))]
            #[command(Activate)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                pub name: String,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let output = generate(&entity);
        assert!(!output.is_empty());
        let output_str = output.to_string();
        assert!(output_str.contains("user_router"));
        assert!(output_str.contains("UserApi"));
    }
}
