// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, utoipa::ToSchema, serde::Serialize, serde::Deserialize)]
pub struct Money {
    pub amount_cents: i64,
    pub currency: String,
}

#[derive(Debug, Clone, Entity)]
#[entity(table = "products", migrations)]
pub struct Product {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    #[embed(prefix = "price_", fields(amount_cents: i64, currency: String))]
    pub price: Money,

    #[field(create, response)]
    pub name: String,
}

async fn exercise(pool: sqlx::PgPool) -> Result<(), sqlx::Error> {
    let created: Product = pool
        .create(CreateProductRequest {
            price: Money {
                amount_cents: 1999,
                currency: "USD".into(),
            },
            name: "Widget".into(),
        })
        .await?;
    assert_eq!(created.price.amount_cents, 1999);
    Ok(())
}

fn main() {
    assert!(Product::MIGRATION_UP.contains("price_amount_cents BIGINT NOT NULL"));
    assert!(Product::MIGRATION_UP.contains("price_currency TEXT NOT NULL"));
    assert!(!Product::MIGRATION_UP.contains(" price "));
    let _ = exercise;
}
