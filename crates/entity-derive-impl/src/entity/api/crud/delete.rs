// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! DELETE handler generation for removing entities.
//!
//! This module generates the `delete_{entity}` HTTP handler function
//! that removes entities from the database.
//!
//! # Generated Handler
//!
//! For an entity `User`, generates:
//!
//! ```rust,ignore
//! /// Delete User by ID.
//! ///
//! /// # Path Parameters
//! ///
//! /// - `id` - The unique identifier of the User to delete
//! ///
//! /// # Responses
//! ///
//! /// - `204 No Content` - User deleted successfully
//! /// - `401 Unauthorized` - Authentication required (if security enabled)
//! /// - `404 Not Found` - User with given ID does not exist
//! /// - `500 Internal Server Error` - Database or server error
//! #[utoipa::path(
//!     delete,
//!     path = "/users/{id}",
//!     tag = "Users",
//!     params(("id" = Uuid, Path, description = "User ID")),
//!     responses(
//!         (status = 204, description = "User deleted"),
//!         (status = 401, description = "Authentication required"),
//!         (status = 404, description = "User not found"),
//!         (status = 500, description = "Internal server error")
//!     ),
//!     security(("bearerAuth" = []))
//! )]
//! pub async fn delete_user<R>(
//!     State(repo): State<Arc<R>>,
//!     Path(id): Path<Uuid>,
//! ) -> AppResult<StatusCode>
//! where
//!     R: UserRepository + 'static,
//! {
//!     let deleted = repo
//!         .delete(id)
//!         .await
//!         .map_err(|e| AppError::internal(e.to_string()))?;
//!     if deleted {
//!         Ok(StatusCode::NO_CONTENT)
//!     } else {
//!         Err(AppError::not_found("User not found"))
//!     }
//! }
//! ```
//!
//! # Soft Delete vs Hard Delete
//!
//! The actual deletion behavior depends on the entity's configuration:
//!
//! | Configuration | SQL Generated | Effect |
//! |---------------|---------------|--------|
//! | Default | `DELETE FROM table WHERE id = $1` | Row removed |
//! | `soft_delete` | `UPDATE table SET deleted_at = NOW()` | Row marked |
//!
//! Soft delete is enabled via `#[entity(soft_delete)]` and requires
//! a `deleted_at: Option<DateTime>` field.
//!
//! # Request Flow
//!
//! ```text
//! Client                Handler              Repository           Database
//!   │                      │                      │                   │
//!   │ DELETE /users/{id}   │                      │                   │
//!   │─────────────────────>│                      │                   │
//!   │                      │                      │                   │
//!   │                      │ repo.delete(id)      │                   │
//!   │                      │─────────────────────>│                   │
//!   │                      │                      │                   │
//!   │                      │                      │ DELETE/UPDATE     │
//!   │                      │                      │──────────────────>│
//!   │                      │                      │                   │
//!   │                      │                      │<──────────────────│
//!   │                      │                      │   rows_affected   │
//!   │                      │<─────────────────────│                   │
//!   │                      │      bool            │                   │
//!   │                      │                      │                   │
//!   │<─────────────────────│                      │                   │
//!   │ 204 No Content / 404 │                      │                   │
//! ```
//!
//! # Response Codes
//!
//! | Code | Meaning | Body |
//! |------|---------|------|
//! | 204 | Successfully deleted | Empty |
//! | 401 | Not authenticated | Error JSON |
//! | 404 | Entity not found | Error JSON |
//! | 500 | Database error | Error JSON |
//!
//! Note: 204 No Content has no response body per HTTP spec.

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::helpers::{build_deprecated_attr, build_item_path, build_security_attr};
use crate::entity::parse::EntityDef;

/// Generates the DELETE handler for removing entities.
///
/// Creates a handler function that:
///
/// 1. Extracts entity ID from URL path parameter
/// 2. Calls `repository.delete(id)` to remove the entity
/// 3. Returns `204 No Content` on success or `404 Not Found`
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
/// | Function name | `delete_{entity_snake}` (e.g., `delete_user`) |
/// | Path | Item path with `{id}` (e.g., `/users/{id}`) |
/// | Method | DELETE |
/// | Path parameter | `id` with entity's ID type |
/// | Response | `StatusCode::NO_CONTENT` (204) |
/// | Status codes | 204, 401 (if auth), 404, 500 |
///
/// # Return Type
///
/// Unlike other handlers, DELETE returns only a status code:
///
/// ```rust,ignore
/// -> AppResult<StatusCode>  // Not Json<...>
/// ```
///
/// This follows REST conventions where successful DELETE returns
/// 204 No Content with an empty body.
///
/// # Repository Contract
///
/// The `repository.delete(id)` method returns `Result<bool, Error>`:
/// - `Ok(true)` - Entity was found and deleted
/// - `Ok(false)` - Entity with given ID doesn't exist
/// - `Err(e)` - Database error occurred
pub fn generate_delete_handler(entity: &EntityDef) -> TokenStream {
    let vis = &entity.vis;
    let entity_name = entity.name();
    let entity_name_str = entity.name_str();
    let api_config = entity.api_config();
    let id_field = entity.id_field();
    let id_type = &id_field.ty;
    let repo_trait = entity.ident_with("", "Repository");
    let has_security = api_config.security.is_some();

    let handler_name = format_ident!("delete_{}", entity_name_str.to_case(Case::Snake));

    let path = build_item_path(entity);
    let tag = api_config.tag_or_default(&entity_name_str);

    let security_attr = build_security_attr(entity);
    let deprecated_attr = build_deprecated_attr(entity);

    let id_desc = format!("{} unique identifier", entity_name);
    let success_desc = format!("{} deleted successfully", entity_name);
    let not_found_desc = format!("{} not found", entity_name);

    let utoipa_attr = if has_security {
        quote! {
            #[utoipa::path(
                delete,
                path = #path,
                tag = #tag,
                params(("id" = #id_type, Path, description = #id_desc)),
                responses(
                    (status = 204, description = #success_desc),
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
                delete,
                path = #path,
                tag = #tag,
                params(("id" = #id_type, Path, description = #id_desc)),
                responses(
                    (status = 204, description = #success_desc),
                    (status = 404, description = #not_found_desc),
                    (status = 500, description = "Internal server error")
                )
                #deprecated_attr
            )]
        }
    };

    let doc = format!(
        "Delete {} by ID.\n\n\
         # Responses\n\n\
         - `204 No Content` - {} deleted successfully\n\
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
        ) -> masterror::AppResult<axum::http::StatusCode>
        where
            R: #repo_trait + 'static,
        {
            let deleted = repo
                .delete(id)
                .await
                .map_err(|e| masterror::AppError::internal(e.to_string()))?;
            if deleted {
                Ok(axum::http::StatusCode::NO_CONTENT)
            } else {
                Err(masterror::AppError::not_found(#not_found_msg))
            }
        }
    }
}
