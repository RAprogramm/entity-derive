// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "orders")]
pub struct Order {
    #[id]
    pub id: Uuid,

    #[owner]
    pub user_id: Uuid,

    #[owner]
    pub tenant_id: Uuid,

    #[field(create, response)]
    pub note: String,
}

fn main() {}
