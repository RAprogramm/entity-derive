// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::{Entity, ValueObject};
use uuid::Uuid;

#[derive(ValueObject, Debug, Clone, PartialEq, Eq, utoipa::ToSchema, serde::Serialize, serde::Deserialize)]
#[value_object(pg_type = "order_status", sqlx)]
pub enum OrderStatus {
    Pending,
    Shipped,
    Delivered,
}

#[derive(Entity)]
#[entity(table = "orders", migrations)]
pub struct Order {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    #[column(pg_enum = "order_status")]
    pub status: OrderStatus,

    #[field(create, update, response)]
    pub note: Option<String>,
}

fn main() {
    assert_eq!(OrderStatus::PG_TYPE, "order_status");
    assert!(OrderStatus::PG_CREATE_TYPE.contains("CREATE TYPE order_status AS ENUM"));
    assert!(OrderStatus::PG_CREATE_TYPE.contains("'pending', 'shipped', 'delivered'"));

    assert_eq!(Order::MIGRATION_TYPES.len(), 1);
    assert_eq!(Order::MIGRATION_TYPES[0], OrderStatus::PG_CREATE_TYPE);
    assert!(Order::MIGRATION_UP.contains("status order_status NOT NULL"));
}
