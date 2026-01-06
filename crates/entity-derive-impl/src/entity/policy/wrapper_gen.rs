// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Policy-aware repository wrapper generation.
//!
//! Generates `{Entity}PolicyRepository` that wraps a repository
//! and enforces policy checks before operations.

use proc_macro2::TokenStream;
use quote::quote;

use crate::{entity::parse::EntityDef, utils::marker};

/// Generate the policy repository wrapper.
pub fn generate(entity: &EntityDef) -> TokenStream {
    let vis = &entity.vis;
    let entity_name = entity.name();
    let policy_trait = entity.ident_with("", "Policy");
    let repo_trait = entity.ident_with("", "Repository");
    let wrapper_name = entity.ident_with("", "PolicyRepository");
    let marker = marker::generated();

    let create_dto = entity.ident_with("Create", "Request");
    let update_dto = entity.ident_with("Update", "Request");
    let id_type = entity.id_field().ty();

    let doc = format!(
        "Repository wrapper for [`{entity_name}`] with authorization checks.\n\n\
         Wraps any [`{repo_trait}`] implementation and enforces policy before operations."
    );

    quote! {
        #marker
        #[doc = #doc]
        #vis struct #wrapper_name<R, P>
        where
            R: #repo_trait,
            P: #policy_trait,
        {
            repo: R,
            policy: P,
        }

        impl<R, P> #wrapper_name<R, P>
        where
            R: #repo_trait,
            P: #policy_trait,
        {
            /// Create a new policy-aware repository wrapper.
            pub fn new(repo: R, policy: P) -> Self {
                Self { repo, policy }
            }

            /// Get reference to the underlying repository.
            pub fn inner(&self) -> &R {
                &self.repo
            }

            /// Get reference to the policy.
            pub fn policy(&self) -> &P {
                &self.policy
            }

            /// Create a new entity with authorization check.
            pub async fn create(
                &self,
                dto: #create_dto,
                ctx: &P::Context,
            ) -> Result<#entity_name, ::entity_core::policy::PolicyError<R::Error, P::Error>> {
                self.policy
                    .can_create(&dto, ctx)
                    .await
                    .map_err(::entity_core::policy::PolicyError::Policy)?;
                self.repo
                    .create(dto)
                    .await
                    .map_err(::entity_core::policy::PolicyError::Repository)
            }

            /// Find entity by ID with authorization check.
            pub async fn find_by_id(
                &self,
                id: #id_type,
                ctx: &P::Context,
            ) -> Result<Option<#entity_name>, ::entity_core::policy::PolicyError<R::Error, P::Error>>
            {
                self.policy
                    .can_read(&id, ctx)
                    .await
                    .map_err(::entity_core::policy::PolicyError::Policy)?;
                self.repo
                    .find_by_id(id)
                    .await
                    .map_err(::entity_core::policy::PolicyError::Repository)
            }

            /// Update entity with authorization check.
            pub async fn update(
                &self,
                id: #id_type,
                dto: #update_dto,
                ctx: &P::Context,
            ) -> Result<#entity_name, ::entity_core::policy::PolicyError<R::Error, P::Error>> {
                self.policy
                    .can_update(&id, &dto, ctx)
                    .await
                    .map_err(::entity_core::policy::PolicyError::Policy)?;
                self.repo
                    .update(id, dto)
                    .await
                    .map_err(::entity_core::policy::PolicyError::Repository)
            }

            /// Delete entity with authorization check.
            pub async fn delete(
                &self,
                id: #id_type,
                ctx: &P::Context,
            ) -> Result<bool, ::entity_core::policy::PolicyError<R::Error, P::Error>> {
                self.policy
                    .can_delete(&id, ctx)
                    .await
                    .map_err(::entity_core::policy::PolicyError::Policy)?;
                self.repo
                    .delete(id)
                    .await
                    .map_err(::entity_core::policy::PolicyError::Repository)
            }

            /// List entities with authorization check.
            pub async fn list(
                &self,
                pagination: ::entity_core::Pagination,
                ctx: &P::Context,
            ) -> Result<Vec<#entity_name>, ::entity_core::policy::PolicyError<R::Error, P::Error>>
            {
                self.policy
                    .can_list(ctx)
                    .await
                    .map_err(::entity_core::policy::PolicyError::Policy)?;
                self.repo
                    .list(pagination)
                    .await
                    .map_err(::entity_core::policy::PolicyError::Repository)
            }
        }
    }
}
