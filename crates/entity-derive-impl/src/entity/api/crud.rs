// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! CRUD handler generation with utoipa OpenAPI annotations.
//!
//! This module generates production-ready REST API handlers for entities.
//! Each handler includes comprehensive OpenAPI documentation via
//! `#[utoipa::path]` attributes, enabling automatic Swagger UI generation.
//!
//! # Overview
//!
//! When you add `handlers` to your entity's API configuration:
//!
//! ```rust,ignore
//! #[entity(table = "users", api(tag = "Users", handlers))]
//! pub struct User {
//!     #[id]
//!     pub id: Uuid,
//!     #[field(create, update, response)]
//!     pub name: String,
//! }
//! ```
//!
//! This module generates five handler functions:
//!
//! | Handler | HTTP | Path | Description |
//! |---------|------|------|-------------|
//! | `create_user` | POST | `/users` | Create new entity |
//! | `get_user` | GET | `/users/{id}` | Get entity by ID |
//! | `update_user` | PATCH | `/users/{id}` | Update entity fields |
//! | `delete_user` | DELETE | `/users/{id}` | Delete entity |
//! | `list_user` | GET | `/users` | List with pagination |
//!
//! # Selective Handler Generation
//!
//! You can generate only specific handlers:
//!
//! ```rust,ignore
//! // Only generate get and list handlers (read-only API)
//! #[entity(table = "users", api(tag = "Users", handlers(get, list)))]
//! pub struct User { ... }
//! ```
//!
//! Available handler options: `create`, `get`, `update`, `delete`, `list`.
//!
//! # Security Integration
//!
//! Handlers automatically include security annotations when configured:
//!
//! ```rust,ignore
//! #[entity(
//!     table = "users",
//!     api(tag = "Users", security = "bearer", handlers)
//! )]
//! pub struct User { ... }
//! ```
//!
//! This adds `401 Unauthorized` responses and security requirements to
//! the OpenAPI spec.
//!
//! # Generated Code Structure
//!
//! Each handler follows this pattern:
//!
//! ```rust,ignore
//! /// Create a new User.
//! ///
//! /// # Responses
//! ///
//! /// - `201 Created` - User created successfully
//! /// - `400 Bad Request` - Invalid request data
//! /// - `401 Unauthorized` - Authentication required
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
//! { ... }
//! ```
//!
//! # Module Structure
//!
//! ```text
//! crud/
//! ├── mod.rs      — Main generate() function and re-exports
//! ├── helpers.rs  — Path building and attribute helpers
//! ├── create.rs   — POST handler generation
//! ├── get.rs      — GET by ID handler generation
//! ├── update.rs   — PATCH handler generation
//! ├── delete.rs   — DELETE handler generation
//! ├── list.rs     — GET collection handler generation
//! └── tests.rs    — Unit tests
//! ```
//!
//! # Error Handling
//!
//! All handlers use `masterror::AppResult` for consistent error responses:
//!
//! - `AppError::internal(...)` for database/server errors (500)
//! - `AppError::not_found(...)` for missing entities (404)
//! - Validation errors return 400 Bad Request
//!
//! # Integration with Repository
//!
//! Handlers are generic over the repository trait:
//!
//! ```rust,ignore
//! // In your application:
//! let pool = Arc::new(PgPool::connect(url).await?);
//!
//! let app = Router::new()
//!     .route("/users", post(create_user::<PgPool>).get(list_user::<PgPool>))
//!     .route("/users/:id", get(get_user::<PgPool>)
//!         .patch(update_user::<PgPool>)
//!         .delete(delete_user::<PgPool>))
//!     .with_state(pool);
//! ```

mod create;
mod delete;
mod get;
mod helpers;
mod list;
mod update;

use create::generate_create_handler;
use delete::generate_delete_handler;
use get::generate_get_handler;
#[cfg(test)]
pub use helpers::{build_collection_path, build_item_path};
use list::generate_list_handler;
use proc_macro2::TokenStream;
use quote::quote;
use update::generate_update_handler;

use crate::entity::parse::EntityDef;

/// Generates all CRUD handler functions based on entity configuration.
///
/// This is the main entry point for CRUD handler generation. It examines
/// the entity's API configuration and generates handlers for each enabled
/// operation.
///
/// # Generation Process
///
/// 1. **Check Configuration**: Reads `api(handlers(...))` from entity
/// 2. **Filter Handlers**: Only generates handlers that are enabled
/// 3. **Combine Output**: Merges all handler code into single TokenStream
///
/// # Arguments
///
/// * `entity` - The parsed entity definition with API configuration
///
/// # Returns
///
/// A `TokenStream` containing all generated handler functions, or an empty
/// stream if no handlers are enabled.
///
/// # Handler Generation
///
/// | Config | Handler Generated |
/// |--------|-------------------|
/// | `handlers` | All 5 handlers |
/// | `handlers(create, get)` | Only create and get |
/// | `handlers(list)` | Only list |
/// | No `handlers` | Nothing (empty stream) |
///
/// # Example Usage
///
/// ```rust,ignore
/// // In the main derive macro:
/// let crud_handlers = crud::generate(&entity);
///
/// quote! {
///     #crud_handlers
///     // ... other generated code
/// }
/// ```
///
/// # Generated Functions
///
/// For entity `User` with all handlers enabled:
///
/// - `create_user<R>` - POST /users
/// - `get_user<R>` - GET /users/{id}
/// - `update_user<R>` - PATCH /users/{id}
/// - `delete_user<R>` - DELETE /users/{id}
/// - `list_user<R>` - GET /users
///
/// Each function is generic over `R: UserRepository + 'static`.
pub fn generate(entity: &EntityDef) -> TokenStream {
    if !entity.api_config().has_handlers() {
        return TokenStream::new();
    }

    let handlers = entity.api_config().handlers();

    let create = if handlers.create {
        generate_create_handler(entity)
    } else {
        TokenStream::new()
    };
    let get = if handlers.get {
        generate_get_handler(entity)
    } else {
        TokenStream::new()
    };
    let update = if handlers.update {
        generate_update_handler(entity)
    } else {
        TokenStream::new()
    };
    let delete = if handlers.delete {
        generate_delete_handler(entity)
    } else {
        TokenStream::new()
    };
    let list = if handlers.list {
        generate_list_handler(entity)
    } else {
        TokenStream::new()
    };

    quote! {
        #create
        #get
        #update
        #delete
        #list
    }
}

#[cfg(test)]
mod tests;
