// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;

/// Malformed nested meta inside `#[map(...)]` must surface a compile error
/// from the attribute parser rather than silently degrading to
/// `MapConfig::None`.
#[derive(Entity)]
#[entity(table = "items")]
pub struct Item {
    #[id]
    pub id: Uuid,

    #[field(response)]
    #[map(empty_to_none, expr(missing_equals))]
    pub broken: Option<String>,
}

fn main() {}
