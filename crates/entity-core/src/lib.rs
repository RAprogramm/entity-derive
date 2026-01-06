// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Core traits and types for entity-derive.
//!
//! This crate provides the foundational traits and types used by entity-derive
//! generated code. It can also be used standalone for manual implementations.
//!
//! # Overview
//!
//! - [`Repository`] — Base trait for all generated repository traits
//! - [`Pagination`] — Common pagination parameters
//! - [`prelude`] — Convenient re-exports
//!
//! # Usage
//!
//! Most users should use `entity-derive` directly, which re-exports this crate.
//! For manual implementations:
//!
//! ```rust,ignore
//! use entity_core::prelude::*;
//!
//! #[async_trait]
//! impl UserRepository for MyPool {
//!     type Error = MyError;
//!     type Pool = PgPool;
//!     // ...
//! }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod prelude;

/// Re-export async_trait for generated code.
pub use async_trait::async_trait;

/// Base repository trait.
///
/// All generated `{Entity}Repository` traits include these associated types
/// and methods. This trait is not directly extended but serves as documentation
/// for the common interface.
///
/// # Associated Types
///
/// - `Error` — Error type for repository operations
/// - `Pool` — Underlying database pool type
///
/// # Example
///
/// Generated traits follow this pattern:
///
/// ```rust,ignore
/// #[async_trait]
/// pub trait UserRepository: Send + Sync {
///     type Error: std::error::Error + Send + Sync;
///     type Pool;
///
///     fn pool(&self) -> &Self::Pool;
///     async fn create(&self, dto: CreateUserRequest) -> Result<User, Self::Error>;
///     async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, Self::Error>;
///     // ...
/// }
/// ```
pub trait Repository: Send + Sync {
    /// Error type for repository operations.
    ///
    /// Must implement `std::error::Error + Send + Sync` for async
    /// compatibility.
    type Error: std::error::Error + Send + Sync;

    /// Underlying database pool type.
    ///
    /// Enables access to the pool for transactions and custom queries.
    type Pool;

    /// Get reference to the underlying database pool.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let pool = repo.pool();
    /// let mut tx = pool.begin().await?;
    /// // Custom operations...
    /// tx.commit().await?;
    /// ```
    fn pool(&self) -> &Self::Pool;
}

/// Pagination parameters for list operations.
///
/// Used by `list` and `query` methods to control result pagination.
///
/// # Example
///
/// ```rust
/// use entity_core::Pagination;
///
/// let page = Pagination::new(10, 0); // First 10 items
/// let next = Pagination::new(10, 10); // Next 10 items
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pagination {
    /// Maximum number of results to return.
    pub limit: i64,

    /// Number of results to skip.
    pub offset: i64
}

impl Pagination {
    /// Create new pagination parameters.
    ///
    /// # Arguments
    ///
    /// * `limit` — Maximum results to return
    /// * `offset` — Number of results to skip
    pub const fn new(limit: i64, offset: i64) -> Self {
        Self {
            limit,
            offset
        }
    }

    /// Create pagination for a specific page.
    ///
    /// # Arguments
    ///
    /// * `page` — Page number (0-indexed)
    /// * `per_page` — Items per page
    ///
    /// # Example
    ///
    /// ```rust
    /// use entity_core::Pagination;
    ///
    /// let page_0 = Pagination::page(0, 25); // offset=0, limit=25
    /// let page_2 = Pagination::page(2, 25); // offset=50, limit=25
    /// ```
    pub const fn page(page: i64, per_page: i64) -> Self {
        Self {
            limit:  per_page,
            offset: page * per_page
        }
    }
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            limit:  100,
            offset: 0
        }
    }
}

/// Sort direction for ordered queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortDirection {
    /// Ascending order (A-Z, 0-9, oldest first).
    #[default]
    Asc,

    /// Descending order (Z-A, 9-0, newest first).
    Desc
}

impl SortDirection {
    /// Convert to SQL keyword.
    pub const fn as_sql(&self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC"
        }
    }
}

/// Kind of lifecycle event.
///
/// Used by generated event enums to categorize events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventKind {
    /// Entity was created.
    Created,

    /// Entity was updated.
    Updated,

    /// Entity was soft-deleted.
    SoftDeleted,

    /// Entity was hard-deleted (permanently removed).
    HardDeleted,

    /// Entity was restored from soft-delete.
    Restored
}

impl EventKind {
    /// Check if this is a delete event (soft or hard).
    pub const fn is_delete(&self) -> bool {
        matches!(self, Self::SoftDeleted | Self::HardDeleted)
    }

    /// Check if this is a mutation event (create, update, delete).
    pub const fn is_mutation(&self) -> bool {
        !matches!(self, Self::Restored)
    }
}

/// Base trait for entity lifecycle events.
///
/// Generated event enums implement this trait, enabling generic
/// event handling and dispatching.
///
/// # Example
///
/// ```rust,ignore
/// fn handle_event<E: EntityEvent>(event: &E) {
///     println!("Event {:?} for entity {:?}", event.kind(), event.entity_id());
/// }
/// ```
pub trait EntityEvent: Send + Sync + std::fmt::Debug {
    /// Type of entity ID.
    type Id;

    /// Get the kind of event.
    fn kind(&self) -> EventKind;

    /// Get the entity ID associated with this event.
    fn entity_id(&self) -> &Self::Id;
}

/// Kind of business command.
///
/// Used by generated command enums to categorize commands for auditing
/// and routing purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandKind {
    /// Creates a new entity (e.g., Register, Create).
    Create,

    /// Modifies an existing entity (e.g., UpdateEmail, ChangeStatus).
    Update,

    /// Removes an entity (e.g., Delete, Deactivate).
    Delete,

    /// Custom business operation that doesn't fit CRUD.
    Custom
}

impl CommandKind {
    /// Check if this command creates an entity.
    pub const fn is_create(&self) -> bool {
        matches!(self, Self::Create)
    }

    /// Check if this command modifies state.
    pub const fn is_mutation(&self) -> bool {
        !matches!(self, Self::Custom)
    }
}

/// Base trait for entity commands.
///
/// Generated command enums implement this trait, enabling generic
/// command handling, auditing, and dispatching.
///
/// # Example
///
/// ```rust,ignore
/// fn audit_command<C: EntityCommand>(cmd: &C) {
///     log::info!("Executing command: {} ({:?})", cmd.name(), cmd.kind());
/// }
/// ```
pub trait EntityCommand: Send + Sync + std::fmt::Debug {
    /// Get the kind of command for categorization.
    fn kind(&self) -> CommandKind;

    /// Get the command name as a string for logging/auditing.
    fn name(&self) -> &'static str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pagination_new() {
        let p = Pagination::new(50, 100);
        assert_eq!(p.limit, 50);
        assert_eq!(p.offset, 100);
    }

    #[test]
    fn pagination_page() {
        let p = Pagination::page(2, 25);
        assert_eq!(p.limit, 25);
        assert_eq!(p.offset, 50);
    }

    #[test]
    fn pagination_default() {
        let p = Pagination::default();
        assert_eq!(p.limit, 100);
        assert_eq!(p.offset, 0);
    }

    #[test]
    fn sort_direction_sql() {
        assert_eq!(SortDirection::Asc.as_sql(), "ASC");
        assert_eq!(SortDirection::Desc.as_sql(), "DESC");
    }

    #[test]
    fn sort_direction_default() {
        assert_eq!(SortDirection::default(), SortDirection::Asc);
    }

    #[test]
    fn event_kind_is_delete() {
        assert!(!EventKind::Created.is_delete());
        assert!(!EventKind::Updated.is_delete());
        assert!(EventKind::SoftDeleted.is_delete());
        assert!(EventKind::HardDeleted.is_delete());
        assert!(!EventKind::Restored.is_delete());
    }

    #[test]
    fn event_kind_is_mutation() {
        assert!(EventKind::Created.is_mutation());
        assert!(EventKind::Updated.is_mutation());
        assert!(EventKind::SoftDeleted.is_mutation());
        assert!(EventKind::HardDeleted.is_mutation());
        assert!(!EventKind::Restored.is_mutation());
    }

    #[test]
    fn command_kind_is_create() {
        assert!(CommandKind::Create.is_create());
        assert!(!CommandKind::Update.is_create());
        assert!(!CommandKind::Delete.is_create());
        assert!(!CommandKind::Custom.is_create());
    }

    #[test]
    fn command_kind_is_mutation() {
        assert!(CommandKind::Create.is_mutation());
        assert!(CommandKind::Update.is_mutation());
        assert!(CommandKind::Delete.is_mutation());
        assert!(!CommandKind::Custom.is_mutation());
    }
}
