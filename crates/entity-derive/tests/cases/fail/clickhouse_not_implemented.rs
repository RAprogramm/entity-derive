// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "events", dialect = "clickhouse")]
pub struct Event {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub name: String,
}

fn main() {}
