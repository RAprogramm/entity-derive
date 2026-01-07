// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Axum CRUD Example with entity-derive
//!
//! Demonstrates full CRUD operations using:
//! - entity-derive for code generation
//! - Axum for HTTP routing
//! - sqlx for PostgreSQL access
//! - utoipa for OpenAPI docs

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post},
};
use chrono::{DateTime, Utc};
use entity_derive::Entity;
use serde::Deserialize;
use sqlx::PgPool;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

// ============================================================================
// Entity Definition
// ============================================================================

/// User entity with full CRUD support.
#[derive(Debug, Clone, Entity)]
#[entity(table = "users", schema = "public")]
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
// Application State
// ============================================================================

#[derive(Clone)]
struct AppState {
    pool: Arc<PgPool>,
}

impl AppState {
    fn new(pool: PgPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    fn repo(&self) -> &PgPool {
        &self.pool
    }
}

// ============================================================================
// Query Parameters
// ============================================================================

#[derive(Debug, Deserialize)]
struct ListParams {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}

fn default_limit() -> i64 {
    20
}

// ============================================================================
// Error Handling
// ============================================================================

enum AppError {
    NotFound,
    Database(sqlx::Error),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Self::NotFound,
            _ => Self::Database(err),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::NotFound => (StatusCode::NOT_FOUND, "Not found").into_response(),
            Self::Database(e) => {
                tracing::error!("Database error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal error").into_response()
            }
        }
    }
}

// ============================================================================
// HTTP Handlers
// ============================================================================

/// Create a new user.
async fn create_user(
    State(state): State<AppState>,
    Json(dto): Json<CreateUserRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = state.repo().create(dto).await?;
    Ok((StatusCode::CREATED, Json(UserResponse::from(user))))
}

/// Get user by ID.
async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = state.repo().find_by_id(id).await?.ok_or(AppError::NotFound)?;
    Ok(Json(UserResponse::from(user)))
}

/// Update user by ID.
async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateUserRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = state.repo().update(id, dto).await?;
    Ok(Json(UserResponse::from(user)))
}

/// Delete user by ID.
async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let deleted = state.repo().delete(id).await?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}

/// List users with pagination.
async fn list_users(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<impl IntoResponse, AppError> {
    let users = state.repo().list(params.limit, params.offset).await?;
    let responses: Vec<UserResponse> = users.into_iter().map(UserResponse::from).collect();
    Ok(Json(responses))
}

// ============================================================================
// OpenAPI Documentation
// ============================================================================

#[derive(OpenApi)]
#[openapi(
    paths(
        create_user,
        get_user,
        update_user,
        delete_user,
        list_users,
    ),
    components(schemas(CreateUserRequest, UpdateUserRequest, UserResponse))
)]
struct ApiDoc;

// ============================================================================
// Router Setup
// ============================================================================

fn app(state: AppState) -> Router {
    Router::new()
        .route("/users", post(create_user).get(list_users))
        .route("/users/{id}", get(get_user).patch(update_user).delete(delete_user))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .with_state(state)
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("axum_crud_example=debug,tower_http=debug")
        .init();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost:5432/entity_example".to_string()
        });

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let state = AppState::new(pool);
    let app = app(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("Listening on http://localhost:3000");
    tracing::info!("Swagger UI: http://localhost:3000/swagger-ui");

    axum::serve(listener, app).await.unwrap();
}
