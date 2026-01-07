// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Streams Example with entity-derive
//!
//! Demonstrates async streaming for large datasets:
//! - `#[entity(streams)]` enables streaming support
//! - `stream_all()` returns async Stream
//! - Memory-efficient processing of large result sets
//! - Supports filtering during stream

use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use chrono::{DateTime, Utc};
use entity_derive::Entity;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

// ============================================================================
// Entity Definition with Streams
// ============================================================================

/// Audit log entity with streaming support.
#[derive(Debug, Clone, Entity)]
#[entity(table = "audit_logs", streams)]
pub struct AuditLog {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    #[filter]
    pub action: String,

    #[field(create, response)]
    #[filter]
    pub resource_type: String,

    #[field(create, response)]
    pub resource_id: Uuid,

    #[field(create, response)]
    pub user_id: Option<Uuid>,

    #[field(create, response)]
    pub details: Option<serde_json::Value>,

    #[field(response)]
    #[auto]
    #[filter(range)]
    pub created_at: DateTime<Utc>,
}

// Generated streaming methods:
// - stream_all() -> impl Stream<Item = Result<AuditLog>>
// - stream_filtered(filter) -> impl Stream<Item = Result<AuditLog>>
// - stream_by_action(action) -> impl Stream
// - stream_by_resource_type(type) -> impl Stream

// ============================================================================
// Application State
// ============================================================================

#[derive(Clone)]
struct AppState {
    pool: Arc<PgPool>,
}

// ============================================================================
// Query Parameters
// ============================================================================

#[derive(Debug, Deserialize)]
struct LogQuery {
    action: Option<String>,
    resource_type: Option<String>,
    limit: Option<i64>,
}

// ============================================================================
// Statistics Response
// ============================================================================

#[derive(Debug, Serialize)]
struct StreamStats {
    total_processed: usize,
    actions: std::collections::HashMap<String, usize>,
    resource_types: std::collections::HashMap<String, usize>,
}

// ============================================================================
// HTTP Handlers
// ============================================================================

/// Stream and aggregate logs - demonstrates memory-efficient processing.
///
/// Instead of loading all records into memory, we process them one by one.
async fn aggregate_logs(
    State(state): State<AppState>,
    Query(query): Query<LogQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let filter = AuditLogFilter {
        action: query.action,
        resource_type: query.resource_type,
        created_at_min: None,
        created_at_max: None,
    };

    let mut stream = state
        .pool
        .stream_filtered(filter)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut stats = StreamStats {
        total_processed: 0,
        actions: std::collections::HashMap::new(),
        resource_types: std::collections::HashMap::new(),
    };

    let limit = query.limit.unwrap_or(1000) as usize;

    // Process stream without loading all into memory
    while let Some(result) = stream.next().await {
        if stats.total_processed >= limit {
            break;
        }

        match result {
            Ok(log) => {
                stats.total_processed += 1;
                *stats.actions.entry(log.action).or_insert(0) += 1;
                *stats.resource_types.entry(log.resource_type).or_insert(0) += 1;
            }
            Err(e) => {
                tracing::error!("Stream error: {}", e);
                break;
            }
        }
    }

    Ok(Json(stats))
}

/// Stream logs as JSON array with chunked processing.
async fn list_logs_streamed(
    State(state): State<AppState>,
    Query(query): Query<LogQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let filter = AuditLogFilter {
        action: query.action,
        resource_type: query.resource_type,
        created_at_min: None,
        created_at_max: None,
    };

    let mut stream = state
        .pool
        .stream_filtered(filter)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let limit = query.limit.unwrap_or(100) as usize;
    let mut logs = Vec::with_capacity(limit.min(100));

    while let Some(result) = stream.next().await {
        if logs.len() >= limit {
            break;
        }

        match result {
            Ok(log) => logs.push(AuditLogResponse::from(log)),
            Err(e) => {
                tracing::error!("Stream error: {}", e);
                break;
            }
        }
    }

    Ok(Json(logs))
}

/// Create a new audit log entry.
async fn create_log(
    State(state): State<AppState>,
    Json(dto): Json<CreateAuditLogRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let log = state
        .pool
        .create(dto)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(AuditLogResponse::from(log))))
}

/// Export logs by action - demonstrates filtered streaming.
async fn export_by_action(
    State(state): State<AppState>,
    axum::extract::Path(action): axum::extract::Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let filter = AuditLogFilter {
        action: Some(action.clone()),
        resource_type: None,
        created_at_min: None,
        created_at_max: None,
    };

    let mut stream = state
        .pool
        .stream_filtered(filter)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut logs = Vec::new();

    while let Some(result) = stream.next().await {
        match result {
            Ok(log) => logs.push(AuditLogResponse::from(log)),
            Err(_) => break,
        }
    }

    tracing::info!("Exported {} logs for action '{}'", logs.len(), action);
    Ok(Json(logs))
}

// ============================================================================
// Router Setup
// ============================================================================

fn app(state: AppState) -> Router {
    Router::new()
        .route("/logs", get(list_logs_streamed).post(create_log))
        .route("/logs/aggregate", get(aggregate_logs))
        .route("/logs/export/{action}", get(export_by_action))
        .with_state(state)
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("example_streams=debug")
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
    tracing::info!("  GET /logs - stream logs with filtering");
    tracing::info!("  GET /logs/aggregate - aggregate stats from stream");
    tracing::info!("  GET /logs/export/{{action}} - export by action");

    axum::serve(listener, app(state)).await.unwrap();
}
