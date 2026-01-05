// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Entity)]
#[entity(table = "products")]
pub struct Product {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,

    #[field(create, response)]
    pub sku: String,

    #[field(update, response)]
    pub price: f64,

    #[field(response)]
    pub views: i64,

    #[field(skip)]
    pub internal_cost: f64,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,

    #[field(response)]
    #[auto]
    pub updated_at: DateTime<Utc>,
}

fn main() {
    // CreateProductRequest has: name, sku
    let create = CreateProductRequest {
        name: "Widget".to_string(),
        sku: "WDG-001".to_string(),
    };
    assert_eq!(create.name, "Widget");
    assert_eq!(create.sku, "WDG-001");

    // UpdateProductRequest has: name, price (both Option)
    let update = UpdateProductRequest {
        name: Some("Super Widget".to_string()),
        price: Some(29.99),
    };
    assert!(update.name.is_some());
    assert!(update.price.is_some());

    // ProductResponse has: id, name, sku, price, views, created_at, updated_at
    let response = ProductResponse {
        id: Uuid::nil(),
        name: "Widget".to_string(),
        sku: "WDG-001".to_string(),
        price: 19.99,
        views: 100,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    assert_eq!(response.views, 100);
}
