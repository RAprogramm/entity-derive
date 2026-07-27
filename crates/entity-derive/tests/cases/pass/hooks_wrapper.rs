// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! The generated repository wrapper invokes the hooks; the bare pool
//! keeps working without them.

use entity_derive::{Entity, async_trait};
use uuid::Uuid;

#[derive(Debug, Clone, Entity)]
#[entity(table = "users", hooks)]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,
}

struct Audit;

#[async_trait]
impl UserHooks for Audit {
    type Error = sqlx::Error;
}

async fn exercise(pool: sqlx::PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    let repo = UserRepo::new(pool.clone(), Audit);

    let created: User = repo.create(CreateUserRequest { name: "Ada".into() }).await?;
    let updated: User = repo
        .update(created.id, UpdateUserRequest { name: Some("Grace".into()) })
        .await?;
    let removed: bool = repo.delete(updated.id).await?;

    // Reads reach the pool through the wrapper.
    let found: Option<User> = repo.find_by_id(id).await?;

    // The bare pool still works, without hooks.
    let listed: Vec<User> = pool.list(10, 0).await?;

    let _ = (removed, found, listed, repo.pool(), repo.hooks());
    Ok(())
}

fn main() {
    let _ = exercise;
}
