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

#[cfg(feature = "outbox")]
pub mod outbox;
pub mod policy;
pub mod prelude;
#[cfg(feature = "postgres")]
pub mod schema;
#[cfg(feature = "streams")]
pub mod stream;
pub mod transaction;

/// Re-export `async_trait` for generated code.
pub use async_trait::async_trait;
/// Re-export `futures` for generated streaming methods.
///
/// Generated code reaches it through the `entity-derive` facade, so a
/// consumer never has to depend on `futures` itself.
#[cfg(feature = "streams")]
pub use futures;

/// Compare two strings in const context.
///
/// Used by generated code to verify at compile time that
/// `#[column(pg_enum = "...")]` matches the `ValueObject`'s
/// `#[value_object(pg_type = "...")]` declaration.
#[must_use]
pub const fn const_str_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

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
    #[must_use]
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
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    pub const fn is_delete(&self) -> bool {
        matches!(self, Self::SoftDeleted | Self::HardDeleted)
    }

    /// Check if this is a mutation event (create, update, delete).
    #[must_use]
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

    /// Modifies an existing entity (e.g., `UpdateEmail`, `ChangeStatus`).
    Update,

    /// Removes an entity (e.g., Delete, Deactivate).
    Delete,

    /// Custom business operation that doesn't fit CRUD.
    Custom
}

impl CommandKind {
    /// Check if this command creates an entity.
    #[must_use]
    pub const fn is_create(&self) -> bool {
        matches!(self, Self::Create)
    }

    /// Check if this command modifies state.
    #[must_use]
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
    fn constraint_error_display_unique_field() {
        let err = ConstraintError {
            kind:       ConstraintKind::Unique,
            constraint: "users_email_key".to_string(),
            field:      Some("email")
        };
        assert_eq!(err.to_string(), "duplicate value for unique field `email`");
    }

    #[test]
    fn constraint_error_display_fk_field() {
        let err = ConstraintError {
            kind:       ConstraintKind::ForeignKey,
            constraint: "orders_user_id_fkey".to_string(),
            field:      Some("user_id")
        };
        assert_eq!(
            err.to_string(),
            "referenced row missing for field `user_id`"
        );
    }

    #[test]
    fn constraint_error_display_unknown_field() {
        let err = ConstraintError {
            kind:       ConstraintKind::Check,
            constraint: "orders_amount_check".to_string(),
            field:      None
        };
        assert_eq!(
            err.to_string(),
            "Check constraint `orders_amount_check` violated"
        );
    }

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

    #[test]
    fn const_str_eq_equal_strings() {
        assert!(const_str_eq("order_status", "order_status"));
        assert!(const_str_eq("", ""));
    }

    #[test]
    fn const_str_eq_different_lengths() {
        assert!(!const_str_eq("order", "order_status"));
        assert!(!const_str_eq("order_status", ""));
    }

    #[test]
    fn const_str_eq_same_length_different_content() {
        assert!(!const_str_eq("order_status", "order_states"));
        assert!(!const_str_eq("abc", "abd"));
    }

    #[test]
    fn const_str_eq_in_const_context() {
        const OK: bool = const_str_eq("user_role", "user_role");
        const MISMATCH: bool = const_str_eq("user_role", "user_rank");
        assert_eq!((OK, MISMATCH), (true, false));
    }
}

/// Serde helpers for generated DTOs.
#[cfg(feature = "serde")]
pub mod serde_helpers {
    /// Double-`Option` (de)serialization for PATCH semantics.
    ///
    /// | JSON | Rust |
    /// |------|------|
    /// | field absent | `None` (leave unchanged) |
    /// | `"field": null` | `Some(None)` (set column to NULL) |
    /// | `"field": v` | `Some(Some(v))` (set column to v) |
    pub mod double_option {
        use serde::{Deserialize, Deserializer, Serialize, Serializer};

        /// Deserialize a present-but-maybe-null field into `Some(inner)`.
        ///
        /// # Errors
        ///
        /// Propagates inner deserialization errors.
        pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
        where
            T: Deserialize<'de>,
            D: Deserializer<'de>
        {
            Option::<T>::deserialize(deserializer).map(Some)
        }

        /// Serialize the inner `Option`, treating outer `None` as null.
        ///
        /// # Errors
        ///
        /// Propagates inner serialization errors.
        pub fn serialize<T, S>(value: &Option<Option<T>>, serializer: S) -> Result<S::Ok, S::Error>
        where
            T: Serialize,
            S: Serializer
        {
            match value {
                Some(inner) => inner.serialize(serializer),
                None => serializer.serialize_none()
            }
        }
    }

    #[cfg(all(test, feature = "serde_json"))]
    mod tests {
        #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Default)]
        struct Patch {
            #[serde(
                default,
                skip_serializing_if = "Option::is_none",
                with = "super::double_option"
            )]
            nick: Option<Option<String>>
        }

        #[test]
        fn absent_field_is_outer_none() {
            let patch: Patch = serde_json::from_str("{}").unwrap();
            assert_eq!(patch.nick, None);
        }

        #[test]
        fn null_field_is_some_none() {
            let patch: Patch = serde_json::from_str(r#"{"nick": null}"#).unwrap();
            assert_eq!(patch.nick, Some(None));
        }

        #[test]
        fn value_field_is_some_some() {
            let patch: Patch = serde_json::from_str(r#"{"nick": "neo"}"#).unwrap();
            assert_eq!(patch.nick, Some(Some("neo".to_string())));
        }

        #[test]
        fn outer_none_skipped_on_serialize() {
            let json = serde_json::to_string(&Patch {
                nick: None
            })
            .unwrap();
            assert_eq!(json, "{}");
        }

        #[test]
        fn some_none_serializes_null() {
            let json = serde_json::to_string(&Patch {
                nick: Some(None)
            })
            .unwrap();
            assert_eq!(json, r#"{"nick":null}"#);
        }
    }
}

/// Kind of database constraint that was violated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConstraintKind {
    /// UNIQUE constraint or unique index.
    Unique,

    /// FOREIGN KEY constraint.
    ForeignKey,

    /// CHECK constraint.
    Check
}

/// A database constraint violation resolved to entity metadata.
///
/// Produced by repositories generated with
/// `#[entity(typed_constraints)]`: the generated code matches the
/// violated constraint name against the set of constraints it created
/// (unique columns, foreign keys, unique indexes) and hands callers a
/// structured error instead of a raw driver error.
///
/// The repository `Error` type must implement
/// `From<ConstraintError>` in addition to `From<sqlx::Error>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintError {
    /// What kind of constraint was violated.
    pub kind: ConstraintKind,

    /// Constraint name as reported by the database.
    pub constraint: String,

    /// Entity field the constraint maps to, when known.
    pub field: Option<&'static str>
}

impl std::fmt::Display for ConstraintError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.kind, self.field) {
            (ConstraintKind::Unique, Some(field)) => {
                write!(f, "duplicate value for unique field `{field}`")
            }
            (ConstraintKind::ForeignKey, Some(field)) => {
                write!(f, "referenced row missing for field `{field}`")
            }
            (kind, _) => write!(f, "{kind:?} constraint `{}` violated", self.constraint)
        }
    }
}

impl std::error::Error for ConstraintError {}

/// A state-machine transition was attempted from a status it is not
/// declared for.
///
/// Produced by repository methods generated from `#[transition(...)]`
/// declarations. The consumer's error type must implement
/// `From<TransitionError>`; map it to an HTTP 409 or a domain conflict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionError {
    /// Entity name the transition belongs to.
    pub entity: &'static str,

    /// Current status of the row, `Debug`-formatted.
    pub from: String,

    /// Target status of the attempted transition.
    pub to: &'static str
}

impl std::fmt::Display for TransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} cannot transition from `{}` to `{}`",
            self.entity, self.from, self.to
        )
    }
}

impl std::error::Error for TransitionError {}
