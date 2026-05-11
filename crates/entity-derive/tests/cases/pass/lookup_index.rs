// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Test for `#[column(index)]` — generates only find_by_ method.

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Debug, Clone, Entity)]
#[entity(table = "posts")]
pub struct Post {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    #[column(index)]
    pub slug: String,

    #[field(create, update, response)]
    pub title: String,
}

fn main() {
    let _: fn(CreatePostRequest) = |_| {};
    let _: fn(PostResponse) = |_| {};

    fn _check_trait<T: PostRepository>() {}
}
