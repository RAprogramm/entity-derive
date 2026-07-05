// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Lookup method generators for `PostgreSQL`.
//!
//! Generates `find_by_{field}` and `exists_by_{field}` methods for fields
//! with `#[column(unique)]` or `#[column(index)]` constraints.
//!
//! # Generated Methods
//!
//! | Attribute | Methods Generated |
//! |-----------|-------------------|
//! | `#[column(unique)]` | `find_by_{field}`, `exists_by_{field}` |
//! | `#[column(index)]` | `find_by_{field}` |
//! | `#[column(unique, index)]` | `find_by_{field}`, `exists_by_{field}` |
//!
//! # SQL Patterns
//!
//! `find_by_{field}`:
//! ```sql
//! SELECT {columns} FROM {table} WHERE {field} = $1
//! ```
//!
//! `exists_by_{field}`:
//! ```sql
//! SELECT EXISTS(SELECT 1 FROM {table} WHERE {field} = $1)
//! ```
//!
//! # Example
//!
//! For an entity with `#[column(unique)] pub email: String`:
//!
//! ```rust,ignore
//! async fn find_by_email(&self, email: String) -> Result<Option<User>, Self::Error> {
//!     let row: Option<UserRow> = sqlx::query_as(
//!         ::sqlx::AssertSqlSafe(format!("SELECT * FROM users WHERE email = $1"))
//!     ).bind(&email).fetch_optional(self).await?;
//!     Ok(row.map(User::from))
//! }
//!
//! async fn exists_by_email(&self, email: String) -> Result<bool, Self::Error> {
//!     let exists: bool = sqlx::query_scalar(
//!         ::sqlx::AssertSqlSafe(format!("SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)"))
//!     ).bind(&email).fetch_one(self).await?;
//!     Ok(exists)
//! }
//! ```

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::context::Context;
use crate::{
    entity::parse::{DatabaseDialect, FieldDef, SqlLevel},
    utils::tracing::instrument
};

/// `a_and_b` method-name suffix for a composite lookup.
fn composite_suffix(fields: &[&FieldDef]) -> String {
    fields
        .iter()
        .map(|f| f.name_str())
        .collect::<Vec<_>>()
        .join("_and_")
}

/// `a = $1 AND b = $2` WHERE fragment for a composite lookup.
fn composite_where_clause(fields: &[&FieldDef], dialect: &DatabaseDialect) -> String {
    fields
        .iter()
        .enumerate()
        .map(|(i, f)| format!("{} = {}", f.name_str(), dialect.placeholder(i + 1)))
        .collect::<Vec<_>>()
        .join(" AND ")
}

impl Context<'_> {
    /// Generate all lookup method implementations.
    ///
    /// Creates `find_by_{field}` and `exists_by_{field}` methods for each
    /// field with `#[column(unique)]` or `#[column(index)]`.
    ///
    /// # Returns
    ///
    /// Empty `TokenStream` if `sql != "full"` or no fields have unique
    /// or index constraints.
    pub fn lookup_methods(&self) -> TokenStream {
        if self.entity.sql != SqlLevel::Full {
            return TokenStream::new();
        }

        let methods: Vec<TokenStream> = self
            .entity
            .lookup_fields()
            .iter()
            .flat_map(|field| self.lookup_method_impls(field))
            .collect();

        let composite: Vec<TokenStream> = self
            .entity
            .indexes
            .iter()
            .filter(|index| index.unique && index.columns.len() > 1)
            .flat_map(|index| self.composite_lookup_impls(index))
            .collect();

        quote! { #(#methods)* #(#composite)* }
    }

    /// Generate `find_by_...` / `exists_by_...` implementations for a
    /// composite unique index.
    ///
    /// Column names resolve to entity fields (validated at parse time),
    /// the method name joins them with `_and_`.
    fn composite_lookup_impls(
        &self,
        index: &crate::entity::parse::CompositeIndexDef
    ) -> Vec<TokenStream> {
        let fields: Vec<&FieldDef> = index
            .columns
            .iter()
            .filter_map(|col| {
                self.entity
                    .all_fields()
                    .iter()
                    .find(|f| f.name_str() == *col)
            })
            .collect();
        if fields.len() != index.columns.len() {
            return Vec::new();
        }
        vec![
            self.composite_find_impl(&fields),
            self.composite_exists_impl(&fields),
        ]
    }

    /// Generate the `find_by_{a}_and_{b}` implementation for a composite
    /// unique index.
    fn composite_find_impl(&self, fields: &[&FieldDef]) -> TokenStream {
        let Self {
            entity_name,
            row_name,
            table,
            dialect,
            ..
        } = self;

        let suffix = composite_suffix(fields);
        let method_name = format_ident!("find_by_{}", suffix);
        let op = format!("find_by_{suffix}");
        let span = instrument(&entity_name.to_string(), &op);
        let params = fields.iter().map(|f| {
            let name = f.name();
            let ty = f.ty();
            quote! { #name: #ty }
        });
        let binds = fields.iter().map(|f| {
            let name = f.name();
            quote! { .bind(&#name) }
        });
        let where_clause = composite_where_clause(fields, dialect);
        let sql = format!("SELECT * FROM {table} WHERE {where_clause}");

        quote! {
            #span
            async fn #method_name(&self, #(#params),*) -> Result<Option<#entity_name>, Self::Error> {
                let row: Option<#row_name> = sqlx::query_as(#sql)
                    #(#binds)*
                    .fetch_optional(self).await?;
                Ok(row.map(#entity_name::from))
            }
        }
    }

    /// Generate the `exists_by_{a}_and_{b}` implementation for a composite
    /// unique index.
    fn composite_exists_impl(&self, fields: &[&FieldDef]) -> TokenStream {
        let Self {
            entity_name,
            table,
            dialect,
            ..
        } = self;

        let suffix = composite_suffix(fields);
        let method_name = format_ident!("exists_by_{}", suffix);
        let op = format!("exists_by_{suffix}");
        let span = instrument(&entity_name.to_string(), &op);
        let params = fields.iter().map(|f| {
            let name = f.name();
            let ty = f.ty();
            quote! { #name: #ty }
        });
        let binds = fields.iter().map(|f| {
            let name = f.name();
            quote! { .bind(&#name) }
        });
        let where_clause = composite_where_clause(fields, dialect);
        let sql = format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE {where_clause})");

        quote! {
            #span
            async fn #method_name(&self, #(#params),*) -> Result<bool, Self::Error> {
                let exists: bool = sqlx::query_scalar(#sql)
                    #(#binds)*
                    .fetch_one(self).await?;
                Ok(exists)
            }
        }
    }

    /// Generate implementation blocks for a single lookup field.
    ///
    /// Returns one implementation for `find_by_{field}` and optionally
    /// one for `exists_by_{field}` (unique fields only).
    fn lookup_method_impls(&self, field: &FieldDef) -> Vec<TokenStream> {
        let mut methods = Vec::new();

        let find_impl = self.find_by_method(field);
        methods.push(find_impl);

        if field.column.unique {
            let exists_impl = self.exists_by_method(field);
            methods.push(exists_impl);
        }

        methods
    }

    /// Generate the `find_by_{field}` method implementation.
    ///
    /// # SQL Pattern
    ///
    /// ```sql
    /// SELECT {columns} FROM {schema}.{table} WHERE {field} = $1
    /// ```
    fn find_by_method(&self, field: &FieldDef) -> TokenStream {
        let Self {
            entity_name,
            row_name,
            table,
            dialect,
            ..
        } = self;

        let field_name = field.name();
        let field_name_str = field.name_str();
        let field_type = field.ty();
        let method_name = format_ident!("find_by_{}", field_name_str);
        let placeholder = dialect.placeholder(1);
        let op = format!("find_by_{field_name_str}");
        let span = instrument(&entity_name.to_string(), &op);

        quote! {
            #span
            async fn #method_name(&self, #field_name: #field_type) -> Result<Option<#entity_name>, Self::Error> {
                let row: Option<#row_name> = sqlx::query_as(
                    ::sqlx::AssertSqlSafe(format!("SELECT * FROM {} WHERE {} = {}", #table, stringify!(#field_name), #placeholder))
                ).bind(&#field_name).fetch_optional(self).await?;
                Ok(row.map(#entity_name::from))
            }
        }
    }

    /// Generate the `exists_by_{field}` method implementation.
    ///
    /// # SQL Pattern
    ///
    /// ```sql
    /// SELECT EXISTS(SELECT 1 FROM {schema}.{table} WHERE {field} = $1)
    /// ```
    fn exists_by_method(&self, field: &FieldDef) -> TokenStream {
        let Self {
            entity_name,
            table,
            dialect,
            ..
        } = self;

        let field_name = field.name();
        let field_name_str = field.name_str();
        let field_type = field.ty();
        let method_name = format_ident!("exists_by_{}", field_name_str);
        let placeholder = dialect.placeholder(1);
        let op = format!("exists_by_{field_name_str}");
        let span = instrument(&entity_name.to_string(), &op);

        quote! {
            #span
            async fn #method_name(&self, #field_name: #field_type) -> Result<bool, Self::Error> {
                let exists: bool = sqlx::query_scalar(
                    ::sqlx::AssertSqlSafe(format!("SELECT EXISTS(SELECT 1 FROM {} WHERE {} = {})", #table, stringify!(#field_name), #placeholder))
                ).bind(&#field_name).fetch_one(self).await?;
                Ok(exists)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::parse::EntityDef;

    fn parse_entity(tokens: proc_macro2::TokenStream) -> EntityDef {
        let input: syn::DeriveInput = syn::parse_quote!(#tokens);
        EntityDef::from_derive_input(&input).unwrap()
    }

    #[test]
    fn lookup_methods_unique_field() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique)]
                pub email: String,
            }
        });

        let ctx = Context::new(&entity);
        let methods = ctx.lookup_methods();
        let code = methods.to_string();

        assert!(code.contains("async fn find_by_email"));
        assert!(code.contains("async fn exists_by_email"));
        assert!(code.contains("fetch_optional"));
        assert!(code.contains("fetch_one"));
    }

    #[test]
    fn composite_unique_index_generates_lookup_pair() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "kyc_sessions", unique_index(provider, external_id))]
            pub struct KycSession {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                pub provider: String,
                #[field(create, response)]
                pub external_id: String,
            }
        });

        let ctx = Context::new(&entity);
        let code = ctx.lookup_methods().to_string();

        assert!(code.contains("async fn find_by_provider_and_external_id"));
        assert!(code.contains("async fn exists_by_provider_and_external_id"));
        assert!(code.contains("provider = $1 AND external_id = $2"));
    }

    #[test]
    fn non_unique_composite_index_generates_no_lookup() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "events", index(user_id, created_at))]
            pub struct Event {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                pub user_id: uuid::Uuid,
                #[field(create, response)]
                pub created_at: chrono::DateTime<chrono::Utc>,
            }
        });

        let ctx = Context::new(&entity);
        let code = ctx.lookup_methods().to_string();

        assert!(!code.contains("find_by_user_id_and_created_at"));
    }

    #[test]
    fn index_with_unknown_column_fails_parse() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[entity(table = "kyc_sessions", unique_index(provider, nonexistent))]
            pub struct KycSession {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                pub provider: String,
            }
        };
        let err = EntityDef::from_derive_input(&input).unwrap_err();
        assert!(err.to_string().contains("does not match any entity column"));
    }

    #[test]
    fn lookup_methods_index_only_field() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "posts")]
            pub struct Post {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(index)]
                pub slug: String,
            }
        });

        let ctx = Context::new(&entity);
        let methods = ctx.lookup_methods();
        let code = methods.to_string();

        assert!(code.contains("async fn find_by_slug"));
        assert!(!code.contains("exists_by_slug"));
    }

    #[test]
    fn lookup_methods_no_lookup_fields() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                pub name: String,
            }
        });

        let ctx = Context::new(&entity);
        let methods = ctx.lookup_methods();
        assert!(methods.is_empty());
    }

    #[test]
    fn lookup_methods_multiple_fields() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "products")]
            pub struct Product {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique, index)]
                pub sku: String,
                #[field(create, response)]
                #[column(index)]
                pub status: String,
            }
        });

        let ctx = Context::new(&entity);
        let methods = ctx.lookup_methods();
        let code = methods.to_string();

        assert!(code.contains("async fn find_by_sku"));
        assert!(code.contains("async fn exists_by_sku"));
        assert!(code.contains("async fn find_by_status"));
        assert!(!code.contains("exists_by_status"));
    }

    #[test]
    fn lookup_methods_with_schema() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users", schema = "core")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique)]
                pub email: String,
            }
        });

        let ctx = Context::new(&entity);
        let methods = ctx.lookup_methods();
        let code = methods.to_string();

        assert!(code.contains("core.users"));
    }

    #[test]
    fn lookup_methods_without_schema() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique)]
                pub email: String,
            }
        });

        let ctx = Context::new(&entity);
        let methods = ctx.lookup_methods();
        let code = methods.to_string();

        assert!(code.contains("\"users\""));
        assert!(!code.contains("\"public.users\""));
        assert!(!code.contains(".users"));
    }

    #[test]
    fn lookup_methods_sql_none_returns_empty() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users", sql = "none")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique)]
                pub email: String,
            }
        });

        let ctx = Context::new(&entity);
        let methods = ctx.lookup_methods();
        assert!(methods.is_empty());
    }

    #[test]
    fn lookup_methods_sql_trait_returns_empty() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users", sql = "trait")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique)]
                pub email: String,
            }
        });

        let ctx = Context::new(&entity);
        let methods = ctx.lookup_methods();
        assert!(methods.is_empty());
    }

    #[test]
    fn lookup_methods_unique_and_index_field() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "products")]
            pub struct Product {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique, index)]
                pub sku: String,
            }
        });

        let ctx = Context::new(&entity);
        let methods = ctx.lookup_methods();
        let code = methods.to_string();

        assert!(code.contains("async fn find_by_sku"));
        assert!(code.contains("async fn exists_by_sku"));
    }

    #[test]
    fn lookup_methods_gin_index() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "articles")]
            pub struct Article {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(index = "gin")]
                pub tags: Vec<String>,
            }
        });

        let ctx = Context::new(&entity);
        let methods = ctx.lookup_methods();
        let code = methods.to_string();

        assert!(code.contains("async fn find_by_tags"));
        assert!(!code.contains("exists_by_tags"));
    }

    #[test]
    fn lookup_methods_bind_parameter() {
        let entity = parse_entity(quote::quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique)]
                pub email: String,
            }
        });

        let ctx = Context::new(&entity);
        let methods = ctx.lookup_methods();
        let code = methods.to_string();

        assert!(code.contains("bind"));
    }
}
