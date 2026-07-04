// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "users", returning = "id", upsert(conflict = "email"))]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    #[column(unique)]
    pub email: String,

    #[field(create, update, response)]
    pub name: String,
}

fn main() {}
