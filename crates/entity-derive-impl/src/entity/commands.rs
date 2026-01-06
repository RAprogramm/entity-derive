// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! CQRS-style command pattern generation.
//!
//! This module generates command infrastructure for entities with
//! `#[entity(commands)]` enabled.
//!
//! # Architecture
//!
//! ```text
//! commands/
//! ├── mod.rs         — Orchestrator (this file)
//! ├── struct_gen.rs  — Command payload structs (RegisterUser, etc.)
//! ├── enum_gen.rs    — Command enum (UserCommand)
//! ├── result_gen.rs  — Result enum (UserCommandResult)
//! └── handler_gen.rs — Handler trait (UserCommandHandler)
//! ```
//!
//! # Generated Code
//!
//! For an entity like:
//!
//! ```rust,ignore
//! #[derive(Entity)]
//! #[entity(table = "users", commands)]
//! #[command(Register)]
//! #[command(UpdateEmail: email)]
//! #[command(Deactivate, requires_id)]
//! pub struct User {
//!     #[id]
//!     pub id: Uuid,
//!     #[field(create, update, response)]
//!     pub email: String,
//!     #[field(create, response)]
//!     pub name: String,
//! }
//! ```
//!
//! The macro generates:
//!
//! | Type | Purpose |
//! |------|---------|
//! | `RegisterUser` | Command payload for registration |
//! | `UpdateEmailUser` | Command payload for email update |
//! | `DeactivateUser` | Command payload for deactivation |
//! | `UserCommand` | Enum wrapping all commands |
//! | `UserCommandResult` | Enum for command results |
//! | `UserCommandHandler` | Async trait for handling commands |
//!
//! # Usage
//!
//! ```rust,ignore
//! struct MyHandler { pool: PgPool }
//!
//! #[async_trait]
//! impl UserCommandHandler for MyHandler {
//!     type Error = AppError;
//!     type Context = RequestContext;
//!
//!     async fn handle_register(
//!         &self,
//!         cmd: RegisterUser,
//!         ctx: &Self::Context
//!     ) -> Result<User, Self::Error> {
//!         // Business logic here
//!         let user = create_user(&self.pool, cmd).await?;
//!         send_welcome_email(&user.email).await?;
//!         Ok(user)
//!     }
//!
//!     async fn handle_update_email(
//!         &self,
//!         cmd: UpdateEmailUser,
//!         ctx: &Self::Context
//!     ) -> Result<User, Self::Error> {
//!         // Validation, update, notification
//!         Ok(updated_user)
//!     }
//!
//!     async fn handle_deactivate(
//!         &self,
//!         cmd: DeactivateUser,
//!         ctx: &Self::Context
//!     ) -> Result<(), Self::Error> {
//!         // Deactivation logic
//!         Ok(())
//!     }
//! }
//! ```

mod enum_gen;
mod handler_gen;
mod result_gen;
mod struct_gen;

use proc_macro2::TokenStream;
use quote::quote;

use super::parse::EntityDef;

/// Main entry point for command pattern code generation.
///
/// Returns empty `TokenStream` if `commands` is not enabled
/// or no commands are defined.
pub fn generate(entity: &EntityDef) -> TokenStream {
    if !entity.has_commands() {
        return TokenStream::new();
    }

    if entity.command_defs().is_empty() {
        return TokenStream::new();
    }

    let structs = struct_gen::generate(entity);
    let command_enum = enum_gen::generate(entity);
    let result_enum = result_gen::generate(entity);
    let handler_trait = handler_gen::generate(entity);

    quote! {
        #structs
        #command_enum
        #result_enum
        #handler_trait
    }
}
