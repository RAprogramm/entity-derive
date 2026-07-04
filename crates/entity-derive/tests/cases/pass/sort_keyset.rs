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
    #[sort]
    #[filter(like)]
    pub title: String,

    #[field(create, response)]
    #[sort]
    pub views: i64,
}

async fn exercise(pool: sqlx::PgPool, cursor: Option<Uuid>) -> Result<(), sqlx::Error> {
    let query = PostQuery {
        title: Some("%rust%".into()),
        sort: Some(PostSortField::ViewsDesc),
        limit: Some(20),
        ..Default::default()
    };
    let _filtered: Vec<Post> = pool.query(query).await?;

    let first_page: Vec<Post> = pool.list_after(None, 20).await?;
    let _next_page: Vec<Post> = pool
        .list_after(first_page.last().map(|p| p.id).or(cursor), 20)
        .await?;
    Ok(())
}

fn main() {
    assert_eq!(PostSortField::TitleAsc.order_by(), "title ASC");
    assert_eq!(PostSortField::ViewsDesc.order_by(), "views DESC");
    let json = serde_json::to_string(&PostSortField::TitleAsc).unwrap();
    assert_eq!(json, "\"title_asc\"");
    let _ = exercise;
}
