// SPDX-FileCopyrightText: 2025 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! SQL implementation generation for the Entity derive macro.
//!
//! Generates database-specific repository implementations based on dialect.
//!
//! # Supported Dialects
//!
//! | Dialect | Feature | Client | Status |
//! |---------|---------|--------|--------|
//! | PostgreSQL | `postgres` | `sqlx::PgPool` | Stable |
//! | ClickHouse | `clickhouse` | `clickhouse::Client` | Planned |
//! | MongoDB | `mongodb` | `mongodb::Client` | Planned |

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::parse::{DatabaseDialect, EntityDef, SqlLevel};

/// Generate SQL implementation based on entity dialect.
pub fn generate(entity: &EntityDef) -> TokenStream {
    if entity.sql != SqlLevel::Full {
        return TokenStream::new();
    }

    match entity.dialect {
        DatabaseDialect::Postgres => generate_postgres(entity),
        DatabaseDialect::ClickHouse => generate_clickhouse(entity),
        DatabaseDialect::MongoDB => generate_mongodb(entity)
    }
}

/// Generate PostgreSQL implementation for `sqlx::PgPool`.
fn generate_postgres(entity: &EntityDef) -> TokenStream {
    let ctx = PostgresContext::new(entity);
    let trait_name = &ctx.trait_name;
    let feature = entity.dialect.feature_flag();

    let create_impl = ctx.create_method();
    let find_impl = ctx.find_by_id_method();
    let update_impl = ctx.update_method();
    let delete_impl = ctx.delete_method();
    let list_impl = ctx.list_method();

    quote! {
        #[cfg(feature = #feature)]
        #[async_trait::async_trait]
        impl #trait_name for sqlx::PgPool {
            type Error = sqlx::Error;
            #create_impl
            #find_impl
            #update_impl
            #delete_impl
            #list_impl
        }
    }
}

/// Generate ClickHouse implementation (placeholder).
fn generate_clickhouse(entity: &EntityDef) -> TokenStream {
    let _trait_name = format_ident!("{}Repository", entity.name());
    let feature = entity.dialect.feature_flag();

    // ClickHouse implementation will be added later
    // For now, generate a compile error if someone tries to use it
    quote! {
        #[cfg(feature = #feature)]
        compile_error!("ClickHouse support is not yet implemented. Use dialect = \"postgres\" or sql = \"trait\" to implement manually.");
    }
}

/// Generate MongoDB implementation (placeholder).
fn generate_mongodb(entity: &EntityDef) -> TokenStream {
    let _trait_name = format_ident!("{}Repository", entity.name());
    let feature = entity.dialect.feature_flag();

    // MongoDB implementation will be added later
    quote! {
        #[cfg(feature = #feature)]
        compile_error!("MongoDB support is not yet implemented. Use dialect = \"postgres\" or sql = \"trait\" to implement manually.");
    }
}

struct PostgresContext<'a> {
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
    placeholders_str: String
}

impl<'a> PostgresContext<'a> {
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
            placeholders_str: dialect.placeholders(fields.len())
        }
    }

    fn create_method(&self) -> TokenStream {
        if self.entity.create_fields().is_empty() {
            return TokenStream::new();
        }

        let Self {
            entity_name,
            insertable_name,
            create_dto,
            table,
            columns_str,
            placeholders_str,
            entity,
            ..
        } = self;
        let bindings = insert_bindings(entity.all_fields());

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

    fn find_by_id_method(&self) -> TokenStream {
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
            async fn find_by_id(&self, id: #id_type) -> Result<Option<#entity_name>, Self::Error> {
                let row: Option<#row_name> = sqlx::query_as(
                    &format!("SELECT {} FROM {} WHERE {} = {}", #columns_str, #table, stringify!(#id_name), #placeholder)
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
            update_dto,
            table,
            id_name,
            id_type,
            dialect,
            ..
        } = self;

        let field_names: Vec<&str> = update_fields
            .iter()
            .map(|f| f.name_str().leak() as &str)
            .collect();
        let set_clause = dialect.set_clause(&field_names);
        let where_placeholder = dialect.placeholder(update_fields.len() + 1);
        let bindings = update_bindings(&update_fields);

        quote! {
            async fn update(&self, id: #id_type, dto: #update_dto) -> Result<#entity_name, Self::Error> {
                sqlx::query(&format!("UPDATE {} SET {} WHERE {} = {}", #table, #set_clause, stringify!(#id_name), #where_placeholder))
                    #(#bindings)*
                    .bind(&id)
                    .execute(self).await?;
                self.find_by_id(id).await?.ok_or_else(|| sqlx::Error::RowNotFound)
            }
        }
    }

    fn delete_method(&self) -> TokenStream {
        let Self {
            table,
            id_name,
            id_type,
            dialect,
            ..
        } = self;
        let placeholder = dialect.placeholder(1);

        quote! {
            async fn delete(&self, id: #id_type) -> Result<bool, Self::Error> {
                let result = sqlx::query(&format!("DELETE FROM {} WHERE {} = {}", #table, stringify!(#id_name), #placeholder))
                    .bind(&id).execute(self).await?;
                Ok(result.rows_affected() > 0)
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
            ..
        } = self;
        let limit_placeholder = dialect.placeholder(1);
        let offset_placeholder = dialect.placeholder(2);

        quote! {
            async fn list(&self, limit: i64, offset: i64) -> Result<Vec<#entity_name>, Self::Error> {
                let rows: Vec<#row_name> = sqlx::query_as(
                    &format!("SELECT {} FROM {} ORDER BY {} DESC LIMIT {} OFFSET {}",
                        #columns_str, #table, stringify!(#id_name), #limit_placeholder, #offset_placeholder)
                ).bind(limit).bind(offset).fetch_all(self).await?;
                Ok(rows.into_iter().map(#entity_name::from).collect())
            }
        }
    }
}

// Helper functions moved from utils/sql.rs to avoid circular dependency

use super::parse::FieldDef;

/// Join field names with comma separator.
fn join_columns(fields: &[FieldDef]) -> String {
    fields
        .iter()
        .map(|f| f.name_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Build `.bind(insertable.field)` chain.
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
