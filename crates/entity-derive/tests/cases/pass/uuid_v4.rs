// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Test UUID v4 generation option.

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "sessions", uuid = "v4", sql = "none")]
pub struct Session {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub user_id: Uuid,

    #[field(create, response)]
    pub token: String,
}

fn main() {
    // Test that CreateSessionRequest exists and works
    let create = CreateSessionRequest {
        user_id: Uuid::nil(),
        token: "abc123".to_string(),
    };
    assert_eq!(create.token, "abc123");

    // Test Session creation via From
    let session = Session::from(create);
    // ID should be generated (not nil)
    assert_ne!(session.id, Uuid::nil());

    // Test response
    let response = SessionResponse {
        id: session.id,
        user_id: session.user_id,
        token: session.token.clone(),
    };
    assert_eq!(response.token, "abc123");
}
