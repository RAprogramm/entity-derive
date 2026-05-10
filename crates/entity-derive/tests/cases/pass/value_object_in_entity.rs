// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::{Entity, ValueObject};
use uuid::Uuid;

#[derive(ValueObject, Debug, Clone, PartialEq, Eq, PartialOrd, sqlx::Type, serde::Serialize, serde::Deserialize)]
#[value_object(pg_type = "order_status")]
pub enum OrderStatusVO {
    Pending,
    Confirmed,
    Cancelled,
}

#[derive(Entity)]
#[entity(table = "orders")]
pub struct Order {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub status: OrderStatusVO,

    #[field(create, update, response)]
    pub total: f64,
}

fn main() {
    let create = CreateOrderRequest {
        status: OrderStatusVO::Pending,
        total: 99.99,
    };
    assert_eq!(create.status.as_ref(), "pending");
    assert_eq!(create.total, 99.99);

    let parsed: OrderStatusVO = "confirmed".parse().unwrap();
    assert_eq!(parsed, OrderStatusVO::Confirmed);
}
