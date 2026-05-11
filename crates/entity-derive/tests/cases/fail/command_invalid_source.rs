// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;

/// `source = "invalid"` is not one of the accepted values and must report
/// a diagnostic instead of silently producing zero commands.
#[derive(Entity)]
#[entity(table = "items", commands)]
#[command(Test, source = "invalid")]
pub struct Item {
    #[id]
    pub id: Uuid,

    #[field(create)]
    pub name: String,
}

fn main() {}
