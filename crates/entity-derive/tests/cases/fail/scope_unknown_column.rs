// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! A scope naming a column the entity does not have is rejected where
//! it is declared.

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "disputes")]
#[scope(involving: requester_id | reviewer_id)]
pub struct Dispute {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub requester_id: Uuid,
}

fn main() {}
