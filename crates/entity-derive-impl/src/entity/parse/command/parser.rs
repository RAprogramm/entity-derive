// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Command attribute parsing.

use syn::{Attribute, Ident, Type};

use super::types::{CommandDef, CommandKindHint, CommandSource};

/// Parse `#[command(...)]` attributes.
///
/// Extracts all command definitions from the struct's attributes.
///
/// # Arguments
///
/// * `attrs` - Slice of syn Attributes from the struct
///
/// # Returns
///
/// Vector of parsed command definitions.
///
/// # Syntax Examples
///
/// ```text
/// #[command(Register)]                              // name only (create fields)
/// #[command(Register, source = "create")]           // explicit source
/// #[command(UpdateEmail: email)]                    // specific fields
/// #[command(UpdateEmail: email, name)]              // multiple fields
/// #[command(Deactivate, requires_id)]               // id-only command
/// #[command(Deactivate, requires_id, kind = "delete")] // with kind hint
/// #[command(Transfer, payload = "TransferPayload")] // custom payload
/// #[command(Transfer, payload = "TransferPayload", result = "TransferResult")] // custom result
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
