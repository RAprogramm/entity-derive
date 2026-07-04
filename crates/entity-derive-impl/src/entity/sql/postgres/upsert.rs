// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Upsert method generator for `PostgreSQL`.
//!
//! Generates the `upsert` repository method from `#[entity(upsert(...))]`:
//!
//! ```sql
//! INSERT INTO schema.table (col1, col2, ...)
//! VALUES ($1, $2, ...)
//! ON CONFLICT (conflict_cols) DO UPDATE SET col = EXCLUDED.col, ...
//! RETURNING *
//! ```
//!
//! # Action Semantics
//!
//! | Action | SQL | Return type |
//! |--------|-----|-------------|
//! | `update` | `DO UPDATE SET` all non-conflict, non-id columns | `Entity` |
//! | `nothing` | `DO NOTHING` | `Option<Entity>` (`None` = row already existed) |
//!
//! The SQL string is assembled entirely at macro expansion time from
//! entity metadata, so the generated code carries a `'static` literal.
//!
//! # Event Semantics
//!
//! With streams enabled, `upsert` publishes a `Created` event for every
//! row it returns. On the `update` conflict path the row was modified,
//! not created; subscribers must treat upsert notifications as
//! "row is now present in this state".

use proc_macro2::TokenStream;
use quote::quote;

use super::{context::Context, crud::tx_wrapping, helpers::insert_bindings};
use crate::{
    entity::parse::{FieldDef, UpsertAction},
    utils::tracing::instrument,
};

impl Context<'_> {
    /// Generate the `upsert` method implementation.
    ///
    /// # Returns
    ///
    /// Empty `TokenStream` when the entity has no `upsert(...)` attribute.
    pub fn upsert_method(&self) -> TokenStream {
        let Some(upsert) = &self.entity.upsert else {
            return TokenStream::new();
        };

        let Self {
            entity_name,
            row_name,
            insertable_name,
            create_dto,
            entity,
            streams,
            ..
        } = self;

        let bindings = insert_bindings(entity.all_fields());
        let span = instrument(&entity_name.to_string(), "upsert");
        let (tx_open, tx_close, executor) = tx_wrapping(*streams);
        let notify = self.notify_created();
        let sql = self.upsert_sql();

        match upsert.action {
            UpsertAction::Update => {
                quote! {
                    #span
                    async fn upsert(&self, dto: #create_dto) -> Result<#entity_name, Self::Error> {
                        #tx_open
                        let entity = #entity_name::from(dto);
                        let insertable = #insertable_name::from(&entity);
                        let row: #row_name = sqlx::query_as(#sql)
                            #(#bindings)*
                            .fetch_one(#executor).await?;
                        let entity = #entity_name::from(row);
                        #notify
                        #tx_close
                        Ok(entity)
                    }
                }
            }
            UpsertAction::Nothing => {
                quote! {
                    #span
                    async fn upsert(&self, dto: #create_dto) -> Result<Option<#entity_name>, Self::Error> {
                        #tx_open
                        let entity = #entity_name::from(dto);
                        let insertable = #insertable_name::from(&entity);
                        let row: Option<#row_name> = sqlx::query_as(#sql)
                            #(#bindings)*
                            .fetch_optional(#executor).await?;
                        match row {
                            Some(row) => {
                                let entity = #entity_name::from(row);
                                #notify
                                #tx_close
                                Ok(Some(entity))
                            }
                            None => {
                                #tx_close
                                Ok(None)
                            }
                        }
                    }
                }
            }
        }
    }

    /// Assemble the complete upsert SQL string at expansion time.
    fn upsert_sql(&self) -> String {
        let upsert = self
            .entity
            .upsert
            .as_ref()
            .expect("upsert_sql is only called when upsert is configured");
        let conflict = upsert.conflict_columns();
        let target = upsert.conflict_target_sql();

        let action_sql = match upsert.action {
            UpsertAction::Update => {
                let assignments: Vec<String> = self
                    .entity
                    .all_fields()
                    .iter()
                    .filter(|f| {
                        let col = f.name_str();
                        !FieldDef::is_id(f) && !conflict.contains(&col)
                    })
                    .map(|f| {
                        let col = f.name_str();
                        format!("{col} = EXCLUDED.{col}")
                    })
                    .collect();
                format!("DO UPDATE SET {}", assignments.join(", "))
            }
            UpsertAction::Nothing => "DO NOTHING".to_string(),
        };

        format!(
            "INSERT INTO {} ({}) VALUES ({}) ON CONFLICT ({}) {} RETURNING *",
            self.table, self.columns_str, self.placeholders_str, target, action_sql
        )
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::DeriveInput;

    use super::*;
    use crate::entity::parse::EntityDef;

    fn parse_entity(tokens: proc_macro2::TokenStream) -> EntityDef {
        let input: DeriveInput = syn::parse2(tokens).expect("test entity must parse");
        EntityDef::from_derive_input(&input).expect("test entity must be valid")
    }

    fn user_entity(upsert_attr: proc_macro2::TokenStream) -> EntityDef {
        parse_entity(quote! {
            #[entity(table = "users", #upsert_attr)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique)]
                pub email: String,
                #[field(create, update, response)]
                pub name: String,
            }
        })
    }

    #[test]
    fn upsert_sql_update_action() {
        let entity = user_entity(quote!(upsert(conflict = "email")));
        let ctx = Context::new(&entity);
        assert_eq!(
            ctx.upsert_sql(),
            "INSERT INTO users (id, email, name) VALUES ($1, $2, $3) \
             ON CONFLICT (email) DO UPDATE SET name = EXCLUDED.name RETURNING *"
        );
    }

    #[test]
    fn upsert_sql_nothing_action() {
        let entity = user_entity(quote!(upsert(conflict = "email", action = "nothing")));
        let ctx = Context::new(&entity);
        assert_eq!(
            ctx.upsert_sql(),
            "INSERT INTO users (id, email, name) VALUES ($1, $2, $3) \
             ON CONFLICT (email) DO NOTHING RETURNING *"
        );
    }

    #[test]
    fn upsert_sql_respects_schema() {
        let entity = parse_entity(quote! {
            #[entity(table = "users", schema = "core", upsert(conflict = "email"))]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique)]
                pub email: String,
                #[field(create, response)]
                pub name: String,
            }
        });
        let ctx = Context::new(&entity);
        assert!(ctx.upsert_sql().starts_with("INSERT INTO core.users "));
    }

    #[test]
    fn upsert_sql_composite_conflict_via_unique_index() {
        let entity = parse_entity(quote! {
            #[entity(
                table = "members",
                unique_index(tenant_id, email),
                upsert(conflict = "tenant_id, email")
            )]
            pub struct Member {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                pub tenant_id: uuid::Uuid,
                #[field(create, response)]
                pub email: String,
                #[field(create, update, response)]
                pub role: String,
            }
        });
        let ctx = Context::new(&entity);
        assert_eq!(
            ctx.upsert_sql(),
            "INSERT INTO members (id, tenant_id, email, role) VALUES ($1, $2, $3, $4) \
             ON CONFLICT (tenant_id, email) DO UPDATE SET role = EXCLUDED.role RETURNING *"
        );
    }

    #[test]
    fn upsert_method_update_returns_entity() {
        let entity = user_entity(quote!(upsert(conflict = "email")));
        let ctx = Context::new(&entity);
        let code = ctx.upsert_method().to_string();

        assert!(code.contains("async fn upsert"));
        assert!(code.contains("Result < User , Self :: Error >"));
        assert!(code.contains("fetch_one"));
    }

    #[test]
    fn upsert_method_nothing_returns_option() {
        let entity = user_entity(quote!(upsert(conflict = "email", action = "nothing")));
        let ctx = Context::new(&entity);
        let code = ctx.upsert_method().to_string();

        assert!(code.contains("Result < Option < User > , Self :: Error >"));
        assert!(code.contains("fetch_optional"));
    }

    #[test]
    fn upsert_method_empty_without_attribute() {
        let entity = parse_entity(quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                pub name: String,
            }
        });
        let ctx = Context::new(&entity);
        assert!(ctx.upsert_method().is_empty());
    }
}
