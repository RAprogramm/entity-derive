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

use super::{
    parse::{CommandSource, EntityDef, SqlLevel, TransitionDef},
    sql::postgres::Context
};
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
    let insert_columns_str = &ctx.insert_columns_str;
    let columns_str = &ctx.columns_str;
    let placeholders_str = &ctx.placeholders_str;
    let id_name = ctx.id_name;
    let id_type = ctx.id_type;
    let soft_delete = ctx.soft_delete;
    let repo_name = format_ident!("{}TransactionRepo", entity_name);
    let marker = marker::generated();
    let error_type = entity.error_type();
    let constraint_map_err = ctx.constraint_map_err();
    let constraint_mapper = if entity.sql == SqlLevel::Full {
        TokenStream::new()
    } else {
        ctx.constraint_mapper()
    };

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
            ) -> Result<#entity_name, #error_type> {
                let entity = #entity_name::from(dto);
                let insertable = #insertable_name::from(&entity);
                let row: #row_name = sqlx::query_as(
                    concat!("INSERT INTO ", #table, " (", #insert_columns_str, ") VALUES (", #placeholders_str, ") RETURNING *")
                )
                    #(#bindings)*
                    .fetch_one(&mut **self.tx).await #constraint_map_err?;
                Ok(#entity_name::from(row))
            }
        }
    };

    let upsert_span = instrument(&entity_name_str, "tx.upsert");
    let upsert_method = match &entity.upsert {
        Some(upsert_def) if !entity.create_fields().is_empty() => {
            let sql = ctx.upsert_sql();
            match upsert_def.action {
                crate::entity::parse::UpsertAction::Update => quote! {
                    /// Insert or update the conflicting row within the transaction.
                    ///
                    /// Same semantics as the pool-backed `upsert`, executed on
                    /// the transaction handle so it can share atomicity with
                    /// adjacent statements.
                    #upsert_span
                    pub async fn upsert(
                        &mut self,
                        dto: #create_dto
                    ) -> Result<#entity_name, #error_type> {
                        let entity = #entity_name::from(dto);
                        let insertable = #insertable_name::from(&entity);
                        let row: #row_name = sqlx::query_as(#sql)
                            #(#bindings)*
                            .fetch_one(&mut **self.tx).await #constraint_map_err?;
                        Ok(#entity_name::from(row))
                    }
                },
                crate::entity::parse::UpsertAction::Nothing => quote! {
                    /// Insert the entity or keep the conflicting row, within
                    /// the transaction.
                    ///
                    /// Returns `None` when a conflicting row already existed.
                    #upsert_span
                    pub async fn upsert(
                        &mut self,
                        dto: #create_dto
                    ) -> Result<Option<#entity_name>, #error_type> {
                        let entity = #entity_name::from(dto);
                        let insertable = #insertable_name::from(&entity);
                        let row: Option<#row_name> = sqlx::query_as(#sql)
                            #(#bindings)*
                            .fetch_optional(&mut **self.tx).await #constraint_map_err?;
                        Ok(row.map(#entity_name::from))
                    }
                }
            }
        }
        _ => TokenStream::new()
    };

    let update_span = instrument(&entity_name_str, "tx.update");
    let update_method = if entity.update_fields().is_empty() {
        TokenStream::new()
    } else {
        let update_fields = entity.update_fields();
        let set_stmts = super::sql::postgres::helpers::dynamic_set_stmts(&update_fields);
        let set_binds = super::sql::postgres::helpers::dynamic_set_binds(&update_fields);
        let (version_stmts, version_where, version_bind) =
            super::sql::postgres::helpers::version_guard(entity, &quote! { __idx + 1 });

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
            ) -> Result<#entity_name, #error_type> {
                #set_stmts
                if __sets.is_empty() {
                    return self.find_by_id(id).await?.ok_or_else(|| sqlx::Error::RowNotFound.into());
                }
                #version_stmts
                let mut q = sqlx::query_as::<_, #row_name>(
                    ::sqlx::AssertSqlSafe(format!("UPDATE {} SET {} WHERE {} = ${}{} RETURNING *",
                        #table, __sets.join(", "), stringify!(#id_name), __idx, #version_where))
                );
                #set_binds
                q = q.bind(&id);
                #version_bind
                let row: Option<#row_name> = q.fetch_optional(&mut **self.tx).await #constraint_map_err?;
                let row: #row_name = row
                    .ok_or_else(|| sqlx::Error::Protocol("row not found or version conflict".into()))?;
                Ok(#entity_name::from(row))
            }
        }
    };

    let delete_sql = if soft_delete {
        quote! {
            let result = sqlx::query(::sqlx::AssertSqlSafe(format!(
                "UPDATE {} SET deleted_at = NOW() WHERE {} = $1 AND deleted_at IS NULL",
                #table, stringify!(#id_name)
            ))).bind(&id).execute(&mut **self.tx).await #constraint_map_err?;
            Ok(result.rows_affected() > 0)
        }
    } else {
        quote! {
            let result = sqlx::query(::sqlx::AssertSqlSafe(format!(
                "DELETE FROM {} WHERE {} = $1",
                #table, stringify!(#id_name)
            ))).bind(&id).execute(&mut **self.tx).await #constraint_map_err?;
            Ok(result.rows_affected() > 0)
        }
    };

    let transition_methods = transition_methods(entity, &ctx, error_type);
    let domain_operation_methods = domain_operation_methods(entity, &ctx, error_type);

    let find_span = instrument(&entity_name_str, "tx.find_by_id");
    let find_for_update_span = instrument(&entity_name_str, "tx.find_by_id_for_update");
    let delete_op = if soft_delete {
        "tx.soft_delete"
    } else {
        "tx.delete"
    };
    let delete_span = instrument(&entity_name_str, delete_op);
    let list_span = instrument(&entity_name_str, "tx.list");

    quote! {
        #marker
        #constraint_mapper

        /// Transaction repository adapter for #entity_name.
        ///
        /// Provides repository operations that execute within an active transaction,
        /// including the declared transitions and domain operations.
        /// Access via `ctx.{entities}()` within a transaction closure.
        ///
        /// Methods return the entity's configured `error` type; with
        /// `typed_constraints`, write paths resolve violated constraints
        /// exactly like the pool-backed repository.
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

            #upsert_method

            /// Find an entity by ID within the transaction.
            #find_span
            pub async fn find_by_id(
                &mut self,
                id: #id_type
            ) -> Result<Option<#entity_name>, #error_type> {
                let row: Option<#row_name> = sqlx::query_as(
                    ::sqlx::AssertSqlSafe(format!("SELECT {} FROM {} WHERE {} = $1{}",
                        #columns_str, #table, stringify!(#id_name), #deleted_filter))
                ).bind(&id).fetch_optional(&mut **self.tx).await?;
                Ok(row.map(#entity_name::from))
            }

            /// Find an entity by ID and lock its row with `FOR UPDATE`.
            ///
            /// Same lookup as `find_by_id` (including the soft-delete
            /// filter), but the returned row stays locked until the
            /// transaction commits or rolls back — use it to guard
            /// read-validate-write state transitions against concurrent
            /// writers.
            #find_for_update_span
            pub async fn find_by_id_for_update(
                &mut self,
                id: #id_type
            ) -> Result<Option<#entity_name>, #error_type> {
                let row: Option<#row_name> = sqlx::query_as(
                    ::sqlx::AssertSqlSafe(format!("SELECT {} FROM {} WHERE {} = $1{} FOR UPDATE",
                        #columns_str, #table, stringify!(#id_name), #deleted_filter))
                ).bind(&id).fetch_optional(&mut **self.tx).await?;
                Ok(row.map(#entity_name::from))
            }

            #update_method

            #(#transition_methods)*

            #(#domain_operation_methods)*

            /// Delete an entity within the transaction.
            #delete_span
            pub async fn delete(
                &mut self,
                id: #id_type
            ) -> Result<bool, #error_type> {
                #delete_sql
            }

            /// List entities within the transaction.
            #list_span
            pub async fn list(
                &mut self,
                limit: i64,
                offset: i64
            ) -> Result<Vec<#entity_name>, #error_type> {
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

/// Generate the declared domain operations against the transaction.
///
/// Mirrors the pool repository's `#[command(..., sets(...))]` methods so
/// an operation that must land together with other writes does not have
/// to be rewritten as raw SQL. Same statement, same column checking; the
/// only difference is the executor and that a missing row is `Ok(None)`
/// rather than an error, matching the other adapter methods.
fn domain_operation_methods(
    entity: &EntityDef,
    ctx: &Context<'_>,
    error_type: &syn::Path
) -> Vec<TokenStream> {
    let entity_name = ctx.entity_name;
    let entity_name_str = entity_name.to_string();
    let row_name = &ctx.row_name;
    let table = &ctx.table;
    let id_name = &ctx.id_name;
    let columns_str = &ctx.columns_str;
    let soft_delete = ctx.soft_delete;

    entity
        .command_defs()
        .iter()
        .filter(|cmd| !cmd.sets.is_empty())
        .map(|cmd| {
            let method_name = format_ident!("{}", cmd.name.to_string().to_case(Case::Snake));
            let command_struct = cmd.struct_name(&entity.name_str());
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
            let deleted_filter = if soft_delete {
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
            let span = instrument(&entity_name_str, &format!("tx.{method_name}"));
            let doc = format!(
                "Run the `{}` operation within the transaction.\n\n\
                 Writes the declared expressions plus the payload columns in\n\
                 one UPDATE; returns `Ok(None)` when the row does not exist.",
                cmd.name
            );

            quote! {
                #[doc = #doc]
                #span
                pub async fn #method_name(
                    &mut self,
                    command: #command_struct,
                ) -> Result<Option<#entity_name>, #error_type> {
                    let row: Option<#row_name> = sqlx::query_as(#sql)
                        #(#binds)*
                        .bind(&command.id)
                        .fetch_optional(&mut **self.tx)
                        .await?;
                    Ok(row.map(#entity_name::from))
                }
            }
        })
        .collect()
}

/// Generate `transition_to_{target}` methods from `#[transition(...)]`
/// declarations.
///
/// Each method locks the row with `find_by_id_for_update`, verifies the
/// current status is one of the declared sources (typed
/// `::entity_derive::TransitionError` otherwise), patches `status` plus the
/// declared `sets(...)` columns in one UPDATE and returns the persisted
/// row. `Ok(None)` means the row does not exist.
fn transition_methods(
    entity: &EntityDef,
    ctx: &Context<'_>,
    error_type: &syn::Path
) -> Vec<TokenStream> {
    if entity.transitions.is_empty() {
        return Vec::new();
    }

    let entity_name = ctx.entity_name;
    let entity_name_str = entity_name.to_string();
    let row_name = &ctx.row_name;
    let table = &ctx.table;
    let id_type = ctx.id_type;
    let id_name = ctx.id_name;
    let status_field = entity
        .all_fields()
        .iter()
        .find(|f| f.name_str() == "status")
        .expect("validated at parse time: transitions require a status field");
    let status_type = status_field.ty();

    entity
        .transitions
        .iter()
        .map(|t| {
            let method_name = format_ident!("{}", t.method_name());
            let target_variant = format_ident!("{}", TransitionDef::variant(&t.target));
            let target_str = TransitionDef::variant(&t.target);
            let source_variants = t
                .sources
                .iter()
                .map(|s| format_ident!("{}", TransitionDef::variant(s)));
            let span = instrument(&entity_name_str, &t.method_name());

            let set_fields: Vec<_> = t
                .sets
                .iter()
                .map(|col| {
                    entity
                        .all_fields()
                        .iter()
                        .find(|f| f.name_str() == *col)
                        .expect("validated at parse time: sets columns are entity fields")
                })
                .collect();
            let params = set_fields.iter().map(|f| {
                let name = f.name();
                let ty = f.option_inner_type();
                quote! { #name: #ty }
            });
            let set_clause: String = std::iter::once("status = $1".to_string())
                .chain(
                    t.sets
                        .iter()
                        .enumerate()
                        .map(|(i, col)| format!("{col} = ${}", i + 2))
                )
                .collect::<Vec<_>>()
                .join(", ");
            let id_placeholder = t.sets.len() + 2;
            let sql = format!(
                "UPDATE {table} SET {set_clause} WHERE {id_name} = ${id_placeholder} RETURNING *"
            );
            let binds = set_fields.iter().map(|f| {
                let name = f.name();
                quote! { .bind(&#name) }
            });
            let doc = format!(
                "Transition the row to `{target_str}` from {}, patching {}.\n\n\
                 Locks the row for the duration of the transaction; returns\n\
                 `Ok(None)` when the row does not exist and a typed\n\
                 [`::entity_derive::TransitionError`] when the current status does\n\
                 not allow this transition.",
                t.sources.join("/"),
                if t.sets.is_empty() {
                    "nothing else".to_string()
                } else {
                    t.sets.join(", ")
                }
            );

            quote! {
                #[doc = #doc]
                #span
                pub async fn #method_name(
                    &mut self,
                    id: #id_type,
                    #(#params,)*
                ) -> Result<Option<#entity_name>, #error_type> {
                    let Some(current) = self.find_by_id_for_update(id).await? else {
                        return Ok(None);
                    };
                    #[allow(unreachable_patterns)]
                    let allowed = matches!(current.status, #(<#status_type>::#source_variants)|*);
                    if !allowed {
                        return Err(::entity_derive::TransitionError {
                            entity: #entity_name_str,
                            from: format!("{:?}", current.status),
                            to: #target_str
                        }
                        .into());
                    }
                    let row: #row_name = sqlx::query_as(#sql)
                        .bind(<#status_type>::#target_variant)
                        #(#binds)*
                        .bind(&id)
                        .fetch_one(&mut **self.tx)
                        .await?;
                    Ok(Some(#entity_name::from(row)))
                }
            }
        })
        .collect()
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

        impl<'p> #trait_name<'p> for ::entity_derive::transaction::Transaction<'p, sqlx::PgPool> {
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

        impl #trait_name for ::entity_derive::transaction::TransactionContext {
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

#[cfg(all(test, feature = "postgres", feature = "transactions"))]
mod tx_upsert_tests {
    use quote::quote;
    use syn::DeriveInput;

    use super::*;

    fn parse_entity(tokens: proc_macro2::TokenStream) -> EntityDef {
        let input: DeriveInput = syn::parse2(tokens).expect("test entity must parse");
        EntityDef::from_derive_input(&input).expect("test entity must be valid")
    }

    #[test]
    fn adapter_gains_upsert_with_both_attributes() {
        let entity = parse_entity(quote! {
            #[entity(table = "users", transactions, upsert(conflict = "email"))]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique)]
                pub email: String,
                #[field(create, update, response)]
                pub name: String,
            }
        });
        let code = generate(&entity).to_string();
        assert!(code.contains("pub async fn upsert"));
        assert!(code.contains("ON CONFLICT (email) DO UPDATE"));
        assert!(code.contains("self . tx"));
    }

    #[test]
    fn adapter_upsert_nothing_returns_option() {
        let entity = parse_entity(quote! {
            #[entity(table = "subs", transactions, upsert(conflict = "email", action = "nothing"))]
            pub struct Sub {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique)]
                pub email: String,
            }
        });
        let code = generate(&entity).to_string();
        assert!(code.contains("Result < Option < Sub > , sqlx :: Error >"));
        assert!(code.contains("fetch_optional"));
    }

    #[test]
    fn adapter_uses_custom_error_type() {
        let entity = parse_entity(quote! {
            #[entity(table = "parcels", transactions, error = "crate::AppError")]
            pub struct Parcel {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                pub title: String,
            }
        });
        let code = generate(&entity).to_string();
        assert!(code.contains("Result < Parcel , crate :: AppError >"));
        assert!(code.contains("Result < Option < Parcel > , crate :: AppError >"));
        assert!(code.contains("Result < bool , crate :: AppError >"));
        assert!(code.contains("Result < Vec < Parcel > , crate :: AppError >"));
        assert!(!code.contains("Result < Parcel , sqlx :: Error >"));
    }

    #[test]
    fn adapter_write_paths_map_typed_constraints() {
        let entity = parse_entity(quote! {
            #[entity(table = "parcels", transactions, typed_constraints, error = "crate::AppError")]
            pub struct Parcel {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                #[column(unique)]
                pub title: String,
            }
        });
        let code = generate(&entity).to_string();
        assert!(code.contains("map_err (__parcel_map_constraint_err)"));
    }

    #[test]
    fn adapter_emits_mapper_only_without_pool_impl() {
        let full = parse_entity(quote! {
            #[entity(table = "parcels", transactions, typed_constraints, error = "crate::AppError")]
            pub struct Parcel {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique)]
                pub title: String,
            }
        });
        let trait_only = parse_entity(quote! {
            #[entity(table = "parcels", sql = "trait", transactions, typed_constraints, error = "crate::AppError")]
            pub struct Parcel {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique)]
                pub title: String,
            }
        });
        let full_code = generate(&full).to_string();
        let trait_code = generate(&trait_only).to_string();
        assert!(!full_code.contains("fn __parcel_map_constraint_err"));
        assert!(trait_code.contains("fn __parcel_map_constraint_err"));
    }

    #[test]
    fn empty_patch_fallback_avoids_needless_question_mark() {
        let entity = parse_entity(quote! {
            #[entity(table = "users", transactions)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                pub name: String,
            }
        });
        let code = generate(&entity).to_string();
        assert!(code.contains("ok_or_else (|| sqlx :: Error :: RowNotFound . into ())"));
        assert!(!code.contains("ok_or (sqlx :: Error :: RowNotFound) ?"));
    }

    #[test]
    fn transitions_generate_locking_methods() {
        let entity = parse_entity(quote! {
            #[entity(table = "parcels", transactions, error = "crate::AppError")]
            #[transition(created -> accepted, sets(courier_id))]
            #[transition(created | accepted -> cancelled)]
            pub struct Parcel {
                #[id]
                pub id: uuid::Uuid,
                #[field(update)]
                pub status: ParcelStatus,
                #[field(update)]
                pub courier_id: Option<uuid::Uuid>,
            }
        });
        let code = generate(&entity).to_string();
        assert!(code.contains("pub async fn transition_to_accepted"));
        assert!(code.contains("pub async fn transition_to_cancelled"));
        assert!(code.contains("find_by_id_for_update"));
        assert!(code.contains("TransitionError"));
        assert!(code.contains(
            "UPDATE parcels SET status = $1, courier_id = $2 WHERE id = $3 RETURNING *"
        ));
        assert!(code.contains("< ParcelStatus > :: Created | < ParcelStatus > :: Accepted"));
        assert!(code.contains("courier_id : uuid :: Uuid"));
    }

    #[test]
    fn adapter_gains_locking_lookup() {
        let entity = parse_entity(quote! {
            #[entity(table = "parcels", transactions)]
            pub struct Parcel {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                pub title: String,
            }
        });
        let code = generate(&entity).to_string();
        assert!(code.contains("pub async fn find_by_id_for_update"));
        assert!(code.contains("FOR UPDATE"));
    }

    #[test]
    fn locking_lookup_respects_soft_delete() {
        let entity = parse_entity(quote! {
            #[entity(table = "docs", transactions, soft_delete)]
            pub struct Doc {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                pub title: String,
                #[field(skip)]
                pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
            }
        });
        let code = generate(&entity).to_string();
        assert!(code.contains("find_by_id_for_update"));
        assert!(code.contains("$1{} FOR UPDATE"));
        assert!(code.contains("AND deleted_at IS NULL"));
    }

    #[test]
    fn adapter_without_typed_constraints_has_no_mapper_calls() {
        let entity = parse_entity(quote! {
            #[entity(table = "users", transactions)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                pub name: String,
            }
        });
        let code = generate(&entity).to_string();
        assert!(!code.contains("map_constraint_err"));
    }

    #[test]
    fn adapter_without_upsert_attribute_has_no_method() {
        let entity = parse_entity(quote! {
            #[entity(table = "users", transactions)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                pub name: String,
            }
        });
        let code = generate(&entity).to_string();
        assert!(!code.contains("pub async fn upsert"));
    }
}
