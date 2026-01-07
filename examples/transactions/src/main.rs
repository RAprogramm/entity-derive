// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Transactions Example with entity-derive
//!
//! Demonstrates multi-entity transactions:
//! - `#[entity(transactions)]` generates transaction adapter
//! - Atomic operations across multiple entities
//! - Automatic rollback on error

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use entity_core::prelude::*;
use entity_derive::Entity;
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

// ============================================================================
// Entity Definitions with Transactions
// ============================================================================

/// Bank account with transaction support.
#[derive(Debug, Clone, Entity)]
#[entity(table = "bank_accounts", transactions)]
pub struct BankAccount {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub owner_name: String,

    #[field(create, update, response)]
    pub balance: i64,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,
}

/// Transfer log for audit.
#[derive(Debug, Clone, Entity)]
#[entity(table = "transfer_logs", transactions)]
pub struct TransferLog {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub from_account_id: Uuid,

    #[field(create, response)]
    pub to_account_id: Uuid,

    #[field(create, response)]
    pub amount: i64,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Application State
// ============================================================================

#[derive(Clone)]
struct AppState {
    pool: Arc<PgPool>,
}

// ============================================================================
// Transfer Request
// ============================================================================

#[derive(Debug, Deserialize)]
struct TransferRequest {
    from_account_id: Uuid,
    to_account_id: Uuid,
    amount: i64,
}

// ============================================================================
// HTTP Handlers
// ============================================================================

/// Transfer money between accounts atomically.
///
/// If ANY step fails, all changes are rolled back.
async fn transfer(
    State(state): State<AppState>,
    Json(req): Json<TransferRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if req.amount <= 0 {
        return Err((StatusCode::BAD_REQUEST, "Amount must be positive".into()));
    }

    let result = Transaction::new(&*state.pool)
        .with_bank_accounts()
        .with_transfer_logs()
        .run(|mut ctx| async move {
            // Step 1: Get source account
            let from = ctx
                .bank_accounts()
                .find_by_id(req.from_account_id)
                .await?
                .ok_or_else(|| sqlx::Error::RowNotFound)?;

            // Step 2: Check balance
            if from.balance < req.amount {
                return Err(sqlx::Error::Protocol(format!(
                    "Insufficient funds: {} < {}",
                    from.balance, req.amount
                )));
            }

            // Step 3: Get destination account
            let to = ctx
                .bank_accounts()
                .find_by_id(req.to_account_id)
                .await?
                .ok_or_else(|| sqlx::Error::RowNotFound)?;

            // Step 4: Subtract from source
            ctx.bank_accounts()
                .update(
                    req.from_account_id,
                    UpdateBankAccountRequest {
                        owner_name: None,
                        balance: Some(from.balance - req.amount),
                    },
                )
                .await?;

            // Step 5: Add to destination
            ctx.bank_accounts()
                .update(
                    req.to_account_id,
                    UpdateBankAccountRequest {
                        owner_name: None,
                        balance: Some(to.balance + req.amount),
                    },
                )
                .await?;

            // Step 6: Create audit log
            let log = ctx
                .transfer_logs()
                .create(CreateTransferLogRequest {
                    from_account_id: req.from_account_id,
                    to_account_id: req.to_account_id,
                    amount: req.amount,
                })
                .await?;

            Ok(log)
        })
        .await;

    match result {
        Ok(log) => {
            tracing::info!(
                "Transfer successful: {} -> {} amount={}",
                req.from_account_id,
                req.to_account_id,
                req.amount
            );
            Ok((StatusCode::OK, Json(TransferLogResponse::from(log))))
        }
        Err(e) => {
            tracing::error!("Transfer failed: {}", e);
            Err((StatusCode::BAD_REQUEST, e.to_string()))
        }
    }
}

/// List all accounts.
async fn list_accounts(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let accounts = BankAccountRepository::list(&*state.pool, 100, 0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let responses: Vec<BankAccountResponse> =
        accounts.into_iter().map(BankAccountResponse::from).collect();
    Ok(Json(responses))
}

/// Get account by ID.
async fn get_account(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let account = BankAccountRepository::find_by_id(&*state.pool, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(BankAccountResponse::from(account)))
}

/// Create a new account.
async fn create_account(
    State(state): State<AppState>,
    Json(dto): Json<CreateBankAccountRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let account = BankAccountRepository::create(&*state.pool, dto)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(BankAccountResponse::from(account))))
}

/// List transfer history.
async fn list_transfers(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let logs = TransferLogRepository::list(&*state.pool, 100, 0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let responses: Vec<TransferLogResponse> =
        logs.into_iter().map(TransferLogResponse::from).collect();
    Ok(Json(responses))
}

// ============================================================================
// Router Setup
// ============================================================================

fn app(state: AppState) -> Router {
    Router::new()
        .route("/accounts", get(list_accounts).post(create_account))
        .route("/accounts/{id}", get(get_account))
        .route("/transfer", post(transfer))
        .route("/transfers", get(list_transfers))
        .with_state(state)
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("example_transactions=debug")
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
    tracing::info!("Try: POST /transfer with {{from_account_id, to_account_id, amount}}");

    axum::serve(listener, app(state)).await.unwrap();
}
