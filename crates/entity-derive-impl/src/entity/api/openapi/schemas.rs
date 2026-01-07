// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! OpenAPI schema generation for DTOs and common types.
//!
//! This module generates schema registrations for the OpenAPI components section.
//! Schemas define the structure of request/response bodies and are referenced
//! throughout the API specification.
//!
//! # OpenAPI Components/Schemas
//!
//! The components/schemas section contains reusable schema definitions:
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │                  OpenAPI Components                              │
//! ├──────────────────────────────────────────────────────────────────┤
//! │                                                                  │
//! │  schemas:                                                        │
//! │  ├─► UserResponse          # Entity response DTO                 │
//! │  ├─► CreateUserRequest     # Create request body                 │
//! │  ├─► UpdateUserRequest     # Update request body                 │
//! │  ├─► ErrorResponse         # Standard error format               │
//! │  └─► PaginationQuery       # List endpoint parameters            │
//! │                                                                  │
//! │  securitySchemes:                                                │
//! │  └─► bearerAuth           # (handled by security module)         │
//! │                                                                  │
//! └──────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Schema Types
//!
//! Two categories of schemas are generated:
//!
//! ## Entity DTOs (Derived)
//!
//! These schemas are derived from structs using `utoipa::ToSchema`:
//!
//! | Schema | Source | When Generated |
//! |--------|--------|----------------|
//! | `{Entity}Response` | Entity struct | Always (if handlers) |
//! | `Create{Entity}Request` | Create DTO | If `create` handler enabled |
//! | `Update{Entity}Request` | Update DTO | If `update` handler enabled |
//! | `{Command}` | Command struct | If commands defined |
//!
//! ## Common Schemas (Runtime)
//!
//! These schemas are built programmatically via the `Modify` trait:
//!
//! | Schema | Purpose | Fields |
//! |--------|---------|--------|
//! | `ErrorResponse` | RFC 7807 Problem Details | type, title, status, detail, code |
//! | `PaginationQuery` | List endpoint params | limit, offset |
//!
//! # ErrorResponse Schema
//!
//! Follows RFC 7807 "Problem Details for HTTP APIs":
//!
//! ```json
//! {
//!   "type": "https://errors.example.com/not-found",
//!   "title": "Resource not found",
//!   "status": 404,
//!   "detail": "User with ID '123' was not found",
//!   "code": "NOT_FOUND"
//! }
//! ```
//!
//! # PaginationQuery Schema
//!
//! Defines parameters for offset-based pagination:
//!
//! ```json
//! {
//!   "limit": 100,    // default: 100, min: 1, max: 1000
//!   "offset": 0      // default: 0, min: 0
//! }
//! ```
//!
//! # Selective Registration
//!
//! Schema types are only registered when needed to keep the spec clean:
//!
//! ```text
//! handlers(get, list)     → UserResponse only
//! handlers(create)        → UserResponse, CreateUserRequest
//! handlers(update)        → UserResponse, UpdateUserRequest
//! handlers                → All DTOs
//! ```

use proc_macro2::TokenStream;
use quote::quote;

use crate::entity::parse::EntityDef;

/// Generates the list of schema types to register with OpenAPI.
///
/// This function produces a comma-separated list of type identifiers
/// for the `components(schemas(...))` attribute of `#[openapi]`.
///
/// # Arguments
///
/// * `entity` - The parsed entity definition
///
/// # Returns
///
/// A `TokenStream` containing comma-separated schema type identifiers.
///
/// # Selection Logic
///
/// ```text
/// HandlerConfig
///     │
///     ├─► any() == true ─────────────► {Entity}Response
///     │       │
///     │       ├─► create == true ────► Create{Entity}Request
///     │       │
///     │       └─► update == true ────► Update{Entity}Request
///     │
///     └─► CommandDefs ───────────────► {Command} for each command
/// ```
///
/// # Example Output
///
/// For `User` with all handlers and a `BanUser` command:
///
/// ```rust,ignore
/// UserResponse, CreateUserRequest, UpdateUserRequest, BanUser
/// ```
pub fn generate_all_schema_types(entity: &EntityDef) -> TokenStream {
    let entity_name_str = entity.name_str();
    let mut types: Vec<TokenStream> = Vec::new();

    let handlers = entity.api_config().handlers();
    if handlers.any() {
        let response = entity.ident_with("", "Response");
        types.push(quote! { #response });

        if handlers.create {
            let create = entity.ident_with("Create", "Request");
            types.push(quote! { #create });
        }

        if handlers.update {
            let update = entity.ident_with("Update", "Request");
            types.push(quote! { #update });
        }
    }

    for cmd in entity.command_defs() {
        let cmd_struct = cmd.struct_name(&entity_name_str);
        types.push(quote! { #cmd_struct });
    }

    quote! { #(#types),* }
}

/// Generates common schemas for the OpenAPI specification.
///
/// This function produces code that registers `ErrorResponse` and
/// `PaginationQuery` schemas in the OpenAPI components section. These
/// schemas are built at runtime using utoipa's builder API rather than
/// being derived from structs.
///
/// # Returns
///
/// A `TokenStream` containing code to insert schemas into `openapi.components`.
///
/// # Generated Schemas
///
/// ## ErrorResponse
///
/// Implements RFC 7807 "Problem Details for HTTP APIs" with fields:
///
/// | Field | Type | Required | Description |
/// |-------|------|----------|-------------|
/// | `type` | string | Yes | URI identifying the problem type |
/// | `title` | string | Yes | Short human-readable summary |
/// | `status` | integer | Yes | HTTP status code |
/// | `detail` | string | No | Detailed explanation |
/// | `code` | string | No | Application-specific error code |
///
/// Example JSON:
///
/// ```json
/// {
///   "type": "https://errors.example.com/validation",
///   "title": "Validation Error",
///   "status": 400,
///   "detail": "Email format is invalid",
///   "code": "INVALID_EMAIL"
/// }
/// ```
///
/// ## PaginationQuery
///
/// Defines offset-based pagination parameters:
///
/// | Field | Type | Default | Min | Max | Description |
/// |-------|------|---------|-----|-----|-------------|
/// | `limit` | integer | 100 | 1 | 1000 | Items per page |
/// | `offset` | integer | 0 | 0 | - | Items to skip |
///
/// # Implementation
///
/// Uses utoipa's builder pattern to construct schemas programmatically:
///
/// ```rust,ignore
/// schema::ObjectBuilder::new()
///     .schema_type(schema::Type::Object)
///     .title(Some("ErrorResponse"))
///     .property("type", schema::ObjectBuilder::new()
///         .schema_type(schema::Type::String)
///         .build())
///     .required("type")
///     .build()
/// ```
///
/// # Usage in Generated Code
///
/// Called within the `Modify::modify()` implementation:
///
/// ```rust,ignore
/// if let Some(components) = openapi.components.as_mut() {
///     // Insert ErrorResponse schema
///     // Insert PaginationQuery schema
/// }
/// ```
pub fn generate_common_schemas_code() -> TokenStream {
    quote! {
        if let Some(components) = openapi.components.as_mut() {
            let error_schema = schema::ObjectBuilder::new()
                .schema_type(schema::Type::Object)
                .title(Some("ErrorResponse"))
                .description(Some("Error response following RFC 7807 Problem Details"))
                .property("type", schema::ObjectBuilder::new()
                    .schema_type(schema::Type::String)
                    .description(Some("A URI reference that identifies the problem type"))
                    .example(Some(serde_json::json!("https://errors.example.com/not-found")))
                    .build())
                .required("type")
                .property("title", schema::ObjectBuilder::new()
                    .schema_type(schema::Type::String)
                    .description(Some("A short, human-readable summary of the problem"))
                    .example(Some(serde_json::json!("Resource not found")))
                    .build())
                .required("title")
                .property("status", schema::ObjectBuilder::new()
                    .schema_type(schema::Type::Integer)
                    .description(Some("HTTP status code"))
                    .example(Some(serde_json::json!(404)))
                    .build())
                .required("status")
                .property("detail", schema::ObjectBuilder::new()
                    .schema_type(schema::Type::String)
                    .description(Some("A human-readable explanation specific to this occurrence"))
                    .example(Some(serde_json::json!("User with ID '123' was not found")))
                    .build())
                .property("code", schema::ObjectBuilder::new()
                    .schema_type(schema::Type::String)
                    .description(Some("Application-specific error code"))
                    .example(Some(serde_json::json!("NOT_FOUND")))
                    .build())
                .build();

            components.schemas.insert("ErrorResponse".to_string(), error_schema.into());

            let pagination_schema = schema::ObjectBuilder::new()
                .schema_type(schema::Type::Object)
                .title(Some("PaginationQuery"))
                .description(Some("Query parameters for paginated list endpoints"))
                .property("limit", schema::ObjectBuilder::new()
                    .schema_type(schema::Type::Integer)
                    .description(Some("Maximum number of items to return"))
                    .default(Some(serde_json::json!(100)))
                    .minimum(Some(1.0))
                    .maximum(Some(1000.0))
                    .build())
                .property("offset", schema::ObjectBuilder::new()
                    .schema_type(schema::Type::Integer)
                    .description(Some("Number of items to skip for pagination"))
                    .default(Some(serde_json::json!(0)))
                    .minimum(Some(0.0))
                    .build())
                .build();

            components.schemas.insert("PaginationQuery".to_string(), pagination_schema.into());
        }
    }
}
