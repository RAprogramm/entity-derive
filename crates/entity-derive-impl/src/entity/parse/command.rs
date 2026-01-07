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

use proc_macro2::Span;
use syn::{Attribute, Ident, Type};

/// Source of fields for a command.
///
/// Determines which entity fields are included in the command payload.
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
    #[allow(dead_code)] // Used in tests and for API inspection
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
        // Parse command name (required)
        let name: Ident = input.parse()?;
        let mut cmd = CommandDef::new(name);

        // Check for field list syntax: `Name: field1, field2`
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

        // Parse optional comma-separated options
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_command() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[command(Register)]
            struct User {}
        };
        let cmds = parse_command_attrs(&input.attrs);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].name.to_string(), "Register");
        assert_eq!(cmds[0].source, CommandSource::Create);
        assert!(!cmds[0].requires_id);
    }

    #[test]
    fn parse_command_with_fields() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[command(UpdateEmail: email)]
            struct User {}
        };
        let cmds = parse_command_attrs(&input.attrs);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].name.to_string(), "UpdateEmail");
        if let CommandSource::Fields(ref fields) = cmds[0].source {
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].to_string(), "email");
        } else {
            panic!("Expected Fields source");
        }
        assert!(cmds[0].requires_id);
    }

    #[test]
    fn parse_command_with_multiple_fields() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[command(UpdateProfile: name, avatar, bio)]
            struct User {}
        };
        let cmds = parse_command_attrs(&input.attrs);
        assert_eq!(cmds.len(), 1);
        if let CommandSource::Fields(ref fields) = cmds[0].source {
            assert_eq!(fields.len(), 3);
            assert_eq!(fields[0].to_string(), "name");
            assert_eq!(fields[1].to_string(), "avatar");
            assert_eq!(fields[2].to_string(), "bio");
        } else {
            panic!("Expected Fields source");
        }
    }

    #[test]
    fn parse_requires_id_command() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[command(Deactivate, requires_id)]
            struct User {}
        };
        let cmds = parse_command_attrs(&input.attrs);
        assert_eq!(cmds.len(), 1);
        assert!(cmds[0].requires_id);
        assert_eq!(cmds[0].source, CommandSource::None);
    }

    #[test]
    fn parse_custom_payload_command() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[command(Transfer, payload = "TransferPayload")]
            struct Account {}
        };
        let cmds = parse_command_attrs(&input.attrs);
        assert_eq!(cmds.len(), 1);
        assert!(matches!(cmds[0].source, CommandSource::Custom(_)));
    }

    #[test]
    fn parse_command_with_result() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[command(Transfer, payload = "TransferPayload", result = "TransferResult")]
            struct Account {}
        };
        let cmds = parse_command_attrs(&input.attrs);
        assert_eq!(cmds.len(), 1);
        assert!(cmds[0].result_type.is_some());
    }

    #[test]
    fn parse_multiple_commands() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[command(Register)]
            #[command(UpdateEmail: email)]
            #[command(Deactivate, requires_id)]
            struct User {}
        };
        let cmds = parse_command_attrs(&input.attrs);
        assert_eq!(cmds.len(), 3);
        assert_eq!(cmds[0].name.to_string(), "Register");
        assert_eq!(cmds[1].name.to_string(), "UpdateEmail");
        assert_eq!(cmds[2].name.to_string(), "Deactivate");
    }

    #[test]
    fn parse_kind_hint() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[command(Delete, requires_id, kind = "delete")]
            struct User {}
        };
        let cmds = parse_command_attrs(&input.attrs);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].kind, CommandKindHint::Delete);
    }

    #[test]
    fn struct_name_generation() {
        let cmd = CommandDef::new(Ident::new("Register", Span::call_site()));
        assert_eq!(cmd.struct_name("User").to_string(), "RegisterUser");
    }

    #[test]
    fn handler_method_name_generation() {
        let cmd = CommandDef::new(Ident::new("UpdateEmail", Span::call_site()));
        assert_eq!(cmd.handler_method_name().to_string(), "handle_update_email");
    }

    #[test]
    fn parse_source_update() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[command(Modify, source = "update")]
            struct User {}
        };
        let cmds = parse_command_attrs(&input.attrs);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].source, CommandSource::Update);
        assert!(cmds[0].requires_id);
        assert_eq!(cmds[0].kind, CommandKindHint::Update);
    }

    #[test]
    fn parse_source_none() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[command(Ping, source = "none")]
            struct User {}
        };
        let cmds = parse_command_attrs(&input.attrs);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].source, CommandSource::None);
    }

    #[test]
    fn parse_source_create_explicit() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[command(Register, source = "create")]
            struct User {}
        };
        let cmds = parse_command_attrs(&input.attrs);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].source, CommandSource::Create);
    }

    #[test]
    fn parse_kind_create() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[command(Register, kind = "create")]
            struct User {}
        };
        let cmds = parse_command_attrs(&input.attrs);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].kind, CommandKindHint::Create);
    }

    #[test]
    fn parse_kind_update() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[command(Modify, kind = "update")]
            struct User {}
        };
        let cmds = parse_command_attrs(&input.attrs);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].kind, CommandKindHint::Update);
    }

    #[test]
    fn parse_kind_custom() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[command(Process, kind = "custom")]
            struct User {}
        };
        let cmds = parse_command_attrs(&input.attrs);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].kind, CommandKindHint::Custom);
    }

    #[test]
    fn parse_trailing_comma() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[command(Register,)]
            struct User {}
        };
        let cmds = parse_command_attrs(&input.attrs);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].name.to_string(), "Register");
    }

    #[test]
    fn parse_invalid_source_returns_empty() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[command(Test, source = "invalid")]
            struct User {}
        };
        let cmds = parse_command_attrs(&input.attrs);
        assert!(cmds.is_empty());
    }

    #[test]
    fn parse_invalid_kind_returns_empty() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[command(Test, kind = "invalid")]
            struct User {}
        };
        let cmds = parse_command_attrs(&input.attrs);
        assert!(cmds.is_empty());
    }

    #[test]
    fn parse_unknown_option_returns_empty() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[command(Test, unknown_option)]
            struct User {}
        };
        let cmds = parse_command_attrs(&input.attrs);
        assert!(cmds.is_empty());
    }

    #[test]
    fn ignores_non_command_attributes() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[derive(Debug)]
            #[entity(table = "users")]
            struct User {}
        };
        let cmds = parse_command_attrs(&input.attrs);
        assert!(cmds.is_empty());
    }

    #[test]
    fn parse_security_bearer() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[command(AdminDelete, requires_id, security = "admin")]
            struct User {}
        };
        let cmds = parse_command_attrs(&input.attrs);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].security(), Some("admin"));
        assert!(!cmds[0].is_public());
    }

    #[test]
    fn parse_security_none() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[command(PublicList, security = "none")]
            struct User {}
        };
        let cmds = parse_command_attrs(&input.attrs);
        assert_eq!(cmds.len(), 1);
        assert!(cmds[0].is_public());
        assert!(cmds[0].has_security_override());
    }

    #[test]
    fn default_no_security_override() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[command(Register)]
            struct User {}
        };
        let cmds = parse_command_attrs(&input.attrs);
        assert_eq!(cmds.len(), 1);
        assert!(!cmds[0].has_security_override());
        assert!(!cmds[0].is_public());
        assert_eq!(cmds[0].security(), None);
    }
}
