// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;

// Stub types for belongs_to references
pub struct User;
pub struct Category;

#[derive(Entity)]
#[entity(table = "posts", schema = "blog", migrations, sql = "none")]
pub struct Post {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub title: String,

    #[field(create, response)]
    #[belongs_to(User, on_delete = "cascade")]
    pub author_id: Uuid,

    #[field(create, response)]
    #[belongs_to(Category, on_delete = "set null")]
    pub category_id: Option<Uuid>,
}

fn main() {
    let up = Post::MIGRATION_UP;

    // Check table creation
    assert!(up.contains("CREATE TABLE IF NOT EXISTS blog.posts"));

    // Check foreign key with CASCADE
    assert!(up.contains("author_id UUID NOT NULL REFERENCES blog.users(id) ON DELETE CASCADE"));

    // Check foreign key with SET NULL (nullable field)
    assert!(up.contains("category_id UUID REFERENCES blog.categories(id) ON DELETE SET NULL"));
}
