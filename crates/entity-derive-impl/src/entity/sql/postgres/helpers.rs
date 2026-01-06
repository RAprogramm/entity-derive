// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Helper functions for SQL generation.
//!
//! This module contains utility functions used across multiple method
//! generators:
//!
//! - [`join_columns`] — builds column list for SELECT/INSERT
//! - [`insert_bindings`] — builds `.bind()` chain for INSERT
//! - [`update_bindings`] — builds `.bind()` chain for UPDATE
//! - [`generate_where_conditions`] — builds WHERE clause for query method
//! - [`generate_query_bindings`] — builds parameter bindings for query method

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::entity::parse::{FieldDef, FilterType};

/// Join field names into comma-separated column list.
///
/// # Example
///
/// ```text
/// ["id", "name", "email"] -> "id, name, email"
/// ```
pub fn join_columns(fields: &[FieldDef]) -> String {
    fields
        .iter()
        .map(|f| f.name_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Build `.bind(insertable.field)` chain for INSERT.
///
/// # Generated Code
///
/// ```rust,ignore
/// .bind(insertable.id)
/// .bind(insertable.name)
/// .bind(insertable.email)
/// ```
pub fn insert_bindings(fields: &[FieldDef]) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|f| {
            let name = f.name();
            quote! { .bind(insertable.#name) }
        })
        .collect()
}

/// Build `.bind(dto.field)` chain for UPDATE.
///
/// # Generated Code
///
/// ```rust,ignore
/// .bind(dto.name)
/// .bind(dto.email)
/// ```
pub fn update_bindings(fields: &[&FieldDef]) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|f| {
            let name = f.name();
            quote! { .bind(dto.#name) }
        })
        .collect()
}

/// Generate WHERE condition building code for query method.
///
/// Creates runtime code that builds a dynamic WHERE clause based on
/// which filter fields are set in the query struct.
///
/// # Filter Types
///
/// | Type | SQL Generated |
/// |------|---------------|
/// | `Eq` | `field = $n` |
/// | `Like` | `field ILIKE $n` |
/// | `Range` | `field >= $n` and `field <= $n` |
///
/// # Soft Delete
///
/// When `soft_delete` is true, adds `deleted_at IS NULL` condition.
pub fn generate_where_conditions(fields: &[&FieldDef], soft_delete: bool) -> TokenStream {
    let conditions: Vec<TokenStream> = fields
        .iter()
        .flat_map(|f| {
            let name = f.name();
            let name_str = f.name_str();
            let filter = f.filter();

            match filter.filter_type {
                FilterType::Eq => {
                    vec![quote! {
                        if query.#name.is_some() {
                            conditions.push(format!("{} = ${}", #name_str, param_idx));
                            param_idx += 1;
                        }
                    }]
                }
                FilterType::Like => {
                    vec![quote! {
                        if query.#name.is_some() {
                            conditions.push(format!("{} ILIKE ${}", #name_str, param_idx));
                            param_idx += 1;
                        }
                    }]
                }
                FilterType::Range => {
                    let from_name = format_ident!("{}_from", name);
                    let to_name = format_ident!("{}_to", name);
                    vec![
                        quote! {
                            if query.#from_name.is_some() {
                                conditions.push(format!("{} >= ${}", #name_str, param_idx));
                                param_idx += 1;
                            }
                        },
                        quote! {
                            if query.#to_name.is_some() {
                                conditions.push(format!("{} <= ${}", #name_str, param_idx));
                                param_idx += 1;
                            }
                        },
                    ]
                }
                // Skip: filter_fields() should only return fields with filters,
                // but handle gracefully if None slips through
                FilterType::None => vec![]
            }
        })
        .collect();

    let soft_delete_condition = if soft_delete {
        quote! {
            conditions.push("deleted_at IS NULL".to_string());
        }
    } else {
        TokenStream::new()
    };

    quote! {
        #soft_delete_condition
        #(#conditions)*
    }
}

/// Generate query parameter binding code.
///
/// Creates runtime code that binds filter values to the query.
/// Only binds values for fields that are `Some`.
///
/// # LIKE Pattern
///
/// For `Like` filters, wraps the value in `%...%` for substring matching.
pub fn generate_query_bindings(fields: &[&FieldDef]) -> TokenStream {
    let bindings: Vec<TokenStream> = fields
        .iter()
        .flat_map(|f| {
            let name = f.name();
            let filter = f.filter();

            match filter.filter_type {
                FilterType::Eq => {
                    vec![quote! {
                        if let Some(ref v) = query.#name {
                            q = q.bind(v);
                        }
                    }]
                }
                FilterType::Like => {
                    vec![quote! {
                        if let Some(ref v) = query.#name {
                            q = q.bind(format!("%{}%", v));
                        }
                    }]
                }
                FilterType::Range => {
                    let from_name = format_ident!("{}_from", name);
                    let to_name = format_ident!("{}_to", name);
                    vec![
                        quote! {
                            if let Some(ref v) = query.#from_name {
                                q = q.bind(v);
                            }
                        },
                        quote! {
                            if let Some(ref v) = query.#to_name {
                                q = q.bind(v);
                            }
                        },
                    ]
                }
                // Skip: filter_fields() should only return fields with filters,
                // but handle gracefully if None slips through
                FilterType::None => vec![]
            }
        })
        .collect();

    quote! { #(#bindings)* }
}
