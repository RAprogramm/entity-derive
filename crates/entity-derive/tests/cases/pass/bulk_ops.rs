// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Debug, Clone, Entity)]
#[entity(table = "posts")]
pub struct Post {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub title: String,
}

async fn exercise(pool: sqlx::PgPool, ids: Vec<Uuid>) -> Result<(), sqlx::Error> {
    let created: Vec<Post> = pool
        .create_many(vec![
            CreatePostRequest {
                title: "a".into(),
            },
            CreatePostRequest {
                title: "b".into(),
            },
        ])
        .await?;
    let found: Vec<Post> = pool.find_by_ids(created.iter().map(|p| p.id).collect()).await?;
    let removed: u64 = pool.delete_many(ids).await?;
    let _ = (found, removed);
    Ok(())
}

fn main() {
    let _ = exercise;
}
