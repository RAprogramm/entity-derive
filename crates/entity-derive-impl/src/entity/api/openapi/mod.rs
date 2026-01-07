// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! OpenAPI struct generation for utoipa 5.x.
//!
//! This module generates complete OpenAPI documentation structs that implement
//! `utoipa::OpenApi` for seamless Swagger UI integration. It leverages the
//! `Modify` trait pattern to dynamically add security schemes, paths, and
//! additional components at runtime.
//!
//! # Architecture Overview
//!
//! The generation process produces two interconnected components:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                     OpenAPI Generation                          │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │  EntityDef ─────────────────────────────────────────────────┐  │
//! │       │                                                      │  │
//! │       ▼                                                      │  │
//! │  ┌─────────────────┐     ┌────────────────────────────────┐ │  │
//! │  │  {Entity}Api    │────>│  {Entity}ApiModifier           │ │  │
//! │  │  #[OpenApi]     │     │  impl Modify                   │ │  │
//! │  │  - schemas      │     │  - add_security_scheme()       │ │  │
//! │  │  - modifiers    │     │  - add_path_operation()        │ │  │
//! │  │  - tags         │     │  - insert schemas              │ │  │
//! │  └─────────────────┘     └────────────────────────────────┘ │  │
//! │                                                              │  │
//! │                          Generated at                        │  │
//! │                          compile time                        │  │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Generated Code
//!
//! For a `User` entity with CRUD handlers and bearer security:
//!
//! ```rust,ignore
//! /// OpenAPI modifier for User entity.
//! ///
//! /// Implements utoipa's Modify trait to dynamically configure
//! /// the OpenAPI specification at runtime.
//! struct UserApiModifier;
//!
//! impl utoipa::Modify for UserApiModifier {
//!     fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
//!         use utoipa::openapi::*;
//!
//!         // Configure API metadata
//!         openapi.info.title = "User API".to_string();
//!         openapi.info.version = "1.0.0".to_string();
//!
//!         // Add bearer authentication scheme
//!         if let Some(components) = openapi.components.as_mut() {
//!             components.add_security_scheme("bearerAuth",
//!                 security::SecurityScheme::Http(
//!                     security::HttpBuilder::new()
//!                         .scheme(security::HttpAuthScheme::Bearer)
//!                         .bearer_format("JWT")
//!                         .build()
//!                 )
//!             );
//!
//!             // Add ErrorResponse and PaginationQuery schemas
//!             components.schemas.insert("ErrorResponse".to_string(), ...);
//!             components.schemas.insert("PaginationQuery".to_string(), ...);
//!         }
//!
//!         // Add CRUD path operations
//!         // POST /users - Create user
//!         // GET /users - List users
//!         // GET /users/{id} - Get user by ID
//!         // PATCH /users/{id} - Update user
//!         // DELETE /users/{id} - Delete user
//!     }
//! }
//!
//! /// OpenAPI documentation for User entity endpoints.
//! ///
//! /// # Usage
//! ///
//! /// ```rust,ignore
//! /// use utoipa::OpenApi;
//! /// let openapi = UserApi::openapi();
//! /// ```
//! #[derive(utoipa::OpenApi)]
//! #[openapi(
//!     components(schemas(UserResponse, CreateUserRequest, UpdateUserRequest)),
//!     modifiers(&UserApiModifier),
//!     tags((name = "Users", description = "User management"))
//! )]
//! pub struct UserApi;
//! ```
//!
//! # Module Structure
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`info`] | API metadata (title, version, contact, license) |
//! | [`paths`] | CRUD operation paths with parameters and responses |
//! | [`schemas`] | DTO schemas and common types (ErrorResponse) |
//! | [`security`] | Authentication schemes (bearer, cookie, api_key) |
//!
//! # Swagger UI Integration
//!
//! The generated `{Entity}Api` struct can be served via utoipa-swagger-ui:
//!
//! ```rust,ignore
//! use utoipa::OpenApi;
//! use utoipa_swagger_ui::SwaggerUi;
//!
//! let app = Router::new()
//!     .merge(SwaggerUi::new("/swagger-ui")
//!         .url("/api-docs/openapi.json", UserApi::openapi()));
//! ```
//!
//! # Conditional Generation
//!
//! OpenAPI struct is only generated when either:
//! - CRUD handlers are enabled via `api(handlers)` or `api(handlers(...))`
//! - Custom commands are defined via `#[command(...)]`
//!
//! If neither is present, `generate()` returns an empty `TokenStream`.

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

/// Generates the complete OpenAPI documentation struct with modifier.
///
/// This is the main entry point for OpenAPI generation. It produces:
///
/// 1. A modifier struct implementing `utoipa::Modify`
/// 2. An API struct deriving `utoipa::OpenApi`
///
/// # Arguments
///
/// * `entity` - The parsed entity definition containing API configuration
///
/// # Returns
///
/// A `TokenStream` containing both the modifier and API structs, or an empty
/// stream if no handlers or commands are configured.
///
/// # Generation Flow
///
/// ```text
/// EntityDef
///     │
///     ├─► has_crud? ────────────────────────────────────────┐
///     │       │                                              │
///     ├─► has_commands? ────────────────────────────────────┤
///     │                                                      │
///     │   Neither? ─► Return empty TokenStream              │
///     │                                                      │
///     └───────────────────────────────────────────────────────┘
///                                │
///                                ▼
///                    ┌─────────────────────┐
///                    │ Generate components │
///                    │ - schema_types      │
///                    │ - modifier_impl     │
///                    │ - api_struct        │
///                    └─────────────────────┘
/// ```
///
/// # Generated Components
///
/// | Component | Naming | Purpose |
/// |-----------|--------|---------|
/// | Modifier | `{Entity}ApiModifier` | Runtime OpenAPI customization |
/// | API struct | `{Entity}Api` | Main OpenAPI entry point |
/// | Tag | Configured or entity name | API grouping in Swagger UI |
///
/// # Example Output
///
/// For `User` entity with all handlers enabled:
///
/// ```rust,ignore
/// struct UserApiModifier;
/// impl utoipa::Modify for UserApiModifier { ... }
///
/// #[derive(utoipa::OpenApi)]
/// #[openapi(
///     components(schemas(UserResponse, CreateUserRequest, UpdateUserRequest)),
///     modifiers(&UserApiModifier),
///     tags((name = "Users", description = "User management"))
/// )]
/// pub struct UserApi;
/// ```
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

/// Generates the modifier struct with `utoipa::Modify` implementation.
///
/// The modifier pattern allows runtime customization of the OpenAPI spec
/// that cannot be expressed through derive macros alone. This includes:
///
/// - Dynamic security scheme configuration
/// - Additional schemas not derived from struct definitions
/// - Path operations with complex parameter types
/// - API info metadata (title, version, contact)
///
/// # Arguments
///
/// * `entity` - The parsed entity definition
/// * `modifier_name` - The identifier for the modifier struct
///
/// # Returns
///
/// A `TokenStream` containing:
/// - The modifier struct definition
/// - The `impl utoipa::Modify` block
///
/// # Modifier Responsibilities
///
/// ```text
/// ┌────────────────────────────────────────────────────────────┐
/// │              {Entity}ApiModifier::modify()                 │
/// ├────────────────────────────────────────────────────────────┤
/// │                                                            │
/// │  1. Info Configuration                                     │
/// │     ├─► title, version, description                        │
/// │     ├─► license (name, URL)                                │
/// │     └─► contact (name, email, URL)                         │
/// │                                                            │
/// │  2. Security Schemes                                       │
/// │     ├─► Bearer JWT (Authorization header)                  │
/// │     ├─► Cookie authentication (HTTP-only cookie)           │
/// │     └─► API Key (X-API-Key header)                         │
/// │                                                            │
/// │  3. Common Schemas                                         │
/// │     ├─► ErrorResponse (RFC 7807 Problem Details)           │
/// │     └─► PaginationQuery (limit, offset)                    │
/// │                                                            │
/// │  4. CRUD Paths                                             │
/// │     ├─► POST /entities     (create)                        │
/// │     ├─► GET  /entities     (list)                          │
/// │     ├─► GET  /entities/{id} (get)                          │
/// │     ├─► PATCH /entities/{id} (update)                      │
/// │     └─► DELETE /entities/{id} (delete)                     │
/// │                                                            │
/// └────────────────────────────────────────────────────────────┘
/// ```
///
/// # Generated Structure
///
/// ```rust,ignore
/// /// OpenAPI modifier for User entity.
/// struct UserApiModifier;
///
/// impl utoipa::Modify for UserApiModifier {
///     fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
///         use utoipa::openapi::*;
///
///         // Info configuration code
///         // Security scheme code
///         // Common schemas code
///         // CRUD paths code
///     }
/// }
/// ```
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
