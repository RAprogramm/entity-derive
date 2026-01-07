// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Command type definitions and data structures.
//!
//! This module defines the types used to represent parsed command definitions.
//! These types capture all configuration from `#[command(...)]` attributes
//! and are used by code generation to produce command structs, enums, and
//! handler traits.
//!
//! # Type Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                   Command Types                                 │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │  CommandDef                                                     │
//! │  ├─► name: Ident           # Command name (e.g., "Register")    │
//! │  ├─► source: CommandSource # Where to get fields                │
//! │  ├─► requires_id: bool     # Needs entity ID?                   │
//! │  ├─► result_type: Option<Type>  # Custom result                 │
//! │  ├─► kind: CommandKindHint # Categorization                     │
//! │  └─► security: Option<String>   # Security override             │
//! │                                                                 │
//! │  CommandSource                                                  │
//! │  ├─► Create   # Use #[field(create)] fields                     │
//! │  ├─► Update   # Use #[field(update)] fields                     │
//! │  ├─► Fields   # Use specific named fields                       │
//! │  ├─► Custom   # Use external payload struct                     │
//! │  └─► None     # No payload fields                               │
//! │                                                                 │
//! │  CommandKindHint                                                │
//! │  ├─► Create   # Creates new entity                              │
//! │  ├─► Update   # Modifies existing entity                        │
//! │  ├─► Delete   # Removes entity                                  │
//! │  └─► Custom   # Business-specific operation                     │
//! │                                                                 │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Field Selection
//!
//! `CommandSource` determines which entity fields appear in the command struct:
//!
//! | Source | Behavior |
//! |--------|----------|
//! | `Create` | Include fields with `#[field(create)]` |
//! | `Update` | Include fields with `#[field(update)]` |
//! | `Fields(vec)` | Include only the named fields |
//! | `Custom(ty)` | Use the specified type directly |
//! | `None` | No fields (ID-only or action commands) |
//!
//! # Naming Conventions
//!
//! Command names are transformed for generated code:
//!
//! | Method | Input | Output |
//! |--------|-------|--------|
//! | `struct_name("User")` | `Register` | `RegisterUser` |
//! | `handler_method_name()` | `UpdateEmail` | `handle_update_email` |

use proc_macro2::Span;
use syn::{Ident, Type};

/// Determines the source of fields for a command payload.
///
/// The source specifies which entity fields should be included in the
/// generated command struct. This enables flexible command definitions
/// that can share fields with CRUD DTOs or define custom payloads.
///
/// # Variants
///
/// ```text
/// CommandSource
///     │
///     ├─► Create ──► Fields from #[field(create)]
///     │
///     ├─► Update ──► Fields from #[field(update)]
///     │
///     ├─► Fields ──► Explicitly listed fields
///     │
///     ├─► Custom ──► External struct type
///     │
///     └─► None ────► No payload (ID-only)
/// ```
///
/// # Examples
///
/// | Attribute | Source |
/// |-----------|--------|
/// | `#[command(Register)]` | `Create` |
/// | `#[command(Modify, source = "update")]` | `Update` |
/// | `#[command(UpdateEmail: email)]` | `Fields(["email"])` |
/// | `#[command(Transfer, payload = "TransferPayload")]` | `Custom(TransferPayload)` |
/// | `#[command(Delete, requires_id)]` | `None` |
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CommandSource {
    /// Use fields marked with `#[field(create)]`.
    ///
    /// Default for commands that create new entities.
    #[default]
    Create,

    /// Use fields marked with `#[field(update)]`.
    ///
    /// For commands that modify existing entities.
    Update,

    /// Use specific fields listed after colon.
    ///
    /// Example: `#[command(UpdateEmail: email)]`
    Fields(Vec<Ident>),

    /// Use a custom payload struct.
    ///
    /// Example: `#[command(Transfer, payload = "TransferPayload")]`
    Custom(Type),

    /// No fields in payload.
    ///
    /// Combined with `requires_id` for id-only commands.
    None
}

/// Kind of command for categorization.
///
/// Inferred from source or explicitly specified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CommandKindHint {
    /// Creates new entity.
    #[default]
    Create,

    /// Modifies existing entity.
    Update,

    /// Removes entity.
    Delete,

    /// Custom business operation.
    Custom
}

/// A command definition parsed from `#[command(...)]`.
///
/// # Fields
///
/// | Field | Description |
/// |-------|-------------|
/// | `name` | Command name (e.g., `Register`, `UpdateEmail`) |
/// | `source` | Where to get fields for the command payload |
/// | `requires_id` | Whether command requires entity ID parameter |
/// | `result_type` | Custom result type (default: entity or unit) |
/// | `kind` | Command kind hint for categorization |
///
/// # Example
///
/// For `#[command(Register)]`:
/// ```rust,ignore
/// CommandDef {
///     name: Ident("Register"),
///     source: CommandSource::Create,
///     requires_id: false,
///     result_type: None,
///     kind: CommandKindHint::Create
/// }
/// ```
#[derive(Debug, Clone)]
pub struct CommandDef {
    /// Command name (e.g., `Register`, `UpdateEmail`).
    pub name: Ident,

    /// Source of fields for the command payload.
    pub source: CommandSource,

    /// Whether the command requires an entity ID.
    ///
    /// When `true`, the command struct includes an `id` field
    /// and handler receives the ID separately.
    pub requires_id: bool,

    /// Custom result type for this command.
    ///
    /// When `None`, returns the entity for create/update commands
    /// or unit `()` for delete commands.
    pub result_type: Option<Type>,

    /// Kind hint for command categorization.
    pub kind: CommandKindHint,

    /// Security scheme override for this command.
    ///
    /// When set, overrides the entity-level default security.
    /// Use `"none"` to make a command public.
    pub security: Option<String>
}

impl CommandDef {
    /// Create a new command definition with defaults.
    ///
    /// # Arguments
    ///
    /// * `name` - Command name identifier
    pub fn new(name: Ident) -> Self {
        Self {
            name,
            source: CommandSource::default(),
            requires_id: false,
            result_type: None,
            kind: CommandKindHint::default(),
            security: None
        }
    }

    /// Get the full command struct name.
    ///
    /// Combines command name with entity name.
    ///
    /// # Arguments
    ///
    /// * `entity_name` - The entity name (e.g., "User")
    ///
    /// # Returns
    ///
    /// Full command name (e.g., "RegisterUser")
    pub fn struct_name(&self, entity_name: &str) -> Ident {
        Ident::new(&format!("{}{}", self.name, entity_name), Span::call_site())
    }

    /// Get the handler method name.
    ///
    /// Converts command name to snake_case handler method.
    ///
    /// # Returns
    ///
    /// Handler method name (e.g., "handle_register")
    pub fn handler_method_name(&self) -> Ident {
        use convert_case::{Case, Casing};
        let snake = self.name.to_string().to_case(Case::Snake);
        Ident::new(&format!("handle_{}", snake), Span::call_site())
    }

    /// Check if this command has explicit security override.
    #[must_use]
    #[allow(dead_code)]
    pub fn has_security_override(&self) -> bool {
        self.security.is_some()
    }

    /// Check if this command is explicitly marked as public.
    ///
    /// Returns `true` if `security = "none"` is set.
    #[must_use]
    pub fn is_public(&self) -> bool {
        self.security.as_deref() == Some("none")
    }

    /// Get the security scheme for this command.
    ///
    /// Returns command-level override if set, otherwise `None`.
    #[must_use]
    pub fn security(&self) -> Option<&str> {
        self.security.as_deref()
    }
}
