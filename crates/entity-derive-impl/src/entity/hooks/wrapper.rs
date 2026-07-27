// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Hook-invoking repository wrapper.
//!
//! The orphan rule keeps a user crate from implementing
//! `{Entity}Hooks` for `sqlx::PgPool`, so the generated repository impl
//! on the pool cannot call hooks: both the trait and the type are
//! foreign to the crate that owns the hooks. `{Entity}Repo<H>` is the
//! type that can — it owns the pool and the hooks together:
//!
//! ```rust,ignore
//! let repo = UserRepo::new(pool, Audit);
//!
//! let user = repo.create(dto).await?;   // before_create → INSERT → after_create
//! let found = repo.find_by_id(id).await?; // no hooks for reads: straight to the pool
//! ```
//!
//! Reads and everything else reach the pool through `Deref`, so the
//! wrapper answers every repository method. The mutating operations are
//! inherent methods on the wrapper, which take precedence over the
//! ones reached through `Deref` — that is what makes the hooks run.
//!
//! Using the bare pool keeps working exactly as before; the wrapper is
//! opt-in.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{
    entity::parse::{EntityDef, SqlLevel},
    utils::marker
};

/// Generate `{Entity}Repo<H>` for an entity declaring `hooks`.
///
/// Returns empty tokens when the entity has no hooks or generates no
/// repository implementation to delegate to.
pub fn generate(entity: &EntityDef) -> TokenStream {
    if !entity.has_hooks() || entity.sql != SqlLevel::Full {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let entity_name = entity.name();
    let wrapper = entity.ident_with("", "Repo");
    let hooks_trait = format_ident!("{}Hooks", entity_name);
    let error_type = entity.error_type();
    let marker = marker::generated();

    let create = create_method(entity);
    let update = update_method(entity);
    let delete = delete_method(entity);
    let soft_delete_extras = soft_delete_methods(entity);
    let save = save_method(entity);

    let doc = format!(
        "Repository for [`{entity_name}`] that invokes [`{hooks_trait}`].\n\n\
         Wraps a pool and a hooks implementation. Mutating operations run \
         `before_*`, the statement, then `after_*`; a failing `before_*` \
         aborts before anything is written. Reads and every other \
         repository method reach the pool unchanged.\n\n\
         The hook error only has to convert into the repository error, so \
         hooks may keep their own error type.\n\n\
         ```rust,ignore\n\
         let repo = {wrapper}::new(pool, MyHooks);\n\
         let created = repo.create(dto).await?;\n\
         ```"
    );

    quote! {
        #marker
        #[doc = #doc]
        #vis struct #wrapper<H>
        where
            H: #hooks_trait
        {
            pool:  sqlx::PgPool,
            hooks: H
        }

        impl<H> #wrapper<H>
        where
            H: #hooks_trait,
            #error_type: From<<H as #hooks_trait>::Error>
        {
            /// Bind a pool to a hooks implementation.
            pub const fn new(pool: sqlx::PgPool, hooks: H) -> Self {
                Self {
                    pool,
                    hooks
                }
            }

            /// The wrapped pool, for statements this type does not cover.
            pub const fn pool(&self) -> &sqlx::PgPool {
                &self.pool
            }

            /// The wrapped hooks.
            pub const fn hooks(&self) -> &H {
                &self.hooks
            }

            #create
            #update
            #delete
            #soft_delete_extras
            #save
        }

        #marker
        /// Reads and unhooked operations go straight to the pool.
        impl<H> std::ops::Deref for #wrapper<H>
        where
            H: #hooks_trait
        {
            type Target = sqlx::PgPool;

            fn deref(&self) -> &Self::Target {
                &self.pool
            }
        }

        #marker
        impl<H> std::fmt::Debug for #wrapper<H>
        where
            H: #hooks_trait
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct(stringify!(#wrapper)).finish_non_exhaustive()
            }
        }
    }
}

/// `create` with `before_create` / `after_create` around it.
fn create_method(entity: &EntityDef) -> TokenStream {
    if entity.create_fields().is_empty() {
        return TokenStream::new();
    }

    let entity_name = entity.name();
    let create_dto = entity.ident_with("Create", "Request");
    let error_type = entity.error_type();
    let repo_trait = format_ident!("{}Repository", entity_name);

    quote! {
        /// Create a row, running the create hooks around the INSERT.
        ///
        /// `before_create` may rewrite the DTO; returning an error from
        /// it means nothing is written.
        pub async fn create(&self, dto: #create_dto) -> Result<#entity_name, #error_type> {
            let mut dto = dto;
            self.hooks.before_create(&mut dto).await?;
            let entity = <sqlx::PgPool as #repo_trait>::create(&self.pool, dto).await?;
            self.hooks.after_create(&entity).await?;
            Ok(entity)
        }
    }
}

/// `update` with `before_update` / `after_update` around it.
fn update_method(entity: &EntityDef) -> TokenStream {
    if entity.update_fields().is_empty() {
        return TokenStream::new();
    }

    let entity_name = entity.name();
    let update_dto = entity.ident_with("Update", "Request");
    let id_type = entity.id_field().ty();
    let error_type = entity.error_type();
    let repo_trait = format_ident!("{}Repository", entity_name);

    quote! {
        /// Update a row, running the update hooks around the UPDATE.
        ///
        /// `before_update` may rewrite the patch; returning an error
        /// from it means nothing is written.
        pub async fn update(
            &self,
            id: #id_type,
            dto: #update_dto
        ) -> Result<#entity_name, #error_type> {
            let mut dto = dto;
            self.hooks.before_update(&id, &mut dto).await?;
            let entity = <sqlx::PgPool as #repo_trait>::update(&self.pool, id, dto).await?;
            self.hooks.after_update(&entity).await?;
            Ok(entity)
        }
    }
}

/// `delete` with `before_delete` / `after_delete` around it.
fn delete_method(entity: &EntityDef) -> TokenStream {
    let entity_name = entity.name();
    let id_type = entity.id_field().ty();
    let error_type = entity.error_type();
    let repo_trait = format_ident!("{}Repository", entity_name);
    let doc = if entity.is_soft_delete() {
        "Soft-delete a row, running the delete hooks around the UPDATE."
    } else {
        "Delete a row, running the delete hooks around the DELETE."
    };

    quote! {
        #[doc = #doc]
        ///
        /// A failing `before_delete` aborts before anything is written.
        /// `after_delete` runs only when a row was actually affected.
        pub async fn delete(&self, id: #id_type) -> Result<bool, #error_type> {
            self.hooks.before_delete(&id).await?;
            let removed = <sqlx::PgPool as #repo_trait>::delete(&self.pool, id).await?;
            if removed {
                self.hooks.after_delete(&id).await?;
            }
            Ok(removed)
        }
    }
}

/// `hard_delete` and `restore` for soft-delete entities.
fn soft_delete_methods(entity: &EntityDef) -> TokenStream {
    if !entity.is_soft_delete() {
        return TokenStream::new();
    }

    let entity_name = entity.name();
    let id_type = entity.id_field().ty();
    let error_type = entity.error_type();
    let repo_trait = format_ident!("{}Repository", entity_name);

    quote! {
        /// Remove a row for good, running the hard-delete hooks.
        pub async fn hard_delete(&self, id: #id_type) -> Result<bool, #error_type> {
            self.hooks.before_hard_delete(&id).await?;
            let removed = <sqlx::PgPool as #repo_trait>::hard_delete(&self.pool, id).await?;
            if removed {
                self.hooks.after_hard_delete(&id).await?;
            }
            Ok(removed)
        }

        /// Bring a soft-deleted row back, running the restore hooks.
        pub async fn restore(&self, id: #id_type) -> Result<bool, #error_type> {
            self.hooks.before_restore(&id).await?;
            let restored = <sqlx::PgPool as #repo_trait>::restore(&self.pool, id).await?;
            if restored {
                self.hooks.after_restore(&id).await?;
            }
            Ok(restored)
        }
    }
}

/// `save` for aggregate roots, hooked like `create`.
fn save_method(entity: &EntityDef) -> TokenStream {
    if !entity.is_aggregate_root() || entity.create_fields().is_empty() {
        return TokenStream::new();
    }

    let entity_name = entity.name();
    let new_name = entity.ident_with("New", "");
    let error_type = entity.error_type();
    let repo_trait = format_ident!("{}Repository", entity_name);

    quote! {
        /// Persist a new aggregate, running the create hooks around it.
        ///
        /// The aggregate is already built here, so `before_create` has
        /// no DTO to rewrite; it runs as a guard and `after_create`
        /// sees the persisted row.
        pub async fn save(&self, new: #new_name) -> Result<#entity_name, #error_type> {
            let entity = <sqlx::PgPool as #repo_trait>::save(&self.pool, new).await?;
            self.hooks.after_create(&entity).await?;
            Ok(entity)
        }
    }
}
