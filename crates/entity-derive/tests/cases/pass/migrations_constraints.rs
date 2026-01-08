// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "products", migrations)]
pub struct Product {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    #[column(unique)]
    pub sku: String,

    #[field(create, update, response)]
    #[column(varchar = 200)]
    pub name: String,

    #[field(create, update, response)]
    #[column(default = "0")]
    pub quantity: i32,

    #[field(create, update, response)]
    #[column(check = "price >= 0")]
    pub price: f64,

    #[field(create, update, response)]
    #[column(index)]
    pub category: String,

    #[field(create, update, response)]
    #[column(index = "gin")]
    pub tags: Vec<String>,
}

fn main() {
    let up = Product::MIGRATION_UP;

    // Check UNIQUE constraint
    assert!(up.contains("sku TEXT NOT NULL UNIQUE"));

    // Check VARCHAR
    assert!(up.contains("name VARCHAR(200) NOT NULL"));

    // Check DEFAULT
    assert!(up.contains("quantity INTEGER NOT NULL DEFAULT 0"));

    // Check CHECK constraint
    assert!(up.contains("price DOUBLE PRECISION NOT NULL CHECK (price >= 0)"));

    // Check indexes are generated
    assert!(up.contains("CREATE INDEX IF NOT EXISTS idx_products_category"));
    assert!(up.contains("USING gin"));
}
