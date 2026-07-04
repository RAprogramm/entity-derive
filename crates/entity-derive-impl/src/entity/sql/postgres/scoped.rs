// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Row-level ownership-scoped method generators for `PostgreSQL`.
//!
//! Generated from the `#[owner]` field attribute:
//!
//! | Method | SQL |
//! |--------|-----|
//! | `find_by_id_scoped` | `SELECT ... WHERE id = $1 AND owner = $2` |
//! | `list_by_owner` | `SELECT ... WHERE owner = $1 ... LIMIT $2 OFFSET $3` |
//! | `update_scoped` | `UPDATE ... WHERE id AND owner RETURNING *` |
//! | `delete_scoped` | `DELETE ... WHERE id AND owner` (soft-delete aware) |
//!
//! All queries append `deleted_at IS NULL` when soft delete is enabled.
//! Zero affected rows means "no such row for this owner" — surfaced as
//! `None` / `false`, never leaking whether the row exists for someone
//! else.

use proc_macro2::TokenStream;
use quote::quote;

use super::context::Context;
use crate::utils::tracing::instrument;

impl Context<'_> {
    /// Generate all ownership-scoped method implementations.
    ///
    /// # Returns
    ///
    /// Empty `TokenStream` when the entity has no `#[owner]` field.
    pub fn scoped_methods(&self) -> TokenStream {
        let Some(owner) = self.entity.owner_field() else {
            return TokenStream::new();
        };

        let Self {
            entity_name,
            row_name,
            table,
            columns_str,
            id_name,
            id_type,
            soft_delete,
            ..
        } = self;

        let owner_name = owner.name();
        let owner_type = owner.ty();
        let owner_col = owner.name_str();
        let id_col = id_name.to_string();
        let deleted_filter = if *soft_delete {
            " AND deleted_at IS NULL"
        } else {
            ""
        };

        let find_sql = format!(
            "SELECT {columns_str} FROM {table} WHERE {id_col} = $1 AND {owner_col} = $2{deleted_filter}"
        );
        let list_sql = format!(
            "SELECT {columns_str} FROM {table} WHERE {owner_col} = $1{deleted_filter} \
             ORDER BY {id_col} DESC LIMIT $2 OFFSET $3"
        );
        let delete_sql = if *soft_delete {
            format!(
                "UPDATE {table} SET deleted_at = NOW() WHERE {id_col} = $1 AND {owner_col} = $2 \
                 AND deleted_at IS NULL"
            )
        } else {
            format!("DELETE FROM {table} WHERE {id_col} = $1 AND {owner_col} = $2")
        };

        let find_span = instrument(&entity_name.to_string(), "find_by_id_scoped");
        let list_span = instrument(&entity_name.to_string(), "list_by_owner");
        let delete_span = instrument(&entity_name.to_string(), "delete_scoped");

        let update_impl = self.update_scoped_method(&owner_col);

        quote! {
            #find_span
            async fn find_by_id_scoped(
                &self,
                id: #id_type,
                #owner_name: #owner_type,
            ) -> Result<Option<#entity_name>, Self::Error> {
                let row: Option<#row_name> = sqlx::query_as(#find_sql)
                    .bind(&id)
                    .bind(&#owner_name)
                    .fetch_optional(self).await?;
                Ok(row.map(#entity_name::from))
            }

            #list_span
            async fn list_by_owner(
                &self,
                #owner_name: #owner_type,
                limit: i64,
                offset: i64,
            ) -> Result<Vec<#entity_name>, Self::Error> {
                let rows: Vec<#row_name> = sqlx::query_as(#list_sql)
                    .bind(&#owner_name)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(self).await?;
                Ok(rows.into_iter().map(#entity_name::from).collect())
            }

            #delete_span
            async fn delete_scoped(
                &self,
                id: #id_type,
                #owner_name: #owner_type,
            ) -> Result<bool, Self::Error> {
                let result = sqlx::query(#delete_sql)
                    .bind(&id)
                    .bind(&#owner_name)
                    .execute(self).await?;
                Ok(result.rows_affected() > 0)
            }

            #update_impl
        }
    }

    /// Generate `update_scoped` when the entity has update fields.
    fn update_scoped_method(&self, owner_col: &str) -> TokenStream {
        let update_fields = self.entity.update_fields();
        if update_fields.is_empty() {
            return TokenStream::new();
        }

        let Self {
            entity_name,
            row_name,
            update_dto,
            table,
            id_name,
            id_type,
            dialect,
            entity,
            ..
        } = self;

        let owner = entity
            .owner_field()
            .expect("update_scoped_method is only called with an owner field");
        let owner_name = owner.name();
        let owner_type = owner.ty();

        let set_stmts = super::helpers::dynamic_set_stmts(&update_fields);
        let set_binds = super::helpers::dynamic_set_binds(&update_fields);
        let (version_stmts, version_where, version_bind) =
            super::helpers::version_guard(entity, &quote! { __idx + 2 });
        let _ = dialect;
        let owner_col_str = owner_col.to_string();

        let span = instrument(&entity_name.to_string(), "update_scoped");

        quote! {
            #span
            async fn update_scoped(
                &self,
                id: #id_type,
                #owner_name: #owner_type,
                dto: #update_dto,
            ) -> Result<Option<#entity_name>, Self::Error> {
                #set_stmts
                if __sets.is_empty() {
                    return self.find_by_id_scoped(id, #owner_name).await;
                }
                #version_stmts
                let __owner_where = format!(" AND {} = ${}", #owner_col_str, __idx + 1);
                let mut q = sqlx::query_as::<_, #row_name>(::sqlx::AssertSqlSafe(format!(
                    "UPDATE {} SET {} WHERE {} = ${}{}{} RETURNING *",
                    #table, __sets.join(", "), stringify!(#id_name), __idx, __owner_where, #version_where
                )));
                #set_binds
                q = q.bind(&id);
                q = q.bind(&#owner_name);
                #version_bind
                let row: Option<#row_name> = q.fetch_optional(self).await?;
                Ok(row.map(#entity_name::from))
            }
        }
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

    fn owned_entity(extra: proc_macro2::TokenStream) -> EntityDef {
        parse_entity(quote! {
            #[entity(table = "orders" #extra)]
            pub struct Order {
                #[id]
                pub id: uuid::Uuid,
                #[owner]
                pub user_id: uuid::Uuid,
                #[field(create, update, response)]
                pub note: String,
            }
        })
    }

    #[test]
    fn scoped_methods_generated_with_owner() {
        let entity = owned_entity(quote!());
        let code = Context::new(&entity).scoped_methods().to_string();
        assert!(code.contains("find_by_id_scoped"));
        assert!(code.contains("list_by_owner"));
        assert!(code.contains("delete_scoped"));
        assert!(code.contains("update_scoped"));
        assert!(code.contains("user_id = $2"));
    }

    #[test]
    fn scoped_methods_empty_without_owner() {
        let entity = parse_entity(quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                pub name: String,
            }
        });
        assert!(Context::new(&entity).scoped_methods().is_empty());
    }

    #[test]
    fn soft_delete_scopes_filter_deleted() {
        let entity = owned_entity(quote!(, soft_delete));
        let code = Context::new(&entity).scoped_methods().to_string();
        assert!(code.contains("deleted_at IS NULL"));
        assert!(code.contains("SET deleted_at = NOW()"));
    }

    #[test]
    fn update_scoped_builds_dynamic_set() {
        let entity = owned_entity(quote!());
        let code = Context::new(&entity).scoped_methods().to_string();
        assert!(code.contains("__sets"));
        assert!(code.contains("\"user_id\""));
        assert!(code.contains("find_by_id_scoped (id , user_id)"));
    }
}

#[cfg(test)]
mod no_update_fields_tests {
    use quote::quote;
    use syn::DeriveInput;

    use super::super::context::Context;
    use crate::entity::parse::EntityDef;

    #[test]
    fn update_scoped_absent_without_update_fields() {
        let input: DeriveInput = syn::parse_quote! {
            #[entity(table = "orders")]
            pub struct Order {
                #[id]
                pub id: uuid::Uuid,
                #[owner]
                pub user_id: uuid::Uuid,
                #[field(create, response)]
                pub note: String,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let code = Context::new(&entity).scoped_methods().to_string();
        assert!(code.contains("find_by_id_scoped"));
        assert!(!code.contains("update_scoped"));
        let _ = quote!();
    }
}
