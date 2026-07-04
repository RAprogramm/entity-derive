// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use chrono::{DateTime, Utc};
use entity_derive::Entity;
use uuid::Uuid;

#[derive(Debug, Clone, Entity)]
#[entity(
    table = "articles",
    migrations(touch_updated_at, audit, extensions = "pg_trgm")
)]
pub struct Article {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub title: String,

    #[field(response)]
    #[auto]
    pub updated_at: DateTime<Utc>,
}

fn main() {
    assert_eq!(Article::MIGRATION_EXTENSIONS.len(), 1);
    assert!(Article::MIGRATION_EXTENSIONS[0].contains("pg_trgm"));
    assert!(Article::MIGRATION_TRIGGERS.len() >= 4);
    assert!(Article::MIGRATION_TRIGGERS[0].contains("entity_touch_updated_at"));
    assert!(
        Article::MIGRATION_TRIGGERS
            .iter()
            .any(|d| d.contains("entity_audit_log"))
    );
    assert!(Article::MIGRATION_UP.contains("CREATE TABLE IF NOT EXISTS articles"));
}
