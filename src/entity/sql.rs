//! SQL implementation generation for the Entity derive macro.
//!
//! Generates `impl {Entity}Repository for sqlx::PgPool` with CRUD queries.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::parse::{EntityDef, SqlLevel};
use crate::utils::sql;

/// Generate SQL implementation for `PgPool`.
pub fn generate(entity: &EntityDef) -> TokenStream {
    if entity.sql != SqlLevel::Full {
        return TokenStream::new();
    }

    let ctx = SqlContext::new(entity);
    let trait_name = &ctx.trait_name;
    let create_impl = ctx.create_method();
    let find_impl = ctx.find_by_id_method();
    let update_impl = ctx.update_method();
    let delete_impl = ctx.delete_method();
    let list_impl = ctx.list_method();

    quote! {
        #[cfg(feature = "db")]
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

struct SqlContext<'a> {
    entity:           &'a EntityDef,
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

impl<'a> SqlContext<'a> {
    fn new(entity: &'a EntityDef) -> Self {
        let id_field = entity.id_field().expect("Entity must have #[id] field");
        let fields = entity.all_fields();

        Self {
            entity,
            trait_name: format_ident!("{}Repository", entity.name()),
            entity_name: entity.name(),
            row_name: entity.ident_with("", "Row"),
            insertable_name: entity.ident_with("Insertable", ""),
            create_dto: entity.ident_with("Create", "Request"),
            update_dto: entity.ident_with("Update", "Request"),
            table: entity.full_table_name(),
            id_name: id_field.name(),
            id_type: id_field.ty(),
            columns_str: sql::join_columns(fields),
            placeholders_str: sql::placeholders(fields.len())
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
        let bindings = sql::insert_bindings(entity.all_fields());

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
            ..
        } = self;

        quote! {
            async fn find_by_id(&self, id: #id_type) -> Result<Option<#entity_name>, Self::Error> {
                let row: Option<#row_name> = sqlx::query_as(
                    &format!("SELECT {} FROM {} WHERE {} = $1", #columns_str, #table, stringify!(#id_name))
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
            ..
        } = self;
        let set_clause = sql::set_clause(&update_fields);
        let where_idx = update_fields.len() + 1;
        let bindings = sql::update_bindings(&update_fields);

        quote! {
            async fn update(&self, id: #id_type, dto: #update_dto) -> Result<#entity_name, Self::Error> {
                sqlx::query(&format!("UPDATE {} SET {} WHERE {} = ${}", #table, #set_clause, stringify!(#id_name), #where_idx))
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
            ..
        } = self;

        quote! {
            async fn delete(&self, id: #id_type) -> Result<bool, Self::Error> {
                let result = sqlx::query(&format!("DELETE FROM {} WHERE {} = $1", #table, stringify!(#id_name)))
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
            ..
        } = self;

        quote! {
            async fn list(&self, limit: i64, offset: i64) -> Result<Vec<#entity_name>, Self::Error> {
                let rows: Vec<#row_name> = sqlx::query_as(
                    &format!("SELECT {} FROM {} ORDER BY {} DESC LIMIT $1 OFFSET $2", #columns_str, #table, stringify!(#id_name))
                ).bind(limit).bind(offset).fetch_all(self).await?;
                Ok(rows.into_iter().map(#entity_name::from).collect())
            }
        }
    }
}
