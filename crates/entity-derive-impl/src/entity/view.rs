// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Joined read-model generation from `#[join(...)]` declarations.
//!
//! For an entity with at least one `#[join(...)]`, generates:
//!
//! - `{Entity}View` — a flat struct with every entity column plus the declared
//!   joined columns, deriving `sqlx::FromRow` and `serde::Serialize`
//! - `{Entity}View::SELECT` — the canonical `SELECT ... FROM ... JOIN ...`
//!   fragment (no WHERE), for custom filters via `format!("{} WHERE ...",
//!   TicketView::SELECT)`
//! - `{Entity}View::find_by_id(pool, id)` — single row by primary key
//! - `{Entity}View::list(pool, limit, offset)` — id-descending page
//!
//! Joins are `INNER JOIN`s; the base table is aliased by its own name
//! and every column is qualified, so join aliases can never collide
//! with it. The joined columns' Rust types are part of the `#[join]`
//! declaration — the macro cannot see the foreign table's schema.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::parse::{EntityDef, SqlLevel};
use crate::utils::marker;

/// Generate the read-model struct and its inherent impl.
///
/// Returns empty tokens when the entity has no `#[join(...)]`
/// declarations or `sql = "none"`.
pub fn generate(entity: &EntityDef) -> TokenStream {
    if entity.joins.is_empty() || entity.sql == SqlLevel::None {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let entity_name = entity.name();
    let view_name = format_ident!("{}View", entity_name);
    let marker = marker::generated();
    let id_type = entity.id_field().ty();
    let id_name = entity.id_field().name_str();
    let table = entity.table_name();

    let entity_fields: Vec<TokenStream> = entity
        .column_fields()
        .into_iter()
        .map(|f| {
            let name = f.name();
            let ty = f.ty();
            quote! { pub #name: #ty }
        })
        .collect();
    let join_fields: Vec<TokenStream> = entity
        .joins
        .iter()
        .flat_map(|j| &j.fields)
        .map(|f| {
            let name = &f.alias;
            let ty = &f.ty;
            quote! { pub #name: #ty }
        })
        .collect();

    let select_sql = build_select(entity);
    let find_sql = format!("{select_sql} WHERE {table}.{id_name} = $1");
    let list_sql = format!("{select_sql} ORDER BY {table}.{id_name} DESC LIMIT $1 OFFSET $2");

    let doc = format!(
        "Joined read model for [`{entity_name}`], generated from its `#[join(...)]` declarations."
    );
    let select_doc = format!(
        "Canonical `SELECT ... FROM ... JOIN ...` fragment (no WHERE clause).\n\n\
         Compose custom filters with\n\
         `sqlx::query_as::<_, {view_name}>(&format!(\"{{}} WHERE ...\", Self::SELECT))`."
    );

    quote! {
        #marker
        #[doc = #doc]
        #[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
        #vis struct #view_name {
            #(#entity_fields,)*
            #(#join_fields,)*
        }

        impl #view_name {
            #[doc = #select_doc]
            pub const SELECT: &'static str = #select_sql;

            /// Fetch a single row of the read model by primary key.
            pub async fn find_by_id(
                pool: &sqlx::PgPool,
                id: #id_type
            ) -> Result<Option<Self>, sqlx::Error> {
                sqlx::query_as(#find_sql).bind(&id).fetch_optional(pool).await
            }

            /// List the read model, newest ids first.
            pub async fn list(
                pool: &sqlx::PgPool,
                limit: i64,
                offset: i64
            ) -> Result<Vec<Self>, sqlx::Error> {
                sqlx::query_as(#list_sql)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(pool)
                    .await
            }
        }
    }
}

/// Build the canonical SELECT fragment for the view.
fn build_select(entity: &EntityDef) -> String {
    let table = entity.table_name();

    let mut columns: Vec<String> = entity
        .column_fields()
        .into_iter()
        .map(|f| format!("{table}.{}", f.name_str()))
        .collect();
    for join in &entity.joins {
        for field in &join.fields {
            columns.push(format!(
                "{}.{} AS {}",
                join.alias, field.source, field.alias
            ));
        }
    }

    let mut sql = format!("SELECT {} FROM {table}", columns.join(", "));
    for join in &entity.joins {
        sql.push_str(&format!(
            " JOIN {} AS {} ON {}.{} = {table}.{}",
            join.table, join.alias, join.alias, join.foreign_column, join.local_column
        ));
    }
    sql
}

#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::DeriveInput;

    use super::*;

    fn parse_entity(tokens: proc_macro2::TokenStream) -> EntityDef {
        let input: DeriveInput = syn::parse2(tokens).expect("test entity must parse");
        EntityDef::from_derive_input(&input).expect("test entity must be valid")
    }

    fn ticket_entity() -> EntityDef {
        parse_entity(quote! {
            #[entity(table = "tickets")]
            #[join(airports as origin, on = origin_iata = iata, fields(
                lat as origin_lat: f64,
                city as origin_city: String
            ))]
            #[join(airports as dest, on = destination_iata = iata, fields(
                lat as destination_lat: f64
            ))]
            pub struct Ticket {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                pub origin_iata: String,
                #[field(create, response)]
                pub destination_iata: String,
            }
        })
    }

    #[test]
    fn generates_view_struct_and_methods() {
        let code = generate(&ticket_entity()).to_string();
        assert!(code.contains("pub struct TicketView"));
        assert!(code.contains("pub origin_lat : f64"));
        assert!(code.contains("pub const SELECT"));
        assert!(code.contains("pub async fn find_by_id"));
        assert!(code.contains("pub async fn list"));
    }

    #[test]
    fn select_qualifies_and_aliases_columns() {
        let sql = build_select(&ticket_entity());
        assert_eq!(
            sql,
            "SELECT tickets.id, tickets.origin_iata, tickets.destination_iata, \
             origin.lat AS origin_lat, origin.city AS origin_city, dest.lat AS destination_lat \
             FROM tickets \
             JOIN airports AS origin ON origin.iata = tickets.origin_iata \
             JOIN airports AS dest ON dest.iata = tickets.destination_iata"
        );
    }

    #[test]
    fn no_joins_generates_nothing() {
        let entity = parse_entity(quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                pub name: String,
            }
        });
        assert!(generate(&entity).is_empty());
    }
}
