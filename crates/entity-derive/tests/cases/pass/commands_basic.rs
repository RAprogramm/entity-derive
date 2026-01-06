// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use entity_derive::{Entity, EntityCommand};
use uuid::Uuid;

#[derive(Entity, Debug, Clone)]
#[entity(table = "users", commands)]
#[command(Register)]
#[command(UpdateEmail: email)]
#[command(Deactivate, requires_id)]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub email: String,

    #[field(create, response)]
    pub name: String,
}

/// Mock command handler.
struct MockHandler;

/// Context for command execution.
struct Context;

#[async_trait]
impl UserCommandHandler for MockHandler {
    type Error = std::io::Error;
    type Context = Context;

    async fn handle_register(
        &self,
        cmd: RegisterUser,
        _ctx: &Self::Context,
    ) -> Result<User, Self::Error> {
        Ok(User {
            id: Uuid::new_v4(),
            email: cmd.email,
            name: cmd.name,
        })
    }

    async fn handle_update_email(
        &self,
        cmd: UpdateEmailUser,
        _ctx: &Self::Context,
    ) -> Result<User, Self::Error> {
        Ok(User {
            id: cmd.id,
            email: cmd.email,
            name: String::from("Updated"),
        })
    }

    async fn handle_deactivate(
        &self,
        _cmd: DeactivateUser,
        _ctx: &Self::Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

fn main() {
    // Verify command structs exist
    let _register = RegisterUser {
        email: String::from("test@example.com"),
        name: String::from("Test User"),
    };

    let _update_email = UpdateEmailUser {
        id: Uuid::new_v4(),
        email: String::from("new@example.com"),
    };

    let _deactivate = DeactivateUser { id: Uuid::new_v4() };

    // Verify command enum exists
    let cmd: UserCommand = UserCommand::Register(_register.clone());
    assert_eq!(cmd.name(), "Register");

    // Verify EntityCommand trait implementation
    assert!(matches!(cmd.kind(), entity_derive::CommandKind::Create));

    // Verify result enum exists
    let _result: UserCommandResult = UserCommandResult::Deactivate;
}
