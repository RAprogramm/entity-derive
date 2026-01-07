// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Tests for command parsing.

use proc_macro2::Span;
use syn::Ident;

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
