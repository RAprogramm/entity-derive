// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Test for `#[column(unique)]` — generates find_by_ and exists_by_ methods.

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Debug, Clone, Entity)]
#[entity(table = "users")]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    #[column(unique)]
    pub email: String,

    #[field(create, update, response)]
    pub name: String,
}

fn main() {
    let _: fn(CreateUserRequest) = |_| {};
    let _: fn(UserResponse) = |_| {};

    fn _check_trait<T: UserRepository>() {}
}
