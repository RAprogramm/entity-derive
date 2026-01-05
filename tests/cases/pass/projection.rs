// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Test for `#[projection]` attribute.

use chrono::{DateTime, Utc};
use entity_derive::Entity;
use uuid::Uuid;

#[derive(Debug, Clone, Entity)]
#[entity(table = "users")]
#[projection(Public: id, name)]
#[projection(Admin: id, name, email, created_at)]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,

    #[field(create, response)]
    pub email: String,

    #[field(skip)]
    pub password_hash: String,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,
}

fn main() {
    // Verify projection structs exist
    let _: fn(UserPublic) = |_| {};
    let _: fn(UserAdmin) = |_| {};

    // Verify From impls exist
    fn _check_from_public(_: impl From<User>) {}
    fn _check_from_admin(_: impl From<User>) {}

    _check_from_public(UserPublic { id: Uuid::nil(), name: String::new() });
    _check_from_admin(UserAdmin {
        id: Uuid::nil(),
        name: String::new(),
        email: String::new(),
        created_at: Utc::now(),
    });
}
