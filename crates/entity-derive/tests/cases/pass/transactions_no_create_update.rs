// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Test transactions with entity that has no create/update fields.

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "audit_logs", transactions)]
pub struct AuditLog {
    #[id]
    pub id: Uuid,

    #[field(response)]
    pub action: String,

    #[field(response)]
    pub timestamp: i64,
}

fn main() {
    // Test that TransactionRepo type exists
    fn _assert_repo_exists<'t>(_repo: AuditLogTransactionRepo<'t>) {}

    // No CreateAuditLogRequest since no create fields
    // No UpdateAuditLogRequest since no update fields

    let _response = AuditLogResponse {
        id: Uuid::nil(),
        action: "test".to_string(),
        timestamp: 0,
    };
}
