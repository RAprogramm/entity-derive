// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! PATCH handler generation for updating existing entities.
//!
//! This module generates the `update_{entity}` HTTP handler function
//! that performs partial updates on existing entities.
//!
//! # Generated Handler
//!
//! For an entity `User`, generates:
//!
//! ```rust,ignore
//! /// Update User by ID.
//! ///
//! /// # Path Parameters
//! ///
//! /// - `id` - The unique identifier of the User to update
//! ///
//! /// # Responses
//! ///
//! /// - `200 OK` - User updated successfully
//! /// - `400 Bad Request` - Invalid request data
//! /// - `401 Unauthorized` - Authentication required (if security enabled)
//! /// - `404 Not Found` - User with given ID does not exist
//! /// - `500 Internal Server Error` - Database or server error
//! #[utoipa::path(
//!     patch,
//!     path = "/users/{id}",
//!     tag = "Users",
//!     params(("id" = Uuid, Path, description = "User ID")),
//!     request_body(content = UpdateUserRequest, description = "..."),
//!     responses(
//!         (status = 200, description = "User updated", body = UserResponse),
//!         (status = 400, description = "Invalid request data"),
//!         (status = 401, description = "Authentication required"),
//!         (status = 404, description = "User not found"),
//!         (status = 500, description = "Internal server error")
//!     ),
//!     security(("bearerAuth" = []))
//! )]
//! pub async fn update_user<R>(
//!     State(repo): State<Arc<R>>,
//!     Path(id): Path<Uuid>,
//!     Json(dto): Json<UpdateUserRequest>,
//! ) -> AppResult<Json<UserResponse>>
//! where
//!     R: UserRepository + 'static,
//! { ... }
//! ```
//!
//! # PATCH vs PUT Semantics
//!
//! This handler uses PATCH (partial update) semantics:
//!
//! | Method | Semantics | UpdateRequest Fields |
//! |--------|-----------|---------------------|
//! | PATCH | Partial update | All fields `Option<T>` |
//! | PUT | Full replacement | All fields required |
//!
//! The `UpdateUserRequest` DTO has optional fields, allowing clients
//! to update only specific fields:
//!
//! ```json
//! // Only update name, leave email unchanged
//! { "name": "New Name" }
//!
//! // Update both fields
//! { "name": "New Name", "email": "new@example.com" }
//! ```
//!
//! # Request Flow
//!
//! ```text
//! Client                Handler              Repository           Database
//!   │                      │                      │                   │
//!   │ PATCH /users/{id}    │                      │                   │
//!   │ UpdateUserRequest    │                      │                   │
//!   │─────────────────────>│                      │                   │
//!   │                      │                      │                   │
//!   │                      │ repo.update(id, dto) │                   │
//!   │                      │─────────────────────>│                   │
//!   │                      │                      │                   │
//!   │                      │                      │ UPDATE users SET  │
//!   │                      │                      │──────────────────>│
//!   │                      │                      │                   │
//!   │                      │                      │<──────────────────│
//!   │                      │                      │ UserRow           │
//!   │                      │<─────────────────────│                   │
//!   │                      │   User               │                   │
//!   │                      │                      │                   │
//!   │<─────────────────────│                      │                   │
//!   │ 200 OK               │                      │                   │
//!   │ UserResponse         │                      │                   │
//! ```
//!
//! # Error Handling
//!
//! | Case | Response | Description |
//! |------|----------|-------------|
//! | Invalid JSON | 400 | Request body parsing failed |
//! | Validation error | 400 | Field constraints violated |
//! | Not authenticated | 401 | Missing or invalid token |
//! | Database error | 500 | Query execution failed |

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::helpers::{build_deprecated_attr, build_item_path, build_security_attr};
use crate::entity::parse::EntityDef;

/// Generates the PATCH handler for updating existing entities.
///
/// Creates a handler function that:
///
/// 1. Extracts entity ID from URL path parameter
/// 2. Accepts `UpdateEntityRequest` in JSON body
/// 3. Calls `repository.update(id, dto)` to persist changes
/// 4. Returns `200 OK` with updated entity
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
/// | Function name | `update_{entity_snake}` (e.g., `update_user`) |
/// | Path | Item path with `{id}` (e.g., `/users/{id}`) |
/// | Method | PATCH |
/// | Path parameter | `id` with entity's ID type |
/// | Request body | `Update{Entity}Request` |
/// | Response body | `{Entity}Response` |
/// | Status codes | 200, 400, 401 (if auth), 500 |
///
/// # UpdateRequest Generation
///
/// The `UpdateEntityRequest` is generated separately with all fields
/// marked with `#[field(update)]` as `Option<T>`:
///
/// ```rust,ignore
/// #[derive(Debug, Deserialize, ToSchema)]
/// pub struct UpdateUserRequest {
///     pub name: Option<String>,   // from #[field(update)]
///     pub email: Option<String>,  // from #[field(update)]
/// }
/// ```
pub fn generate_update_handler(entity: &EntityDef) -> TokenStream {
    let vis = &entity.vis;
    let entity_name = entity.name();
    let entity_name_str = entity.name_str();
    let api_config = entity.api_config();
    let id_field = entity.id_field();
    let id_type = &id_field.ty;
    let repo_trait = entity.ident_with("", "Repository");
    let has_security = api_config.security.is_some();

    let handler_name = format_ident!("update_{}", entity_name_str.to_case(Case::Snake));
    let update_dto = entity.ident_with("Update", "Request");
    let response_dto = entity.ident_with("", "Response");

    let path = build_item_path(entity);
    let tag = api_config.tag_or_default(&entity_name_str);

    let security_attr = build_security_attr(entity);
    let deprecated_attr = build_deprecated_attr(entity);

    let id_desc = format!("{} unique identifier", entity_name);
    let request_body_desc = format!("Fields to update for {}", entity_name);
    let success_desc = format!("{} updated successfully", entity_name);
    let not_found_desc = format!("{} not found", entity_name);

    let utoipa_attr = if has_security {
        quote! {
            #[utoipa::path(
                patch,
                path = #path,
                tag = #tag,
                params(("id" = #id_type, Path, description = #id_desc)),
                request_body(content = #update_dto, description = #request_body_desc),
                responses(
                    (status = 200, description = #success_desc, body = #response_dto),
                    (status = 400, description = "Invalid request data"),
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
                patch,
                path = #path,
                tag = #tag,
                params(("id" = #id_type, Path, description = #id_desc)),
                request_body(content = #update_dto, description = #request_body_desc),
                responses(
                    (status = 200, description = #success_desc, body = #response_dto),
                    (status = 400, description = "Invalid request data"),
                    (status = 404, description = #not_found_desc),
                    (status = 500, description = "Internal server error")
                )
                #deprecated_attr
            )]
        }
    };

    let doc = format!(
        "Update {} by ID.\n\n\
         # Responses\n\n\
         - `200 OK` - {} updated successfully\n\
         - `400 Bad Request` - Invalid request data\n\
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

    quote! {
        #[doc = #doc]
        #utoipa_attr
        #vis async fn #handler_name<R>(
            axum::extract::State(repo): axum::extract::State<std::sync::Arc<R>>,
            axum::extract::Path(id): axum::extract::Path<#id_type>,
            axum::extract::Json(dto): axum::extract::Json<#update_dto>,
        ) -> masterror::AppResult<axum::response::Json<#response_dto>>
        where
            R: #repo_trait + 'static,
        {
            let entity = repo
                .update(id, dto)
                .await
                .map_err(|e| masterror::AppError::internal(e.to_string()))?;
            Ok(axum::response::Json(#response_dto::from(entity)))
        }
    }
}
