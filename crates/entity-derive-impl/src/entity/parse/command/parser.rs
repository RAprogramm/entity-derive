// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Command attribute parsing from `#[command(...)]`.
//!
//! This module provides the parser that extracts command definitions from
//! `#[command(...)]` attributes on entity structs. It handles all syntax
//! variations and produces `CommandDef` instances for code generation.
//!
//! # Parsing Architecture
//!
//! ```text
//! Input Attributes              Parser                    Output
//!
//! #[command(Register)]     parse_command_attrs()    Vec<CommandDef>
//! #[command(Update: email)]       │                       │
//! #[command(Delete,               │                       ├── CommandDef {
//!   requires_id)]                 │                       │     name: "Register"
//!         │                       │                       │     source: Create
//!         ▼                       │                       │   }
//! &[Attribute] ──────────────────►│                       ├── CommandDef {
//!                                 │                       │     name: "Update"
//!                                 │                       │     source: Fields
//!                                 │                       │   }
//!                                 │                       └── ...
//!                                 ▼
//!                           filter "command"
//!                           parse_single_command()
//!                                 │
//!                                 ▼
//!                           Vec<CommandDef>
//! ```
//!
//! # Syntax Forms
//!
//! The parser supports several syntax variations:
//!
//! ## Basic Command
//!
//! ```rust,ignore
//! #[command(Register)]  // Uses create fields, no ID
//! ```
//!
//! ## Field Selection with Colon
//!
//! ```rust,ignore
//! #[command(UpdateEmail: email)]        // Single field
//! #[command(UpdateProfile: name, bio)]  // Multiple fields
//! ```
//!
//! ## Options After Comma
//!
//! ```rust,ignore
//! #[command(Delete, requires_id)]
//! #[command(Modify, source = "update")]
//! #[command(Process, kind = "custom")]
//! #[command(Transfer, payload = "TransferPayload")]
//! #[command(AdminOp, security = "admin")]
//! ```
//!
//! # Option Reference
//!
//! | Option | Syntax | Effect |
//! |--------|--------|--------|
//! | `requires_id` | flag | Sets `requires_id = true`, source to `None` |
//! | `source` | `= "create/update/none"` | Sets field source |
//! | `payload` | `= "TypeName"` | Uses custom payload type |
//! | `result` | `= "TypeName"` | Uses custom result type |
//! | `kind` | `= "create/update/delete/custom"` | Sets kind hint |
//! | `security` | `= "scheme/none"` | Sets security override |
//!
//! # Error Handling
//!
//! Invalid commands are silently filtered out (via `filter_map`).
//! This allows partial compilation with some valid commands even if
//! others have syntax errors.

use syn::{Attribute, Ident, Type};

use super::types::{CommandDef, CommandKindHint, CommandSource};

/// Parses all `#[command(...)]` attributes from a struct.
///
/// This function filters struct attributes for `#[command(...)]`, parses
/// each one, and collects valid command definitions. Invalid commands are
/// silently skipped to allow partial success.
///
/// # Arguments
///
/// * `attrs` - Slice of `syn::Attribute` from the struct definition
///
/// # Returns
///
/// A `Vec<CommandDef>` containing all successfully parsed commands.
/// May be empty if no valid commands are found.
///
/// # Parsing Process
///
/// ```text
/// attrs.iter()
///     │
///     ├─► filter(is "command") ──► Only #[command(...)] attrs
///     │
///     ├─► filter_map(parse) ────► Parse each, skip errors
///     │
///     └─► collect() ────────────► Vec<CommandDef>
/// ```
///
/// # Syntax Examples
///
/// ```text
/// // Basic command (uses create fields)
/// #[command(Register)]
///
/// // Explicit source selection
/// #[command(Register, source = "create")]
///
/// // Specific fields (colon syntax)
/// #[command(UpdateEmail: email)]
/// #[command(UpdateProfile: name, avatar, bio)]
///
/// // ID-only command
/// #[command(Deactivate, requires_id)]
/// #[command(Delete, requires_id, kind = "delete")]
///
/// // Custom payload
/// #[command(Transfer, payload = "TransferPayload")]
///
/// // Custom result
/// #[command(Transfer, payload = "TransferPayload", result = "TransferResult")]
///
/// // Security override
/// #[command(PublicList, security = "none")]
/// #[command(AdminDelete, requires_id, security = "admin")]
/// ```
pub fn parse_command_attrs(attrs: &[Attribute]) -> Vec<CommandDef> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("command"))
        .filter_map(|attr| parse_single_command(attr).ok())
        .collect()
}

/// Parse a single `#[command(...)]` attribute.
fn parse_single_command(attr: &Attribute) -> syn::Result<CommandDef> {
    attr.parse_args_with(|input: syn::parse::ParseStream<'_>| {
        let name: Ident = input.parse()?;
        let mut cmd = CommandDef::new(name);

        if input.peek(syn::Token![:]) && !input.peek2(syn::Token![:]) {
            let _: syn::Token![:] = input.parse()?;
            let fields =
                syn::punctuated::Punctuated::<Ident, syn::Token![,]>::parse_separated_nonempty(
                    input
                )?;
            cmd.source = CommandSource::Fields(fields.into_iter().collect());
            cmd.requires_id = true;
            cmd.kind = CommandKindHint::Update;
            return Ok(cmd);
        }

        while input.peek(syn::Token![,]) {
            let _: syn::Token![,] = input.parse()?;

            if input.is_empty() {
                break;
            }

            let option_name: Ident = input.parse()?;
            let option_str = option_name.to_string();

            match option_str.as_str() {
                "requires_id" => {
                    cmd.requires_id = true;
                    if matches!(cmd.source, CommandSource::Create) {
                        cmd.source = CommandSource::None;
                        cmd.kind = CommandKindHint::Update;
                    }
                }
                "source" => {
                    let _: syn::Token![=] = input.parse()?;
                    let source_lit: syn::LitStr = input.parse()?;
                    let source_val = source_lit.value();
                    match source_val.as_str() {
                        "create" => cmd.source = CommandSource::Create,
                        "update" => {
                            cmd.source = CommandSource::Update;
                            cmd.requires_id = true;
                            cmd.kind = CommandKindHint::Update;
                        }
                        "none" => cmd.source = CommandSource::None,
                        _ => {
                            return Err(syn::Error::new(
                                source_lit.span(),
                                "source must be \"create\", \"update\", or \"none\""
                            ));
                        }
                    }
                }
                "payload" => {
                    let _: syn::Token![=] = input.parse()?;
                    let payload_lit: syn::LitStr = input.parse()?;
                    let payload_str = payload_lit.value();
                    let ty: Type = syn::parse_str(&payload_str)?;
                    cmd.source = CommandSource::Custom(ty);
                    cmd.kind = CommandKindHint::Custom;
                }
                "result" => {
                    let _: syn::Token![=] = input.parse()?;
                    let result_lit: syn::LitStr = input.parse()?;
                    let result_str = result_lit.value();
                    let ty: Type = syn::parse_str(&result_str)?;
                    cmd.result_type = Some(ty);
                }
                "kind" => {
                    let _: syn::Token![=] = input.parse()?;
                    let kind_lit: syn::LitStr = input.parse()?;
                    let kind_val = kind_lit.value();
                    match kind_val.as_str() {
                        "create" => cmd.kind = CommandKindHint::Create,
                        "update" => cmd.kind = CommandKindHint::Update,
                        "delete" => cmd.kind = CommandKindHint::Delete,
                        "custom" => cmd.kind = CommandKindHint::Custom,
                        _ => {
                            return Err(syn::Error::new(
                                kind_lit.span(),
                                "kind must be \"create\", \"update\", \"delete\", or \"custom\""
                            ));
                        }
                    }
                }
                "security" => {
                    let _: syn::Token![=] = input.parse()?;
                    let security_lit: syn::LitStr = input.parse()?;
                    cmd.security = Some(security_lit.value());
                }
                _ => {
                    return Err(syn::Error::new(
                        option_name.span(),
                        format!(
                            "unknown command option '{}', expected: requires_id, source, \
                             payload, result, kind, security",
                            option_str
                        )
                    ));
                }
            }
        }

        Ok(cmd)
    })
}
