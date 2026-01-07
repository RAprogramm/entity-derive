// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Command definition and parsing.
//!
//! Commands define business operations on entities, following CQRS pattern.
//! Instead of generic CRUD, you get domain-specific commands like
//! `RegisterUser`, `UpdateEmail`, `DeactivateAccount`.
//!
//! # Syntax
//!
//! ```rust,ignore
//! #[command(Register)]                              // uses create fields
//! #[command(UpdateEmail: email)]                    // specific fields only
//! #[command(Deactivate, requires_id)]               // id only, no fields
//! #[command(Transfer, payload = "TransferPayload")] // custom payload struct
//! ```
//!
//! # Generated Code
//!
//! Each command generates:
//! - A command struct (e.g., `RegisterUser`)
//! - An entry in `UserCommand` enum
//! - An entry in `UserCommandResult` enum
//! - A handler method in `UserCommandHandler` trait

mod parser;
mod types;

pub use parser::parse_command_attrs;
pub use types::{CommandDef, CommandKindHint, CommandSource};

#[cfg(test)]
mod tests;
