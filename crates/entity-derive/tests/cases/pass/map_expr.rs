// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Test for `#[map(expr = "...")]` escape hatch.

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "products")]
pub struct Product {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,

    #[field(response)]
    pub price_cents: i32,

    #[field(response)]
    pub is_featured: bool,
}

fn main() {
    // Verify generated types exist
    let _: fn(CreateProductRequest) = |_| {};
    let _: fn(ProductResponse) = |_| {};

    // Verify repository trait exists
    fn _check_trait<T: ProductRepository>() {}
}
