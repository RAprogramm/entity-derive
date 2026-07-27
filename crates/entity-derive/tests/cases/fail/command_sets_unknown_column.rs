// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! A domain operation writing a column the entity does not have is
//! rejected at the declaration.

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "users", commands)]
#[command(VerifyPassport, sets(passport_checked = "true"))]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(response)]
    pub passport_verified: bool,
}

fn main() {}
