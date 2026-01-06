// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::{Entity, EntityEvent, EventKind};
use uuid::Uuid;

#[derive(Entity, Debug, Clone)]
#[entity(table = "products", events)]
pub struct Product {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,

    #[field(create, update, response)]
    pub price: i64,
}

fn main() {
    let product = Product {
        id: Uuid::nil(),
        name: "Widget".to_string(),
        price: 100,
    };

    // Test Created event
    let event = ProductEvent::created(product.clone());
    assert_eq!(event.kind(), EventKind::Created);
    assert_eq!(event.entity_id(), &Uuid::nil());

    // Test Updated event
    let old = product.clone();
    let mut new = product.clone();
    new.price = 150;
    let event = ProductEvent::updated(old, new);
    assert_eq!(event.kind(), EventKind::Updated);

    // Test HardDeleted event
    let event = ProductEvent::hard_deleted(Uuid::nil());
    assert_eq!(event.kind(), EventKind::HardDeleted);
    assert_eq!(event.entity_id(), &Uuid::nil());
}
