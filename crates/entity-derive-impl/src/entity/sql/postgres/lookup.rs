// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Lookup method generators for PostgreSQL.
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
//!         &format!("SELECT * FROM users WHERE email = $1")
//!     ).bind(&email).fetch_optional(self).await?;
//!     Ok(row.map(User::from))
//! }
//!
//! async fn exists_by_email(&self, email: String) -> Result<bool, Self::Error> {
//!     let exists: bool = sqlx::query_scalar(
//!         &format!("SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)")
//!     ).bind(&email).fetch_one(self).await?;
//!     Ok(exists)
//! }
//! ```

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::context::Context;
use crate::entity::parse::{FieldDef, SqlLevel};

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

        quote! { #(#methods)* }
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

        quote! {
            async fn #method_name(&self, #field_name: #field_type) -> Result<Option<#entity_name>, Self::Error> {
                let row: Option<#row_name> = sqlx::query_as(
                    &format!("SELECT * FROM {} WHERE {} = {}", #table, stringify!(#field_name), #placeholder)
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
            table,
            dialect,
            ..
        } = self;

        let field_name = field.name();
        let field_name_str = field.name_str();
        let field_type = field.ty();
        let method_name = format_ident!("exists_by_{}", field_name_str);
        let placeholder = dialect.placeholder(1);

        quote! {
            async fn #method_name(&self, #field_name: #field_type) -> Result<bool, Self::Error> {
                let exists: bool = sqlx::query_scalar(
                    &format!("SELECT EXISTS(SELECT 1 FROM {} WHERE {} = {})", #table, stringify!(#field_name), #placeholder)
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
