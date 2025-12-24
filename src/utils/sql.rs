// SPDX-FileCopyrightText: 2025 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! SQL query building utilities.

use proc_macro2::TokenStream;
use quote::quote;

use crate::entity::parse::FieldDef;

/// Join field names with comma separator.
pub fn join_columns(fields: &[FieldDef]) -> String {
    fields
        .iter()
        .map(|f: &FieldDef| f.name_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Build PostgreSQL placeholders: `$1, $2, $3, ...`
pub fn placeholders(count: usize) -> String {
    (1..=count)
        .map(|i| format!("${i}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Build SET clause: `col1 = $1, col2 = $2, ...`
pub fn set_clause(fields: &[&FieldDef]) -> String {
    fields
        .iter()
        .enumerate()
        .map(|(i, f): (usize, &&FieldDef)| format!("{} = ${}", f.name_str(), i + 1))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Build `.bind(insertable.field)` chain.
pub fn insert_bindings(fields: &[FieldDef]) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|f: &FieldDef| {
            let name = f.name();
            quote! { .bind(insertable.#name) }
        })
        .collect()
}

/// Build `.bind(dto.field)` chain for UPDATE.
pub fn update_bindings(fields: &[&FieldDef]) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|f: &&FieldDef| {
            let name = f.name();
            quote! { .bind(dto.#name) }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_placeholders_empty() {
        assert_eq!(placeholders(0), "");
    }

    #[test]
    fn test_placeholders_one() {
        assert_eq!(placeholders(1), "$1");
    }

    #[test]
    fn test_placeholders_three() {
        assert_eq!(placeholders(3), "$1, $2, $3");
    }

    #[test]
    fn test_placeholders_ten() {
        assert_eq!(placeholders(10), "$1, $2, $3, $4, $5, $6, $7, $8, $9, $10");
    }
}
