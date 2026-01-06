// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;

/// Entity without #[id] field should fail.
#[derive(Entity)]
#[entity(table = "items")]
pub struct Item {
    pub name: String,
    pub value: i32,
}

fn main() {}
