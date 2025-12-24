// SPDX-FileCopyrightText: 2025 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Entity with no create fields - only ID and response fields.
//! Tests that empty create/update DTOs are not generated.

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "logs", sql = "trait")]
pub struct Log {
    #[id]
    pub id: Uuid,

    /// Response-only field (not in create or update)
    #[field(response)]
    pub message: String,

    #[field(response)]
    pub level: i32,
}

fn main() {
    // LogResponse should exist
    let response = LogResponse {
        id: Uuid::nil(),
        message: "Test log".to_string(),
        level: 1,
    };
    assert_eq!(response.level, 1);

    // CreateLogRequest should NOT exist (no create fields)
    // UpdateLogRequest should NOT exist (no update fields)
}
