// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! `PostgreSQL` migration generation.
//!
//! Generates `MIGRATION_UP` and `MIGRATION_DOWN` constants for `PostgreSQL`.

mod ddl;

use convert_case::Casing;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

use crate::{entity::parse::EntityDef, utils::marker};

/// Strip `Option<...>` and `Vec<...>` wrappers to reach the enum type.
fn base_type(ty: &Type) -> &Type {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
        && (segment.ident == "Option" || segment.ident == "Vec")
        && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
    {
        return base_type(inner);
    }
    ty
}

/// Generate migration constants for `PostgreSQL`.
///
/// # Generated Code
///
/// ```rust,ignore
/// impl User {
///     pub const MIGRATION_UP: &'static str = "CREATE TABLE...";
///     pub const MIGRATION_DOWN: &'static str = "DROP TABLE...";
/// }
/// ```
pub fn generate(entity: &EntityDef) -> TokenStream {
    let entity_name = entity.name();
    let vis = &entity.vis;

    let mut up_sql = ddl::generate_up(entity);
    for field in entity.search_fields() {
        let column = field.name_str();
        let table = entity.full_table_name();
        let base = entity.table_name();
        up_sql.push_str(&format!(
            " CREATE INDEX IF NOT EXISTS idx_{base}_{column}_trgm \
             ON {table} USING gin ({column} gin_trgm_ops);"
        ));
    }
    let down_sql = ddl::generate_down(entity);

    let enum_fields: Vec<(&Type, String)> = entity
        .all_fields()
        .iter()
        .filter_map(|f| {
            f.column
                .pg_enum
                .as_ref()
                .map(|pg_enum| (base_type(f.ty()), pg_enum.clone()))
        })
        .collect();

    let create_type_refs: Vec<TokenStream> = enum_fields
        .iter()
        .map(|(ty, _)| quote! { <#ty>::PG_CREATE_TYPE })
        .collect();

    let name_assertions: Vec<TokenStream> = enum_fields
        .iter()
        .map(|(ty, declared)| {
            quote! {
                assert!(
                    ::entity_derive::const_str_eq(<#ty>::PG_TYPE, #declared),
                    "#[column(pg_enum = ...)] does not match the ValueObject's pg_type"
                );
            }
        })
        .collect();

    let outbox_const = outbox_migration_const(entity);
    let junctions_const = junction_migration_const(entity);
    let triggers_const = trigger_migration_const(entity);
    let extensions_const = extension_migration_const(entity);
    let marker = marker::generated();

    quote! {
        #marker
        impl #entity_name {
            /// SQL migration to create this entity's table, indexes, and constraints.
            ///
            /// # Usage
            ///
            /// ```rust,ignore
            /// sqlx::query(User::MIGRATION_UP).execute(&pool).await?;
            /// ```
            #vis const MIGRATION_UP: &'static str = #up_sql;

            /// SQL migration to drop this entity's table.
            ///
            /// Uses CASCADE to drop dependent objects.
            #vis const MIGRATION_DOWN: &'static str = #down_sql;

            /// Idempotent DDL for Postgres enum types this table depends on.
            ///
            /// One statement per `#[column(pg_enum = "...")]` field, in
            /// declaration order. Execute before [`Self::MIGRATION_UP`]:
            ///
            /// ```rust,ignore
            /// for ddl in User::MIGRATION_TYPES {
            ///     sqlx::query(ddl).execute(&pool).await?;
            /// }
            /// sqlx::query(User::MIGRATION_UP).execute(&pool).await?;
            /// ```
            #vis const MIGRATION_TYPES: &'static [&'static str] = &[#(#create_type_refs),*];

            #outbox_const

            #junctions_const

            #triggers_const

            #extensions_const
        }

        const _: () = {
            #(#name_assertions)*
        };
    }
}

/// Generate the `MIGRATION_OUTBOX` constant for outbox-enabled entities.
///
/// The DDL is idempotent and shared: every outbox entity emits the same
/// statement, so running it once per entity is safe.
fn outbox_migration_const(entity: &EntityDef) -> TokenStream {
    if !entity.has_outbox() {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let ddl = "CREATE TABLE IF NOT EXISTS entity_outbox (\
                   id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY, \
                   entity TEXT NOT NULL, \
                   kind TEXT NOT NULL, \
                   entity_id TEXT NOT NULL, \
                   payload JSONB NOT NULL, \
                   attempts INTEGER NOT NULL DEFAULT 0, \
                   next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), \
                   created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), \
                   processed_at TIMESTAMPTZ\
               ); \
               CREATE INDEX IF NOT EXISTS idx_entity_outbox_pending \
               ON entity_outbox (next_attempt_at) WHERE processed_at IS NULL;";

    quote! {
        /// Idempotent DDL for the shared transactional-outbox table.
        ///
        /// Execute alongside [`Self::MIGRATION_UP`]. Safe to run once
        /// per outbox-enabled entity — the statements are `IF NOT EXISTS`.
        #vis const MIGRATION_OUTBOX: &'static str = #ddl;
    }
}

/// Generate the `MIGRATION_JUNCTIONS` constant for many-to-many relations.
///
/// One idempotent `CREATE TABLE IF NOT EXISTS` statement per
/// `#[has_many(Child, through = "...")]` declaration: composite primary
/// key over both foreign keys, `ON DELETE CASCADE` on each side. The
/// child id column type mirrors the parent's `#[id]` type.
fn junction_migration_const(entity: &EntityDef) -> TokenStream {
    let through: Vec<_> = entity
        .has_many_relations()
        .iter()
        .filter_map(|r| r.through.as_ref().map(|j| (r, j)))
        .collect();
    if through.is_empty() {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let mapper = crate::entity::migrations::types::PostgresTypeMapper;
    let id_field = entity.id_field();
    let id_sql = crate::entity::migrations::types::TypeMapper::map_type(
        &mapper,
        id_field.ty(),
        &Default::default()
    )
    .name;
    let parent_snake = entity.name_str().to_case(convert_case::Case::Snake);
    let parent_table = entity.full_table_name();

    let ddls: Vec<String> = through
        .iter()
        .map(|(relation, junction)| {
            let child_snake = relation
                .entity
                .to_string()
                .to_case(convert_case::Case::Snake);
            let child_table = entity.full_table_name_for(&format!("{child_snake}s"));
            let junction_table = entity.full_table_name_for(junction);
            format!(
                "CREATE TABLE IF NOT EXISTS {junction_table} (\
                     {parent_snake}_id {id_sql} NOT NULL REFERENCES {parent_table}(id) ON DELETE CASCADE, \
                     {child_snake}_id {id_sql} NOT NULL REFERENCES {child_table}(id) ON DELETE CASCADE, \
                     PRIMARY KEY ({parent_snake}_id, {child_snake}_id)\
                 );"
            )
        })
        .collect();

    quote! {
        /// Idempotent DDL for many-to-many junction tables.
        ///
        /// Execute after [`Self::MIGRATION_UP`] of both related tables.
        #vis const MIGRATION_JUNCTIONS: &'static [&'static str] = &[#(#ddls),*];
    }
}

/// Generate the `MIGRATION_TRIGGERS` constant.
///
/// Emitted when `migrations(touch_updated_at)` and/or
/// `migrations(audit)` are set. All DDL is idempotent
/// (`CREATE OR REPLACE` / `DROP TRIGGER IF EXISTS`).
fn trigger_migration_const(entity: &EntityDef) -> TokenStream {
    if !entity.touch_updated_at && !entity.audit {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let table = entity.full_table_name();
    let trigger_base = entity.table_name();
    let mut ddls: Vec<String> = Vec::new();

    if entity.touch_updated_at {
        ddls.push(
            "CREATE OR REPLACE FUNCTION entity_touch_updated_at() RETURNS TRIGGER AS $$ \
             BEGIN NEW.updated_at = NOW(); RETURN NEW; END; $$ LANGUAGE plpgsql;"
                .to_string()
        );
        ddls.push(format!(
            "DROP TRIGGER IF EXISTS trg_{trigger_base}_touch_updated_at ON {table}; \
             CREATE TRIGGER trg_{trigger_base}_touch_updated_at BEFORE UPDATE ON {table} \
             FOR EACH ROW EXECUTE FUNCTION entity_touch_updated_at();"
        ));
    }

    if entity.audit {
        ddls.push(
            "CREATE TABLE IF NOT EXISTS entity_audit_log (\
                 id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY, \
                 table_name TEXT NOT NULL, \
                 operation TEXT NOT NULL, \
                 old_row JSONB, \
                 new_row JSONB, \
                 recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()\
             );"
            .to_string()
        );
        ddls.push(
            "CREATE OR REPLACE FUNCTION entity_audit() RETURNS TRIGGER AS $$ \
             BEGIN \
                 INSERT INTO entity_audit_log (table_name, operation, old_row, new_row) \
                 VALUES (TG_TABLE_NAME, TG_OP, to_jsonb(OLD), to_jsonb(NEW)); \
                 RETURN COALESCE(NEW, OLD); \
             END; $$ LANGUAGE plpgsql;"
                .to_string()
        );
        ddls.push(format!(
            "DROP TRIGGER IF EXISTS trg_{trigger_base}_audit ON {table}; \
             CREATE TRIGGER trg_{trigger_base}_audit AFTER INSERT OR UPDATE OR DELETE ON {table} \
             FOR EACH ROW EXECUTE FUNCTION entity_audit();"
        ));
    }

    quote! {
        /// Idempotent trigger DDL for this table.
        ///
        /// Execute after [`Self::MIGRATION_UP`]. Includes the shared
        /// trigger functions (safe to run once per entity).
        #vis const MIGRATION_TRIGGERS: &'static [&'static str] = &[#(#ddls),*];
    }
}

/// Generate the `MIGRATION_EXTENSIONS` constant.
fn extension_migration_const(entity: &EntityDef) -> TokenStream {
    let mut extensions: Vec<String> = entity.extensions.clone();
    if !entity.search_fields().is_empty() && !extensions.iter().any(|e| e == "pg_trgm") {
        extensions.push("pg_trgm".to_string());
    }
    if extensions.is_empty() {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let ddls: Vec<String> = extensions
        .iter()
        .map(|ext| format!("CREATE EXTENSION IF NOT EXISTS \"{ext}\";"))
        .collect();

    quote! {
        /// Idempotent `CREATE EXTENSION` statements this table relies on.
        ///
        /// Execute before [`Self::MIGRATION_UP`].
        #vis const MIGRATION_EXTENSIONS: &'static [&'static str] = &[#(#ddls),*];
    }
}

#[cfg(test)]
mod pg_enum_tests {
    use quote::quote;
    use syn::DeriveInput;

    use super::*;

    fn parse_entity(tokens: proc_macro2::TokenStream) -> EntityDef {
        let input: DeriveInput = syn::parse2(tokens).expect("test entity must parse");
        EntityDef::from_derive_input(&input).expect("test entity must be valid")
    }

    fn status_entity() -> EntityDef {
        parse_entity(quote! {
            #[entity(table = "orders", migrations)]
            pub struct Order {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                #[column(pg_enum = "order_status")]
                pub status: OrderStatus,
            }
        })
    }

    #[test]
    fn migration_types_lists_enum_ddl() {
        let code = generate(&status_entity()).to_string();
        assert!(code.contains("MIGRATION_TYPES"));
        assert!(code.contains("OrderStatus > :: PG_CREATE_TYPE"));
    }

    #[test]
    fn name_assertion_emitted() {
        let code = generate(&status_entity()).to_string();
        assert!(code.contains("const_str_eq"));
        assert!(code.contains("\"order_status\""));
    }

    #[test]
    fn ddl_uses_enum_type_name() {
        let code = generate(&status_entity()).to_string();
        assert!(code.contains("status order_status NOT NULL"));
    }

    #[test]
    fn no_enums_produces_empty_list() {
        let entity = parse_entity(quote! {
            #[entity(table = "users", migrations)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                pub name: String,
            }
        });
        let code = generate(&entity).to_string();
        assert!(code.contains("MIGRATION_TYPES"));
        assert!(!code.contains("PG_CREATE_TYPE"));
    }

    #[test]
    fn option_wrapper_unwrapped_for_const_ref() {
        let entity = parse_entity(quote! {
            #[entity(table = "orders", migrations)]
            pub struct Order {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                #[column(pg_enum = "order_status")]
                pub status: Option<OrderStatus>,
            }
        });
        let code = generate(&entity).to_string();
        assert!(code.contains("OrderStatus > :: PG_CREATE_TYPE"));
        assert!(!code.contains("Option < OrderStatus > :: PG_CREATE_TYPE"));
    }
}

#[cfg(test)]
mod junction_tests {
    use syn::DeriveInput;

    use super::*;

    fn team_entity() -> EntityDef {
        let input: DeriveInput = syn::parse_quote! {
            #[entity(table = "teams", migrations)]
            #[has_many(User, through = "team_members")]
            pub struct Team {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                pub name: String,
            }
        };
        EntityDef::from_derive_input(&input).unwrap()
    }

    #[test]
    fn junction_ddl_emitted_for_through() {
        let code = generate(&team_entity()).to_string();
        assert!(code.contains("MIGRATION_JUNCTIONS"));
        assert!(code.contains("CREATE TABLE IF NOT EXISTS team_members"));
        assert!(code.contains("team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE"));
        assert!(code.contains("user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE"));
        assert!(code.contains("PRIMARY KEY (team_id, user_id)"));
    }

    #[test]
    fn junction_const_absent_for_plain_has_many() {
        let input: DeriveInput = syn::parse_quote! {
            #[entity(table = "teams", migrations)]
            #[has_many(User)]
            pub struct Team {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                pub name: String,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let code = generate(&entity).to_string();
        assert!(!code.contains("MIGRATION_JUNCTIONS"));
    }
}

#[cfg(test)]
mod trigger_tests {
    use quote::quote;
    use syn::DeriveInput;

    use super::*;

    fn parse_entity(tokens: proc_macro2::TokenStream) -> EntityDef {
        let input: DeriveInput = syn::parse2(tokens).expect("test entity must parse");
        EntityDef::from_derive_input(&input).expect("test entity must be valid")
    }

    #[test]
    fn touch_updated_at_emits_function_and_trigger() {
        let entity = parse_entity(quote! {
            #[entity(table = "users", migrations(touch_updated_at))]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                pub name: String,
                #[field(response)]
                #[auto]
                pub updated_at: chrono::DateTime<chrono::Utc>,
            }
        });
        let code = generate(&entity).to_string();
        assert!(code.contains("MIGRATION_TRIGGERS"));
        assert!(code.contains("entity_touch_updated_at"));
        assert!(code.contains("trg_users_touch_updated_at"));
    }

    #[test]
    fn audit_emits_log_table_and_trigger() {
        let entity = parse_entity(quote! {
            #[entity(table = "users", migrations(audit))]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                pub name: String,
            }
        });
        let code = generate(&entity).to_string();
        assert!(code.contains("entity_audit_log"));
        assert!(code.contains("to_jsonb(OLD)"));
        assert!(code.contains("trg_users_audit"));
    }

    #[test]
    fn extensions_emitted_in_order() {
        let entity = parse_entity(quote! {
            #[entity(table = "users", migrations(extensions = "pg_trgm, pgcrypto"))]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                pub name: String,
            }
        });
        let code = generate(&entity).to_string();
        assert!(code.contains("MIGRATION_EXTENSIONS"));
        assert!(code.contains("CREATE EXTENSION IF NOT EXISTS \\\"pg_trgm\\\";"));
        assert!(code.contains("CREATE EXTENSION IF NOT EXISTS \\\"pgcrypto\\\";"));
    }

    #[test]
    fn plain_migrations_have_no_new_consts() {
        let entity = parse_entity(quote! {
            #[entity(table = "users", migrations)]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                pub name: String,
            }
        });
        let code = generate(&entity).to_string();
        assert!(!code.contains("MIGRATION_TRIGGERS"));
        assert!(!code.contains("MIGRATION_EXTENSIONS"));
    }

    #[test]
    fn touch_without_updated_at_rejected() {
        let input: DeriveInput = syn::parse_quote! {
            #[entity(table = "users", migrations(touch_updated_at))]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                pub name: String,
            }
        };
        let err = EntityDef::from_derive_input(&input).unwrap_err();
        assert!(err.to_string().contains("requires an `updated_at` field"));
    }

    #[test]
    fn unknown_migrations_option_rejected() {
        let input: DeriveInput = syn::parse_quote! {
            #[entity(table = "users", migrations(triggers))]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        assert!(EntityDef::from_derive_input(&input).is_err());
    }
}

#[cfg(test)]
mod search_tests {
    use syn::DeriveInput;

    use super::*;

    fn search_entity() -> EntityDef {
        let input: DeriveInput = syn::parse_quote! {
            #[entity(table = "articles", migrations)]
            pub struct Article {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                #[filter(search)]
                pub title: String,
            }
        };
        EntityDef::from_derive_input(&input).unwrap()
    }

    #[test]
    fn search_filter_emits_trgm_index_and_extension() {
        let code = generate(&search_entity()).to_string();
        assert!(code.contains("idx_articles_title_trgm"));
        assert!(code.contains("gin_trgm_ops"));
        assert!(code.contains("pg_trgm"));
    }

    #[test]
    fn search_on_non_string_rejected() {
        let input: DeriveInput = syn::parse_quote! {
            #[entity(table = "articles")]
            pub struct Article {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[filter(search)]
                pub views: i64,
            }
        };
        let err = EntityDef::from_derive_input(&input).unwrap_err();
        assert!(err.to_string().contains("requires a String field"));
    }

    #[test]
    fn search_condition_uses_contains_ilike() {
        let entity = search_entity();
        let code = crate::entity::sql::postgres::Context::new(&entity)
            .query_method()
            .to_string();
        assert!(code.contains("ILIKE '%' || ${} || '%'"));
    }
}
