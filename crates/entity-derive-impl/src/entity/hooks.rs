// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Lifecycle hooks trait generation.
//!
//! Generates a hooks trait for entities with `#[entity(hooks)]`.
//! Hooks provide before/after callbacks for CRUD operations.
//!
//! # Generated Code
//!
//! For an entity `User`, generates:
//!
//! ```rust,ignore
//! #[async_trait]
//! pub trait UserHooks: Send + Sync {
//!     type Error: std::error::Error + Send + Sync;
//!
//!     async fn before_create(&self, dto: &mut CreateUserRequest) -> Result<(), Self::Error> { Ok(()) }
//!     async fn after_create(&self, entity: &User) -> Result<(), Self::Error> { Ok(()) }
//!     async fn before_update(&self, id: &Uuid, dto: &mut UpdateUserRequest) -> Result<(), Self::Error> { Ok(()) }
//!     async fn after_update(&self, entity: &User) -> Result<(), Self::Error> { Ok(()) }
//!     async fn before_delete(&self, id: &Uuid) -> Result<(), Self::Error> { Ok(()) }
//!     async fn after_delete(&self, id: &Uuid) -> Result<(), Self::Error> { Ok(()) }
//! }
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! struct MyRepo(PgPool);
//!
//! impl UserHooks for MyRepo {
//!     type Error = AppError;
//!
//!     async fn before_create(&self, dto: &mut CreateUserRequest) -> Result<(), Self::Error> {
//!         dto.email = dto.email.to_lowercase();
//!         Ok(())
//!     }
//!
//!     async fn after_create(&self, user: &User) -> Result<(), Self::Error> {
//!         send_welcome_email(&user.email).await?;
//!         Ok(())
//!     }
//! }
//! ```

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::parse::EntityDef;
use crate::utils::marker;

/// Generates the lifecycle hooks trait for an entity.
///
/// Returns empty `TokenStream` if `hooks` is not enabled.
pub fn generate(entity: &EntityDef) -> TokenStream {
    if !entity.has_hooks() {
        return TokenStream::new();
    }

    let vis = &entity.vis;
    let entity_name = entity.name();
    let hooks_trait = format_ident!("{}Hooks", entity_name);

    let id_type = entity.id_field().ty();

    let create_hooks = generate_create_hooks(entity);
    let update_hooks = generate_update_hooks(entity, id_type);
    let delete_hooks = generate_delete_hooks(id_type, entity.is_soft_delete());
    let command_hooks = generate_command_hooks(entity);

    let marker = marker::generated();

    quote! {
        #marker
        /// Lifecycle hooks for [`#entity_name`].
        ///
        /// Implement this trait to add custom logic before/after CRUD operations.
        /// All methods have default no-op implementations.
        ///
        /// # Error Handling
        ///
        /// If a `before_*` hook returns an error, the operation is aborted.
        /// If an `after_*` hook returns an error, the operation has already
        /// completed but the error is propagated to the caller.
        #[async_trait::async_trait]
        #vis trait #hooks_trait: Send + Sync {
            /// Error type for hook operations.
            type Error: std::error::Error + Send + Sync;

            #create_hooks
            #update_hooks
            #delete_hooks
            #command_hooks
        }
    }
}

/// Generate before/after hooks for create operation.
fn generate_create_hooks(entity: &EntityDef) -> TokenStream {
    if entity.create_fields().is_empty() {
        return TokenStream::new();
    }

    let entity_name = entity.name();
    let create_dto = entity.ident_with("Create", "Request");

    quote! {
        /// Called before entity creation.
        ///
        /// Use for validation, normalization, or rejecting invalid data.
        /// Modify `dto` to transform input before persistence.
        async fn before_create(&self, dto: &mut #create_dto) -> Result<(), Self::Error> {
            let _ = dto;
            Ok(())
        }

        /// Called after entity creation.
        ///
        /// Use for sending notifications, updating caches, or audit logging.
        async fn after_create(&self, entity: &#entity_name) -> Result<(), Self::Error> {
            let _ = entity;
            Ok(())
        }
    }
}

/// Generate before/after hooks for update operation.
fn generate_update_hooks(entity: &EntityDef, id_type: &syn::Type) -> TokenStream {
    if entity.update_fields().is_empty() {
        return TokenStream::new();
    }

    let entity_name = entity.name();
    let update_dto = entity.ident_with("Update", "Request");

    quote! {
        /// Called before entity update.
        ///
        /// Use for validation or rejecting invalid updates.
        /// Modify `dto` to transform input before persistence.
        async fn before_update(
            &self,
            id: &#id_type,
            dto: &mut #update_dto
        ) -> Result<(), Self::Error> {
            let _ = (id, dto);
            Ok(())
        }

        /// Called after entity update.
        ///
        /// Use for cache invalidation, notifications, or audit logging.
        async fn after_update(&self, entity: &#entity_name) -> Result<(), Self::Error> {
            let _ = entity;
            Ok(())
        }
    }
}

/// Generate before/after hooks for delete operations.
fn generate_delete_hooks(id_type: &syn::Type, soft_delete: bool) -> TokenStream {
    let soft_delete_hooks = if soft_delete {
        quote! {
            /// Called before hard delete (permanent removal).
            ///
            /// Use to check if hard delete is allowed.
            async fn before_hard_delete(&self, id: &#id_type) -> Result<(), Self::Error> {
                let _ = id;
                Ok(())
            }

            /// Called after hard delete (permanent removal).
            async fn after_hard_delete(&self, id: &#id_type) -> Result<(), Self::Error> {
                let _ = id;
                Ok(())
            }

            /// Called before restore from soft-delete.
            async fn before_restore(&self, id: &#id_type) -> Result<(), Self::Error> {
                let _ = id;
                Ok(())
            }

            /// Called after restore from soft-delete.
            async fn after_restore(&self, id: &#id_type) -> Result<(), Self::Error> {
                let _ = id;
                Ok(())
            }
        }
    } else {
        TokenStream::new()
    };

    quote! {
        /// Called before entity deletion.
        ///
        /// Use to check if deletion is allowed or perform cleanup.
        async fn before_delete(&self, id: &#id_type) -> Result<(), Self::Error> {
            let _ = id;
            Ok(())
        }

        /// Called after entity deletion.
        ///
        /// Use for cascade cleanup, notifications, or audit logging.
        async fn after_delete(&self, id: &#id_type) -> Result<(), Self::Error> {
            let _ = id;
            Ok(())
        }

        #soft_delete_hooks
    }
}

/// Generate before/after hooks for command execution.
fn generate_command_hooks(entity: &EntityDef) -> TokenStream {
    if !entity.has_commands() || entity.command_defs().is_empty() {
        return TokenStream::new();
    }

    let entity_name = entity.name();
    let command_enum = format_ident!("{}Command", entity_name);
    let result_enum = format_ident!("{}CommandResult", entity_name);

    quote! {
        /// Called before any command execution.
        ///
        /// Use for authorization, validation, or audit logging.
        /// Returning an error aborts the command.
        async fn before_command(&self, cmd: &#command_enum) -> Result<(), Self::Error> {
            let _ = cmd;
            Ok(())
        }

        /// Called after successful command execution.
        ///
        /// Use for notifications, cache updates, or audit logging.
        async fn after_command(
            &self,
            cmd: &#command_enum,
            result: &#result_enum
        ) -> Result<(), Self::Error> {
            let _ = (cmd, result);
            Ok(())
        }
    }
}
