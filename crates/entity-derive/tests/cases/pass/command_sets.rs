// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! A command declaring `sets(...)` writes columns that stay out of the
//! public patch DTO.

use chrono::{DateTime, Utc};
use entity_derive::Entity;
use uuid::Uuid;

#[derive(Debug, Clone, Entity)]
#[entity(table = "users", commands)]
#[command(VerifyPassport, payload(passport_provider), sets(
    passport_verified = "true",
    passport_verified_at = "NOW()"
))]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,

    #[field(response)]
    pub passport_verified: bool,

    #[field(response)]
    pub passport_provider: Option<String>,

    #[field(response)]
    pub passport_verified_at: Option<DateTime<Utc>>,
}

async fn exercise(pool: sqlx::PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    let user: User = pool
        .verify_passport(VerifyPassportUser {
            id,
            passport_provider: Some("gov".into()),
        })
        .await?;
    let _ = user;
    Ok(())
}

fn main() {
    // The patch DTO carries only the update-marked column.
    let patch = UpdateUserRequest {
        name: Some("Ada".into()),
    };
    assert_eq!(patch.name.as_deref(), Some("Ada"));
    let _ = exercise;
}
