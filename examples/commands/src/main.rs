// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Commands Example with entity-derive
//!
//! Demonstrates CQRS command pattern:
//! - `#[entity(commands)]` enables commands
//! - `#[command(Name)]` defines a command
//! - `#[command(Name, requires_id)]` for existing entity

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post
};
use chrono::{DateTime, Utc};
use entity_derive::Entity;
use sqlx::PgPool;
use uuid::Uuid;

// ============================================================================
// Entity Definition with Commands
// ============================================================================

/// Account entity with CQRS commands.
#[derive(Debug, Clone, Entity)]
#[entity(table = "accounts", commands)]
#[command(Register)]
#[command(Activate, requires_id)]
#[allow(clippy::duplicated_attributes)]
#[command(Deactivate, requires_id)]
#[command(UpdateEmail, source = "update")]
pub struct Account {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub email: String,

    #[field(create, update, response)]
    pub name: String,

    #[field(update, response)]
    pub active: bool,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>
}

// Generated commands:
// - RegisterAccount { email, name, active }
// - ActivateAccount { id }
// - DeactivateAccount { id }
// - UpdateEmailAccount { id, email, name, active }

// ============================================================================
// Command Handler
// ============================================================================

#[derive(Debug)]
struct CommandError(String);

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for CommandError {}

struct MyAccountHandler {
    pool: Arc<PgPool>
}

impl MyAccountHandler {
    async fn handle_register(&self, cmd: RegisterAccount) -> Result<Account, CommandError> {
        tracing::info!("[CMD] Register: email={}", cmd.email);

        let dto = CreateAccountRequest {
            email: cmd.email.to_lowercase(),
            name:  cmd.name
        };

        AccountRepository::create(&*self.pool, dto)
            .await
            .map_err(|e| CommandError(e.to_string()))
    }

    async fn handle_activate(&self, cmd: ActivateAccount) -> Result<Account, CommandError> {
        tracing::info!("[CMD] Activate: id={}", cmd.id);

        let dto = UpdateAccountRequest {
            email:  None,
            name:   None,
            active: Some(true)
        };

        AccountRepository::update(&*self.pool, cmd.id, dto)
            .await
            .map_err(|e| CommandError(e.to_string()))
    }

    async fn handle_deactivate(&self, cmd: DeactivateAccount) -> Result<Account, CommandError> {
        tracing::info!("[CMD] Deactivate: id={}", cmd.id);

        let dto = UpdateAccountRequest {
            email:  None,
            name:   None,
            active: Some(false)
        };

        AccountRepository::update(&*self.pool, cmd.id, dto)
            .await
            .map_err(|e| CommandError(e.to_string()))
    }

    async fn handle_update_email(&self, cmd: UpdateEmailAccount) -> Result<Account, CommandError> {
        tracing::info!("[CMD] UpdateEmail: id={}, email={:?}", cmd.id, cmd.email);

        let dto = UpdateAccountRequest {
            email:  cmd.email.map(|e| e.to_lowercase()),
            name:   cmd.name,
            active: cmd.active
        };

        AccountRepository::update(&*self.pool, cmd.id, dto)
            .await
            .map_err(|e| CommandError(e.to_string()))
    }
}

// ============================================================================
// Application State
// ============================================================================

#[derive(Clone)]
struct AppState {
    handler: Arc<MyAccountHandler>
}

// ============================================================================
// HTTP Handlers - Command Endpoints
// ============================================================================

/// Input for register command.
#[derive(serde::Deserialize)]
struct RegisterInput {
    email: String,
    name:  String
}

/// Input for update email command.
#[derive(serde::Deserialize)]
struct UpdateEmailInput {
    email:  Option<String>,
    name:   Option<String>,
    active: Option<bool>
}

async fn register(
    State(state): State<AppState>,
    Json(input): Json<RegisterInput>
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let cmd = RegisterAccount {
        email: input.email,
        name:  input.name
    };
    let account = state
        .handler
        .handle_register(cmd)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(AccountResponse::from(account))))
}

async fn activate(
    State(state): State<AppState>,
    Path(id): Path<Uuid>
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let cmd = ActivateAccount {
        id
    };
    let account = state
        .handler
        .handle_activate(cmd)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok(Json(AccountResponse::from(account)))
}

async fn deactivate(
    State(state): State<AppState>,
    Path(id): Path<Uuid>
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let cmd = DeactivateAccount {
        id
    };
    let account = state
        .handler
        .handle_deactivate(cmd)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok(Json(AccountResponse::from(account)))
}

async fn update_email(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateEmailInput>
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let cmd = UpdateEmailAccount {
        id,
        email: input.email,
        name: input.name,
        active: input.active
    };
    let account = state
        .handler
        .handle_update_email(cmd)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok(Json(AccountResponse::from(account)))
}

// ============================================================================
// Router Setup
// ============================================================================

fn app(state: AppState) -> Router {
    Router::new()
        // Command endpoints (verbs, not resources)
        .route("/commands/register", post(register))
        .route("/commands/accounts/{id}/activate", post(activate))
        .route("/commands/accounts/{id}/deactivate", post(deactivate))
        .route("/commands/accounts/{id}/update-email", post(update_email))
        .with_state(state)
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("example_commands=debug")
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

    let handler = MyAccountHandler {
        pool: Arc::new(pool)
    };

    let state = AppState {
        handler: Arc::new(handler)
    };

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("Listening on http://localhost:3000");
    tracing::info!("Try: POST /commands/register");
    tracing::info!("     POST /commands/accounts/{{id}}/activate");

    axum::serve(listener, app(state)).await.unwrap();
}
