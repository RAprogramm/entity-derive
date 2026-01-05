// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! PostgreSQL repository implementation generator.
//!
//! Generates `impl {Name}Repository for sqlx::PgPool` with complete CRUD
//! operations. This is the primary database backend, providing full SQL support
//! via sqlx.
//!
//! # Generated Implementation
//!
//! ```rust,ignore
//! #[cfg(feature = "postgres")]
//! #[async_trait]
//! impl UserRepository for sqlx::PgPool {
//!     type Error = sqlx::Error;  // or custom error type
//!     type Pool = sqlx::PgPool;
//!
//!     fn pool(&self) -> &Self::Pool { self }
//!     async fn create(&self, dto: CreateUserRequest) -> Result<User, Self::Error> { ... }
//!     async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, Self::Error> { ... }
//!     async fn update(&self, id: Uuid, dto: UpdateUserRequest) -> Result<User, Self::Error> { ... }
//!     async fn delete(&self, id: Uuid) -> Result<bool, Self::Error> { ... }
//!     async fn list(&self, limit: i64, offset: i64) -> Result<Vec<User>, Self::Error> { ... }
//! }
//! ```
//!
//! # SQL Queries
//!
//! | Method | Query Pattern |
//! |--------|---------------|
//! | `create` | `INSERT INTO schema.table (...) VALUES ($1, $2, ...)` |
//! | `find_by_id` | `SELECT ... FROM schema.table WHERE id = $1` |
//! | `update` | `UPDATE schema.table SET ... WHERE id = $n` |
//! | `delete` | `DELETE FROM schema.table WHERE id = $1` |
//! | `list` | `SELECT ... FROM schema.table ORDER BY id DESC LIMIT $1 OFFSET $2` |
//!
//! # Feature Flag
//!
//! Generated code is gated behind `#[cfg(feature = "postgres")]`.

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{
    entity::parse::{DatabaseDialect, EntityDef, FieldDef, ReturningMode},
    utils::marker
};

/// Generate PostgreSQL repository implementation.
///
/// Creates `impl {Name}Repository for sqlx::PgPool` with all CRUD methods.
pub fn generate(entity: &EntityDef) -> TokenStream {
    let ctx = Context::new(entity);
    let trait_name = &ctx.trait_name;
    let feature = entity.dialect.feature_flag();
    let error_type = entity.error_type();

    let create_impl = ctx.create_method();
    let find_impl = ctx.find_by_id_method();
    let update_impl = ctx.update_method();
    let delete_impl = ctx.delete_method();
    let list_impl = ctx.list_method();
    let relation_impls = ctx.relation_methods();
    let projection_impls = ctx.projection_methods();
    let soft_delete_impls = ctx.soft_delete_methods();
    let marker = marker::generated();

    quote! {
        #marker
        #[cfg(feature = #feature)]
        #[async_trait::async_trait]
        impl #trait_name for sqlx::PgPool {
            type Error = #error_type;
            type Pool = sqlx::PgPool;

            fn pool(&self) -> &Self::Pool {
                self
            }

            #create_impl
            #find_impl
            #update_impl
            #delete_impl
            #list_impl
            #relation_impls
            #projection_impls
            #soft_delete_impls
        }
    }
}

/// Context for PostgreSQL code generation.
///
/// Precomputes all identifiers and SQL fragments needed for method generation.
struct Context<'a> {
    entity:           &'a EntityDef,
    dialect:          DatabaseDialect,
    trait_name:       syn::Ident,
    entity_name:      &'a syn::Ident,
    row_name:         syn::Ident,
    insertable_name:  syn::Ident,
    create_dto:       syn::Ident,
    update_dto:       syn::Ident,
    table:            String,
    id_name:          &'a syn::Ident,
    id_type:          &'a syn::Type,
    columns_str:      String,
    placeholders_str: String,
    soft_delete:      bool,
    returning:        ReturningMode
}

impl<'a> Context<'a> {
    fn new(entity: &'a EntityDef) -> Self {
        let id_field = entity.id_field().expect("Entity must have #[id] field");
        let fields = entity.all_fields();
        let dialect = entity.dialect;

        Self {
            entity,
            dialect,
            trait_name: format_ident!("{}Repository", entity.name()),
            entity_name: entity.name(),
            row_name: entity.ident_with("", "Row"),
            insertable_name: entity.ident_with("Insertable", ""),
            create_dto: entity.ident_with("Create", "Request"),
            update_dto: entity.ident_with("Update", "Request"),
            table: entity.full_table_name(),
            id_name: id_field.name(),
            id_type: id_field.ty(),
            columns_str: join_columns(fields),
            placeholders_str: dialect.placeholders(fields.len()),
            soft_delete: entity.is_soft_delete(),
            returning: entity.returning.clone()
        }
    }

    fn create_method(&self) -> TokenStream {
        if self.entity.create_fields().is_empty() {
            return TokenStream::new();
        }

        let Self {
            entity_name,
            row_name,
            insertable_name,
            create_dto,
            table,
            columns_str,
            placeholders_str,
            entity,
            returning,
            ..
        } = self;
        let bindings = insert_bindings(entity.all_fields());

        match returning {
            ReturningMode::Full => {
                quote! {
                    async fn create(&self, dto: #create_dto) -> Result<#entity_name, Self::Error> {
                        let entity = #entity_name::from(dto);
                        let insertable = #insertable_name::from(&entity);
                        let row: #row_name = sqlx::query_as(
                            concat!("INSERT INTO ", #table, " (", #columns_str, ") VALUES (", #placeholders_str, ") RETURNING *")
                        )
                            #(#bindings)*
                            .fetch_one(self).await?;
                        Ok(#entity_name::from(row))
                    }
                }
            }
            ReturningMode::Id => {
                let id_name = self.id_name;
                quote! {
                    async fn create(&self, dto: #create_dto) -> Result<#entity_name, Self::Error> {
                        let entity = #entity_name::from(dto);
                        let insertable = #insertable_name::from(&entity);
                        sqlx::query(concat!("INSERT INTO ", #table, " (", #columns_str, ") VALUES (", #placeholders_str, ") RETURNING ", stringify!(#id_name)))
                            #(#bindings)*
                            .execute(self).await?;
                        Ok(entity)
                    }
                }
            }
            ReturningMode::None => {
                quote! {
                    async fn create(&self, dto: #create_dto) -> Result<#entity_name, Self::Error> {
                        let entity = #entity_name::from(dto);
                        let insertable = #insertable_name::from(&entity);
                        sqlx::query(concat!("INSERT INTO ", #table, " (", #columns_str, ") VALUES (", #placeholders_str, ")"))
                            #(#bindings)*
                            .execute(self).await?;
                        Ok(entity)
                    }
                }
            }
            ReturningMode::Custom(columns) => {
                let returning_cols = columns.join(", ");
                quote! {
                    async fn create(&self, dto: #create_dto) -> Result<#entity_name, Self::Error> {
                        let entity = #entity_name::from(dto);
                        let insertable = #insertable_name::from(&entity);
                        sqlx::query(&format!("INSERT INTO {} ({}) VALUES ({}) RETURNING {}", #table, #columns_str, #placeholders_str, #returning_cols))
                            #(#bindings)*
                            .execute(self).await?;
                        Ok(entity)
                    }
                }
            }
        }
    }

    fn find_by_id_method(&self) -> TokenStream {
        let Self {
            entity_name,
            row_name,
            table,
            columns_str,
            id_name,
            id_type,
            dialect,
            soft_delete,
            ..
        } = self;
        let placeholder = dialect.placeholder(1);
        let deleted_filter = if *soft_delete {
            " AND deleted_at IS NULL"
        } else {
            ""
        };

        quote! {
            async fn find_by_id(&self, id: #id_type) -> Result<Option<#entity_name>, Self::Error> {
                let row: Option<#row_name> = sqlx::query_as(
                    &format!("SELECT {} FROM {} WHERE {} = {}{}", #columns_str, #table, stringify!(#id_name), #placeholder, #deleted_filter)
                ).bind(&id).fetch_optional(self).await?;
                Ok(row.map(#entity_name::from))
            }
        }
    }

    fn update_method(&self) -> TokenStream {
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
            trait_name,
            returning,
            ..
        } = self;

        let field_names: Vec<&str> = update_fields
            .iter()
            .map(|f| f.name_str().leak() as &str)
            .collect();
        let set_clause = dialect.set_clause(&field_names);
        let where_placeholder = dialect.placeholder(update_fields.len() + 1);
        let bindings = update_bindings(&update_fields);

        match returning {
            ReturningMode::Full => {
                quote! {
                    async fn update(&self, id: #id_type, dto: #update_dto) -> Result<#entity_name, Self::Error> {
                        let row: #row_name = sqlx::query_as(
                            &format!("UPDATE {} SET {} WHERE {} = {} RETURNING *", #table, #set_clause, stringify!(#id_name), #where_placeholder)
                        )
                            #(#bindings)*
                            .bind(&id)
                            .fetch_one(self).await?;
                        Ok(#entity_name::from(row))
                    }
                }
            }
            ReturningMode::Id | ReturningMode::None => {
                quote! {
                    async fn update(&self, id: #id_type, dto: #update_dto) -> Result<#entity_name, Self::Error> {
                        sqlx::query(&format!("UPDATE {} SET {} WHERE {} = {}", #table, #set_clause, stringify!(#id_name), #where_placeholder))
                            #(#bindings)*
                            .bind(&id)
                            .execute(self).await?;
                        <Self as #trait_name>::find_by_id(self, id).await?.ok_or_else(|| sqlx::Error::RowNotFound.into())
                    }
                }
            }
            ReturningMode::Custom(columns) => {
                let returning_cols = columns.join(", ");
                quote! {
                    async fn update(&self, id: #id_type, dto: #update_dto) -> Result<#entity_name, Self::Error> {
                        sqlx::query(&format!("UPDATE {} SET {} WHERE {} = {} RETURNING {}", #table, #set_clause, stringify!(#id_name), #where_placeholder, #returning_cols))
                            #(#bindings)*
                            .bind(&id)
                            .execute(self).await?;
                        <Self as #trait_name>::find_by_id(self, id).await?.ok_or_else(|| sqlx::Error::RowNotFound.into())
                    }
                }
            }
        }
    }

    fn delete_method(&self) -> TokenStream {
        let Self {
            table,
            id_name,
            id_type,
            dialect,
            soft_delete,
            ..
        } = self;
        let placeholder = dialect.placeholder(1);

        if *soft_delete {
            quote! {
                async fn delete(&self, id: #id_type) -> Result<bool, Self::Error> {
                    let result = sqlx::query(&format!(
                        "UPDATE {} SET deleted_at = NOW() WHERE {} = {} AND deleted_at IS NULL",
                        #table, stringify!(#id_name), #placeholder
                    )).bind(&id).execute(self).await?;
                    Ok(result.rows_affected() > 0)
                }
            }
        } else {
            quote! {
                async fn delete(&self, id: #id_type) -> Result<bool, Self::Error> {
                    let result = sqlx::query(&format!("DELETE FROM {} WHERE {} = {}", #table, stringify!(#id_name), #placeholder))
                        .bind(&id).execute(self).await?;
                    Ok(result.rows_affected() > 0)
                }
            }
        }
    }

    fn list_method(&self) -> TokenStream {
        let Self {
            entity_name,
            row_name,
            table,
            columns_str,
            id_name,
            dialect,
            soft_delete,
            ..
        } = self;
        let limit_placeholder = dialect.placeholder(1);
        let offset_placeholder = dialect.placeholder(2);
        let where_clause = if *soft_delete {
            "WHERE deleted_at IS NULL "
        } else {
            ""
        };

        quote! {
            async fn list(&self, limit: i64, offset: i64) -> Result<Vec<#entity_name>, Self::Error> {
                let rows: Vec<#row_name> = sqlx::query_as(
                    &format!("SELECT {} FROM {} {}ORDER BY {} DESC LIMIT {} OFFSET {}",
                        #columns_str, #table, #where_clause, stringify!(#id_name), #limit_placeholder, #offset_placeholder)
                ).bind(limit).bind(offset).fetch_all(self).await?;
                Ok(rows.into_iter().map(#entity_name::from).collect())
            }
        }
    }

    fn relation_methods(&self) -> TokenStream {
        let belongs_to_methods: Vec<TokenStream> = self
            .entity
            .relation_fields()
            .iter()
            .filter_map(|field| self.belongs_to_method(field))
            .collect();

        let has_many_methods: Vec<TokenStream> = self
            .entity
            .has_many_relations()
            .iter()
            .map(|related| self.has_many_method(related))
            .collect();

        quote! {
            #(#belongs_to_methods)*
            #(#has_many_methods)*
        }
    }

    fn belongs_to_method(&self, field: &FieldDef) -> Option<TokenStream> {
        let related_entity = field.belongs_to()?;
        let related_snake = related_entity.to_string().to_case(Case::Snake);
        let method_name = format_ident!("find_{}", related_snake);
        let related_row = format_ident!("{}Row", related_entity);
        let related_table = format!("public.{}s", related_snake);
        let fk_name = field.name();
        let id_type = self.id_type;
        let placeholder = self.dialect.placeholder(1);
        let trait_name = &self.trait_name;

        Some(quote! {
            async fn #method_name(&self, id: #id_type) -> Result<Option<#related_entity>, Self::Error> {
                let entity = <Self as #trait_name>::find_by_id(self, id).await?;
                match entity {
                    Some(e) => {
                        let row: Option<#related_row> = sqlx::query_as(
                            &format!("SELECT * FROM {} WHERE id = {}", #related_table, #placeholder)
                        ).bind(&e.#fk_name).fetch_optional(self).await?;
                        Ok(row.map(#related_entity::from))
                    }
                    None => Ok(None)
                }
            }
        })
    }

    fn has_many_method(&self, related: &syn::Ident) -> TokenStream {
        let related_snake = related.to_string().to_case(Case::Snake);
        let method_name = format_ident!("find_{}s", related_snake);
        let related_row = format_ident!("{}Row", related);
        let related_table = format!("public.{}s", related_snake);
        let entity_snake = self.entity.name_str().to_case(Case::Snake);
        let fk_field = format_ident!("{}_id", entity_snake);
        let id_type = self.id_type;
        let placeholder = self.dialect.placeholder(1);

        quote! {
            async fn #method_name(&self, #fk_field: #id_type) -> Result<Vec<#related>, Self::Error> {
                let rows: Vec<#related_row> = sqlx::query_as(
                    &format!("SELECT * FROM {} WHERE {}_id = {}", #related_table, #entity_snake, #placeholder)
                ).bind(&#fk_field).fetch_all(self).await?;
                Ok(rows.into_iter().map(#related::from).collect())
            }
        }
    }

    fn projection_methods(&self) -> TokenStream {
        let methods: Vec<TokenStream> = self
            .entity
            .projections
            .iter()
            .map(|proj| self.projection_method(proj))
            .collect();

        quote! { #(#methods)* }
    }

    fn projection_method(&self, proj: &crate::entity::parse::ProjectionDef) -> TokenStream {
        let entity_name = self.entity_name;
        let proj_snake = proj.name.to_string().to_case(Case::Snake);
        let method_name = format_ident!("find_by_id_{}", proj_snake);
        let proj_type = format_ident!("{}{}", entity_name, proj.name);
        let id_name = self.id_name;
        let id_type = self.id_type;
        let table = &self.table;
        let placeholder = self.dialect.placeholder(1);

        let columns_str: String = proj
            .fields
            .iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        quote! {
            async fn #method_name(&self, id: #id_type) -> Result<Option<#proj_type>, Self::Error> {
                let row = sqlx::query_as::<_, #proj_type>(
                    &format!("SELECT {} FROM {} WHERE {} = {}", #columns_str, #table, stringify!(#id_name), #placeholder)
                ).bind(&id).fetch_optional(self).await?;
                Ok(row)
            }
        }
    }

    fn soft_delete_methods(&self) -> TokenStream {
        if !self.soft_delete {
            return TokenStream::new();
        }

        let hard_delete = self.hard_delete_method();
        let restore = self.restore_method();
        let find_with_deleted = self.find_by_id_with_deleted_method();
        let list_with_deleted = self.list_with_deleted_method();

        quote! {
            #hard_delete
            #restore
            #find_with_deleted
            #list_with_deleted
        }
    }

    fn hard_delete_method(&self) -> TokenStream {
        let Self {
            table,
            id_name,
            id_type,
            dialect,
            ..
        } = self;
        let placeholder = dialect.placeholder(1);

        quote! {
            async fn hard_delete(&self, id: #id_type) -> Result<bool, Self::Error> {
                let result = sqlx::query(&format!(
                    "DELETE FROM {} WHERE {} = {}",
                    #table, stringify!(#id_name), #placeholder
                )).bind(&id).execute(self).await?;
                Ok(result.rows_affected() > 0)
            }
        }
    }

    fn restore_method(&self) -> TokenStream {
        let Self {
            table,
            id_name,
            id_type,
            dialect,
            ..
        } = self;
        let placeholder = dialect.placeholder(1);

        quote! {
            async fn restore(&self, id: #id_type) -> Result<bool, Self::Error> {
                let result = sqlx::query(&format!(
                    "UPDATE {} SET deleted_at = NULL WHERE {} = {} AND deleted_at IS NOT NULL",
                    #table, stringify!(#id_name), #placeholder
                )).bind(&id).execute(self).await?;
                Ok(result.rows_affected() > 0)
            }
        }
    }

    fn find_by_id_with_deleted_method(&self) -> TokenStream {
        let Self {
            entity_name,
            row_name,
            table,
            columns_str,
            id_name,
            id_type,
            dialect,
            ..
        } = self;
        let placeholder = dialect.placeholder(1);

        quote! {
            async fn find_by_id_with_deleted(&self, id: #id_type) -> Result<Option<#entity_name>, Self::Error> {
                let row: Option<#row_name> = sqlx::query_as(
                    &format!("SELECT {} FROM {} WHERE {} = {}", #columns_str, #table, stringify!(#id_name), #placeholder)
                ).bind(&id).fetch_optional(self).await?;
                Ok(row.map(#entity_name::from))
            }
        }
    }

    fn list_with_deleted_method(&self) -> TokenStream {
        let Self {
            entity_name,
            row_name,
            table,
            columns_str,
            id_name,
            dialect,
            ..
        } = self;
        let limit_placeholder = dialect.placeholder(1);
        let offset_placeholder = dialect.placeholder(2);

        quote! {
            async fn list_with_deleted(&self, limit: i64, offset: i64) -> Result<Vec<#entity_name>, Self::Error> {
                let rows: Vec<#row_name> = sqlx::query_as(
                    &format!("SELECT {} FROM {} ORDER BY {} DESC LIMIT {} OFFSET {}",
                        #columns_str, #table, stringify!(#id_name), #limit_placeholder, #offset_placeholder)
                ).bind(limit).bind(offset).fetch_all(self).await?;
                Ok(rows.into_iter().map(#entity_name::from).collect())
            }
        }
    }
}

/// Join field names into comma-separated column list.
fn join_columns(fields: &[FieldDef]) -> String {
    fields
        .iter()
        .map(|f| f.name_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Build `.bind(insertable.field)` chain for INSERT.
fn insert_bindings(fields: &[FieldDef]) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|f| {
            let name = f.name();
            quote! { .bind(insertable.#name) }
        })
        .collect()
}

/// Build `.bind(dto.field)` chain for UPDATE.
fn update_bindings(fields: &[&FieldDef]) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|f| {
            let name = f.name();
            quote! { .bind(dto.#name) }
        })
        .collect()
}
