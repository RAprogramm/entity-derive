// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Filtering Example with entity-derive
//!
//! Demonstrates type-safe query filtering:
//! - `#[filter]` for exact match
//! - `#[filter(like)]` for pattern matching
//! - `#[filter(range)]` for date/number ranges

use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use chrono::{DateTime, Utc};
use entity_derive::Entity;
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

// ============================================================================
// Entity Definition with Filters
// ============================================================================

/// Product entity with various filter types.
#[derive(Debug, Clone, Entity)]
#[entity(table = "products")]
pub struct Product {
    #[id]
    pub id: Uuid,

    /// Product name - supports pattern matching.
    #[field(create, update, response)]
    #[filter(like)]
    pub name: String,

    /// Product category - exact match filter.
    #[field(create, update, response)]
    #[filter]
    pub category: String,

    /// Price in cents - range filter.
    #[field(create, update, response)]
    #[filter(range)]
    pub price: i64,

    /// Stock quantity - range filter.
    #[field(create, update, response)]
    #[filter(range)]
    pub stock: i32,

    /// Is product active - exact match.
    #[field(create, update, response)]
    #[filter]
    pub active: bool,

    /// Creation timestamp - range filter.
    #[field(response)]
    #[auto]
    #[filter(range)]
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
// Query Parameters
// ============================================================================

/// Query parameters that map to generated ProductQuery.
#[derive(Debug, Deserialize)]
struct ProductQueryParams {
    /// Filter by name pattern (ILIKE).
    name: Option<String>,
    /// Filter by exact category.
    category: Option<String>,
    /// Minimum price.
    price_min: Option<i64>,
    /// Maximum price.
    price_max: Option<i64>,
    /// Minimum stock.
    stock_min: Option<i32>,
    /// Only active products.
    active: Option<bool>,
    /// Pagination limit.
    #[serde(default = "default_limit")]
    limit: i64,
    /// Pagination offset.
    #[serde(default)]
    offset: i64,
}

fn default_limit() -> i64 {
    20
}

impl From<ProductQueryParams> for ProductQuery {
    fn from(p: ProductQueryParams) -> Self {
        Self {
            name: p.name,
            category: p.category,
            price_from: p.price_min,
            price_to: p.price_max,
            stock_from: p.stock_min,
            stock_to: None,
            active: p.active,
            created_at_from: None,
            created_at_to: None,
            limit: Some(p.limit),
            offset: Some(p.offset),
        }
    }
}

// ============================================================================
// HTTP Handlers
// ============================================================================

/// List products with filters.
///
/// Examples:
/// - GET /products?category=electronics
/// - GET /products?name=phone&price_max=100000
/// - GET /products?active=true&stock_min=10
async fn list_products(
    State(state): State<AppState>,
    Query(params): Query<ProductQueryParams>,
) -> Result<impl IntoResponse, StatusCode> {
    // Convert to generated ProductQuery (includes limit/offset)
    let query: ProductQuery = params.into();

    // Use generated query method for type-safe filtering with pagination
    let products = state
        .pool
        .query(query)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let responses: Vec<ProductResponse> = products.into_iter().map(ProductResponse::from).collect();
    Ok(Json(responses))
}

/// Get filter statistics.
async fn get_categories(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let products = state.pool.list(1000, 0).await.map_err(|e| {
        tracing::error!("Database error: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut categories: Vec<String> = products
        .iter()
        .map(|p| p.category.clone())
        .collect();
    categories.sort();
    categories.dedup();

    Ok(Json(categories))
}

// ============================================================================
// Router Setup
// ============================================================================

fn app(state: AppState) -> Router {
    Router::new()
        .route("/products", get(list_products))
        .route("/categories", get(get_categories))
        .with_state(state)
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("example_filtering=debug")
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
    tracing::info!("Try: GET /products?category=electronics&price_max=50000");

    axum::serve(listener, app(state)).await.unwrap();
}
