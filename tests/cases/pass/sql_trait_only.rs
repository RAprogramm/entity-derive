// SPDX-FileCopyrightText: 2025 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Entity)]
#[entity(table = "posts", schema = "blog", sql = "trait")]
pub struct Post {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub title: String,

    #[field(create, update, response)]
    pub content: String,

    #[field(create, response)]
    pub author_id: Uuid,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,

    #[field(response)]
    #[auto]
    pub updated_at: DateTime<Utc>,
}

fn main() {
    let create = CreatePostRequest {
        title: "Hello".to_string(),
        content: "World".to_string(),
        author_id: Uuid::nil(),
    };
    assert_eq!(create.title, "Hello");

    let update = UpdatePostRequest {
        title: Some("Updated".to_string()),
        content: None,
    };
    assert!(update.title.is_some());
}
