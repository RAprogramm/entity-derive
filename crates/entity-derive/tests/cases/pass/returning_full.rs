// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Test for `#[entity(returning = "full")]` attribute (default).

use chrono::{DateTime, Utc};
use entity_derive::Entity;
use uuid::Uuid;

#[derive(Debug, Clone, Entity)]
#[entity(table = "items", returning = "full")]
pub struct Item {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,
}

fn main() {
    // Verify generated types exist
    let _: fn(CreateItemRequest) = |_| {};
    let _: fn(ItemResponse) = |_| {};

    // Verify repository trait exists
    fn _check_trait<T: ItemRepository>() {}
}
