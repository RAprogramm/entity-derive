// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "subscriptions", upsert(conflict = "email"))]
pub struct Subscription {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    #[column(unique)]
    pub email: String,
}

fn main() {}
