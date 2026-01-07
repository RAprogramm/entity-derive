// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! POST handler generation for creating new entities.
//!
//! This module generates the `create_{entity}` HTTP handler function
//! that creates new entities via POST requests.
//!
//! # Generated Handler
//!
//! For an entity `User`, generates:
//!
//! ```rust,ignore
//! /// Create a new User.
//! ///
//! /// # Responses
//! ///
//! /// - `201 Created` - User created successfully
//! /// - `400 Bad Request` - Invalid request data
//! /// - `401 Unauthorized` - Authentication required (if security enabled)
//! /// - `500 Internal Server Error` - Database or server error
//! #[utoipa::path(
//!     post,
//!     path = "/users",
//!     tag = "Users",
//!     request_body(content = CreateUserRequest, description = "..."),
//!     responses(
//!         (status = 201, description = "User created", body = UserResponse),
//!         (status = 400, description = "Invalid request data"),
//!         (status = 401, description = "Authentication required"),
//!         (status = 500, description = "Internal server error")
//!     ),
//!     security(("bearerAuth" = []))
//! )]
//! pub async fn create_user<R>(
//!     State(repo): State<Arc<R>>,
//!     Json(dto): Json<CreateUserRequest>,
//! ) -> AppResult<(StatusCode, Json<UserResponse>)>
//! where
//!     R: UserRepository + 'static,
//! {
//!     let entity = repo
//!         .create(dto)
//!         .await
//!         .map_err(|e| AppError::internal(e.to_string()))?;
//!     Ok((StatusCode::CREATED, Json(UserResponse::from(entity))))
//! }
//! ```
//!
//! # Request Flow
//!
//! ```text
//! Client                Handler              Repository           Database
//!   │                      │                      │                   │
//!   │ POST /users          │                      │                   │
//!   │ CreateUserRequest    │                      │                   │
//!   │─────────────────────>│                      │                   │
//!   │                      │                      │                   │
//!   │                      │ repo.create(dto)     │                   │
//!   │                      │─────────────────────>│                   │
//!   │                      │                      │                   │
//!   │                      │                      │ INSERT INTO users │
//!   │                      │                      │──────────────────>│
//!   │                      │                      │                   │
//!   │                      │                      │<──────────────────│
//!   │                      │                      │   UserRow         │
//!   │                      │<─────────────────────│                   │
//!   │                      │      User            │                   │
//!   │                      │                      │                   │
//!   │<─────────────────────│                      │                   │
//!   │ 201 Created          │                      │                   │
//!   │ UserResponse         │                      │                   │
//! ```
//!
//! # DTO Transformation
//!
//! The handler uses three types:
//!
//! | Type | Purpose | Direction |
//! |------|---------|-----------|
//! | `CreateUserRequest` | Validated input from client | Request body |
//! | `User` | Internal domain entity | Repository return |
//! | `UserResponse` | Serialized output to client | Response body |
//!
//! The `UserResponse::from(entity)` conversion is automatically generated
//! by the derive macro based on `#[field(response)]` attributes.

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::helpers::{build_collection_path, build_deprecated_attr, build_security_attr};
use crate::entity::parse::EntityDef;

/// Generates the POST handler for creating new entities.
///
/// Creates a handler function that:
///
/// 1. Accepts `CreateEntityRequest` in JSON body
/// 2. Validates the request data (via serde/validator)
/// 3. Calls `repository.create(dto)` to persist the entity
/// 4. Returns `201 Created` with `EntityResponse` body
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
/// | Function name | `create_{entity_snake}` (e.g., `create_user`) |
/// | Path | Collection path (e.g., `/users`) |
/// | Method | POST |
/// | Request body | `Create{Entity}Request` |
/// | Response body | `{Entity}Response` |
/// | Status code | 201 Created on success |
///
/// # Security Handling
///
/// When security is configured on the entity:
///
/// - Adds `401 Unauthorized` to response list
/// - Includes `security((...))` attribute in utoipa
///
/// Without security:
///
/// - Only 201, 400, 500 responses documented
/// - No security attribute generated
///
/// # Error Handling
///
/// The handler wraps repository errors in `AppError::internal(...)`:
///
/// ```rust,ignore
/// repo.create(dto)
///     .await
///     .map_err(|e| AppError::internal(e.to_string()))?
/// ```
///
/// This ensures all database errors return 500 Internal Server Error
/// with a safe error message (no SQL details leaked).
pub fn generate_create_handler(entity: &EntityDef) -> TokenStream {
    let vis = &entity.vis;
    let entity_name = entity.name();
    let entity_name_str = entity.name_str();
    let api_config = entity.api_config();
    let repo_trait = entity.ident_with("", "Repository");
    let has_security = api_config.security.is_some();

    let handler_name = format_ident!("create_{}", entity_name_str.to_case(Case::Snake));
    let create_dto = entity.ident_with("Create", "Request");
    let response_dto = entity.ident_with("", "Response");

    let path = build_collection_path(entity);
    let tag = api_config.tag_or_default(&entity_name_str);

    let security_attr = build_security_attr(entity);
    let deprecated_attr = build_deprecated_attr(entity);

    let request_body_desc = format!("Data for creating a new {}", entity_name);
    let success_desc = format!("{} created successfully", entity_name);

    let utoipa_attr = if has_security {
        quote! {
            #[utoipa::path(
                post,
                path = #path,
                tag = #tag,
                request_body(content = #create_dto, description = #request_body_desc),
                responses(
                    (status = 201, description = #success_desc, body = #response_dto),
                    (status = 400, description = "Invalid request data"),
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
                post,
                path = #path,
                tag = #tag,
                request_body(content = #create_dto, description = #request_body_desc),
                responses(
                    (status = 201, description = #success_desc, body = #response_dto),
                    (status = 400, description = "Invalid request data"),
                    (status = 500, description = "Internal server error")
                )
                #deprecated_attr
            )]
        }
    };

    let doc = format!(
        "Create a new {}.\n\n\
         # Responses\n\n\
         - `201 Created` - {} created successfully\n\
         - `400 Bad Request` - Invalid request data\n\
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
        #[doc = #doc]
        #utoipa_attr
        #vis async fn #handler_name<R>(
            axum::extract::State(repo): axum::extract::State<std::sync::Arc<R>>,
            axum::extract::Json(dto): axum::extract::Json<#create_dto>,
        ) -> masterror::AppResult<(axum::http::StatusCode, axum::response::Json<#response_dto>)>
        where
            R: #repo_trait + 'static,
        {
            let entity = repo
                .create(dto)
                .await
                .map_err(|e| masterror::AppError::internal(e.to_string()))?;
            Ok((axum::http::StatusCode::CREATED, axum::response::Json(#response_dto::from(entity))))
        }
    }
}
