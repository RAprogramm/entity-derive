// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use entity_derive::{Entity, EntityCommand};
use uuid::Uuid;

/// Custom transfer payload.
#[derive(Debug, Clone)]
pub struct TransferPayload {
    pub from_account: Uuid,
    pub to_account: Uuid,
    pub amount: i64,
}

/// Custom transfer result.
#[derive(Debug, Clone)]
pub struct TransferResult {
    pub transaction_id: Uuid,
    pub success: bool,
}

#[derive(Entity, Debug, Clone)]
#[entity(table = "accounts", commands)]
#[command(Create)]
#[command(Transfer, payload = "TransferPayload", result = "TransferResult")]
pub struct Account {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub owner: String,

    #[field(response)]
    pub balance: i64,
}

/// Mock command handler.
struct MockHandler;

/// Context for command execution.
struct Context;

#[async_trait]
impl AccountCommandHandler for MockHandler {
    type Error = std::io::Error;
    type Context = Context;

    async fn handle_create(
        &self,
        cmd: CreateAccount,
        _ctx: &Self::Context,
    ) -> Result<Account, Self::Error> {
        Ok(Account {
            id: Uuid::new_v4(),
            owner: cmd.owner,
            balance: 0,
        })
    }

    async fn handle_transfer(
        &self,
        cmd: TransferPayload,
        _ctx: &Self::Context,
    ) -> Result<TransferResult, Self::Error> {
        Ok(TransferResult {
            transaction_id: Uuid::new_v4(),
            success: cmd.amount > 0,
        })
    }
}

fn main() {
    // Verify custom payload command
    let transfer = TransferPayload {
        from_account: Uuid::new_v4(),
        to_account: Uuid::new_v4(),
        amount: 100,
    };

    // Verify command enum contains Transfer variant
    let cmd = AccountCommand::Transfer(transfer);
    assert_eq!(cmd.name(), "Transfer");

    // Verify result enum has custom result type
    let result = TransferResult {
        transaction_id: Uuid::new_v4(),
        success: true,
    };
    let _enum_result = AccountCommandResult::Transfer(result);
}
