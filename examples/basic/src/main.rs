// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Basic CRUD Example with Generated Handlers
//!
//! Demonstrates full CRUD operations using:
//! - entity-derive for code generation including HTTP handlers
//! - Axum for HTTP routing
//! - sqlx for PostgreSQL access
//! - utoipa for OpenAPI docs
//!
//! Key features:
//! - `api(tag = "Users", handlers)` generates CRUD handlers automatically
//! - `user_router()` provides ready-to-use axum Router
//! - `UserApi` provides OpenAPI documentation

use std::sync::Arc;

use axum::Router;
use chrono::{DateTime, Utc};
use entity_derive::Entity;
use sqlx::PgPool;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

// ============================================================================
// Entity Definition with Generated API
// ============================================================================

/// User entity with full CRUD support and cookie authentication.
///
/// The `api(tag = "Users", security = "cookie", handlers)` attribute generates:
/// - `create_user()` - POST /users (requires auth)
/// - `get_user()` - GET /users/{id} (requires auth)
/// - `update_user()` - PATCH /users/{id} (requires auth)
/// - `delete_user()` - DELETE /users/{id} (requires auth)
/// - `list_user()` - GET /users (requires auth)
/// - `user_router()` - axum Router with all routes
/// - `UserApi` - OpenAPI documentation with security scheme
#[derive(Debug, Clone, Entity)]
#[entity(
    table = "users",
    schema = "public",
    api(
        tag = "Users",
        security = "cookie",
        handlers,
        title = "User Service API",
        description = "RESTful API for user management with cookie-based authentication",
        api_version = "1.0.0",
        license = "MIT",
        contact_name = "API Support",
        contact_email = "support@example.com"
    )
)]
pub struct User {
    /// Unique identifier (UUID v7).
    #[id]
    pub id: Uuid,

    /// User's display name.
    #[field(create, update, response)]
    pub name: String,

    /// User's email address.
    #[field(create, update, response)]
    pub email: String,

    /// Hashed password (never exposed in API).
    #[field(create, skip)]
    pub password_hash: String,

    /// Account creation timestamp.
    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,

    /// Last update timestamp.
    #[field(response)]
    #[auto]
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// Router Setup
// ============================================================================

/// Create the application router.
///
/// Uses the generated `user_router()` function which includes:
/// - POST /users - create user
/// - GET /users - list users
/// - GET /users/{id} - get user
/// - PATCH /users/{id} - update user
/// - DELETE /users/{id} - delete user
fn app(pool: Arc<PgPool>) -> Router {
    Router::new()
        // Use the generated router for CRUD operations
        .merge(user_router::<PgPool>())
        // Add Swagger UI using generated OpenAPI struct
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", UserApi::openapi()))
        .with_state(pool)
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("example_basic=debug,tower_http=debug")
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/entity_example".into());

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let app = app(Arc::new(pool));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("Listening on http://localhost:3000");
    tracing::info!("Swagger UI: http://localhost:3000/swagger-ui");
    tracing::info!("");
    tracing::info!("Try these endpoints:");
    tracing::info!("  POST   /users        - Create a user");
    tracing::info!("  GET    /users        - List users");
    tracing::info!("  GET    /users/{{id}}   - Get user by ID");
    tracing::info!("  PATCH  /users/{{id}}   - Update user");
    tracing::info!("  DELETE /users/{{id}}   - Delete user");

    axum::serve(listener, app).await.unwrap();
}
