// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::{Entity, ValueObject};
use uuid::Uuid;

#[derive(ValueObject, Debug, Clone, utoipa::ToSchema, serde::Serialize, serde::Deserialize)]
#[value_object(pg_type = "order_status", sqlx)]
pub enum OrderStatus {
    Pending,
    Shipped,
}

#[derive(Entity)]
#[entity(table = "orders", migrations)]
pub struct Order {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    #[column(pg_enum = "order_state")]
    pub status: OrderStatus,
}

fn main() {}
