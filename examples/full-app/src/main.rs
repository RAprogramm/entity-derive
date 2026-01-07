// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Full Application Example with entity-derive
//!
//! A complete e-commerce application demonstrating ALL entity-derive features:
//! - Relations (`#[belongs_to]`, `#[has_many]`)
//! - Soft Delete (`#[entity(soft_delete)]`)
//! - Transactions (`#[entity(transactions)]`)
//! - Events (`#[entity(events)]`)
//! - Hooks (`#[entity(hooks)]`)
//! - Commands (`#[entity(commands)]`)
//! - Streams (`#[entity(streams)]`)
//! - Filtering (`#[filter]`, `#[filter(like)]`, `#[filter(range)]`)

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post},
};
use chrono::{DateTime, Utc};
use entity_core::prelude::*;
use entity_derive::Entity;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

// ============================================================================
// Entity Definitions
// ============================================================================

/// User entity with soft delete, events, and hooks.
#[derive(Debug, Clone, Entity)]
#[entity(table = "users", soft_delete, events, hooks)]
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

    /// User's orders
    #[has_many(Order, foreign_key = "user_id")]
    pub orders: Vec<Order>,
}

/// Category entity (basic CRUD).
#[derive(Debug, Clone, Entity)]
#[entity(table = "categories")]
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

    /// Products in this category
    #[has_many(Product, foreign_key = "category_id")]
    pub products: Vec<Product>,
}

/// Product entity with relations, filtering, and soft delete.
#[derive(Debug, Clone, Entity)]
#[entity(table = "products", soft_delete, transactions)]
pub struct Product {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
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

    /// Parent category
    #[belongs_to(Category)]
    pub category: Option<Category>,
}

/// Order entity with transactions and events.
#[derive(Debug, Clone, Entity)]
#[entity(table = "orders", transactions, events)]
#[command(PlaceOrder)]
#[command(UpdateStatus, requires_id)]
pub struct Order {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
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

    /// Customer who placed the order
    #[belongs_to(User)]
    pub user: Option<User>,

    /// Items in this order
    #[has_many(OrderItem, foreign_key = "order_id")]
    pub items: Vec<OrderItem>,
}

/// Order item entity (line items).
#[derive(Debug, Clone, Entity)]
#[entity(table = "order_items", transactions)]
pub struct OrderItem {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub order_id: Uuid,

    #[field(create, response)]
    pub product_id: Uuid,

    #[field(create, response)]
    pub quantity: i32,

    #[field(create, response)]
    pub unit_price: i64,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,

    /// Parent order
    #[belongs_to(Order)]
    pub order: Option<Order>,

    /// Product reference
    #[belongs_to(Product)]
    pub product: Option<Product>,
}

/// Audit log for streaming.
#[derive(Debug, Clone, Entity)]
#[entity(table = "audit_logs", streams)]
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
// Application State
// ============================================================================

#[derive(Clone)]
struct AppState {
    pool: Arc<PgPool>,
}

// ============================================================================
// Request/Response DTOs
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
struct OrderWithItems {
    #[serde(flatten)]
    order: OrderResponse,
    items: Vec<OrderItemResponse>,
    total_formatted: String,
}

// ============================================================================
// User Handlers
// ============================================================================

async fn list_users(
    State(state): State<AppState>,
    Query(filter): Query<UserFilter>,
) -> Result<impl IntoResponse, StatusCode> {
    let users = state
        .pool
        .list_filtered(filter, 100, 0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let responses: Vec<UserResponse> = users.into_iter().map(UserResponse::from).collect();
    Ok(Json(responses))
}

async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let user = state
        .pool
        .find_by_id(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(UserResponse::from(user)))
}

async fn create_user(
    State(state): State<AppState>,
    Json(dto): Json<CreateUserRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let user = state
        .pool
        .create(dto)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(UserResponse::from(user))))
}

async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let deleted = state
        .pool
        .delete(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// ============================================================================
// Category Handlers
// ============================================================================

async fn list_categories(State(state): State<AppState>) -> Result<impl IntoResponse, StatusCode> {
    let categories: Vec<Category> = state
        .pool
        .list(100, 0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let responses: Vec<CategoryResponse> = categories
        .into_iter()
        .map(CategoryResponse::from)
        .collect();
    Ok(Json(responses))
}

async fn get_category_with_products(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let category = state
        .pool
        .find_by_id_with_products(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(CategoryResponse::from(category)))
}

// ============================================================================
// Product Handlers
// ============================================================================

async fn list_products(
    State(state): State<AppState>,
    Query(filter): Query<ProductFilter>,
) -> Result<impl IntoResponse, StatusCode> {
    let products = state
        .pool
        .list_filtered(filter, 100, 0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let responses: Vec<ProductResponse> = products.into_iter().map(ProductResponse::from).collect();
    Ok(Json(responses))
}

async fn get_product(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let product = state
        .pool
        .find_by_id_with_category(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(ProductResponse::from(product)))
}

async fn create_product(
    State(state): State<AppState>,
    Json(dto): Json<CreateProductRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let product = state
        .pool
        .create(dto)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(ProductResponse::from(product))))
}

async fn update_product(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateProductRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let product = state
        .pool
        .update(id, dto)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ProductResponse::from(product)))
}

// ============================================================================
// Order Handlers (with Transactions)
// ============================================================================

/// Place an order atomically.
///
/// This demonstrates transactions across multiple entities:
/// 1. Create order
/// 2. Create order items
/// 3. Update product stock
/// 4. Calculate and update order total
async fn place_order(
    State(state): State<AppState>,
    Json(req): Json<PlaceOrderRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if req.items.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Order must have items".into()));
    }

    let result = Transaction::new(&*state.pool)
        .with_orders()
        .with_order_items()
        .with_products()
        .run(|mut ctx| async move {
            // Step 1: Create the order
            let order = ctx
                .orders()
                .create(CreateOrderRequest {
                    user_id: req.user_id,
                    status: "pending".to_string(),
                    total: 0,
                })
                .await?;

            let mut total: i64 = 0;
            let mut created_items = Vec::new();

            // Step 2: Process each item
            for item in &req.items {
                // Get product and check stock
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

                // Create order item
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

                // Update stock
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

            // Step 3: Update order total
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

            Ok((final_order, created_items))
        })
        .await;

    match result {
        Ok((order, items)) => {
            tracing::info!("Order {} placed successfully", order.id);
            let response = OrderWithItems {
                order: OrderResponse::from(order),
                items: items.into_iter().map(OrderItemResponse::from).collect(),
                total_formatted: format!("${:.2}", items.iter().map(|i| i.unit_price * i.quantity as i64).sum::<i64>() as f64 / 100.0),
            };
            Ok((StatusCode::CREATED, Json(response)))
        }
        Err(e) => {
            tracing::error!("Order placement failed: {}", e);
            Err((StatusCode::BAD_REQUEST, e.to_string()))
        }
    }
}

async fn list_orders(
    State(state): State<AppState>,
    Query(filter): Query<OrderFilter>,
) -> Result<impl IntoResponse, StatusCode> {
    let orders = state
        .pool
        .list_filtered(filter, 100, 0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let responses: Vec<OrderResponse> = orders.into_iter().map(OrderResponse::from).collect();
    Ok(Json(responses))
}

async fn get_order(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let order = state
        .pool
        .find_by_id_with_items(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(OrderResponse::from(order)))
}

async fn update_order_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateOrderRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let order = state
        .pool
        .update(id, dto)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(OrderResponse::from(order)))
}

// ============================================================================
// Audit Log Handlers (Streaming)
// ============================================================================

#[derive(Debug, Deserialize)]
struct AuditQuery {
    entity_type: Option<String>,
    action: Option<String>,
    limit: Option<i64>,
}

async fn stream_audit_logs(
    State(state): State<AppState>,
    Query(query): Query<AuditQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let filter = AuditLogFilter {
        entity_type: query.entity_type,
        action: query.action,
        created_at_min: None,
        created_at_max: None,
    };

    let mut stream = state
        .pool
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

fn app(state: AppState) -> Router {
    Router::new()
        // User routes
        .route("/users", get(list_users).post(create_user))
        .route("/users/{id}", get(get_user).delete(delete_user))
        // Category routes
        .route("/categories", get(list_categories))
        .route("/categories/{id}", get(get_category_with_products))
        // Product routes
        .route("/products", get(list_products).post(create_product))
        .route("/products/{id}", get(get_product).patch(update_product))
        // Order routes
        .route("/orders", get(list_orders).post(place_order))
        .route(
            "/orders/{id}",
            get(get_order).patch(update_order_status),
        )
        // Audit routes
        .route("/audit", get(stream_audit_logs))
        .with_state(state)
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

    let state = AppState {
        pool: Arc::new(pool),
    };

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("=================================================");
    tracing::info!("Full Application Example - All Features Combined");
    tracing::info!("=================================================");
    tracing::info!("Listening on http://localhost:3000");
    tracing::info!("");
    tracing::info!("Features demonstrated:");
    tracing::info!("  - Relations: Category -> Products, User -> Orders");
    tracing::info!("  - Soft Delete: Users, Products");
    tracing::info!("  - Transactions: Order placement");
    tracing::info!("  - Filtering: Products by price, Users by role");
    tracing::info!("  - Streams: Audit log processing");
    tracing::info!("");
    tracing::info!("Endpoints:");
    tracing::info!("  GET/POST /users");
    tracing::info!("  GET/POST /products?price_min=&price_max=");
    tracing::info!("  GET /categories/{{id}} (with products)");
    tracing::info!("  POST /orders (atomic order placement)");
    tracing::info!("  GET /audit?entity_type=&action=");

    axum::serve(listener, app(state)).await.unwrap();
}
