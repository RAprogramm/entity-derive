// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Helper functions for CRUD handler generation.
//!
//! This module provides utility functions used across all CRUD handler
//! generators. These helpers handle common tasks like URL path construction
//! and security attribute generation.
//!
//! # Overview
//!
//! The helpers in this module are responsible for:
//!
//! - **Path Building**: Constructing RESTful URL paths following conventions
//! - **Security Attributes**: Generating utoipa security annotations
//! - **Deprecation Handling**: Adding deprecated markers to OpenAPI spec
//!
//! # Path Conventions
//!
//! All paths follow REST conventions:
//!
//! | Resource Type | Pattern | Example |
//! |---------------|---------|---------|
//! | Collection | `/{prefix}/{entity}s` | `/api/v1/users` |
//! | Item | `/{prefix}/{entity}s/{id}` | `/api/v1/users/{id}` |
//!
//! Entity names are converted to kebab-case and pluralized.
//!
//! # Security Schemes
//!
//! Supported authentication schemes:
//!
//! | Scheme | OpenAPI Name | Description |
//! |--------|--------------|-------------|
//! | `cookie` | `cookieAuth` | JWT in HTTP-only cookie |
//! | `bearer` | `bearerAuth` | JWT in Authorization header |
//! | `api_key` | `apiKey` | API key in X-API-Key header |
//!
//! # Example
//!
//! ```rust,ignore
//! use crate::entity::api::crud::helpers::*;
//!
//! // For entity "UserProfile" with prefix "/api/v1":
//! let collection = build_collection_path(&entity);
//! // Result: "/api/v1/user-profiles"
//!
//! let item = build_item_path(&entity);
//! // Result: "/api/v1/user-profiles/{id}"
//! ```

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::quote;

use crate::entity::parse::EntityDef;

/// Builds the collection endpoint path for an entity.
///
/// Constructs the URL path for collection-level operations (list, create).
/// The path follows REST conventions: `/{prefix}/{entity}s`.
///
/// # Path Construction
///
/// The path is built from three components:
///
/// 1. **Prefix**: From `api(path_prefix = "...")` attribute
/// 2. **Entity name**: Converted to kebab-case
/// 3. **Plural suffix**: Always adds "s" for collections
///
/// # Arguments
///
/// * `entity` - The parsed entity definition containing API configuration
///
/// # Returns
///
/// A `String` containing the full collection path.
///
/// # Examples
///
/// ```rust,ignore
/// // Entity: User, prefix: /api/v1
/// build_collection_path(&entity) // "/api/v1/users"
///
/// // Entity: UserProfile, prefix: /api
/// build_collection_path(&entity) // "/api/user-profiles"
///
/// // Entity: Order, no prefix
/// build_collection_path(&entity) // "/orders"
/// ```
///
/// # Notes
///
/// - Double slashes (`//`) are automatically normalized to single slashes
/// - Entity names are converted from PascalCase to kebab-case
/// - The plural form is naive (just adds "s"), not grammatically correct
pub fn build_collection_path(entity: &EntityDef) -> String {
    let api_config = entity.api_config();
    let prefix = api_config.full_path_prefix();
    let entity_path = entity.name_str().to_case(Case::Kebab);

    let path = format!("{}/{}s", prefix, entity_path);
    path.replace("//", "/")
}

/// Builds the item endpoint path for an entity.
///
/// Constructs the URL path for item-level operations (get, update, delete).
/// The path follows REST conventions: `/{prefix}/{entity}s/{id}`.
///
/// # Path Construction
///
/// Extends the collection path with an `{id}` path parameter:
///
/// ```text
/// /api/v1/users/{id}
///        ↑       ↑
///   collection  parameter
/// ```
///
/// # Arguments
///
/// * `entity` - The parsed entity definition containing API configuration
///
/// # Returns
///
/// A `String` containing the full item path with `{id}` placeholder.
///
/// # Examples
///
/// ```rust,ignore
/// // Entity: User, prefix: /api/v1
/// build_item_path(&entity) // "/api/v1/users/{id}"
///
/// // Entity: BlogPost, no prefix
/// build_item_path(&entity) // "/blog-posts/{id}"
/// ```
///
/// # OpenAPI Integration
///
/// The `{id}` placeholder is recognized by utoipa and generates:
///
/// ```yaml
/// parameters:
///   - name: id
///     in: path
///     required: true
/// ```
pub fn build_item_path(entity: &EntityDef) -> String {
    let collection = build_collection_path(entity);
    format!("{}/{{id}}", collection)
}

/// Generates the utoipa security attribute for a handler.
///
/// Creates the `security((...))` attribute used in `#[utoipa::path]`
/// annotations. The security scheme is determined by the entity's
/// API configuration.
///
/// # Security Scheme Mapping
///
/// | Config Value | OpenAPI Scheme | Authentication Method |
/// |--------------|----------------|----------------------|
/// | `"cookie"` | `cookieAuth` | JWT in HTTP-only cookie |
/// | `"bearer"` | `bearerAuth` | JWT in Authorization header |
/// | `"api_key"` | `apiKey` | Key in X-API-Key header |
/// | Other | `cookieAuth` | Falls back to cookie auth |
///
/// # Arguments
///
/// * `entity` - The parsed entity definition containing security config
///
/// # Returns
///
/// A `TokenStream` containing either:
/// - `security(("schemeName" = []))` if security is configured
/// - Empty `TokenStream` if no security is configured
///
/// # Generated Code Examples
///
/// With `security = "bearer"`:
/// ```rust,ignore
/// #[utoipa::path(
///     // ...
///     security(("bearerAuth" = []))
/// )]
/// ```
///
/// With `security = "cookie"`:
/// ```rust,ignore
/// #[utoipa::path(
///     // ...
///     security(("cookieAuth" = []))
/// )]
/// ```
///
/// Without security:
/// ```rust,ignore
/// #[utoipa::path(
///     // ... (no security attribute)
/// )]
/// ```
///
/// # OpenAPI Spec
///
/// The generated security requirement references a security scheme
/// that must be defined in the OpenAPI components. See
/// [`crate::entity::api::openapi::security`] for scheme definitions.
pub fn build_security_attr(entity: &EntityDef) -> TokenStream {
    let api_config = entity.api_config();

    if let Some(security) = &api_config.security {
        let security_name = match security.as_str() {
            "cookie" => "cookieAuth",
            "bearer" => "bearerAuth",
            "api_key" => "apiKey",
            _ => "cookieAuth"
        };
        quote! { security((#security_name = [])) }
    } else {
        TokenStream::new()
    }
}

/// Generates the deprecated attribute for API endpoints.
///
/// Creates the `deprecated = true` attribute used in `#[utoipa::path]`
/// annotations when the entity's API is marked as deprecated.
///
/// # Deprecation Flow
///
/// 1. Entity marked with `api(deprecated_in = "v2")`
/// 2. This function returns `deprecated = true` attribute
/// 3. OpenAPI spec shows endpoint as deprecated
/// 4. Swagger UI displays strikethrough on deprecated endpoints
///
/// # Arguments
///
/// * `entity` - The parsed entity definition containing deprecation info
///
/// # Returns
///
/// A `TokenStream` containing either:
/// - `, deprecated = true` if API is deprecated
/// - Empty `TokenStream` if API is not deprecated
///
/// # Generated Code Examples
///
/// With `api(deprecated_in = "v2")`:
/// ```rust,ignore
/// #[utoipa::path(
///     get,
///     path = "/users/{id}",
///     // ...
///     , deprecated = true  // ← generated by this function
/// )]
/// ```
///
/// Without deprecation:
/// ```rust,ignore
/// #[utoipa::path(
///     get,
///     path = "/users/{id}",
///     // ... (no deprecated attribute)
/// )]
/// ```
///
/// # Visual Result
///
/// In Swagger UI, deprecated endpoints appear with:
/// - Strikethrough text on the endpoint name
/// - "Deprecated" badge
/// - Grayed out styling
pub fn build_deprecated_attr(entity: &EntityDef) -> TokenStream {
    if entity.api_config().is_deprecated() {
        quote! { , deprecated = true }
    } else {
        TokenStream::new()
    }
}
