// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Test custom error type support.
//!
//! Verifies that the `error` attribute is accepted and
//! the generated repository implementation uses the custom type.

use entity_derive::Entity;
use uuid::Uuid;

/// Custom error type for domain layer.
#[derive(Debug)]
pub enum AppError {
    Database(sqlx::Error),
    NotFound,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Database(e) => write!(f, "Database error: {}", e),
            AppError::NotFound => write!(f, "Not found"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err)
    }
}

#[derive(Entity)]
#[entity(table = "products", error = "AppError")]
pub struct Product {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,

    #[field(create, response)]
    pub price: i64,
}

fn main() {
    // Test that DTOs are generated correctly
    let create = CreateProductRequest {
        name: "Widget".to_string(),
        price: 1999,
    };
    assert_eq!(create.name, "Widget");
    assert_eq!(create.price, 1999);

    let update = UpdateProductRequest {
        name: Some("Updated Widget".to_string()),
    };
    assert!(update.name.is_some());

    let response = ProductResponse {
        id: Uuid::nil(),
        name: "Widget".to_string(),
        price: 1999,
    };
    assert_eq!(response.price, 1999);
}
