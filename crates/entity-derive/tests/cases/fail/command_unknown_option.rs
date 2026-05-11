// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;

/// `#[command(..., bogus_option)]` must produce a compile error naming the
/// offending option, instead of silently dropping the whole command (the
/// pre-#129 behavior).
#[derive(Entity)]
#[entity(table = "items", commands)]
#[command(Test, bogus_option)]
pub struct Item {
    #[id]
    pub id: Uuid,

    #[field(create)]
    pub name: String,
}

fn main() {}
