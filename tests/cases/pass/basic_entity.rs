// SPDX-FileCopyrightText: 2025 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Entity)]
#[entity(table = "users", schema = "core")]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,

    #[field(create, update, response)]
    pub email: String,

    #[field(skip)]
    pub password_hash: String,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,
}

fn main() {
    // Test CreateUserRequest
    let create = CreateUserRequest {
        name: "John".to_string(),
        email: "john@example.com".to_string(),
    };
    assert_eq!(create.name, "John");

    // Test UpdateUserRequest
    let update = UpdateUserRequest {
        name: Some("Jane".to_string()),
        email: None,
    };
    assert!(update.name.is_some());
    assert!(update.email.is_none());

    // Test UserResponse
    let response = UserResponse {
        id: Uuid::nil(),
        name: "John".to_string(),
        email: "john@example.com".to_string(),
        created_at: Utc::now(),
    };
    assert_eq!(response.name, "John");
}
