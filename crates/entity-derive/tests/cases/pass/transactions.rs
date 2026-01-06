// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Test that transactions attribute generates correct code.

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "accounts", transactions)]
pub struct Account {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,

    #[field(create, update, response)]
    pub balance: i64,
}

fn main() {
    // Test that TransactionRepo type exists
    fn _assert_repo_exists<'t>(_repo: AccountTransactionRepo<'t>) {}

    // Test that DTOs are still generated
    let _create = CreateAccountRequest {
        name: "Test".to_string(),
        balance: 100,
    };

    let _update = UpdateAccountRequest {
        name: Some("Updated".to_string()),
        balance: None,
    };

    let _response = AccountResponse {
        id: Uuid::nil(),
        name: "Test".to_string(),
        balance: 100,
    };
}
