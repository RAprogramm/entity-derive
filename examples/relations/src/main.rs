// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Relations Example with entity-derive
//!
//! Demonstrates entity relationships:
//! - `#[belongs_to(Entity)]` for foreign keys
//! - `#[has_many(Entity)]` for one-to-many

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use chrono::{DateTime, Utc};
use entity_derive::Entity;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

// ============================================================================
// Entity Definitions with Relations
// ============================================================================

/// Author entity - has many posts.
#[derive(Debug, Clone, Entity)]
#[entity(table = "authors")]
#[has_many(Post)]
pub struct Author {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,

    #[field(create, update, response)]
    pub email: String,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,
}

/// Post entity - belongs to author, has many comments.
#[derive(Debug, Clone, Entity)]
#[entity(table = "posts")]
#[has_many(Comment)]
pub struct Post {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub title: String,

    #[field(create, update, response)]
    pub content: String,

    /// Foreign key to author.
    #[field(create, response)]
    #[belongs_to(Author)]
    pub author_id: Uuid,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,
}

/// Comment entity - belongs to post.
#[derive(Debug, Clone, Entity)]
#[entity(table = "comments")]
pub struct Comment {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub text: String,

    #[field(create, response)]
    pub commenter_name: String,

    /// Foreign key to post.
    #[field(create, response)]
    #[belongs_to(Post)]
    pub post_id: Uuid,

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
// HTTP Handlers
// ============================================================================

/// Get author with their posts.
async fn get_author_with_posts(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    // Use fully qualified syntax when multiple Repository traits are in scope
    let author = AuthorRepository::find_by_id(&*state.pool, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Use generated find_posts method (from has_many)
    let posts = AuthorRepository::find_posts(&*state.pool, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({
        "author": AuthorResponse::from(author),
        "posts": posts.into_iter().map(PostResponse::from).collect::<Vec<_>>()
    })))
}

/// Get post with author and comments.
async fn get_post_with_details(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    // Use fully qualified syntax when multiple Repository traits are in scope
    let post = PostRepository::find_by_id(&*state.pool, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Use generated find_author method (from belongs_to)
    let author = PostRepository::find_author(&*state.pool, post.author_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Use generated find_comments method (from has_many)
    let comments = PostRepository::find_comments(&*state.pool, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({
        "post": PostResponse::from(post),
        "author": author.map(AuthorResponse::from),
        "comments": comments.into_iter().map(CommentResponse::from).collect::<Vec<_>>()
    })))
}

/// List all authors.
async fn list_authors(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    // Use fully qualified syntax when multiple Repository traits are in scope
    let authors = AuthorRepository::list(&*state.pool, 100, 0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let responses: Vec<AuthorResponse> = authors.into_iter().map(AuthorResponse::from).collect();
    Ok(Json(responses))
}

// ============================================================================
// Router Setup
// ============================================================================

fn app(state: AppState) -> Router {
    Router::new()
        .route("/authors", get(list_authors))
        .route("/authors/{id}", get(get_author_with_posts))
        .route("/posts/{id}", get(get_post_with_details))
        .with_state(state)
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("example_relations=debug")
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
    tracing::info!("Try: GET /authors/{{id}} to see author with posts");

    axum::serve(listener, app(state)).await.unwrap();
}
