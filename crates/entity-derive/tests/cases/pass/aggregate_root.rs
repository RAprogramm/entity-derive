// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Test for `#[entity(aggregate_root)]` — generates NewOrder struct and From impl.

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Debug, Clone, Entity)]
#[entity(table = "orders", aggregate_root)]
pub struct Order {
    #[id]
    pub id: Uuid,

    #[field(create)]
    pub buyer_id: Uuid,

    #[field(create)]
    pub seller_id: Uuid,

    #[field(response)]
    #[auto]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

fn main() {
    // Test NewOrder struct exists and has correct fields
    let new_order = NewOrder {
        buyer_id: Uuid::nil(),
        seller_id: Uuid::nil(),
    };
    assert_eq!(new_order.buyer_id, Uuid::nil());

    // Test From<NewOrder> for Order compiles
    let _order: Order = new_order.into();

    // Test that OrderRepository trait has save method
    fn _check_trait<T: OrderRepository>() {}
}
