// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Entity with only response fields - no create or update.
//! Tests empty create/update method paths in SQL generation.

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "audit_logs", schema = "audit", sql = "trait")]
pub struct AuditLog {
    #[id]
    pub id: Uuid,

    /// Response-only fields
    #[field(response)]
    pub action: String,

    #[field(response)]
    pub details: String,

    #[field(response)]
    pub timestamp: i64,
}

fn main() {
    // AuditLogResponse should exist
    let response = AuditLogResponse {
        id: Uuid::nil(),
        action: "login".to_string(),
        details: "User logged in".to_string(),
        timestamp: 12345,
    };
    assert_eq!(response.action, "login");
}
