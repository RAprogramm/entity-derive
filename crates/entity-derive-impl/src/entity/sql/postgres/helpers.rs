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
                            // Escape SQL LIKE wildcards to prevent injection
                            let escaped = v
                                .replace('\\', "\\\\")
                                .replace('%', "\\%")
                                .replace('_', "\\_");
                            q = q.bind(format!("%{}%", escaped));
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

#[cfg(test)]
mod tests {
    use syn::{Field, parse_quote};

    use super::*;
    use crate::entity::parse::FieldDef;

    fn parse_field(tokens: proc_macro2::TokenStream) -> FieldDef {
        let field: Field = parse_quote!(#tokens);
        FieldDef::from_field(&field).unwrap()
    }

    #[test]
    fn join_columns_single() {
        let field = parse_field(quote! { pub name: String });
        let result = join_columns(&[field]);
        assert_eq!(result, "name");
    }

    #[test]
    fn join_columns_multiple() {
        let fields = vec![
            parse_field(quote! { pub id: Uuid }),
            parse_field(quote! { pub name: String }),
            parse_field(quote! { pub email: String }),
        ];
        let result = join_columns(&fields);
        assert_eq!(result, "id, name, email");
    }

    #[test]
    fn join_columns_empty() {
        let result = join_columns(&[]);
        assert_eq!(result, "");
    }

    #[test]
    fn insert_bindings_generates_bind_calls() {
        let fields = vec![
            parse_field(quote! { pub id: Uuid }),
            parse_field(quote! { pub name: String }),
        ];
        let bindings = insert_bindings(&fields);
        assert_eq!(bindings.len(), 2);

        let first = bindings[0].to_string();
        assert!(first.contains("bind"), "Expected 'bind' in: {}", first);
        assert!(
            first.contains("insertable"),
            "Expected 'insertable' in: {}",
            first
        );
        assert!(first.contains("id"), "Expected 'id' in: {}", first);

        let second = bindings[1].to_string();
        assert!(second.contains("bind"), "Expected 'bind' in: {}", second);
        assert!(
            second.contains("insertable"),
            "Expected 'insertable' in: {}",
            second
        );
        assert!(second.contains("name"), "Expected 'name' in: {}", second);
    }

    #[test]
    fn insert_bindings_empty() {
        let bindings = insert_bindings(&[]);
        assert!(bindings.is_empty());
    }

    #[test]
    fn update_bindings_generates_bind_calls() {
        let fields = vec![
            parse_field(quote! { pub name: String }),
            parse_field(quote! { pub email: String }),
        ];
        let refs: Vec<&FieldDef> = fields.iter().collect();
        let bindings = update_bindings(&refs);
        assert_eq!(bindings.len(), 2);

        let first = bindings[0].to_string();
        assert!(first.contains("bind"), "Expected 'bind' in: {}", first);
        assert!(first.contains("dto"), "Expected 'dto' in: {}", first);
        assert!(first.contains("name"), "Expected 'name' in: {}", first);
    }

    #[test]
    fn update_bindings_empty() {
        let bindings = update_bindings(&[]);
        assert!(bindings.is_empty());
    }

    #[test]
    fn where_conditions_eq_filter() {
        let field = parse_field(quote! {
            #[filter(eq)]
            pub status: String
        });
        let refs: Vec<&FieldDef> = vec![&field];
        let result = generate_where_conditions(&refs, false);
        let code = result.to_string();
        assert!(code.contains("query . status . is_some"));
        assert!(code.contains("= $"));
    }

    #[test]
    fn where_conditions_like_filter() {
        let field = parse_field(quote! {
            #[filter(like)]
            pub name: String
        });
        let refs: Vec<&FieldDef> = vec![&field];
        let result = generate_where_conditions(&refs, false);
        let code = result.to_string();
        assert!(code.contains("query . name . is_some"));
        assert!(code.contains("ILIKE"));
    }

    #[test]
    fn where_conditions_range_filter() {
        let field = parse_field(quote! {
            #[filter(range)]
            pub age: i32
        });
        let refs: Vec<&FieldDef> = vec![&field];
        let result = generate_where_conditions(&refs, false);
        let code = result.to_string();
        assert!(code.contains("age_from"));
        assert!(code.contains("age_to"));
        assert!(code.contains(">="));
        assert!(code.contains("<="));
    }

    #[test]
    fn where_conditions_none_filter() {
        let field = parse_field(quote! { pub name: String });
        let refs: Vec<&FieldDef> = vec![&field];
        let result = generate_where_conditions(&refs, false);
        let code = result.to_string();
        // No conditions for None filter
        assert!(!code.contains("query"));
    }

    #[test]
    fn where_conditions_with_soft_delete() {
        let result = generate_where_conditions(&[], true);
        let code = result.to_string();
        assert!(code.contains("deleted_at IS NULL"));
    }

    #[test]
    fn where_conditions_without_soft_delete() {
        let result = generate_where_conditions(&[], false);
        let code = result.to_string();
        assert!(!code.contains("deleted_at"));
    }

    #[test]
    fn query_bindings_eq_filter() {
        let field = parse_field(quote! {
            #[filter(eq)]
            pub status: String
        });
        let refs: Vec<&FieldDef> = vec![&field];
        let result = generate_query_bindings(&refs);
        let code = result.to_string();
        assert!(code.contains("if let Some (ref v) = query . status"));
        assert!(code.contains("q = q . bind (v)"));
    }

    #[test]
    fn query_bindings_like_filter() {
        let field = parse_field(quote! {
            #[filter(like)]
            pub name: String
        });
        let refs: Vec<&FieldDef> = vec![&field];
        let result = generate_query_bindings(&refs);
        let code = result.to_string();
        assert!(code.contains("escaped"));
        assert!(code.contains("format !"));
    }

    #[test]
    fn query_bindings_range_filter() {
        let field = parse_field(quote! {
            #[filter(range)]
            pub age: i32
        });
        let refs: Vec<&FieldDef> = vec![&field];
        let result = generate_query_bindings(&refs);
        let code = result.to_string();
        assert!(code.contains("age_from"));
        assert!(code.contains("age_to"));
    }

    #[test]
    fn query_bindings_none_filter() {
        let field = parse_field(quote! { pub name: String });
        let refs: Vec<&FieldDef> = vec![&field];
        let result = generate_query_bindings(&refs);
        let code = result.to_string();
        // No bindings for None filter
        assert!(!code.contains("bind"));
    }

    #[test]
    fn query_bindings_empty() {
        let result = generate_query_bindings(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn where_conditions_multiple_filters() {
        let fields = vec![
            parse_field(quote! {
                #[filter(eq)]
                pub status: String
            }),
            parse_field(quote! {
                #[filter(like)]
                pub name: String
            }),
        ];
        let refs: Vec<&FieldDef> = fields.iter().collect();
        let result = generate_where_conditions(&refs, false);
        let code = result.to_string();
        assert!(code.contains("status"));
        assert!(code.contains("name"));
        assert!(code.contains("= $"));
        assert!(code.contains("ILIKE"));
    }
}
