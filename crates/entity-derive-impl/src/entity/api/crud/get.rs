// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! GET handler generation for retrieving entities by ID.
//!
//! This module generates the `get_{entity}` HTTP handler function
//! that fetches a single entity by its primary key.
//!
//! # Generated Handler
//!
//! For an entity `User`, generates:
//!
//! ```rust,ignore
//! /// Get User by ID.
//! ///
//! /// # Path Parameters
//! ///
//! /// - `id` - The unique identifier of the User
//! ///
//! /// # Responses
//! ///
//! /// - `200 OK` - User found
//! /// - `401 Unauthorized` - Authentication required (if security enabled)
//! /// - `404 Not Found` - User with given ID does not exist
//! /// - `500 Internal Server Error` - Database or server error
//! #[utoipa::path(
//!     get,
//!     path = "/users/{id}",
//!     tag = "Users",
//!     params(("id" = Uuid, Path, description = "User ID")),
//!     responses(
//!         (status = 200, description = "User found", body = UserResponse),
//!         (status = 401, description = "Authentication required"),
//!         (status = 404, description = "User not found"),
//!         (status = 500, description = "Internal server error")
//!     ),
//!     security(("bearerAuth" = []))
//! )]
//! pub async fn get_user<R>(
//!     State(repo): State<Arc<R>>,
//!     Path(id): Path<Uuid>,
//! ) -> AppResult<Json<UserResponse>>
//! where
//!     R: UserRepository + 'static,
//! {
//!     let entity = repo
//!         .find_by_id(id)
//!         .await
//!         .map_err(|e| AppError::internal(e.to_string()))?
//!         .ok_or_else(|| AppError::not_found("User not found"))?;
//!     Ok(Json(UserResponse::from(entity)))
//! }
//! ```
//!
//! # Request Flow
//!
//! ```text
//! Client                Handler              Repository           Database
//!   │                      │                      │                   │
//!   │ GET /users/{id}      │                      │                   │
//!   │─────────────────────>│                      │                   │
//!   │                      │                      │                   │
//!   │                      │ repo.find_by_id(id)  │                   │
//!   │                      │─────────────────────>│                   │
//!   │                      │                      │                   │
//!   │                      │                      │ SELECT * WHERE id │
//!   │                      │                      │──────────────────>│
//!   │                      │                      │                   │
//!   │                      │                      │<──────────────────│
//!   │                      │                      │ Option<UserRow>   │
//!   │                      │<─────────────────────│                   │
//!   │                      │   Option<User>       │                   │
//!   │                      │                      │                   │
//!   │<─────────────────────│                      │                   │
//!   │ 200 OK / 404         │                      │                   │
//!   │ UserResponse         │                      │                   │
//! ```
//!
//! # Error Handling
//!
//! The handler distinguishes between two error cases:
//!
//! | Case | Response | Description |
//! |------|----------|-------------|
//! | Database error | 500 | Query failed (connection, timeout, etc.) |
//! | Not found | 404 | Entity with given ID doesn't exist |
//!
//! The `Option<Entity>` from the repository is converted:
//! - `Some(entity)` → 200 OK with response body
//! - `None` → 404 Not Found error

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::helpers::{build_deprecated_attr, build_item_path, build_security_attr};
use crate::entity::parse::EntityDef;

/// Generates the GET handler for retrieving a single entity by ID.
///
/// Creates a handler function that:
///
/// 1. Extracts entity ID from URL path parameter
/// 2. Calls `repository.find_by_id(id)` to fetch the entity
/// 3. Returns `200 OK` with entity data or `404 Not Found`
///
/// # Arguments
///
/// * `entity` - The parsed entity definition
///
/// # Returns
///
/// A `TokenStream` containing the complete handler function with:
/// - Doc comments describing the endpoint
/// - `#[utoipa::path]` attribute for OpenAPI documentation
/// - The async handler function implementation
///
/// # Generated Components
///
/// | Component | Description |
/// |-----------|-------------|
/// | Function name | `get_{entity_snake}` (e.g., `get_user`) |
/// | Path | Item path with `{id}` (e.g., `/users/{id}`) |
/// | Method | GET |
/// | Path parameter | `id` with entity's ID type |
/// | Response body | `{Entity}Response` |
/// | Status codes | 200, 401 (if auth), 404, 500 |
///
/// # Path Parameter
///
/// The `{id}` path parameter type is derived from the entity's `#[id]` field:
///
/// - `Uuid` for UUID primary keys
/// - `i32`/`i64` for integer primary keys
/// - Custom types are also supported
///
/// # Security Handling
///
/// When security is configured:
/// - Adds `401 Unauthorized` to response list
/// - Includes security requirement in OpenAPI spec
pub fn generate_get_handler(entity: &EntityDef) -> TokenStream {
    let vis = &entity.vis;
    let entity_name = entity.name();
    let entity_name_str = entity.name_str();
    let api_config = entity.api_config();
    let id_field = entity.id_field();
    let id_type = &id_field.ty;
    let repo_trait = entity.ident_with("", "Repository");
    let has_security = api_config.security.is_some();

    let handler_name = format_ident!("get_{}", entity_name_str.to_case(Case::Snake));
    let response_dto = entity.ident_with("", "Response");

    let path = build_item_path(entity);
    let tag = api_config.tag_or_default(&entity_name_str);

    let security_attr = build_security_attr(entity);
    let deprecated_attr = build_deprecated_attr(entity);

    let id_desc = format!("{} unique identifier", entity_name);
    let success_desc = format!("{} found", entity_name);
    let not_found_desc = format!("{} not found", entity_name);

    let utoipa_attr = if has_security {
        quote! {
            #[utoipa::path(
                get,
                path = #path,
                tag = #tag,
                params(("id" = #id_type, Path, description = #id_desc)),
                responses(
                    (status = 200, description = #success_desc, body = #response_dto),
                    (status = 401, description = "Authentication required"),
                    (status = 404, description = #not_found_desc),
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
                params(("id" = #id_type, Path, description = #id_desc)),
                responses(
                    (status = 200, description = #success_desc, body = #response_dto),
                    (status = 404, description = #not_found_desc),
                    (status = 500, description = "Internal server error")
                )
                #deprecated_attr
            )]
        }
    };

    let doc = format!(
        "Get {} by ID.\n\n\
         # Responses\n\n\
         - `200 OK` - {} found\n\
         {}\
         - `404 Not Found` - {} not found\n\
         - `500 Internal Server Error` - Database or server error",
        entity_name,
        entity_name,
        if has_security {
            "- `401 Unauthorized` - Authentication required\n"
        } else {
            ""
        },
        entity_name
    );

    let not_found_msg = format!("{} not found", entity_name);

    quote! {
        #[doc = #doc]
        #utoipa_attr
        #vis async fn #handler_name<R>(
            axum::extract::State(repo): axum::extract::State<std::sync::Arc<R>>,
            axum::extract::Path(id): axum::extract::Path<#id_type>,
        ) -> masterror::AppResult<axum::response::Json<#response_dto>>
        where
            R: #repo_trait + 'static,
        {
            let entity = repo
                .find_by_id(id)
                .await
                .map_err(|e| masterror::AppError::internal(e.to_string()))?
                .ok_or_else(|| masterror::AppError::not_found(#not_found_msg))?;
            Ok(axum::response::Json(#response_dto::from(entity)))
        }
    }
}
