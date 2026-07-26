// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Domain-operation generators for `PostgreSQL`.
//!
//! A command declaring `sets(...)` writes named columns that are
//! deliberately not `#[field(update)]` — keeping them out of the public
//! patch DTO and out of the upsert SET list, which is the whole point
//! of declaring them here instead:
//!
//! ```rust,ignore
//! #[command(VerifyPassport, payload(passport_provider),
//!           sets(passport_verified = "true", passport_verified_at = "NOW()"))]
//! ```
//!
//! ```sql
//! UPDATE users SET passport_verified = true, passport_verified_at = NOW(),
//!                  passport_provider = $1
//! WHERE id = $2 RETURNING ...
//! ```
//!
//! The expressions are written by the developer and land in the
//! statement verbatim, exactly like `#[column(default = "...")]`.

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::context::Context;
use crate::{
    entity::parse::{CommandDef, CommandSource},
    utils::tracing::instrument
};

impl Context<'_> {
    /// Generate every declared domain operation.
    ///
    /// # Returns
    ///
    /// Empty `TokenStream` when no command declares `sets(...)`.
    pub fn domain_operation_methods(&self) -> TokenStream {
        let methods: Vec<TokenStream> = self
            .entity
            .command_defs()
            .iter()
            .filter(|cmd| !cmd.sets.is_empty())
            .map(|cmd| self.domain_operation(cmd))
            .collect();

        quote! { #(#methods)* }
    }

    /// Generate one domain-operation implementation.
    fn domain_operation(&self, cmd: &CommandDef) -> TokenStream {
        let Self {
            entity_name,
            row_name,
            table,
            columns_str,
            id_name,
            soft_delete,
            ..
        } = self;

        let method_name = format_ident!("{}", cmd.name.to_string().to_case(Case::Snake));
        let command_struct = cmd.struct_name(&self.entity.name_str());

        let payload: Vec<syn::Ident> = match &cmd.source {
            CommandSource::Fields(fields) => fields.clone(),
            _ => Vec::new()
        };

        let mut assignments: Vec<String> = cmd
            .sets
            .iter()
            .map(|(column, expression)| format!("{column} = {expression}"))
            .collect();
        for (index, field) in payload.iter().enumerate() {
            assignments.push(format!("{field} = ${}", index + 1));
        }

        let id_placeholder = payload.len() + 1;
        let deleted_filter = if *soft_delete {
            " AND deleted_at IS NULL"
        } else {
            ""
        };
        let sql = format!(
            "UPDATE {table} SET {} WHERE {id_name} = ${id_placeholder}{deleted_filter} \
             RETURNING {columns_str}",
            assignments.join(", ")
        );

        let binds = payload
            .iter()
            .map(|field| quote! { .bind(&command.#field) });
        let span = instrument(&entity_name.to_string(), &method_name.to_string());

        quote! {
            #span
            async fn #method_name(
                &self,
                command: #command_struct,
            ) -> Result<#entity_name, Self::Error> {
                let row: #row_name = sqlx::query_as(#sql)
                    #(#binds)*
                    .bind(&command.id)
                    .fetch_optional(self)
                    .await?
                    .ok_or_else(|| sqlx::Error::RowNotFound)?;
                Ok(#entity_name::from(row))
            }
        }
    }
}
