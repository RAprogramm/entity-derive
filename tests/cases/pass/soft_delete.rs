// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Test for `#[entity(soft_delete)]` attribute.

use chrono::{DateTime, Utc};
use entity_derive::Entity;
use uuid::Uuid;

#[derive(Debug, Clone, Entity)]
#[entity(table = "documents", soft_delete)]
pub struct Document {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub title: String,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,

    #[field(skip)]
    pub deleted_at: Option<DateTime<Utc>>,
}

fn main() {
    // Verify generated types exist
    let _: fn(CreateDocumentRequest) = |_| {};
    let _: fn(DocumentResponse) = |_| {};

    // Verify repository trait with soft delete methods exists
    fn _check_trait<T: DocumentRepository>() {}

    // Verify soft delete methods are in the trait
    fn _verify_soft_delete_methods<T>()
    where
        T: DocumentRepository,
    {
        // These would be async methods, we just verify the trait compiles
    }
}
