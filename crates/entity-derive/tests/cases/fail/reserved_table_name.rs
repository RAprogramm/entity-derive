// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! A reserved SQL word as a table name would produce SQL that only
//! fails at runtime.

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "user")]
pub struct Account {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,
}

fn main() {}
