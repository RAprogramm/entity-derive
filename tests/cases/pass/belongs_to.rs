// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Test for `#[belongs_to]` and `#[has_many]` relation attributes.

use chrono::{DateTime, Utc};
use entity_derive::Entity;
use uuid::Uuid;

// Parent entity with has_many relation
#[derive(Debug, Clone, Entity)]
#[entity(table = "users")]
#[has_many(Post)]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub name: String,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,
}

// Child entity with belongs_to relation
#[derive(Debug, Clone, Entity)]
#[entity(table = "posts")]
pub struct Post {
    #[id]
    pub id: Uuid,

    #[belongs_to(User)]
    pub user_id: Uuid,

    #[field(create, update, response)]
    pub title: String,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,
}

fn main() {
    // Verify generated types exist
    let _: fn(CreatePostRequest) = |_| {};
    let _: fn(PostResponse) = |_| {};

    // PostRepository should have find_user method (belongs_to)
    fn _check_post_trait<T: PostRepository>() {}

    // UserRepository should have find_posts method (has_many)
    fn _check_user_trait<T: UserRepository>() {}
}
