如何在流行的Rust Web框架中使用 `entity-derive`。

## Axum

### 项目结构

```
src/
├── main.rs
├── entities/
│   ├── mod.rs
│   └── user.rs
├── handlers/
│   ├── mod.rs
│   └── users.rs
└── routes.rs
```

### 实体定义

```rust
// src/entities/user.rs
use entity_derive::Entity;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Entity, Clone)]
#[entity(table = "users", schema = "auth")]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub username: String,

    #[field(create, update, response)]
    pub email: String,

    #[field(skip)]
    pub password_hash: String,

    #[auto]
    #[field(response)]
    pub created_at: DateTime<Utc>,
}
```

### 处理器

```rust
// src/handlers/users.rs
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::entities::user::*;

pub async fn create_user(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<UserResponse>), StatusCode> {
    let user = pool
        .create(payload)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(UserResponse::from(&user))))
}

pub async fn get_user(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserResponse>, StatusCode> {
    let user = pool
        .find_by_id(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(UserResponse::from(&user)))
}

pub async fn update_user(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<Json<UserResponse>, StatusCode> {
    let user = pool
        .update(id, payload)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(UserResponse::from(&user)))
}

pub async fn delete_user(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let deleted = pool
        .delete(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

pub async fn list_users(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<UserResponse>>, StatusCode> {
    let users = pool
        .list(100, 0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let responses: Vec<UserResponse> = users.iter().map(UserResponse::from).collect();
    Ok(Json(responses))
}
```

### 路由

```rust
// src/routes.rs
use axum::{
    routing::{get, post, put, delete},
    Router,
};
use sqlx::PgPool;

use crate::handlers::users;

pub fn create_router(pool: PgPool) -> Router {
    Router::new()
        .route("/users", post(users::create_user))
        .route("/users", get(users::list_users))
        .route("/users/:id", get(users::get_user))
        .route("/users/:id", put(users::update_user))
        .route("/users/:id", delete(users::delete_user))
        .with_state(pool)
}
```

### Main

```rust
// src/main.rs
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;

mod entities;
mod handlers;
mod routes;

#[tokio::main]
async fn main() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create pool");

    let app = routes::create_router(pool);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

## Actix Web

### 处理器

```rust
// src/handlers/users.rs
use actix_web::{web, HttpResponse, Responder};
use sqlx::PgPool;
use uuid::Uuid;

use crate::entities::user::*;

pub async fn create_user(
    pool: web::Data<PgPool>,
    payload: web::Json<CreateUserRequest>,
) -> impl Responder {
    match pool.create(payload.into_inner()).await {
        Ok(user) => HttpResponse::Created().json(UserResponse::from(&user)),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub async fn get_user(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> impl Responder {
    let id = path.into_inner();

    match pool.find_by_id(id).await {
        Ok(Some(user)) => HttpResponse::Ok().json(UserResponse::from(&user)),
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub async fn update_user(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    payload: web::Json<UpdateUserRequest>,
) -> impl Responder {
    let id = path.into_inner();

    match pool.update(id, payload.into_inner()).await {
        Ok(user) => HttpResponse::Ok().json(UserResponse::from(&user)),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub async fn delete_user(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> impl Responder {
    let id = path.into_inner();

    match pool.delete(id).await {
        Ok(true) => HttpResponse::NoContent().finish(),
        Ok(false) => HttpResponse::NotFound().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub async fn list_users(pool: web::Data<PgPool>) -> impl Responder {
    match pool.list(100, 0).await {
        Ok(users) => {
            let responses: Vec<UserResponse> = users.iter().map(UserResponse::from).collect();
            HttpResponse::Ok().json(responses)
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}
```

### 路由

```rust
// src/routes.rs
use actix_web::web;

use crate::handlers::users;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/users")
            .route("", web::post().to(users::create_user))
            .route("", web::get().to(users::list_users))
            .route("/{id}", web::get().to(users::get_user))
            .route("/{id}", web::put().to(users::update_user))
            .route("/{id}", web::delete().to(users::delete_user)),
    );
}
```

### Main

```rust
// src/main.rs
use actix_web::{App, HttpServer, web};
use sqlx::postgres::PgPoolOptions;

mod entities;
mod handlers;
mod routes;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create pool");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .configure(routes::configure)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
```

## 错误处理

使用自定义类型进行更好的错误处理：

```rust
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

pub enum AppError {
    NotFound,
    Database(sqlx::Error),
    Validation(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "资源未找到"),
            AppError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "数据库错误"),
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg.as_str()),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err)
    }
}

// 在处理器中使用：
pub async fn get_user(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserResponse>, AppError> {
    let user = pool
        .find_by_id(id)
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(Json(UserResponse::from(&user)))
}
```

## 验证

使用 `validator` crate 添加验证：

```rust
use validator::Validate;

// 手动为生成的结构体添加 Validate derive
// 或在调用repository方法之前进行验证

pub async fn create_user(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<UserResponse>), AppError> {
    // 手动验证
    if payload.username.len() < 3 {
        return Err(AppError::Validation("用户名太短".into()));
    }

    if !payload.email.contains('@') {
        return Err(AppError::Validation("无效的邮箱".into()));
    }

    let user = pool.create(payload).await?;
    Ok((StatusCode::CREATED, Json(UserResponse::from(&user))))
}
```

## 分页

分页辅助示例：

```rust
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Pagination {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

pub async fn list_users(
    State(pool): State<PgPool>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<UserResponse>>, AppError> {
    let users = pool
        .list(pagination.limit.min(100), pagination.offset)
        .await?;

    let responses: Vec<UserResponse> = users.iter().map(UserResponse::from).collect();
    Ok(Json(responses))
}
```

## 另见

- [[案例|案例]] — 实际案例
- [[最佳实践|最佳实践]] — 生产指南
- [[属性|属性]] — 完整属性参考
