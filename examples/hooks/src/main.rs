// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Hooks Example with entity-derive
//!
//! Demonstrates lifecycle hooks:
//! - `#[entity(hooks)]` generates hooks trait
//! - before_create, after_create
//! - before_update, after_update
//! - before_delete, after_delete
//!
//! # Invocation
//!
//! The macro emits the `{Entity}Hooks` trait together with
//! `{Entity}Repo<H>`, a repository that owns a pool and a hooks
//! implementation and runs the hooks around every mutation. The
//! handlers below call `repo.create(dto)` and the hooks fire on their
//! own; reads reach the pool through the same value.
//!
//! The bare pool keeps working without hooks, which is what the
//! `list_users` handler uses.

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{patch, post}
};
use chrono::{DateTime, Utc};
use entity_derive::Entity;
use sqlx::PgPool;
use uuid::Uuid;

// ============================================================================
// Entity Definition with Hooks
// ============================================================================

/// User entity with lifecycle hooks.
#[derive(Debug, Clone, Entity)]
#[entity(table = "users", hooks)]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub email: String,

    #[field(create, update, response)]
    pub name: String,

    #[field(create, skip)]
    pub password_hash: String,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>
}

// Generated trait by macro:
// #[async_trait]
// pub trait UserHooks: Send + Sync {
//     type Error: std::error::Error + Send + Sync;
//     async fn before_create(&self, dto: &mut CreateUserRequest) -> Result<(),
// Self::Error>;     async fn after_create(&self, entity: &User) -> Result<(),
// Self::Error>;     async fn before_update(&self, id: &Uuid, dto: &mut
// UpdateUserRequest) -> Result<(), Self::Error>;     async fn
// after_update(&self, entity: &User) -> Result<(), Self::Error>;     async fn
// before_delete(&self, id: &Uuid) -> Result<(), Self::Error>;     async fn
// after_delete(&self, id: &Uuid) -> Result<(), Self::Error>; }

// ============================================================================
// Hooks Implementation
// ============================================================================

#[derive(Debug)]
struct HookError(String);

impl std::fmt::Display for HookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for HookError {}

// The wrapper needs the hook error to convert into the repository
// error, which for this entity is the sqlx one.
impl From<HookError> for sqlx::Error {
    fn from(e: HookError) -> Self {
        Self::Protocol(e.to_string())
    }
}

struct MyUserHooks;

#[async_trait]
impl UserHooks for MyUserHooks {
    type Error = HookError;

    async fn before_create(&self, dto: &mut CreateUserRequest) -> Result<(), Self::Error> {
        // Normalize email to lowercase
        dto.email = dto.email.to_lowercase();

        // Validate email format
        if !dto.email.contains('@') {
            return Err(HookError("Invalid email format".into()));
        }

        // In real app: hash password here
        // dto.password_hash = hash_password(&dto.password_hash);

        tracing::info!("[HOOK] before_create: email normalized to {}", dto.email);
        Ok(())
    }

    async fn after_create(&self, entity: &User) -> Result<(), Self::Error> {
        tracing::info!("[HOOK] after_create: user {} created", entity.id);
        // In real app: send welcome email, create related records, etc.
        Ok(())
    }

    async fn before_update(
        &self,
        id: &Uuid,
        dto: &mut UpdateUserRequest
    ) -> Result<(), Self::Error> {
        if let Some(ref mut email) = dto.email {
            *email = email.to_lowercase();
        }
        tracing::info!("[HOOK] before_update: updating user {}", id);
        Ok(())
    }

    async fn after_update(&self, entity: &User) -> Result<(), Self::Error> {
        tracing::info!("[HOOK] after_update: user {} updated", entity.id);
        Ok(())
    }

    async fn before_delete(&self, id: &Uuid) -> Result<(), Self::Error> {
        tracing::info!("[HOOK] before_delete: about to delete user {}", id);
        // In real app: check if user can be deleted, archive data, etc.
        Ok(())
    }

    async fn after_delete(&self, id: &Uuid) -> Result<(), Self::Error> {
        tracing::info!("[HOOK] after_delete: user {} deleted", id);
        // In real app: cleanup related data, send notification, etc.
        Ok(())
    }
}

// ============================================================================
// Application State
// ============================================================================

#[derive(Clone)]
struct AppState {
    repo: Arc<UserRepo<MyUserHooks>>
}

// ============================================================================
// HTTP Handlers
// ============================================================================

async fn create_user(
    State(state): State<AppState>,
    Json(dto): Json<CreateUserRequest>
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // before_create runs inside, and may rewrite the DTO.
    let user = state
        .repo
        .create(dto)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(UserResponse::from(user))))
}

async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateUserRequest>
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let user = state
        .repo
        .update(id, dto)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(UserResponse::from(user)))
}

async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // after_delete runs only when a row was actually affected.
    let deleted = state
        .repo
        .delete(id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::NOT_FOUND, "User not found".into()))
    }
}

async fn list_users(State(state): State<AppState>) -> Result<impl IntoResponse, StatusCode> {
    // Reads carry no hooks; the wrapper forwards them to the pool.
    let users = state
        .repo
        .list(100, 0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let responses: Vec<UserResponse> = users.into_iter().map(UserResponse::from).collect();
    Ok(Json(responses))
}

// ============================================================================
// Router Setup
// ============================================================================

fn app(state: AppState) -> Router {
    Router::new()
        .route("/users", post(create_user).get(list_users))
        .route("/users/{id}", patch(update_user).delete(delete_user))
        .with_state(state)
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("example_hooks=debug")
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
        repo: Arc::new(UserRepo::new(pool, MyUserHooks))
    };

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("Listening on http://localhost:3000");
    tracing::info!("Watch logs for [HOOK] messages");

    axum::serve(listener, app(state)).await.unwrap();
}
