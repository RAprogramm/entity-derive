// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "items", schema = "core")]
pub struct Item {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,

    #[field(create, response)]
    pub price: f64,
}

fn main() {
    let create = CreateItemRequest {
        name: "Gadget".to_string(),
        price: 19.99,
    };
    assert_eq!(create.name, "Gadget");
    assert_eq!(create.price, 19.99);

    let response = ItemResponse {
        id: Uuid::nil(),
        name: "Gadget".to_string(),
        price: 19.99,
    };
    assert_eq!(response.name, "Gadget");
}
