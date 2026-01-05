// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Relation method generators for PostgreSQL.
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
    /// SELECT * FROM public.{parent}s WHERE id = $1
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

    /// Generate a `find_{entities}` method for a `#[has_many]` relation.
    ///
    /// # SQL Pattern
    ///
    /// ```sql
    /// SELECT * FROM public.{child}s WHERE {parent}_id = $1
    /// ```
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
}
