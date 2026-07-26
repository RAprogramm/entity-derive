// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! One value is bound against every column of the group, so the group
//! has to agree on a type.

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "disputes")]
#[scope(involving: requester_id | reference)]
pub struct Dispute {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub requester_id: Uuid,

    #[field(create, response)]
    pub reference: String,
}

fn main() {}
