// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Test for `#[map(unwrap_default)]` attribute.

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "items")]
pub struct Item {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,

    #[field(response)]
    #[map(unwrap_default)]
    pub quantity: Option<i32>,

    #[field(response)]
    #[map(unwrap_default)]
    pub priority: Option<i16>,
}

fn main() {
    // Verify generated types exist
    let _: fn(CreateItemRequest) = |_| {};
    let _: fn(ItemResponse) = |_| {};

    // Verify repository trait exists
    fn _check_trait<T: ItemRepository>() {}
}
