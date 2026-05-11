// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;

/// `#[map(expr = "...")]` with non-parseable Rust must report a clear
/// compile error at the use site instead of silently producing empty
/// tokens (which previously caused cryptic unrelated errors).
#[derive(Entity)]
#[entity(table = "items")]
pub struct Item {
    #[id]
    pub id: Uuid,

    #[field(response)]
    #[map(expr = "invalid $$$ rust")]
    pub broken: i32,
}

fn main() {}
