// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Upsert configuration for `#[entity(upsert(...))]`.
//!
//! Parsed from the container-level `upsert(...)` attribute:
//!
//! ```rust,ignore
//! #[entity(table = "users", upsert(conflict = "external_id"))]
//! #[entity(table = "users", upsert(conflict = "tenant_id, email", action = "nothing"))]
//! ```
//!
//! # Options
//!
//! | Option | Required | Default | Description |
//! |--------|----------|---------|-------------|
//! | `conflict` | Yes | — | Comma-separated conflict target columns |
//! | `action` | No | `"update"` | `"update"` (DO UPDATE) or `"nothing"` (DO NOTHING) |
//!
//! # Validation
//!
//! Conflict columns are validated at expansion time against
//! `#[column(unique)]` fields, `unique_index(...)` definitions and the
//! `#[id]` column so that the generated `ON CONFLICT` target always names
//! a genuine uniqueness guarantee.

use darling::FromMeta;

/// Conflict resolution action for the generated `upsert` method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UpsertAction {
    /// `ON CONFLICT ... DO UPDATE SET col = EXCLUDED.col, ...`
    ///
    /// Overwrites all updatable non-conflict columns with the incoming
    /// values and returns the persisted row.
    #[default]
    Update,

    /// `ON CONFLICT ... DO NOTHING`
    ///
    /// Keeps the existing row untouched. The generated method returns
    /// `Option<Entity>` — `None` signals that a conflicting row already
    /// existed and nothing was inserted.
    Nothing,
}

impl FromMeta for UpsertAction {
    fn from_string(value: &str) -> darling::Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "update" => Ok(Self::Update),
            "nothing" => Ok(Self::Nothing),
            other => Err(darling::Error::custom(format!(
                "unknown upsert action `{other}`; expected \"update\" or \"nothing\""
            ))),
        }
    }
}

/// Upsert definition parsed from `#[entity(upsert(...))]`.
#[derive(Debug, Clone, FromMeta)]
pub struct UpsertDef {
    /// Raw comma-separated conflict target columns.
    conflict: String,

    /// Conflict resolution action.
    #[darling(default)]
    pub action: UpsertAction,
}

impl UpsertDef {
    /// Conflict target columns, trimmed and in declaration order.
    #[must_use]
    pub fn conflict_columns(&self) -> Vec<String> {
        self.conflict
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect()
    }

    /// Conflict target as a comma-separated SQL fragment.
    #[must_use]
    pub fn conflict_target_sql(&self) -> String {
        self.conflict_columns().join(", ")
    }
}

#[cfg(test)]
mod tests {
    use darling::FromMeta;
    use quote::quote;
    use syn::Meta;

    use super::*;

    fn parse(tokens: proc_macro2::TokenStream) -> darling::Result<UpsertDef> {
        let meta: Meta = syn::parse2(tokens).expect("test meta must parse");
        UpsertDef::from_meta(&meta)
    }

    #[test]
    fn parses_single_conflict_column() {
        let def = parse(quote!(upsert(conflict = "email"))).unwrap();
        assert_eq!(def.conflict_columns(), vec!["email"]);
        assert_eq!(def.action, UpsertAction::Update);
    }

    #[test]
    fn parses_multi_conflict_columns_trimmed() {
        let def = parse(quote!(upsert(conflict = "tenant_id, email"))).unwrap();
        assert_eq!(def.conflict_columns(), vec!["tenant_id", "email"]);
        assert_eq!(def.conflict_target_sql(), "tenant_id, email");
    }

    #[test]
    fn parses_action_nothing() {
        let def = parse(quote!(upsert(conflict = "email", action = "nothing"))).unwrap();
        assert_eq!(def.action, UpsertAction::Nothing);
    }

    #[test]
    fn parses_action_update_case_insensitive() {
        let def = parse(quote!(upsert(conflict = "email", action = "Update"))).unwrap();
        assert_eq!(def.action, UpsertAction::Update);
    }

    #[test]
    fn rejects_unknown_action() {
        let err = parse(quote!(upsert(conflict = "email", action = "merge"))).unwrap_err();
        assert!(err.to_string().contains("unknown upsert action"));
    }

    #[test]
    fn rejects_missing_conflict() {
        assert!(parse(quote!(upsert(action = "update"))).is_err());
    }

    #[test]
    fn empty_segments_are_filtered() {
        let def = parse(quote!(upsert(conflict = "email,,  "))).unwrap();
        assert_eq!(def.conflict_columns(), vec!["email"]);
    }
}
