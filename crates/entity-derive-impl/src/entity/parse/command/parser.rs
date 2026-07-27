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
//! Parse errors from malformed `#[command(...)]` attributes are accumulated
//! across the whole struct via [`syn::Error::combine`] and returned as a
//! single error from [`parse_command_attrs`]. The macro then surfaces it
//! as a `compile_error!` at the relevant attribute span, so the user sees
//! every broken command in one compile pass instead of having them
//! silently disappear (the old behavior — fixed in #129).

use syn::{Attribute, Ident, Type};

use super::types::{CommandDef, CommandKindHint, CommandSource};

/// Parses all `#[command(...)]` attributes from a struct.
///
/// Iterates struct attributes, parses every `#[command(...)]`, and collects
/// the successful definitions. Parse failures are accumulated into a single
/// [`syn::Error`] via [`syn::Error::combine`] and returned together — the
/// caller emits the combined error as a `compile_error!` token so the user
/// sees every malformed attribute in one compile pass.
///
/// # Arguments
///
/// * `attrs` - Slice of `syn::Attribute` from the struct definition
///
/// # Errors
///
/// Returns the accumulated [`syn::Error`] if any `#[command(...)]` failed
/// to parse. Valid commands parsed before / after the failure point are
/// dropped in this case because partial code generation with missing
/// commands tends to produce more cryptic downstream errors than the
/// original parse diagnostic.
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
pub fn parse_command_attrs(attrs: &[Attribute]) -> syn::Result<Vec<CommandDef>> {
    let mut commands = Vec::new();
    let mut combined: Option<syn::Error> = None;

    for attr in attrs.iter().filter(|a| a.path().is_ident("command")) {
        match parse_single_command(attr) {
            Ok(cmd) => commands.push(cmd),
            Err(err) => match combined.as_mut() {
                Some(existing) => existing.combine(err),
                None => combined = Some(err)
            }
        }
    }

    if let Some(err) = combined {
        return Err(err);
    }
    Ok(commands)
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
                "sets" => {
                    let content;
                    syn::parenthesized!(content in input);
                    while !content.is_empty() {
                        let column: Ident = content.parse()?;
                        content.parse::<syn::Token![=]>()?;
                        let expression: syn::LitStr = content.parse()?;
                        cmd.sets.push((column.to_string(), expression.value()));
                        if content.peek(syn::Token![,]) {
                            content.parse::<syn::Token![,]>()?;
                        }
                    }
                    if cmd.sets.is_empty() {
                        return Err(syn::Error::new(
                            option_name.span(),
                            "sets(...) needs at least one `column = \"expression\"` pair"
                        ));
                    }
                    cmd.requires_id = true;
                    cmd.kind = CommandKindHint::Update;
                }
                "payload" if input.peek(syn::token::Paren) => {
                    let content;
                    syn::parenthesized!(content in input);
                    let fields =
                        syn::punctuated::Punctuated::<Ident, syn::Token![,]>::parse_terminated(
                            &content
                        )?;
                    if fields.is_empty() {
                        return Err(syn::Error::new(
                            option_name.span(),
                            "payload(...) needs at least one column"
                        ));
                    }
                    cmd.source = CommandSource::Fields(fields.into_iter().collect());
                }
                "payload" => {
                    let _: syn::Token![=] = input.parse()?;
                    let payload_lit: syn::LitStr = input.parse()?;
                    let payload_str = payload_lit.value();
                    let ty: Type = syn::parse_str(&payload_str)?;
                    cmd.source = CommandSource::Custom(Box::new(ty));
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
                            "unknown command option '{option_str}', expected: requires_id, source, \
                             payload, result, kind, security"
                        )
                    ));
                }
            }
        }

        Ok(cmd)
    })
}
