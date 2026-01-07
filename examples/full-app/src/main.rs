// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Full Application Example with entity-derive
//!
//! A complete e-commerce application demonstrating entity-derive features:
//! - Auto-generated CRUD handlers with `api(handlers)`
//! - Relations (`#[belongs_to]`, `#[has_many]`)
//! - Soft Delete (`#[entity(soft_delete)]`)
//! - Transactions (`#[entity(transactions)]`)
//! - Events (`#[entity(events)]`)
//! - Streams (`#[entity(streams)]`)
//! - Filtering (`#[filter]`, `#[filter(like)]`, `#[filter(range)]`)

use std::sync::Arc;

use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::post};
use chrono::{DateTime, Utc};
use entity_core::prelude::*;
use entity_derive::Entity;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

// ============================================================================
// Entity Definitions
// ============================================================================

/// User entity with full CRUD API and soft delete.
/// This entity uses auto-generated handlers via `api(handlers)`.
#[derive(Debug, Clone, Entity)]
#[entity(
    table = "users",
    soft_delete,
    api(tag = "Users", handlers, title = "E-Commerce API", api_version = "1.0.0")
)]
#[has_many(Order)]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    #[filter(like)]
    pub email: String,

    #[field(create, update, response)]
    #[filter(like)]
    pub name: String,

    #[field(create, update, response)]
    #[filter]
    pub role: String,

    #[field(update, response)]
    pub active: bool,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,

    #[field(response)]
    #[auto]
    pub updated_at: DateTime<Utc>,

    #[field(skip)]
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Category entity (basic CRUD without auto-handlers).
#[derive(Debug, Clone, Entity)]
#[entity(table = "categories")]
#[has_many(Product)]
pub struct Category {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    #[filter(like)]
    pub name: String,

    #[field(create, update, response)]
    pub description: Option<String>,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,
}

/// Product entity with soft delete, transactions, and filtering.
#[derive(Debug, Clone, Entity)]
#[entity(table = "products", soft_delete, transactions)]
pub struct Product {
    #[id]
    pub id: Uuid,

    /// Foreign key to category
    #[field(create, update, response)]
    #[belongs_to(Category)]
    pub category_id: Uuid,

    #[field(create, update, response)]
    #[filter(like)]
    pub name: String,

    #[field(create, update, response)]
    pub description: Option<String>,

    #[field(create, update, response)]
    #[filter(range)]
    pub price: i64,

    #[field(create, update, response)]
    #[filter(range)]
    pub stock: i32,

    #[field(update, response)]
    pub active: bool,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,

    #[field(response)]
    #[auto]
    pub updated_at: DateTime<Utc>,

    #[field(skip)]
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Order entity with transactions and events.
#[derive(Debug, Clone, PartialEq, Entity)]
#[entity(table = "orders", transactions, events)]
#[has_many(OrderItem)]
pub struct Order {
    #[id]
    pub id: Uuid,

    /// Foreign key to user
    #[field(create, response)]
    #[belongs_to(User)]
    pub user_id: Uuid,

    #[field(create, update, response)]
    #[filter]
    pub status: String,

    #[field(update, response)]
    pub total: i64,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,

    #[field(response)]
    #[auto]
    pub updated_at: DateTime<Utc>,
}

/// Order item entity (line items).
#[derive(Debug, Clone, Entity)]
#[entity(table = "order_items", transactions)]
pub struct OrderItem {
    #[id]
    pub id: Uuid,

    /// Foreign key to order
    #[field(create, response)]
    #[belongs_to(Order)]
    pub order_id: Uuid,

    /// Foreign key to product
    #[field(create, response)]
    #[belongs_to(Product)]
    pub product_id: Uuid,

    #[field(create, response)]
    pub quantity: i32,

    #[field(create, response)]
    pub unit_price: i64,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,
}

/// Audit log for streaming.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, Entity)]
#[entity(table = "audit_logs", streams, events)]
pub struct AuditLog {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    #[filter]
    pub entity_type: String,

    #[field(create, response)]
    pub entity_id: Uuid,

    #[field(create, response)]
    #[filter]
    pub action: String,

    #[field(create, response)]
    pub user_id: Option<Uuid>,

    #[field(create, response)]
    pub old_data: Option<serde_json::Value>,

    #[field(create, response)]
    pub new_data: Option<serde_json::Value>,

    #[field(response)]
    #[auto]
    #[filter(range)]
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Custom Order Placement (Transaction Example)
// ============================================================================

#[derive(Debug, Deserialize)]
struct PlaceOrderRequest {
    user_id: Uuid,
    items: Vec<OrderItemInput>,
}

#[derive(Debug, Deserialize)]
struct OrderItemInput {
    product_id: Uuid,
    quantity: i32,
}

#[derive(Debug, Serialize)]
struct PlaceOrderResponse {
    order: OrderResponse,
    items: Vec<OrderItemResponse>,
    total_formatted: String,
}

/// Place an order atomically using transactions.
async fn place_order(
    State(pool): State<Arc<PgPool>>,
    Json(req): Json<PlaceOrderRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if req.items.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Order must have items".into()));
    }

    let result = Transaction::new(&*pool)
        .with_orders()
        .with_order_items()
        .with_products()
        .run(|mut ctx| async move {
            // Create order
            let order = ctx
                .orders()
                .create(CreateOrderRequest {
                    user_id: req.user_id,
                    status: "pending".to_string(),
                })
                .await?;

            let mut total: i64 = 0;
            let mut created_items = Vec::new();

            // Process each item
            for item in &req.items {
                let product = ctx
                    .products()
                    .find_by_id(item.product_id)
                    .await?
                    .ok_or_else(|| sqlx::Error::RowNotFound)?;

                if product.stock < item.quantity {
                    return Err(sqlx::Error::Protocol(format!(
                        "Insufficient stock for {}: {} < {}",
                        product.name, product.stock, item.quantity
                    )));
                }

                let order_item = ctx
                    .order_items()
                    .create(CreateOrderItemRequest {
                        order_id: order.id,
                        product_id: item.product_id,
                        quantity: item.quantity,
                        unit_price: product.price,
                    })
                    .await?;

                created_items.push(order_item);
                total += product.price * item.quantity as i64;

                ctx.products()
                    .update(
                        item.product_id,
                        UpdateProductRequest {
                            category_id: None,
                            name: None,
                            description: None,
                            price: None,
                            stock: Some(product.stock - item.quantity),
                            active: None,
                        },
                    )
                    .await?;
            }

            // Update order total
            let final_order = ctx
                .orders()
                .update(
                    order.id,
                    UpdateOrderRequest {
                        status: None,
                        total: Some(total),
                    },
                )
                .await?;

            Ok((final_order, created_items, total))
        })
        .await;

    match result {
        Ok((order, items, total)) => {
            tracing::info!("Order {} placed successfully", order.id);
            let response = PlaceOrderResponse {
                order: OrderResponse::from(order),
                items: items.into_iter().map(OrderItemResponse::from).collect(),
                total_formatted: format!("${:.2}", total as f64 / 100.0),
            };
            Ok((StatusCode::CREATED, Json(response)))
        }
        Err(e) => {
            tracing::error!("Order placement failed: {}", e);
            Err((StatusCode::BAD_REQUEST, e.to_string()))
        }
    }
}

// ============================================================================
// Audit Log Streaming Example
// ============================================================================

#[derive(Debug, Deserialize)]
struct AuditQuery {
    entity_type: Option<String>,
    action: Option<String>,
    limit: Option<i64>,
}

async fn stream_audit_logs(
    State(pool): State<Arc<PgPool>>,
    axum::extract::Query(query): axum::extract::Query<AuditQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let filter = AuditLogFilter {
        entity_type: query.entity_type,
        action: query.action,
        created_at_from: None,
        created_at_to: None,
        limit: None,
        offset: None,
    };

    let mut stream = pool
        .stream_filtered(filter)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let limit = query.limit.unwrap_or(100) as usize;
    let mut logs = Vec::with_capacity(limit);

    while let Some(result) = stream.next().await {
        if logs.len() >= limit {
            break;
        }
        if let Ok(log) = result {
            logs.push(AuditLogResponse::from(log));
        }
    }

    Ok(Json(logs))
}

// ============================================================================
// Router Setup
// ============================================================================

fn app(pool: Arc<PgPool>) -> Router {
    Router::new()
        // Use generated router for Users (auto-generated handlers)
        .merge(user_router::<PgPool>())
        // Custom endpoints
        .route("/orders/place", post(place_order))
        .route("/audit", axum::routing::get(stream_audit_logs))
        // Swagger UI
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", UserApi::openapi()))
        .with_state(pool)
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("example_full_app=debug")
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

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("=================================================");
    tracing::info!("Full Application Example - All Features Combined");
    tracing::info!("=================================================");
    tracing::info!("Listening on http://localhost:3000");
    tracing::info!("Swagger UI: http://localhost:3000/swagger-ui");
    tracing::info!("");
    tracing::info!("Features demonstrated:");
    tracing::info!("  - Auto-generated CRUD handlers (User)");
    tracing::info!("  - Relations: User -> Orders, Category -> Products");
    tracing::info!("  - Soft Delete: Users, Products");
    tracing::info!("  - Transactions: Order placement");
    tracing::info!("  - Streams: Audit log processing");
    tracing::info!("");
    tracing::info!("Endpoints:");
    tracing::info!("  GET/POST /users (auto-generated)");
    tracing::info!("  POST /orders/place (atomic order placement)");
    tracing::info!("  GET /audit?entity_type=&action=");

    axum::serve(listener, app(Arc::new(pool))).await.unwrap();
}
