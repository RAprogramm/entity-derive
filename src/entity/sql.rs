//! SQL implementation generation for Entity derive macro.
//!
//! Generates impl Repository for PgPool with actual SQL queries.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::parse::{EntityDef, SqlLevel};

/// Generate SQL implementation for PgPool.
pub fn generate(entity: &EntityDef) -> TokenStream {
    if entity.sql != SqlLevel::Full {
        return TokenStream::new();
    }

    let entity_name = entity.name();
    let trait_name = format_ident!("{}Repository", entity_name);
    let row_name = entity.ident_with("", "Row");
    let insertable_name = entity.ident_with("Insertable", "");
    let create_dto = entity.ident_with("Create", "Request");
    let update_dto = entity.ident_with("Update", "Request");

    let table = entity.full_table_name();
    let fields = entity.all_fields();

    let id_field = entity.id_field().expect("Entity must have #[id] field");
    let id_name = id_field.name();
    let id_type = id_field.ty();

    // Column names for INSERT
    let column_names: Vec<_> = fields.iter().map(|f| f.name_str()).collect();
    let columns_str = column_names.join(", ");

    // Placeholders for INSERT ($1, $2, ...)
    let placeholders: Vec<_> = (1..=fields.len()).map(|i| format!("${i}")).collect();
    let placeholders_str = placeholders.join(", ");

    // Field bindings for INSERT
    let insert_bindings: Vec<_> = fields
        .iter()
        .map(|f| {
            let name = f.name();
            quote! { insertable.#name }
        })
        .collect();

    // SELECT columns
    let select_columns = columns_str.clone();

    // Create method
    let has_create = !entity.create_fields().is_empty();
    let create_impl = if has_create {
        quote! {
            async fn create(&self, dto: #create_dto) -> Result<#entity_name, Self::Error> {
                let entity = #entity_name::from(dto);
                let insertable = #insertable_name::from(&entity);

                sqlx::query(
                    concat!(
                        "INSERT INTO ", #table, " (", #columns_str, ") ",
                        "VALUES (", #placeholders_str, ")"
                    )
                )
                #(.bind(#insert_bindings))*
                .execute(self)
                .await?;

                Ok(entity)
            }
        }
    } else {
        TokenStream::new()
    };

    // Update method
    let has_update = !entity.update_fields().is_empty();
    let update_impl = if has_update {
        let update_fields = entity.update_fields();
        let set_clauses: Vec<_> = update_fields
            .iter()
            .enumerate()
            .map(|(i, f)| format!("{} = ${}", f.name_str(), i + 1))
            .collect();
        let set_str = set_clauses.join(", ");
        let where_idx = update_fields.len() + 1;

        let update_bindings: Vec<_> = update_fields
            .iter()
            .map(|f| {
                let name = f.name();
                quote! { dto.#name }
            })
            .collect();

        quote! {
            async fn update(&self, id: #id_type, dto: #update_dto) -> Result<#entity_name, Self::Error> {
                sqlx::query(
                    &format!(
                        "UPDATE {} SET {} WHERE {} = ${}",
                        #table, #set_str, stringify!(#id_name), #where_idx
                    )
                )
                #(.bind(#update_bindings))*
                .bind(&id)
                .execute(self)
                .await?;

                self.find_by_id(id).await?.ok_or_else(|| {
                    sqlx::Error::RowNotFound
                })
            }
        }
    } else {
        TokenStream::new()
    };

    quote! {
        #[cfg(feature = "db")]
        #[async_trait::async_trait]
        impl #trait_name for sqlx::PgPool {
            type Error = sqlx::Error;

            #create_impl

            async fn find_by_id(&self, id: #id_type) -> Result<Option<#entity_name>, Self::Error> {
                let row: Option<#row_name> = sqlx::query_as(
                    &format!(
                        "SELECT {} FROM {} WHERE {} = $1",
                        #select_columns, #table, stringify!(#id_name)
                    )
                )
                .bind(&id)
                .fetch_optional(self)
                .await?;

                Ok(row.map(#entity_name::from))
            }

            #update_impl

            async fn delete(&self, id: #id_type) -> Result<bool, Self::Error> {
                let result = sqlx::query(
                    &format!(
                        "DELETE FROM {} WHERE {} = $1",
                        #table, stringify!(#id_name)
                    )
                )
                .bind(&id)
                .execute(self)
                .await?;

                Ok(result.rows_affected() > 0)
            }

            async fn list(&self, limit: i64, offset: i64) -> Result<Vec<#entity_name>, Self::Error> {
                let rows: Vec<#row_name> = sqlx::query_as(
                    &format!(
                        "SELECT {} FROM {} ORDER BY {} DESC LIMIT $1 OFFSET $2",
                        #select_columns, #table, stringify!(#id_name)
                    )
                )
                .bind(limit)
                .bind(offset)
                .fetch_all(self)
                .await?;

                Ok(rows.into_iter().map(#entity_name::from).collect())
            }
        }
    }
}
