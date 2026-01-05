// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use chrono::{DateTime, Utc};
use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "products", schema = "inventory")]
pub struct Product {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    #[filter]
    pub name: String,

    #[field(create, update, response)]
    #[filter(like)]
    pub description: String,

    #[field(create, update, response)]
    #[filter(eq)]
    pub category: String,

    #[field(create, update, response)]
    #[filter(range)]
    pub price: i64,

    #[field(response)]
    #[auto]
    #[filter(range)]
    pub created_at: DateTime<Utc>,
}

fn main() {
    // Test ProductQuery struct with default values
    let query = ProductQuery::default();
    assert!(query.name.is_none());
    assert!(query.description.is_none());
    assert!(query.category.is_none());
    assert!(query.price_from.is_none());
    assert!(query.price_to.is_none());
    assert!(query.created_at_from.is_none());
    assert!(query.created_at_to.is_none());
    assert!(query.limit.is_none());
    assert!(query.offset.is_none());

    // Test ProductQuery with filter values
    let query = ProductQuery {
        name: Some("Widget".to_string()),
        description: Some("electronic".to_string()),
        category: Some("Electronics".to_string()),
        price_from: Some(100),
        price_to: Some(500),
        created_at_from: None,
        created_at_to: None,
        limit: Some(10),
        offset: Some(0),
    };
    assert_eq!(query.name.as_deref(), Some("Widget"));
    assert_eq!(query.description.as_deref(), Some("electronic"));
    assert_eq!(query.category.as_deref(), Some("Electronics"));
    assert_eq!(query.price_from, Some(100));
    assert_eq!(query.price_to, Some(500));
    assert_eq!(query.limit, Some(10));
    assert_eq!(query.offset, Some(0));
}
