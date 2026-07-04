// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Debug, Clone, Entity)]
#[entity(table = "articles", migrations)]
pub struct Article {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    #[filter(search)]
    pub title: String,
}

async fn exercise(pool: sqlx::PgPool) -> Result<(), sqlx::Error> {
    let query = ArticleQuery {
        title: Some("rust".into()),
        limit: Some(10),
        ..Default::default()
    };
    let _hits: Vec<Article> = pool.query(query).await?;
    Ok(())
}

fn main() {
    assert!(Article::MIGRATION_UP.contains("idx_articles_title_trgm"));
    assert!(Article::MIGRATION_EXTENSIONS[0].contains("pg_trgm"));
    let _ = exercise;
}
