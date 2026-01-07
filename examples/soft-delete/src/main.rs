// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Soft Delete Example with entity-derive
//!
//! Demonstrates soft delete functionality:
//! - `#[entity(soft_delete)]` enables soft delete
//! - `delete()` sets `deleted_at` instead of DELETE
//! - `hard_delete()` permanently removes
//! - `restore()` recovers deleted records
//! - Queries automatically filter deleted records

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
};
use chrono::{DateTime, Utc};
use entity_derive::Entity;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

// ============================================================================
// Entity Definition with Soft Delete
// ============================================================================

/// Document entity with soft delete support.
#[derive(Debug, Clone, Entity)]
#[entity(table = "documents", soft_delete)]
pub struct Document {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub title: String,

    #[field(create, update, response)]
    pub content: String,

    #[field(create, response)]
    pub author: String,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,

    /// Required for soft_delete - stores deletion timestamp.
    #[field(skip)]
    pub deleted_at: Option<DateTime<Utc>>,
}

// Generated methods:
// - delete(id) -> sets deleted_at = NOW()
// - hard_delete(id) -> DELETE FROM
// - restore(id) -> sets deleted_at = NULL
// - find_by_id() -> WHERE deleted_at IS NULL
// - list() -> WHERE deleted_at IS NULL
// - find_by_id_with_deleted() -> includes deleted
// - list_with_deleted() -> includes deleted

// ============================================================================
// Application State
// ============================================================================

#[derive(Clone)]
struct AppState {
    pool: Arc<PgPool>,
}

// ============================================================================
// HTTP Handlers
// ============================================================================

/// Create a new document.
async fn create_document(
    State(state): State<AppState>,
    Json(dto): Json<CreateDocumentRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let doc = state
        .pool
        .create(dto)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(DocumentResponse::from(doc))))
}

/// List active documents (excludes deleted).
async fn list_documents(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let docs = state
        .pool
        .list(100, 0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let responses: Vec<DocumentResponse> = docs.into_iter().map(DocumentResponse::from).collect();
    Ok(Json(responses))
}

/// List ALL documents including deleted.
async fn list_all_documents(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let docs = state
        .pool
        .list_with_deleted(100, 0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let responses: Vec<DocumentResponse> = docs.into_iter().map(DocumentResponse::from).collect();
    Ok(Json(responses))
}

/// Get document by ID.
async fn get_document(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let doc = state
        .pool
        .find_by_id(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(DocumentResponse::from(doc)))
}

/// Update document.
async fn update_document(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateDocumentRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let doc = state
        .pool
        .update(id, dto)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(DocumentResponse::from(doc)))
}

/// Soft delete a document.
async fn delete_document(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let deleted = state
        .pool
        .delete(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if deleted {
        tracing::info!("Document {} soft deleted", id);
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Restore a soft-deleted document.
async fn restore_document(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let restored = state
        .pool
        .restore(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if restored {
        tracing::info!("Document {} restored", id);

        // Fetch and return restored document
        let doc = state
            .pool
            .find_by_id(id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

        Ok(Json(DocumentResponse::from(doc)))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Permanently delete a document.
async fn hard_delete_document(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let deleted = state
        .pool
        .hard_delete(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if deleted {
        tracing::info!("Document {} permanently deleted", id);
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// ============================================================================
// Router Setup
// ============================================================================

fn app(state: AppState) -> Router {
    Router::new()
        .route("/documents", get(list_documents).post(create_document))
        .route("/documents/all", get(list_all_documents))
        .route(
            "/documents/{id}",
            get(get_document).patch(update_document).delete(delete_document),
        )
        .route("/documents/{id}/restore", post(restore_document))
        .route("/documents/{id}/hard-delete", delete(hard_delete_document))
        .with_state(state)
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("example_soft_delete=debug")
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

    let state = AppState {
        pool: Arc::new(pool),
    };

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("Listening on http://localhost:3000");
    tracing::info!("Endpoints:");
    tracing::info!("  DELETE /documents/{{id}} - soft delete");
    tracing::info!("  POST /documents/{{id}}/restore - restore");
    tracing::info!("  DELETE /documents/{{id}}/hard-delete - permanent delete");
    tracing::info!("  GET /documents/all - list including deleted");

    axum::serve(listener, app(state)).await.unwrap();
}
