// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Test for `#[column(unique, index)]` — generates both find_by_ and exists_by_ methods.

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Debug, Clone, Entity)]
#[entity(table = "products")]
pub struct Product {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    #[column(unique, index)]
    pub sku: String,

    #[field(create, update, response)]
    #[column(index)]
    pub status: String,

    #[field(create, update, response)]
    pub name: String,
}

fn main() {
    let _: fn(CreateProductRequest) = |_| {};
    let _: fn(ProductResponse) = |_| {};

    fn _check_trait<T: ProductRepository>() {}
}
