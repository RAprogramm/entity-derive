// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! `PostgreSQL` migration generation.
//!
//! Generates `MIGRATION_UP` and `MIGRATION_DOWN` constants for `PostgreSQL`.

mod ddl;

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

    let up_sql = ddl::generate_up(entity);
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
                    ::entity_core::const_str_eq(<#ty>::PG_TYPE, #declared),
                    "#[column(pg_enum = ...)] does not match the ValueObject's pg_type"
                );
            }
        })
        .collect();

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
        }

        const _: () = {
            #(#name_assertions)*
        };
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
