// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Test for `#[map(empty_to_none)]` attribute.

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "users")]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,

    #[field(response)]
    #[map(empty_to_none)]
    pub nickname: Option<String>,

    #[field(response)]
    #[map(empty_to_none)]
    pub bio: Option<String>,
}

fn main() {
    // Verify generated types exist
    let _: fn(CreateUserRequest) = |_| {};
    let _: fn(UserResponse) = |_| {};

    // Verify repository trait exists
    fn _check_trait<T: UserRepository>() {}

    // Verify From<CreateUserRequest> for User compiles
    let create = CreateUserRequest {
        name: "John".to_string(),
    };
    let _user: User = create.into();
}
