// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_core::ConstraintError;
use entity_derive::Entity;
use uuid::Uuid;

#[derive(Debug)]
pub enum AppError {
    Database(sqlx::Error),
    Constraint(ConstraintError),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(e) => write!(f, "database error: {e}"),
            Self::Constraint(e) => write!(f, "constraint violation: {e}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e)
    }
}

impl From<ConstraintError> for AppError {
    fn from(e: ConstraintError) -> Self {
        Self::Constraint(e)
    }
}

#[derive(Debug, Clone, Entity)]
#[entity(
    table = "users",
    typed_constraints,
    error = "AppError",
    constraint(name = "users_referral_fkey", kind = "foreign_key", field = "referral_code")
)]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    #[column(unique)]
    pub email: String,

    #[field(create, update, response)]
    pub name: String,

    #[field(create, response)]
    pub referral_code: Option<String>,
}

async fn exercise(pool: sqlx::PgPool) -> Result<(), AppError> {
    let result = pool
        .create(CreateUserRequest {
            email: "a@b.c".into(),
            name: "A".into(),
            referral_code: None,
        })
        .await;
    if let Err(AppError::Constraint(violation)) = &result {
        assert_eq!(violation.field, Some("email"));
    }
    let _ = result;
    Ok(())
}

fn main() {
    let _ = exercise;
}
