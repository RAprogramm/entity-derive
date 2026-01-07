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
//! ├── handlers.rs    — Axum handler functions with #[utoipa::path]
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

mod handlers;
mod openapi;
mod router;

use proc_macro2::TokenStream;
use quote::quote;

use super::parse::EntityDef;

/// Main entry point for API code generation.
///
/// Returns empty `TokenStream` if `api(...)` is not configured
/// or no commands are defined.
pub fn generate(entity: &EntityDef) -> TokenStream {
    if !entity.has_api() {
        return TokenStream::new();
    }

    // API generation requires commands to be enabled
    if !entity.has_commands() || entity.command_defs().is_empty() {
        return TokenStream::new();
    }

    let handlers = handlers::generate(entity);
    let router = router::generate(entity);
    let openapi = openapi::generate(entity);

    quote! {
        #handlers
        #router
        #openapi
    }
}
