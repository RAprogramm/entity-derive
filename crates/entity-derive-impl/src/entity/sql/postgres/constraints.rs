// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Typed constraint-violation mapping for `PostgreSQL`.
//!
//! With `#[entity(typed_constraints)]`, generated write methods resolve
//! the violated constraint name (as reported by Postgres) against the
//! constraints the macro itself created and surface
//! `::entity_derive::ConstraintError` instead of a raw driver error.
//!
//! # Known Constraint Names
//!
//! | Source | Postgres default name |
//! |--------|----------------------|
//! | `#[column(unique)]` | `{table}_{column}_key` |
//! | `#[belongs_to(...)]` | `{table}_{column}_fkey` |
//! | `#[column(check = "...")]` | `{table}_{column}_check` |
//! | `unique_index(...)` | index name (`idx_{table}_{cols}` or custom) |
//!
//! The repository `Error` type must implement
//! `From<::entity_derive::ConstraintError>`.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::context::Context;

impl Context<'_> {
    /// Name of the generated error-mapping helper function.
    pub fn constraint_mapper_name(&self) -> syn::Ident {
        format_ident!(
            "__{}_map_constraint_err",
            self.entity.name_str().to_lowercase()
        )
    }

    /// Generate the constraint registry and mapping helper.
    ///
    /// # Returns
    ///
    /// Empty `TokenStream` without `#[entity(typed_constraints)]`.
    pub fn constraint_mapper(&self) -> TokenStream {
        if !self.entity.typed_constraints {
            return TokenStream::new();
        }

        let table = self.entity.table_name();
        let mut arms: Vec<TokenStream> = Vec::new();
        let mut covered: std::collections::HashSet<String> = std::collections::HashSet::new();

        for custom in &self.entity.custom_constraints {
            let name = custom.name.clone();
            let kind = match custom.kind.as_str() {
                "unique" => quote! { ::entity_derive::ConstraintKind::Unique },
                "foreign_key" => quote! { ::entity_derive::ConstraintKind::ForeignKey },
                _ => quote! { ::entity_derive::ConstraintKind::Check }
            };
            let field = match &custom.field {
                Some(f) => quote! { Some(#f) },
                None => quote! { None }
            };
            covered.insert(name.clone());
            arms.push(quote! {
                #name => Some((#kind, #field)),
            });
        }

        for field in self.entity.all_fields() {
            let column = field.name_str();
            if field.column.unique {
                let name = if field.column.ci {
                    format!("{table}_{column}_lower_key")
                } else {
                    format!("{table}_{column}_key")
                };
                if covered.insert(name.clone()) {
                    arms.push(quote! {
                        #name => Some((::entity_derive::ConstraintKind::Unique, Some(#column))),
                    });
                }
            }
            if field.belongs_to().is_some() {
                let name = format!("{table}_{column}_fkey");
                if covered.insert(name.clone()) {
                    arms.push(quote! {
                        #name => Some((::entity_derive::ConstraintKind::ForeignKey, Some(#column))),
                    });
                }
            }
            if field.column.check.is_some() {
                let name = format!("{table}_{column}_check");
                if covered.insert(name.clone()) {
                    arms.push(quote! {
                        #name => Some((::entity_derive::ConstraintKind::Check, Some(#column))),
                    });
                }
            }
        }

        for index in &self.entity.indexes {
            if index.unique {
                let name = index.name_or_default(table);
                if covered.insert(name.clone()) {
                    arms.push(quote! {
                        #name => Some((::entity_derive::ConstraintKind::Unique, None)),
                    });
                }
            }
        }

        let fn_name = self.constraint_mapper_name();
        let error_type = self.entity.error_type();

        quote! {
            /// Resolve a driver error against this entity's constraints.
            fn #fn_name(err: ::sqlx::Error) -> #error_type {
                if let Some(db_err) = err.as_database_error()
                    && let Some(constraint) = db_err.constraint()
                {
                    let resolved: Option<(::entity_derive::ConstraintKind, Option<&'static str>)> =
                        match constraint {
                            #(#arms)*
                            _ => None
                        };
                    if let Some((kind, field)) = resolved {
                        return ::entity_derive::ConstraintError {
                            kind,
                            constraint: constraint.to_string(),
                            field
                        }
                        .into();
                    }
                }
                err.into()
            }
        }
    }

    /// `.map_err(mapper)` fragment for write paths, or empty tokens.
    pub fn constraint_map_err(&self) -> TokenStream {
        if !self.entity.typed_constraints {
            return TokenStream::new();
        }
        let fn_name = self.constraint_mapper_name();
        quote! { .map_err(#fn_name) }
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::DeriveInput;

    use super::super::context::Context;
    use crate::entity::parse::EntityDef;

    fn typed_entity() -> EntityDef {
        let input: DeriveInput = syn::parse_quote! {
            #[entity(table = "users", typed_constraints, error = "MyError", unique_index(tenant_id, email))]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique)]
                pub email: String,
                #[field(create, response)]
                pub tenant_id: uuid::Uuid,
                #[field(create, response)]
                #[belongs_to(Org)]
                pub org_id: uuid::Uuid,
                #[field(create, response)]
                #[column(check = "age >= 0")]
                pub age: i32,
            }
        };
        EntityDef::from_derive_input(&input).unwrap()
    }

    #[test]
    fn mapper_covers_all_constraint_sources() {
        let entity = typed_entity();
        let code = Context::new(&entity).constraint_mapper().to_string();
        assert!(code.contains("users_email_key"));
        assert!(code.contains("users_org_id_fkey"));
        assert!(code.contains("users_age_check"));
        assert!(code.contains("idx_users_tenant_id_email"));
        assert!(code.contains("ConstraintError"));
        let _ = quote!();
    }

    #[test]
    fn mapper_absent_without_flag() {
        let input: DeriveInput = syn::parse_quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique)]
                pub email: String,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let ctx = Context::new(&entity);
        assert!(ctx.constraint_mapper().is_empty());
        assert!(ctx.constraint_map_err().is_empty());
    }
}

#[cfg(test)]
mod custom_constraint_tests {
    use quote::quote;
    use syn::DeriveInput;

    use super::super::context::Context;
    use crate::entity::parse::EntityDef;

    fn parse_entity(tokens: proc_macro2::TokenStream) -> EntityDef {
        let input: DeriveInput = syn::parse2(tokens).expect("test entity must parse");
        EntityDef::from_derive_input(&input).expect("test entity must be valid")
    }

    #[test]
    fn custom_constraints_join_the_registry() {
        let entity = parse_entity(quote! {
            #[entity(
                table = "orders",
                typed_constraints,
                error = "MyError",
                constraint(name = "orders_currency_fkey", kind = "foreign_key", field = "currency"),
                constraint(name = "orders_window_check", kind = "check")
            )]
            pub struct Order {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                pub currency: String,
            }
        });
        let code = Context::new(&entity).constraint_mapper().to_string();
        assert!(code.contains("orders_currency_fkey"));
        assert!(code.contains("ForeignKey , Some (\"currency\")"));
        assert!(code.contains("orders_window_check"));
        assert!(code.contains("Check , None"));
    }

    #[test]
    fn custom_entry_overrides_derived_name() {
        let entity = parse_entity(quote! {
            #[entity(
                table = "users",
                typed_constraints,
                error = "MyError",
                constraint(name = "users_email_key", kind = "unique", field = "primary_email")
            )]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                #[column(unique)]
                pub email: String,
            }
        });
        let code = Context::new(&entity).constraint_mapper().to_string();
        assert_eq!(code.matches("users_email_key").count(), 1);
        assert!(code.contains("Some (\"primary_email\")"));
    }

    #[test]
    fn constraint_without_typed_constraints_rejected() {
        let input: DeriveInput = syn::parse_quote! {
            #[entity(table = "orders", constraint(name = "x", kind = "check"))]
            pub struct Order {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        let err = EntityDef::from_derive_input(&input).unwrap_err();
        assert!(
            err.to_string()
                .contains("requires #[entity(typed_constraints)]")
        );
    }

    #[test]
    fn unknown_kind_rejected() {
        let input: DeriveInput = syn::parse_quote! {
            #[entity(table = "orders", typed_constraints, constraint(name = "x", kind = "exclusion"))]
            pub struct Order {
                #[id]
                pub id: uuid::Uuid,
            }
        };
        assert!(EntityDef::from_derive_input(&input).is_err());
    }
}

#[cfg(all(test, not(feature = "postgres")))]
mod feature_gating_tests {
    use quote::quote;
    use syn::DeriveInput;

    use crate::entity::parse::EntityDef;

    #[test]
    fn repository_impl_omitted_without_postgres_feature() {
        let input: DeriveInput = syn::parse_quote! {
            #[entity(table = "users")]
            pub struct User {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, response)]
                pub name: String,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let code = crate::entity::sql::generate(&entity).to_string();
        assert!(code.is_empty());
        let _ = quote!();
    }
}
