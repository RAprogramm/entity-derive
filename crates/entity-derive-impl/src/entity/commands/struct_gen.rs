// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Command payload struct generation.
//!
//! Generates individual command structs like `RegisterUser`, `UpdateEmailUser`.
//!
//! # Generated Code
//!
//! For `#[command(Register)]` on `User` entity:
//!
//! ```rust,ignore
//! #[derive(Debug, Clone)]
//! pub struct RegisterUser {
//!     pub email: String,
//!     pub name: String,
//! }
//! ```

use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::{
    entity::parse::{CommandDef, CommandSource, EntityDef, FieldDef},
    utils::marker
};

/// Generate all command payload structs.
pub fn generate(entity: &EntityDef) -> TokenStream {
    let structs: Vec<TokenStream> = entity
        .command_defs()
        .iter()
        .map(|cmd| generate_command_struct(entity, cmd))
        .collect();

    quote! { #(#structs)* }
}

/// Generate a single command struct.
fn generate_command_struct(entity: &EntityDef, cmd: &CommandDef) -> TokenStream {
    let vis = &entity.vis;
    let struct_name = cmd.struct_name(&entity.name_str());
    let marker = marker::generated();

    let id_field = if cmd.requires_id {
        let id_type = entity.id_field().ty();
        quote! {
            /// Entity ID this command targets.
            pub id: #id_type,
        }
    } else {
        TokenStream::new()
    };

    let payload_fields = generate_payload_fields(entity, cmd);

    let doc = format!(
        "Command payload for {} operation on [`{}`].",
        cmd.name,
        entity.name_str()
    );

    quote! {
        #marker
        #[doc = #doc]
        #[derive(Debug, Clone)]
        #vis struct #struct_name {
            #id_field
            #payload_fields
        }
    }
}

/// Generate payload fields based on command source.
fn generate_payload_fields(entity: &EntityDef, cmd: &CommandDef) -> TokenStream {
    match &cmd.source {
        CommandSource::Create => {
            let fields = entity.create_fields();
            generate_fields_tokens(&fields)
        }
        CommandSource::Update => {
            let fields = entity.update_fields();
            generate_optional_fields_tokens(&fields)
        }
        CommandSource::Fields(field_names) => {
            let fields: Vec<&FieldDef> = entity
                .all_fields()
                .iter()
                .filter(|f| field_names.iter().any(|n| n == f.name()))
                .collect();
            generate_fields_tokens(&fields)
        }
        CommandSource::Custom(_) => {
            // Custom payload - no fields generated, uses external type
            TokenStream::new()
        }
        CommandSource::None => {
            // No payload fields
            TokenStream::new()
        }
    }
}

/// Generate required field tokens.
fn generate_fields_tokens(fields: &[&FieldDef]) -> TokenStream {
    let field_tokens: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            let name = f.name();
            let ty = f.ty();
            let doc = format!("Value for `{}`.", name);
            quote! {
                #[doc = #doc]
                pub #name: #ty,
            }
        })
        .collect();

    quote! { #(#field_tokens)* }
}

/// Generate optional field tokens (for update-style commands).
fn generate_optional_fields_tokens(fields: &[&FieldDef]) -> TokenStream {
    let field_tokens: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            let name = f.name();
            let ty = f.ty();
            let doc = format!("Optional new value for `{}`.", name);
            quote! {
                #[doc = #doc]
                pub #name: Option<#ty>,
            }
        })
        .collect();

    quote! { #(#field_tokens)* }
}

/// Get the struct name for a command.
///
/// Public helper for other modules.
pub fn command_struct_name(entity: &EntityDef, cmd: &CommandDef) -> Ident {
    cmd.struct_name(&entity.name_str())
}

/// Check if command uses custom payload type.
pub fn uses_custom_payload(cmd: &CommandDef) -> bool {
    matches!(cmd.source, CommandSource::Custom(_))
}

/// Get custom payload type if present.
pub fn custom_payload_type(cmd: &CommandDef) -> Option<&syn::Type> {
    match &cmd.source {
        CommandSource::Custom(ty) => Some(ty),
        _ => None
    }
}
