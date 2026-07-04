// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Debug, Clone, utoipa::ToSchema, serde::Serialize, serde::Deserialize)]
pub struct Money {
    pub amount_cents: i64,
    pub currency: String,
    pub precision: u8,
}

#[derive(Debug, Clone, Entity)]
#[entity(table = "products")]
pub struct Product {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    #[embed(prefix = "price_", fields(amount_cents: i64, currency: String))]
    pub price: Money,
}

fn main() {}
