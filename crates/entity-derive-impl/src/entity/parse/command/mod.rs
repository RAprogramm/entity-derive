// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Command definition and parsing for CQRS-style operations.
//!
//! This module handles parsing of `#[command(...)]` attributes that define
//! domain-specific business operations on entities. Commands follow the
//! CQRS (Command Query Responsibility Segregation) pattern, providing
//! explicit, named operations instead of generic CRUD.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                     Command Parsing                                 │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │  Attribute                     Parser               Output          │
//! │                                                                     │
//! │  #[command(Register)]    parse_command_attrs()   CommandDef        │
//! │  #[command(UpdateEmail:        │                    │              │
//! │    email, name)]               │                    ├── name       │
//! │  #[command(Deactivate,         │                    ├── source     │
//! │    requires_id)]               │                    ├── requires_id│
//! │                                ▼                    └── kind       │
//! │                                                                     │
//! │                         Vec<CommandDef>                             │
//! │                               │                                     │
//! │                               ▼                                     │
//! │                         Code Generation                             │
//! │                         ├── RegisterUser struct                     │
//! │                         ├── UpdateEmailUser struct                  │
//! │                         ├── UserCommand enum                        │
//! │                         └── UserCommandHandler trait                │
//! │                                                                     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Command Syntax
//!
//! Commands support multiple syntax forms:
//!
//! ## Simple Command
//!
//! Uses fields marked with `#[field(create)]`:
//!
//! ```rust,ignore
//! #[command(Register)]
//! ```
//!
//! ## Field-Specific Command
//!
//! Uses only the listed fields (requires ID):
//!
//! ```rust,ignore
//! #[command(UpdateEmail: email)]
//! #[command(UpdateProfile: name, avatar, bio)]
//! ```
//!
//! ## ID-Only Command
//!
//! No fields, just the entity ID:
//!
//! ```rust,ignore
//! #[command(Deactivate, requires_id)]
//! #[command(Delete, requires_id, kind = "delete")]
//! ```
//!
//! ## Custom Payload Command
//!
//! Uses an external struct for the payload:
//!
//! ```rust,ignore
//! #[command(Transfer, payload = "TransferPayload")]
//! #[command(Transfer, payload = "TransferPayload", result = "TransferResult")]
//! ```
//!
//! # Command Options
//!
//! | Option | Type | Description |
//! |--------|------|-------------|
//! | `requires_id` | flag | Command needs entity ID |
//! | `source` | string | Field source: `"create"`, `"update"`, `"none"` |
//! | `payload` | string | Custom payload struct type |
//! | `result` | string | Custom result type |
//! | `kind` | string | Kind hint: `"create"`, `"update"`, `"delete"`, `"custom"` |
//! | `security` | string | Security override: scheme name or `"none"` |
//!
//! # Generated Code
//!
//! For entity `User` with commands:
//!
//! ```rust,ignore
//! #[command(Register)]
//! #[command(UpdateEmail: email)]
//! #[command(Deactivate, requires_id)]
//! ```
//!
//! Generates:
//!
//! ```rust,ignore
//! // Command structs
//! pub struct RegisterUser {
//!     pub name: String,
//!     pub email: String,
//! }
//!
//! pub struct UpdateEmailUser {
//!     pub id: Uuid,
//!     pub email: String,
//! }
//!
//! pub struct DeactivateUser {
//!     pub id: Uuid,
//! }
//!
//! // Command enum
//! pub enum UserCommand {
//!     Register(RegisterUser),
//!     UpdateEmail(UpdateEmailUser),
//!     Deactivate(DeactivateUser),
//! }
//!
//! // Handler trait
//! #[async_trait]
//! pub trait UserCommandHandler {
//!     async fn handle_register(&self, cmd: RegisterUser) -> Result<User, Error>;
//!     async fn handle_update_email(&self, cmd: UpdateEmailUser) -> Result<User, Error>;
//!     async fn handle_deactivate(&self, cmd: DeactivateUser) -> Result<(), Error>;
//! }
//! ```
//!
//! # Module Structure
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`types`] | Type definitions: `CommandDef`, `CommandSource`, `CommandKindHint` |
//! | [`parser`] | Attribute parsing: `parse_command_attrs` |

mod parser;
mod types;

pub use parser::parse_command_attrs;
pub use types::{CommandDef, CommandKindHint, CommandSource};

#[cfg(test)]
mod tests;
