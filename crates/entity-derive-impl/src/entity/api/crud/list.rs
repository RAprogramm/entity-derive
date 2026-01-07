// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! GET handler generation for listing entities with pagination.
//!
//! This module generates the `list_{entity}` HTTP handler function
//! that returns a paginated list of all entities.
//!
//! # Generated Handler
//!
//! For an entity `User`, generates:
//!
//! ```rust,ignore
//! /// Pagination query parameters.
//! #[derive(Debug, Clone, Deserialize, IntoParams)]
//! pub struct PaginationQuery {
//!     /// Maximum number of items to return.
//!     #[serde(default = "default_limit")]
//!     pub limit: i64,
//!     /// Number of items to skip for pagination.
//!     #[serde(default)]
//!     pub offset: i64,
//! }
//!
//! fn default_limit() -> i64 { 100 }
//!
//! /// List User entities with pagination.
//! ///
//! /// # Query Parameters
//! ///
//! /// - `limit` - Maximum items to return (default: 100)
//! /// - `offset` - Items to skip for pagination
//! ///
//! /// # Responses
//! ///
//! /// - `200 OK` - List of User entities
//! /// - `401 Unauthorized` - Authentication required (if security enabled)
//! /// - `500 Internal Server Error` - Database or server error
//! #[utoipa::path(
//!     get,
//!     path = "/users",
//!     tag = "Users",
//!     params(
//!         ("limit" = Option<i64>, Query, description = "Max items"),
//!         ("offset" = Option<i64>, Query, description = "Items to skip")
//!     ),
//!     responses(
//!         (status = 200, description = "List of users", body = Vec<UserResponse>),
//!         (status = 401, description = "Authentication required"),
//!         (status = 500, description = "Internal server error")
//!     ),
//!     security(("bearerAuth" = []))
//! )]
//! pub async fn list_user<R>(
//!     State(repo): State<Arc<R>>,
//!     Query(pagination): Query<PaginationQuery>,
//! ) -> AppResult<Json<Vec<UserResponse>>>
//! where
//!     R: UserRepository + 'static,
//! { ... }
//! ```
//!
//! # Pagination
//!
//! The handler supports offset-based pagination via query parameters:
//!
//! | Parameter | Type | Default | Description |
//! |-----------|------|---------|-------------|
//! | `limit` | `i64` | `100` | Maximum items per page |
//! | `offset` | `i64` | `0` | Items to skip |
//!
//! ## Usage Examples
//!
//! ```text
//! GET /users              # First 100 users
//! GET /users?limit=10     # First 10 users
//! GET /users?offset=10    # Users 11-110
//! GET /users?limit=10&offset=20  # Users 21-30
//! ```
//!
//! # Request Flow
//!
//! ```text
//! Client                Handler              Repository           Database
//!   │                      │                      │                   │
//!   │ GET /users?limit=10  │                      │                   │
//!   │─────────────────────>│                      │                   │
//!   │                      │                      │                   │
//!   │                      │ repo.list(10, 0)     │                   │
//!   │                      │─────────────────────>│                   │
//!   │                      │                      │                   │
//!   │                      │                      │ SELECT * LIMIT 10 │
//!   │                      │                      │──────────────────>│
//!   │                      │                      │                   │
//!   │                      │                      │<──────────────────│
//!   │                      │                      │   Vec<UserRow>    │
//!   │                      │<─────────────────────│                   │
//!   │                      │   Vec<User>          │                   │
//!   │                      │                      │                   │
//!   │<─────────────────────│                      │                   │
//!   │ 200 OK               │                      │                   │
//!   │ [UserResponse, ...]  │                      │                   │
//! ```
//!
//! # Response Format
//!
//! Returns a JSON array of entity responses:
//!
//! ```json
//! [
//!   { "id": "uuid-1", "name": "Alice", ... },
//!   { "id": "uuid-2", "name": "Bob", ... }
//! ]
//! ```
//!
//! # Performance Considerations
//!
//! - Default limit of 100 prevents unbounded queries
//! - Offset pagination can be slow for large offsets
//! - Consider cursor-based pagination for very large datasets

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::helpers::{build_collection_path, build_deprecated_attr, build_security_attr};
use crate::entity::parse::EntityDef;

/// Generates the GET handler for listing entities with pagination.
///
/// Creates a handler function that:
///
/// 1. Accepts `limit` and `offset` query parameters
/// 2. Calls `repository.list(limit, offset)` to fetch entities
/// 3. Returns `200 OK` with array of entity responses
///
/// # Arguments
///
/// * `entity` - The parsed entity definition
///
/// # Returns
///
/// A `TokenStream` containing:
/// - `PaginationQuery` struct with serde derives
/// - `default_limit()` helper function
/// - The async handler function with OpenAPI annotations
///
/// # Generated Components
///
/// | Component | Description |
/// |-----------|-------------|
/// | Function name | `list_{entity_snake}` (e.g., `list_user`) |
/// | Path | Collection path (e.g., `/users`) |
/// | Method | GET |
/// | Query params | `limit` (default 100), `offset` (default 0) |
/// | Response body | `Vec<{Entity}Response>` |
/// | Status codes | 200, 401 (if auth), 500 |
///
/// # PaginationQuery Struct
///
/// A helper struct is generated alongside the handler:
///
/// ```rust,ignore
/// #[derive(Debug, Clone, Deserialize, IntoParams)]
/// pub struct PaginationQuery {
///     #[serde(default = "default_limit")]
///     pub limit: i64,
///     #[serde(default)]
///     pub offset: i64,
/// }
/// ```
///
/// This struct implements `utoipa::IntoParams` for OpenAPI documentation.
///
/// # Default Limit
///
/// The default limit of 100 items prevents accidental full-table scans.
/// Clients can override this but should implement proper pagination
/// for large datasets.
pub fn generate_list_handler(entity: &EntityDef) -> TokenStream {
    let vis = &entity.vis;
    let entity_name = entity.name();
    let entity_name_str = entity.name_str();
    let api_config = entity.api_config();
    let repo_trait = entity.ident_with("", "Repository");
    let has_security = api_config.security.is_some();

    let handler_name = format_ident!("list_{}", entity_name_str.to_case(Case::Snake));
    let response_dto = entity.ident_with("", "Response");

    let path = build_collection_path(entity);
    let tag = api_config.tag_or_default(&entity_name_str);

    let security_attr = build_security_attr(entity);
    let deprecated_attr = build_deprecated_attr(entity);

    let success_desc = format!("List of {} entities", entity_name);

    let utoipa_attr = if has_security {
        quote! {
            #[utoipa::path(
                get,
                path = #path,
                tag = #tag,
                params(
                    ("limit" = Option<i64>, Query, description = "Maximum number of items to return (default: 100)"),
                    ("offset" = Option<i64>, Query, description = "Number of items to skip for pagination")
                ),
                responses(
                    (status = 200, description = #success_desc, body = Vec<#response_dto>),
                    (status = 401, description = "Authentication required"),
                    (status = 500, description = "Internal server error")
                ),
                #security_attr
                #deprecated_attr
            )]
        }
    } else {
        quote! {
            #[utoipa::path(
                get,
                path = #path,
                tag = #tag,
                params(
                    ("limit" = Option<i64>, Query, description = "Maximum number of items to return (default: 100)"),
                    ("offset" = Option<i64>, Query, description = "Number of items to skip for pagination")
                ),
                responses(
                    (status = 200, description = #success_desc, body = Vec<#response_dto>),
                    (status = 500, description = "Internal server error")
                )
                #deprecated_attr
            )]
        }
    };

    let doc = format!(
        "List {} entities with pagination.\n\n\
         # Query Parameters\n\n\
         - `limit` - Maximum number of items to return (default: 100)\n\
         - `offset` - Number of items to skip for pagination\n\n\
         # Responses\n\n\
         - `200 OK` - List of {} entities\n\
         {}\
         - `500 Internal Server Error` - Database or server error",
        entity_name,
        entity_name,
        if has_security {
            "- `401 Unauthorized` - Authentication required\n"
        } else {
            ""
        }
    );

    quote! {
        /// Pagination query parameters for list endpoints.
        ///
        /// Supports offset-based pagination with configurable page size.
        ///
        /// # Fields
        ///
        /// - `limit` - Maximum items per page (default: 100)
        /// - `offset` - Items to skip (default: 0)
        ///
        /// # Example
        ///
        /// ```text
        /// GET /users?limit=10&offset=20
        /// ```
        #[derive(Debug, Clone, serde::Deserialize, utoipa::IntoParams)]
        #vis struct PaginationQuery {
            /// Maximum number of items to return.
            ///
            /// Defaults to 100 if not specified. Use reasonable limits
            /// to prevent performance issues with large datasets.
            #[serde(default = "default_limit")]
            pub limit: i64,

            /// Number of items to skip for pagination.
            ///
            /// Defaults to 0 (start from beginning). Use with `limit`
            /// to implement page-based navigation.
            #[serde(default)]
            pub offset: i64,
        }

        /// Returns the default pagination limit.
        ///
        /// This value (100) balances usability with performance,
        /// preventing accidental full-table scans while allowing
        /// reasonable batch sizes.
        fn default_limit() -> i64 { 100 }

        #[doc = #doc]
        #utoipa_attr
        #vis async fn #handler_name<R>(
            axum::extract::State(repo): axum::extract::State<std::sync::Arc<R>>,
            axum::extract::Query(pagination): axum::extract::Query<PaginationQuery>,
        ) -> masterror::AppResult<axum::response::Json<Vec<#response_dto>>>
        where
            R: #repo_trait + 'static,
        {
            let entities = repo
                .list(pagination.limit, pagination.offset)
                .await
                .map_err(|e| masterror::AppError::internal(e.to_string()))?;
            let responses: Vec<#response_dto> = entities.into_iter().map(#response_dto::from).collect();
            Ok(axum::response::Json(responses))
        }
    }
}
