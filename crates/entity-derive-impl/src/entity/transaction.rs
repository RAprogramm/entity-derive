// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Transaction support code generation.
//!
//! Generates transaction repository adapters and builder extensions
//! for type-safe multi-entity transactions.
//!
//! # Generated Types
//!
//! For an entity `User` with `#[entity(transactions)]`:
//!
//! - `UserTransactionRepo<'t>` — Repository adapter for transaction context
//! - `with_users()` — Deprecated no-op builder, kept for source compatibility
//! - `users()` — Accessor method on `TransactionContext`
//!
//! # Example
//!
//! ```rust,ignore
//! Transaction::new(&pool)
//!     .run(async |ctx| {
//!         let user = ctx.users().find_by_id(id).await?;
//!         ctx.orders().create(order).await?;
//!         Ok(())
//!     })
//!     .await?;
//! ```

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::{parse::EntityDef, sql::postgres::Context};
use crate::utils::{marker, tracing::instrument};

/// Generate all transaction-related code for an entity.
///
/// Returns empty `TokenStream` if `transactions` is not enabled.
pub fn generate(entity: &EntityDef) -> TokenStream {
    if !entity.has_transactions() {
        return TokenStream::new();
    }

    let repo_adapter = generate_repo_adapter(entity);
    let builder_ext = generate_builder_extension(entity);
    let context_ext = generate_context_extension(entity);

    quote! {
        #repo_adapter
        #builder_ext
        #context_ext
    }
}

/// Generate the transaction repository adapter struct.
///
/// Creates a struct that wraps a transaction reference and provides
/// repository methods that operate within the transaction.
fn generate_repo_adapter(entity: &EntityDef) -> TokenStream {
    let vis = &entity.vis;
    let ctx = Context::new(entity);
    let entity_name = ctx.entity_name;
    let row_name = &ctx.row_name;
    let insertable_name = &ctx.insertable_name;
    let create_dto = &ctx.create_dto;
    let update_dto = &ctx.update_dto;
    let table = &ctx.table;
    let columns_str = &ctx.columns_str;
    let placeholders_str = &ctx.placeholders_str;
    let id_name = ctx.id_name;
    let id_type = ctx.id_type;
    let soft_delete = ctx.soft_delete;
    let repo_name = format_ident!("{}TransactionRepo", entity_name);
    let marker = marker::generated();

    let bindings = super::sql::postgres::helpers::insert_bindings(entity.all_fields());
    let deleted_filter = if soft_delete {
        " AND deleted_at IS NULL"
    } else {
        ""
    };

    let entity_name_str = entity_name.to_string();
    let create_span = instrument(&entity_name_str, "tx.create");
    let create_method = if entity.create_fields().is_empty() {
        TokenStream::new()
    } else {
        quote! {
            /// Create a new entity within the transaction.
            #create_span
            pub async fn create(
                &mut self,
                dto: #create_dto
            ) -> Result<#entity_name, sqlx::Error> {
                let entity = #entity_name::from(dto);
                let insertable = #insertable_name::from(&entity);
                let row: #row_name = sqlx::query_as(
                    concat!("INSERT INTO ", #table, " (", #columns_str, ") VALUES (", #placeholders_str, ") RETURNING *")
                )
                    #(#bindings)*
                    .fetch_one(&mut **self.tx).await?;
                Ok(#entity_name::from(row))
            }
        }
    };

    let update_span = instrument(&entity_name_str, "tx.update");
    let update_method = if entity.update_fields().is_empty() {
        TokenStream::new()
    } else {
        let update_fields = entity.update_fields();
        let set_stmts = super::sql::postgres::helpers::dynamic_set_stmts(&update_fields);
        let set_binds = super::sql::postgres::helpers::dynamic_set_binds(&update_fields);

        quote! {
            /// Update an entity within the transaction.
            ///
            /// Fields absent from the DTO stay unchanged; nullable fields
            /// use double-`Option` semantics (`Some(None)` sets NULL).
            #update_span
            pub async fn update(
                &mut self,
                id: #id_type,
                dto: #update_dto
            ) -> Result<#entity_name, sqlx::Error> {
                #set_stmts
                if __sets.is_empty() {
                    return self.find_by_id(id).await?.ok_or(sqlx::Error::RowNotFound);
                }
                let mut q = sqlx::query_as::<_, #row_name>(
                    ::sqlx::AssertSqlSafe(format!("UPDATE {} SET {} WHERE {} = ${} RETURNING *",
                        #table, __sets.join(", "), stringify!(#id_name), __idx))
                );
                #set_binds
                q = q.bind(&id);
                let row: #row_name = q.fetch_one(&mut **self.tx).await?;
                Ok(#entity_name::from(row))
            }
        }
    };

    let delete_sql = if soft_delete {
        quote! {
            let result = sqlx::query(::sqlx::AssertSqlSafe(format!(
                "UPDATE {} SET deleted_at = NOW() WHERE {} = $1 AND deleted_at IS NULL",
                #table, stringify!(#id_name)
            ))).bind(&id).execute(&mut **self.tx).await?;
            Ok(result.rows_affected() > 0)
        }
    } else {
        quote! {
            let result = sqlx::query(::sqlx::AssertSqlSafe(format!(
                "DELETE FROM {} WHERE {} = $1",
                #table, stringify!(#id_name)
            ))).bind(&id).execute(&mut **self.tx).await?;
            Ok(result.rows_affected() > 0)
        }
    };

    let find_span = instrument(&entity_name_str, "tx.find_by_id");
    let delete_op = if soft_delete {
        "tx.soft_delete"
    } else {
        "tx.delete"
    };
    let delete_span = instrument(&entity_name_str, delete_op);
    let list_span = instrument(&entity_name_str, "tx.list");

    quote! {
        #marker
        /// Transaction repository adapter for #entity_name.
        ///
        /// Provides repository operations that execute within an active transaction.
        /// Access via `ctx.{entities}()` within a transaction closure.
        #vis struct #repo_name<'t> {
            tx: &'t mut sqlx::Transaction<'static, sqlx::Postgres>,
        }

        impl<'t> #repo_name<'t> {
            /// Create a new transaction repository adapter.
            #[doc(hidden)]
            pub fn new(tx: &'t mut sqlx::Transaction<'static, sqlx::Postgres>) -> Self {
                Self { tx }
            }

            #create_method

            /// Find an entity by ID within the transaction.
            #find_span
            pub async fn find_by_id(
                &mut self,
                id: #id_type
            ) -> Result<Option<#entity_name>, sqlx::Error> {
                let row: Option<#row_name> = sqlx::query_as(
                    ::sqlx::AssertSqlSafe(format!("SELECT {} FROM {} WHERE {} = $1{}",
                        #columns_str, #table, stringify!(#id_name), #deleted_filter))
                ).bind(&id).fetch_optional(&mut **self.tx).await?;
                Ok(row.map(#entity_name::from))
            }

            #update_method

            /// Delete an entity within the transaction.
            #delete_span
            pub async fn delete(
                &mut self,
                id: #id_type
            ) -> Result<bool, sqlx::Error> {
                #delete_sql
            }

            /// List entities within the transaction.
            #list_span
            pub async fn list(
                &mut self,
                limit: i64,
                offset: i64
            ) -> Result<Vec<#entity_name>, sqlx::Error> {
                let where_clause = if #soft_delete { "WHERE deleted_at IS NULL " } else { "" };
                let rows: Vec<#row_name> = sqlx::query_as(
                    ::sqlx::AssertSqlSafe(format!("SELECT {} FROM {} {}ORDER BY {} DESC LIMIT $1 OFFSET $2",
                        #columns_str, #table, where_clause, stringify!(#id_name)))
                ).bind(limit).bind(offset).fetch_all(&mut **self.tx).await?;
                Ok(rows.into_iter().map(#entity_name::from).collect())
            }
        }
    }
}

/// Generate the builder extension trait.
///
/// Creates an extension trait that adds `with_{entities}()` method to
/// `Transaction`. The method is a deprecated no-op kept only for source
/// compatibility with code written against earlier 0.x releases.
///
/// Repository access happens inside the closure passed to `run` /
/// `run_with_commit` via the `ContextExt` trait
/// (e.g. `ctx.users().find_by_id(id).await?`), independently of whether
/// `with_*` was called. The fluent chain therefore does not actually
/// register anything; users should drop the calls.
///
/// Planned removal: 0.8.0.
fn generate_builder_extension(entity: &EntityDef) -> TokenStream {
    let vis = &entity.vis;
    let entity_name = entity.name();
    let entity_snake = entity.name_str().to_case(Case::Snake);
    // Pluralize: add 's' for simple pluralization
    let plural = pluralize(&entity_snake);
    let method_name = format_ident!("with_{}", plural);
    let trait_name = format_ident!("TransactionWith{}", entity_name);
    let marker = marker::generated();
    let deprecation_note = format!(
        "no-op; repositories are accessed via `ctx.{plural}()` inside the closure of \
         `Transaction::run` / `run_with_commit`. Drop the `.{method_name}()` call. \
         Slated for removal in 0.8.0."
    );

    quote! {
        #marker
        /// Extension trait left for source compatibility with earlier 0.x releases.
        ///
        /// **Deprecated** — the method is a no-op. Repositories are accessed via
        /// `ctx.<entities>()` inside the closure of `Transaction::run` /
        /// `run_with_commit`, independent of whether this method is called.
        /// Planned for removal in 0.8.0.
        #vis trait #trait_name<'p> {
            /// Deprecated no-op kept for source compatibility.
            ///
            /// See the trait docs for the supported usage.
            #[deprecated(note = #deprecation_note)]
            fn #method_name(self) -> Self;
        }

        impl<'p> #trait_name<'p> for entity_core::transaction::Transaction<'p, sqlx::PgPool> {
            fn #method_name(self) -> Self {
                self
            }
        }
    }
}

/// Generate the context extension trait.
///
/// Creates an extension trait that adds accessor method to
/// `TransactionContext`.
fn generate_context_extension(entity: &EntityDef) -> TokenStream {
    let vis = &entity.vis;
    let entity_name = entity.name();
    let entity_snake = entity.name_str().to_case(Case::Snake);
    let plural = pluralize(&entity_snake);
    let accessor_name = format_ident!("{}", plural);
    let trait_name = format_ident!("{}ContextExt", entity_name);
    let repo_name = format_ident!("{}TransactionRepo", entity_name);
    let marker = marker::generated();

    quote! {
        #marker
        /// Extension trait providing #entity_name access in transaction context.
        #vis trait #trait_name {
            /// Get repository adapter for #entity_name operations.
            fn #accessor_name(&mut self) -> #repo_name<'_>;
        }

        impl #trait_name for entity_core::transaction::TransactionContext {
            fn #accessor_name(&mut self) -> #repo_name<'_> {
                #repo_name::new(self.transaction())
            }
        }
    }
}

/// English pluralization for transaction accessor names.
///
/// Used only to build method names like `ctx.<plural>()` and
/// `Transaction::with_<plural>()`, so this is a "good enough" inflector,
/// not a grammar engine. It covers:
///
/// 1. **Common irregulars** — explicit map for `child`, `person`, `mouse`,
///    `goose`, `foot`, `tooth`, `man`, `woman`, `datum`, `criterion`. Anything
///    not in this list falls through to the rules.
/// 2. **`+es` after sibilants** — words ending in `s`, `x`, `z`, `ch`, `sh` →
///    suffix `es`.
/// 3. **Consonant + `y` → `ies`** — `category` → `categories`.
/// 4. **Default `+s`** — every other word.
///
/// Anything more exotic than the irregulars above (Latin/Greek plurals,
/// foreign loanwords, etc.) keeps the regular `+s` result — rename the
/// entity or open an issue for a `#[entity(plural = "...")]` override.
fn pluralize(word: &str) -> String {
    if let Some(plural) = irregular_plural(word) {
        return plural;
    }

    if word.ends_with('s')
        || word.ends_with('x')
        || word.ends_with('z')
        || word.ends_with("ch")
        || word.ends_with("sh")
    {
        format!("{word}es")
    } else if let Some(without_y) = word.strip_suffix('y') {
        // Check if the letter before 'y' is a consonant
        if let Some(c) = without_y.chars().last()
            && !"aeiou".contains(c)
        {
            return format!("{without_y}ies");
        }
        format!("{word}s")
    } else {
        format!("{word}s")
    }
}

/// Look up a word in the built-in irregular-plural table.
///
/// Returns `Some(plural)` if `word` is one of the recognised irregulars
/// (case-insensitive match on the singular), preserving the original
/// casing strategy of the input: lower-case input → lower-case plural,
/// title-case input → title-case plural.
fn irregular_plural(word: &str) -> Option<String> {
    /// `(singular, plural)` pairs in lower-case. Order is not significant.
    const IRREGULARS: &[(&str, &str)] = &[
        ("child", "children"),
        ("person", "people"),
        ("mouse", "mice"),
        ("goose", "geese"),
        ("foot", "feet"),
        ("tooth", "teeth"),
        ("man", "men"),
        ("woman", "women"),
        ("datum", "data"),
        ("criterion", "criteria")
    ];

    let lower = word.to_ascii_lowercase();
    let (_, plural) = IRREGULARS.iter().find(|(singular, _)| *singular == lower)?;

    // entity names enter `pluralize` already in snake_case (the caller
    // converts them via `convert_case`), so the input is always
    // lower-case here. Keep the dual path anyway so the helper stays
    // robust if a future caller passes a mixed-case word.
    if word.chars().next().is_some_and(char::is_uppercase) {
        let mut capitalised = String::with_capacity(plural.len());
        let mut chars = plural.chars();
        if let Some(first) = chars.next() {
            capitalised.extend(first.to_uppercase());
            capitalised.extend(chars);
        }
        Some(capitalised)
    } else {
        Some((*plural).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regular_plurals_unchanged() {
        assert_eq!(pluralize("user"), "users");
        assert_eq!(pluralize("order"), "orders");
        assert_eq!(pluralize("account"), "accounts");
    }

    #[test]
    fn sibilant_suffix_adds_es() {
        assert_eq!(pluralize("box"), "boxes");
        assert_eq!(pluralize("class"), "classes");
        assert_eq!(pluralize("buzz"), "buzzes");
        assert_eq!(pluralize("match"), "matches");
        assert_eq!(pluralize("brush"), "brushes");
    }

    #[test]
    fn consonant_y_becomes_ies() {
        assert_eq!(pluralize("category"), "categories");
        assert_eq!(pluralize("country"), "countries");
        assert_eq!(pluralize("history"), "histories");
    }

    #[test]
    fn vowel_y_stays_y_plus_s() {
        assert_eq!(pluralize("day"), "days");
        assert_eq!(pluralize("key"), "keys");
        assert_eq!(pluralize("boy"), "boys");
    }

    #[test]
    fn irregular_child_becomes_children() {
        assert_eq!(pluralize("child"), "children");
    }

    #[test]
    fn irregular_person_becomes_people() {
        assert_eq!(pluralize("person"), "people");
    }

    #[test]
    fn irregular_mouse_becomes_mice() {
        assert_eq!(pluralize("mouse"), "mice");
    }

    #[test]
    fn irregular_goose_becomes_geese() {
        assert_eq!(pluralize("goose"), "geese");
    }

    #[test]
    fn irregular_foot_becomes_feet() {
        assert_eq!(pluralize("foot"), "feet");
    }

    #[test]
    fn irregular_tooth_becomes_teeth() {
        assert_eq!(pluralize("tooth"), "teeth");
    }

    #[test]
    fn irregular_man_becomes_men() {
        assert_eq!(pluralize("man"), "men");
    }

    #[test]
    fn irregular_woman_becomes_women() {
        assert_eq!(pluralize("woman"), "women");
    }

    #[test]
    fn irregular_datum_becomes_data() {
        assert_eq!(pluralize("datum"), "data");
    }

    #[test]
    fn irregular_criterion_becomes_criteria() {
        assert_eq!(pluralize("criterion"), "criteria");
    }

    #[test]
    fn irregular_lookup_is_case_insensitive() {
        // Title-case input keeps its capitalisation in the result.
        assert_eq!(pluralize("Child"), "Children");
        assert_eq!(pluralize("Person"), "People");
        // Lower-case still works.
        assert_eq!(pluralize("child"), "children");
    }

    #[test]
    fn irregular_does_not_match_compound_words() {
        // The lookup is exact, so a word that merely *contains* an
        // irregular substring stays in the regular rule path. This avoids
        // surprises like `childcare → childrencare`.
        assert_eq!(pluralize("childcare"), "childcares");
        assert_eq!(pluralize("manager"), "managers");
    }
}
