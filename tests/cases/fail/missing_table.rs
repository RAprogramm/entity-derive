// SPDX-FileCopyrightText: 2025 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(schema = "core")]  // Missing required `table` attribute
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub name: String,
}

fn main() {}
