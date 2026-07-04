// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use axum::{extract::FromRequestParts, http::request::Parts};
use entity_derive::Entity;
use uuid::Uuid;

pub struct RequireAuth;

impl<S> FromRequestParts<S> for RequireAuth
where
    S: Send + Sync,
{
    type Rejection = axum::http::StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if parts.headers.contains_key("authorization") {
            Ok(Self)
        } else {
            Err(axum::http::StatusCode::UNAUTHORIZED)
        }
    }
}

#[derive(Debug, Clone, Entity)]
#[entity(
    table = "users",
    api(
        tag = "Users",
        handlers,
        guard = "RequireAuth",
        guard(list = "none")
    )
)]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,
}

fn main() {
    let _router = user_router::<sqlx::PgPool>();
}
