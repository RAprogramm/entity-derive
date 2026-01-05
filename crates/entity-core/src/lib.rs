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
}
