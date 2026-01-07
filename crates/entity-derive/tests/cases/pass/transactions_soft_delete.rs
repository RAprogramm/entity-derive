// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Test that transactions with soft_delete generates correct code.

use chrono::{DateTime, Utc};
use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "sessions", transactions, soft_delete)]
pub struct Session {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub user_id: Uuid,

    #[field(create, update, response)]
    pub token: String,

    #[field(skip)]
    pub deleted_at: Option<DateTime<Utc>>,
}

fn main() {
    // Test that TransactionRepo type exists
    fn _assert_repo_exists<'t>(_repo: SessionTransactionRepo<'t>) {}

    // Test that DTOs are still generated
    let _create = CreateSessionRequest {
        user_id: Uuid::nil(),
        token: "test_token".to_string(),
    };

    let _update = UpdateSessionRequest {
        token: Some("new_token".to_string()),
    };

    let _response = SessionResponse {
        id: Uuid::nil(),
        user_id: Uuid::nil(),
        token: "test".to_string(),
    };
}
