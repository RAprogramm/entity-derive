// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use chrono::{DateTime, Utc};
use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "users", schema = "core", migrations)]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,

    #[field(create, response)]
    pub email: String,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,
}

fn main() {
    // Verify MIGRATION_UP is generated and contains expected SQL
    let up = User::MIGRATION_UP;
    assert!(up.contains("CREATE TABLE IF NOT EXISTS core.users"));
    assert!(up.contains("id UUID PRIMARY KEY"));
    assert!(up.contains("name TEXT NOT NULL"));
    assert!(up.contains("email TEXT NOT NULL"));
    assert!(up.contains("created_at TIMESTAMPTZ NOT NULL"));

    // Verify MIGRATION_DOWN is generated
    let down = User::MIGRATION_DOWN;
    assert!(down.contains("DROP TABLE IF EXISTS core.users CASCADE"));
}
