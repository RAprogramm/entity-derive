// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Schema-assertion codegen: `{Entity}::SCHEMA` and
//! `{Entity}::assert_schema`.
//!
//! Generated repositories build SQL from entity metadata, so table
//! drift surfaces as runtime decode errors. The generated constant
//! describes the declared columns (name, DDL type, nullability) and
//! `assert_schema(pool)` compares it against
//! `information_schema.columns` via `entity_core::schema::assert_table`
//! — one integration test per entity restores a compile-time-like
//! guarantee:
//!
//! ```rust,ignore
//! User::assert_schema(&pool).await.expect("users table drifted");
//! ```

use proc_macro2::TokenStream;
use quote::quote;

use super::{
    migrations::types::{PostgresTypeMapper, TypeMapper},
    parse::{DatabaseDialect, EntityDef, SqlLevel}
};
use crate::utils::marker;

/// Generate the schema constant and assertion method.
///
/// Returns empty tokens for `sql = "none"` entities or non-Postgres
/// dialects.
pub fn generate(entity: &EntityDef) -> TokenStream {
    if entity.sql == SqlLevel::None || entity.dialect != DatabaseDialect::Postgres {
        return TokenStream::new();
    }

    let entity_name = entity.name();
    let table = entity.table_name();
    let schema = if entity.schema.is_empty() {
        "public".to_string()
    } else {
        entity.schema.clone()
    };
    let marker = marker::generated();
    let mapper = PostgresTypeMapper;

    let columns: Vec<TokenStream> = entity
        .column_fields()
        .into_iter()
        .map(|f| {
            let name = f.name_str();
            let sql_type = mapper.map_type(f.ty(), f.column());
            let type_str = sql_type.to_sql_string();
            let nullable = sql_type.nullable;
            quote! {
                ::entity_core::schema::SchemaColumn {
                    name: #name,
                    sql_type: #type_str,
                    nullable: #nullable
                }
            }
        })
        .collect();

    let schema_doc =
        format!("Declared column shape of `{table}`, generated from the entity definition.");

    quote! {
        #marker
        impl #entity_name {
            #[doc = #schema_doc]
            pub const SCHEMA: ::entity_core::schema::TableSchema =
                ::entity_core::schema::TableSchema {
                    table: #table,
                    schema: #schema,
                    columns: &[#(#columns),*]
                };

            /// Assert the live table matches the entity declaration.
            ///
            /// Compares every declared column against
            /// `information_schema.columns` (presence, nullability, type
            /// family) and reports all drifts at once. Run it once per
            /// entity in an integration test.
            pub async fn assert_schema(
                pool: &sqlx::PgPool
            ) -> Result<(), ::entity_core::schema::SchemaCheckError> {
                ::entity_core::schema::assert_table(pool, &Self::SCHEMA).await
            }
        }
    }
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

    #[test]
    fn emits_schema_constant_and_assertion() {
        let entity = parse_entity(quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                pub username: Option<String>,
                #[auto]
                pub created_at: chrono::DateTime<chrono::Utc>,
            }
        });
        let code = generate(&entity).to_string();
        assert!(code.contains("pub const SCHEMA"));
        assert!(code.contains("pub async fn assert_schema"));
        assert!(code.contains("name : \"username\" , sql_type : \"TEXT\" , nullable : true"));
        assert!(code.contains("name : \"id\" , sql_type : \"UUID\" , nullable : false"));
    }

    #[test]
    fn sql_none_emits_nothing() {
        let entity = parse_entity(quote! {
            #[entity(table = "dto_only", sql = "none")]
            pub struct DtoOnly {
                #[id]
                pub id: uuid::Uuid,
            }
        });
        assert!(generate(&entity).is_empty());
    }
}
