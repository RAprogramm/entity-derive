// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Entity-level attribute parsing with darling.
//!
//! This module defines the internal [`EntityAttrs`] structure used for
//! parsing `#[entity(...)]` attributes. This is an implementation detail;
//! the public API uses [`EntityDef`](super::EntityDef).
//!
//! # Supported Attributes
//!
//! | Attribute | Required | Default | Description |
//! |-----------|----------|---------|-------------|
//! | `table` | Yes | — | Database table name |
//! | `schema` | No | `"public"` | Database schema |
//! | `sql` | No | `Full` | SQL generation level |
//! | `dialect` | No | `Postgres` | Database dialect |
//! | `uuid` | No | `V7` | UUID version for IDs |
//! | `error` | No | `sqlx::Error` | Custom error type |
//! | `soft_delete` | No | `false` | Enable soft delete |
//! | `returning` | No | `Full` | RETURNING clause mode |
//! | `events` | No | `false` | Generate lifecycle events |
//! | `hooks` | No | `false` | Generate lifecycle hooks trait |
//! | `commands` | No | `false` | Generate CQRS command pattern |
//! | `policy` | No | `false` | Generate authorization policy trait |
//! | `streams` | No | `false` | Enable real-time streaming via LISTEN/NOTIFY |

use darling::FromDeriveInput;
use syn::{Ident, Visibility};

use crate::entity::parse::{DatabaseDialect, ReturningMode, SqlLevel, UuidVersion};

/// Returns the default schema name.
///
/// Used by darling for the `schema` attribute default.
pub fn default_schema() -> String {
    "public".to_string()
}

/// Default error type path for SQL implementations.
///
/// Used when no custom error type is specified.
/// Uses `parse_quote!` for compile-time validation.
pub fn default_error_type() -> syn::Path {
    syn::parse_quote!(sqlx::Error)
}

/// Entity-level attributes parsed from `#[entity(...)]`.
///
/// This is an internal struct used by darling for parsing.
/// The public API uses [`EntityDef`](super::EntityDef) which combines these
/// attributes with parsed field definitions.
///
/// # Example
///
/// ```rust,ignore
/// #[entity(
///     table = "users",
///     schema = "core",
///     sql = "full",
///     dialect = "postgres",
///     uuid = "v7",
///     soft_delete,
///     returning = "full"
/// )]
/// ```
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(entity), supports(struct_named), allow_unknown_fields)]
pub struct EntityAttrs {
    /// Struct identifier (e.g., `User`).
    pub ident: Ident,

    /// Struct visibility (e.g., `pub`, `pub(crate)`).
    pub vis: Visibility,

    /// Database table name.
    ///
    /// This is a required attribute with no default value.
    /// The macro will fail with a clear error if not provided.
    pub table: String,

    /// Database schema name.
    ///
    /// Defaults to `"public"` if not specified.
    #[darling(default = "default_schema")]
    pub schema: String,

    /// SQL generation level.
    ///
    /// Defaults to [`SqlLevel::Full`] if not specified.
    #[darling(default)]
    pub sql: SqlLevel,

    /// Database dialect.
    ///
    /// Defaults to [`DatabaseDialect::Postgres`] if not specified.
    #[darling(default)]
    pub dialect: DatabaseDialect,

    /// UUID version for ID generation.
    ///
    /// Defaults to [`UuidVersion::V7`] if not specified.
    #[darling(default)]
    pub uuid: UuidVersion,

    /// Custom error type for repository implementation.
    ///
    /// Defaults to `sqlx::Error` if not specified.
    /// The custom type must implement `From<sqlx::Error>`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// #[entity(table = "users", error = "AppError")]
    /// #[entity(table = "users", error = "crate::errors::DbError")]
    /// ```
    #[darling(default = "default_error_type")]
    pub error: syn::Path,

    /// Enable soft delete for this entity.
    ///
    /// When enabled, the entity must have a `deleted_at: Option<DateTime<Utc>>`
    /// field. The `delete` method will set this timestamp instead of removing
    /// the row, and all queries will automatically filter out deleted records.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[entity(table = "users", soft_delete)]
    /// pub struct User {
    ///     #[id]
    ///     pub id: Uuid,
    ///     pub deleted_at: Option<DateTime<Utc>>,
    /// }
    /// ```
    #[darling(default)]
    pub soft_delete: bool,

    /// RETURNING clause mode for INSERT/UPDATE operations.
    ///
    /// Controls what data is fetched back from the database:
    /// - `full` (default): Use `RETURNING *` to get all fields
    /// - `id`: Use `RETURNING id` to get only the primary key
    /// - `none`: No RETURNING clause, return pre-built entity
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[entity(table = "users", returning = "full")]
    /// #[entity(table = "logs", returning = "none")]
    /// ```
    #[darling(default)]
    pub returning: ReturningMode,

    /// Generate lifecycle event enum.
    ///
    /// When enabled, generates a `{Entity}Event` enum with variants for
    /// each lifecycle operation (Created, Updated, Deleted, etc.).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[entity(table = "users", events)]
    /// pub struct User { ... }
    ///
    /// // Generates:
    /// pub enum UserEvent {
    ///     Created(User),
    ///     Updated { old: User, new: User },
    ///     Deleted { id: Uuid },
    /// }
    /// ```
    #[darling(default)]
    pub events: bool,

    /// Generate lifecycle hooks trait.
    ///
    /// When enabled, generates a `{Entity}Hooks` trait with before/after
    /// methods for each CRUD operation. Default implementations are no-ops.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[entity(table = "users", hooks)]
    /// pub struct User { ... }
    ///
    /// // Implement hooks:
    /// impl UserHooks for MyRepo {
    ///     async fn before_create(&self, dto: &mut CreateUserRequest) -> Result<(), Error> {
    ///         validate(&dto)?;
    ///         Ok(())
    ///     }
    /// }
    /// ```
    #[darling(default)]
    pub hooks: bool,

    /// Generate CQRS-style command pattern.
    ///
    /// When enabled, processes `#[command(...)]` attributes to generate:
    /// - Command structs (e.g., `RegisterUser`, `UpdateEmailUser`)
    /// - Command enum (`UserCommand`)
    /// - Result enum (`UserCommandResult`)
    /// - Handler trait (`UserCommandHandler`)
    #[darling(default)]
    pub commands: bool,

    /// Generate authorization policy trait.
    ///
    /// When enabled, generates:
    /// - `{Entity}Policy` trait with `can_create`, `can_read`, etc.
    /// - `{Entity}AllowAllPolicy` default implementation
    /// - `{Entity}PolicyRepository` wrapper with authorization checks
    #[darling(default)]
    pub policy: bool,

    /// Enable real-time streaming via Postgres LISTEN/NOTIFY.
    ///
    /// When enabled (requires `events`), generates:
    /// - `{Entity}Subscriber` for async event streaming
    /// - NOTIFY calls in CRUD operations
    /// - `CHANNEL` constant with notification channel name
    #[darling(default)]
    pub streams: bool,

    /// Enable transaction support.
    ///
    /// When enabled, generates:
    /// - `{Entity}TransactionRepo` adapter for use in transactions
    /// - `with_{entity}()` method on `Transaction` builder
    /// - Accessor methods on `TransactionContext`
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// #[entity(table = "accounts", transactions)]
    /// pub struct Account { ... }
    ///
    /// // Usage:
    /// Transaction::new(&pool)
    ///     .with_accounts()
    ///     .run(|mut ctx| async move {
    ///         ctx.accounts().create(dto).await
    ///     })
    ///     .await?;
    /// ```
    #[darling(default)]
    pub transactions: bool
}
