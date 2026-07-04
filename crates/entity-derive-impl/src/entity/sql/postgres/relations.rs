// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Relation method generators for `PostgreSQL`.
//!
//! Generates methods for entity relationships:
//!
//! | Attribute | Method Generated | Description |
//! |-----------|------------------|-------------|
//! | `#[belongs_to(Entity)]` | `find_{entity}` | Fetch parent entity |
//! | `#[has_many(Entity)]` | `find_{entities}` | Fetch child entities |
//!
//! # Example
//!
//! ```rust,ignore
//! // For a Post with #[belongs_to(User)]
//! async fn find_user(&self, id: Uuid) -> Result<Option<User>, Self::Error>;
//!
//! // For a User with #[has_many(Post)]
//! async fn find_posts(&self, user_id: Uuid) -> Result<Vec<Post>, Self::Error>;
//! ```

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::context::Context;
use crate::entity::parse::FieldDef;

impl Context<'_> {
    /// Generate all relation methods.
    ///
    /// Combines `belongs_to` and `has_many` methods into a single
    /// `TokenStream`.
    pub fn relation_methods(&self) -> TokenStream {
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

    /// Generate a `find_{entity}` method for a `#[belongs_to]` relation.
    ///
    /// # SQL Pattern
    ///
    /// First fetches the current entity, then queries the parent:
    /// ```sql
    /// SELECT * FROM {schema}.{parent}s WHERE id = $1
    /// ```
    ///
    /// # Returns
    ///
    /// `None` if the field doesn't have a `belongs_to` attribute.
    fn belongs_to_method(&self, field: &FieldDef) -> Option<TokenStream> {
        let related_entity = field.belongs_to()?;
        let related_snake = related_entity.to_string().to_case(Case::Snake);
        let method_name = format_ident!("find_{}", related_snake);
        let related_row = format_ident!("{}Row", related_entity);
        let related_table = self
            .entity
            .full_table_name_for(&format!("{related_snake}s"));
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
                            ::sqlx::AssertSqlSafe(format!("SELECT * FROM {} WHERE id = {}", #related_table, #placeholder))
                        ).bind(&e.#fk_name).fetch_optional(self).await?;
                        Ok(row.map(#related_entity::from))
                    }
                    None => Ok(None)
                }
            }
        })
    }

    /// Generate a `find_{entities}` method for a `#[has_many]` relation.
    ///
    /// # SQL Pattern
    ///
    /// ```sql
    /// SELECT * FROM {schema}.{child}s WHERE {parent}_id = $1
    /// ```
    fn has_many_method(&self, relation: &crate::entity::parse::HasManyDef) -> TokenStream {
        let related = &relation.entity;
        let related_snake = related.to_string().to_case(Case::Snake);
        let method_name = format_ident!("find_{}s", related_snake);
        let related_row = format_ident!("{}Row", related);
        let related_table = self
            .entity
            .full_table_name_for(&format!("{related_snake}s"));
        let entity_snake = self.entity.name_str().to_case(Case::Snake);
        let fk_field = format_ident!("{}_id", entity_snake);
        let id_type = self.id_type;
        let placeholder = self.dialect.placeholder(1);

        if let Some(junction) = &relation.through {
            let junction_table = self.entity.full_table_name_for(junction);
            let find_sql = format!(
                "SELECT c.* FROM {related_table} c \
                 INNER JOIN {junction_table} j ON j.{related_snake}_id = c.id \
                 WHERE j.{entity_snake}_id = $1"
            );
            let add_sql = format!(
                "INSERT INTO {junction_table} ({entity_snake}_id, {related_snake}_id) \
                 VALUES ($1, $2) ON CONFLICT DO NOTHING"
            );
            let remove_sql = format!(
                "DELETE FROM {junction_table} \
                 WHERE {entity_snake}_id = $1 AND {related_snake}_id = $2"
            );
            let has_sql = format!(
                "SELECT EXISTS(SELECT 1 FROM {junction_table} \
                 WHERE {entity_snake}_id = $1 AND {related_snake}_id = $2)"
            );

            let add_name = format_ident!("add_{}", related_snake);
            let remove_name = format_ident!("remove_{}", related_snake);
            let has_name = format_ident!("has_{}", related_snake);
            let child_id = format_ident!("{}_id", related_snake);

            return quote! {
                async fn #method_name(&self, #fk_field: #id_type) -> Result<Vec<#related>, Self::Error> {
                    let rows: Vec<#related_row> = sqlx::query_as(#find_sql)
                        .bind(&#fk_field)
                        .fetch_all(self).await?;
                    Ok(rows.into_iter().map(#related::from).collect())
                }

                async fn #add_name(&self, #fk_field: #id_type, #child_id: #id_type) -> Result<(), Self::Error> {
                    sqlx::query(#add_sql)
                        .bind(&#fk_field)
                        .bind(&#child_id)
                        .execute(self).await?;
                    Ok(())
                }

                async fn #remove_name(&self, #fk_field: #id_type, #child_id: #id_type) -> Result<bool, Self::Error> {
                    let result = sqlx::query(#remove_sql)
                        .bind(&#fk_field)
                        .bind(&#child_id)
                        .execute(self).await?;
                    Ok(result.rows_affected() > 0)
                }

                async fn #has_name(&self, #fk_field: #id_type, #child_id: #id_type) -> Result<bool, Self::Error> {
                    let exists: bool = sqlx::query_scalar(#has_sql)
                        .bind(&#fk_field)
                        .bind(&#child_id)
                        .fetch_one(self).await?;
                    Ok(exists)
                }
            };
        }

        quote! {
            async fn #method_name(&self, #fk_field: #id_type) -> Result<Vec<#related>, Self::Error> {
                let rows: Vec<#related_row> = sqlx::query_as(
                    ::sqlx::AssertSqlSafe(format!("SELECT * FROM {} WHERE {}_id = {}", #related_table, #entity_snake, #placeholder))
                ).bind(&#fk_field).fetch_all(self).await?;
                Ok(rows.into_iter().map(#related::from).collect())
            }
        }
    }
}

#[cfg(test)]
mod through_tests {
    use quote::quote;
    use syn::DeriveInput;

    use super::super::context::Context;
    use crate::entity::parse::EntityDef;

    fn team_entity() -> EntityDef {
        let input: DeriveInput = syn::parse_quote! {
            #[entity(table = "teams")]
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
    fn through_relation_generates_junction_methods() {
        let entity = team_entity();
        let code = Context::new(&entity).relation_methods().to_string();
        assert!(code.contains("find_users"));
        assert!(code.contains("add_user"));
        assert!(code.contains("remove_user"));
        assert!(code.contains("has_user"));
        assert!(code.contains("INNER JOIN team_members j ON j.user_id = c.id"));
        assert!(code.contains("ON CONFLICT DO NOTHING"));
        let _ = quote!();
    }

    #[test]
    fn plain_relation_has_no_junction_methods() {
        let input: DeriveInput = syn::parse_quote! {
            #[entity(table = "teams")]
            #[has_many(User)]
            pub struct Team {
                #[id]
                pub id: uuid::Uuid,
                #[field(create, update, response)]
                pub name: String,
            }
        };
        let entity = EntityDef::from_derive_input(&input).unwrap();
        let code = Context::new(&entity).relation_methods().to_string();
        assert!(code.contains("find_users"));
        assert!(!code.contains("add_user"));
    }
}
