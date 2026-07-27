// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! An explicit column name that needs quoting is rejected at the
//! attribute instead of failing when the statement runs.

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "accounts")]
pub struct Account {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    #[column(name = "Full Name")]
    pub name: String,
}

fn main() {}
