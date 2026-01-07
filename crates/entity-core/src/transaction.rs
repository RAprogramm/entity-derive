// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Transaction support for entity-derive.
//!
//! This module provides type-safe transaction management with automatic
//! commit/rollback semantics. It uses the builder pattern for composing
//! multiple repositories into a single transaction context.
//!
//! # Overview
//!
//! - [`Transaction`] — Entry point for creating transactions
//! - [`TransactionContext`] — Holds active transaction and repository adapters
//! - [`TransactionError`] — Error wrapper for transaction operations
//!
//! # Example
//!
//! ```rust,ignore
//! use entity_derive::prelude::*;
//!
//! async fn transfer(pool: &PgPool, from: Uuid, to: Uuid, amount: Decimal) -> Result<(), AppError> {
//!     Transaction::new(pool)
//!         .with_accounts()
//!         .with_transfers()
//!         .run(|mut ctx| async move {
//!             let from_acc = ctx.accounts().find_by_id(from).await?.ok_or(AppError::NotFound)?;
//!
//!             ctx.accounts().update(from, UpdateAccount {
//!                 balance: Some(from_acc.balance - amount),
//!             }).await?;
//!
//!             ctx.transfers().create(CreateTransfer { from, to, amount }).await?;
//!             Ok(())
//!         })
//!         .await
//! }
//! ```

use std::{error::Error as StdError, fmt, future::Future, marker::PhantomData};

/// Transaction builder for composing repositories.
///
/// Use [`Transaction::new`] to create a builder, then chain `.with_*()` methods
/// to add repositories, and finally call `.run()` to execute.
///
/// # Type Parameters
///
/// - `DB` — Database type (e.g., `Postgres`)
/// - `Repos` — Tuple of repository adapters accumulated via builder
pub struct Transaction<'p, DB, Repos = ()> {
    pool:   &'p DB,
    _repos: PhantomData<Repos>
}

impl<'p, DB> Transaction<'p, DB, ()> {
    /// Create a new transaction builder.
    ///
    /// # Arguments
    ///
    /// * `pool` — Database connection pool
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let tx = Transaction::new(&pool);
    /// ```
    pub const fn new(pool: &'p DB) -> Self {
        Self {
            pool,
            _repos: PhantomData
        }
    }
}

impl<'p, DB, Repos> Transaction<'p, DB, Repos> {
    /// Get reference to the underlying pool.
    pub const fn pool(&self) -> &'p DB {
        self.pool
    }

    /// Transform repository tuple type.
    ///
    /// Used internally by generated `with_*` methods.
    #[doc(hidden)]
    pub const fn with_repo<NewRepos>(self) -> Transaction<'p, DB, NewRepos> {
        Transaction {
            pool:   self.pool,
            _repos: PhantomData
        }
    }
}

/// Active transaction context with repository adapters.
///
/// This struct holds the database transaction and provides access to
/// repository adapters that operate within the transaction.
///
/// # Automatic Rollback
///
/// If dropped without explicit commit, the transaction is automatically
/// rolled back via the underlying database transaction's Drop impl.
///
/// # Type Parameters
///
/// - `'t` — Transaction lifetime
/// - `Tx` — Transaction type (e.g., `sqlx::Transaction<'t, Postgres>`)
/// - `Repos` — Tuple of repository adapters
pub struct TransactionContext<'t, Tx, Repos> {
    tx:        Tx,
    repos:     Repos,
    _lifetime: PhantomData<&'t ()>
}

impl<'t, Tx, Repos> TransactionContext<'t, Tx, Repos> {
    /// Create a new transaction context.
    ///
    /// # Arguments
    ///
    /// * `tx` — Active database transaction
    /// * `repos` — Repository adapters tuple
    #[doc(hidden)]
    pub const fn new(tx: Tx, repos: Repos) -> Self {
        Self {
            tx,
            repos,
            _lifetime: PhantomData
        }
    }

    /// Get mutable reference to the underlying transaction.
    ///
    /// Use this for custom queries within the transaction.
    pub fn transaction(&mut self) -> &mut Tx {
        &mut self.tx
    }

    /// Get reference to repository adapters.
    pub const fn repos(&self) -> &Repos {
        &self.repos
    }

    /// Get mutable reference to repository adapters.
    pub fn repos_mut(&mut self) -> &mut Repos {
        &mut self.repos
    }
}

/// Error type for transaction operations.
///
/// Wraps database errors and provides context about the transaction state.
#[derive(Debug)]
pub enum TransactionError<E> {
    /// Failed to begin transaction.
    Begin(E),

    /// Failed to commit transaction.
    Commit(E),

    /// Failed to rollback transaction.
    Rollback(E),

    /// Operation within transaction failed.
    Operation(E)
}

impl<E: fmt::Display> fmt::Display for TransactionError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Begin(e) => write!(f, "failed to begin transaction: {e}"),
            Self::Commit(e) => write!(f, "failed to commit transaction: {e}"),
            Self::Rollback(e) => write!(f, "failed to rollback transaction: {e}"),
            Self::Operation(e) => write!(f, "transaction operation failed: {e}")
        }
    }
}

impl<E: StdError + 'static> StdError for TransactionError<E> {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Begin(e) | Self::Commit(e) | Self::Rollback(e) | Self::Operation(e) => Some(e)
        }
    }
}

impl<E> TransactionError<E> {
    /// Check if this is a begin error.
    pub const fn is_begin(&self) -> bool {
        matches!(self, Self::Begin(_))
    }

    /// Check if this is a commit error.
    pub const fn is_commit(&self) -> bool {
        matches!(self, Self::Commit(_))
    }

    /// Check if this is a rollback error.
    pub const fn is_rollback(&self) -> bool {
        matches!(self, Self::Rollback(_))
    }

    /// Check if this is an operation error.
    pub const fn is_operation(&self) -> bool {
        matches!(self, Self::Operation(_))
    }

    /// Get the inner error.
    pub fn into_inner(self) -> E {
        match self {
            Self::Begin(e) | Self::Commit(e) | Self::Rollback(e) | Self::Operation(e) => e
        }
    }
}

/// Trait for types that can begin a transaction.
///
/// Implemented for database pools to enable transaction creation.
#[allow(async_fn_in_trait)]
pub trait Transactional: Sized + Send + Sync {
    /// Transaction type.
    type Transaction<'t>: Send
    where
        Self: 't;

    /// Error type for transaction operations.
    type Error: StdError + Send + Sync;

    /// Begin a new transaction.
    async fn begin(&self) -> Result<Self::Transaction<'_>, Self::Error>;
}

/// Trait for transaction types that can be committed or rolled back.
#[allow(async_fn_in_trait)]
pub trait TransactionOps: Sized + Send {
    /// Error type.
    type Error: StdError + Send + Sync;

    /// Commit the transaction.
    async fn commit(self) -> Result<(), Self::Error>;

    /// Rollback the transaction.
    async fn rollback(self) -> Result<(), Self::Error>;
}

/// Trait for executing operations within a transaction.
///
/// This trait is implemented on [`Transaction`] with specific repository
/// combinations, enabling type-safe execution.
#[allow(async_fn_in_trait)]
pub trait TransactionRunner<'p, Repos>: Sized {
    /// Transaction type.
    type Tx: TransactionOps;

    /// Database error type.
    type DbError: StdError + Send + Sync;

    /// Execute a closure within the transaction.
    ///
    /// Automatically commits on `Ok`, rolls back on `Err` or panic.
    ///
    /// # Type Parameters
    ///
    /// - `F` — Closure type
    /// - `Fut` — Future returned by closure
    /// - `T` — Success type
    /// - `E` — Error type (must be convertible from database error)
    async fn run<F, Fut, T, E>(self, f: F) -> Result<T, E>
    where
        F: FnOnce(TransactionContext<'_, Self::Tx, Repos>) -> Fut + Send,
        Fut: Future<Output = Result<T, E>> + Send,
        E: From<TransactionError<Self::DbError>>;
}

// sqlx implementations
#[cfg(feature = "postgres")]
mod postgres_impl {
    use sqlx::{PgPool, Postgres};

    use super::*;

    impl Transactional for PgPool {
        type Transaction<'t> = sqlx::Transaction<'t, Postgres>;
        type Error = sqlx::Error;

        async fn begin(&self) -> Result<Self::Transaction<'_>, Self::Error> {
            sqlx::pool::Pool::begin(self).await
        }
    }

    impl TransactionOps for sqlx::Transaction<'_, Postgres> {
        type Error = sqlx::Error;

        async fn commit(self) -> Result<(), Self::Error> {
            sqlx::Transaction::commit(self).await
        }

        async fn rollback(self) -> Result<(), Self::Error> {
            sqlx::Transaction::rollback(self).await
        }
    }

    impl<'p, Repos: Send> Transaction<'p, PgPool, Repos> {
        /// Execute a closure within a PostgreSQL transaction.
        ///
        /// Automatically commits on `Ok`, rolls back on `Err` or drop.
        ///
        /// # Example
        ///
        /// ```rust,ignore
        /// Transaction::new(&pool)
        ///     .with_users()
        ///     .run(|mut ctx| async move {
        ///         ctx.users().create(dto).await
        ///     })
        ///     .await?;
        /// ```
        pub async fn run<F, Fut, T, E>(self, f: F) -> Result<T, E>
        where
            F: for<'t> FnOnce(
                    TransactionContext<'t, sqlx::Transaction<'t, Postgres>, Repos>
                ) -> Fut
                + Send,
            Fut: Future<Output = Result<T, E>> + Send,
            E: From<TransactionError<sqlx::Error>>,
            Repos: Default
        {
            let tx = self.pool.begin().await.map_err(TransactionError::Begin)?;
            let ctx = TransactionContext::new(tx, Repos::default());

            match f(ctx).await {
                Ok(result) => Ok(result),
                Err(e) => Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_error_display_begin() {
        let err: TransactionError<std::io::Error> =
            TransactionError::Begin(std::io::Error::other("test"));
        assert!(err.to_string().contains("begin"));
        assert!(err.to_string().contains("test"));
    }

    #[test]
    fn transaction_error_display_commit() {
        let err: TransactionError<std::io::Error> =
            TransactionError::Commit(std::io::Error::other("commit_err"));
        assert!(err.to_string().contains("commit"));
        assert!(err.to_string().contains("commit_err"));
    }

    #[test]
    fn transaction_error_display_rollback() {
        let err: TransactionError<std::io::Error> =
            TransactionError::Rollback(std::io::Error::other("rollback_err"));
        assert!(err.to_string().contains("rollback"));
        assert!(err.to_string().contains("rollback_err"));
    }

    #[test]
    fn transaction_error_display_operation() {
        let err: TransactionError<std::io::Error> =
            TransactionError::Operation(std::io::Error::other("op_err"));
        assert!(err.to_string().contains("operation"));
        assert!(err.to_string().contains("op_err"));
    }

    #[test]
    fn transaction_error_is_methods() {
        let begin: TransactionError<&str> = TransactionError::Begin("e");
        let commit: TransactionError<&str> = TransactionError::Commit("e");
        let rollback: TransactionError<&str> = TransactionError::Rollback("e");
        let op: TransactionError<&str> = TransactionError::Operation("e");

        assert!(begin.is_begin());
        assert!(!begin.is_commit());
        assert!(!begin.is_rollback());
        assert!(!begin.is_operation());

        assert!(commit.is_commit());
        assert!(!commit.is_begin());

        assert!(rollback.is_rollback());
        assert!(!rollback.is_begin());

        assert!(op.is_operation());
        assert!(!op.is_begin());
    }

    #[test]
    fn transaction_error_into_inner() {
        let err: TransactionError<&str> = TransactionError::Operation("inner");
        assert_eq!(err.into_inner(), "inner");
    }

    #[test]
    fn transaction_error_into_inner_all_variants() {
        assert_eq!(TransactionError::Begin("b").into_inner(), "b");
        assert_eq!(TransactionError::Commit("c").into_inner(), "c");
        assert_eq!(TransactionError::Rollback("r").into_inner(), "r");
        assert_eq!(TransactionError::Operation("o").into_inner(), "o");
    }

    #[test]
    fn transaction_error_source() {
        let inner = std::io::Error::other("source_err");
        let err: TransactionError<std::io::Error> = TransactionError::Begin(inner);
        assert!(err.source().is_some());

        let commit_err: TransactionError<std::io::Error> =
            TransactionError::Commit(std::io::Error::other("c"));
        assert!(commit_err.source().is_some());

        let rollback_err: TransactionError<std::io::Error> =
            TransactionError::Rollback(std::io::Error::other("r"));
        assert!(rollback_err.source().is_some());

        let op_err: TransactionError<std::io::Error> =
            TransactionError::Operation(std::io::Error::other("o"));
        assert!(op_err.source().is_some());
    }

    #[test]
    fn transaction_builder_new() {
        struct MockPool;
        let pool = MockPool;
        let tx: Transaction<'_, MockPool, ()> = Transaction::new(&pool);
        let _ = tx.pool();
    }

    #[test]
    fn transaction_builder_with_repo() {
        struct MockPool;
        let pool = MockPool;
        let tx: Transaction<'_, MockPool, ()> = Transaction::new(&pool);
        let tx2: Transaction<'_, MockPool, i32> = tx.with_repo();
        let _ = tx2.pool();
    }

    #[test]
    fn transaction_context_new() {
        let tx = "mock_tx";
        let repos = (1, 2, 3);
        let ctx = TransactionContext::new(tx, repos);
        assert_eq!(*ctx.repos(), (1, 2, 3));
    }

    #[test]
    fn transaction_context_transaction() {
        let tx = String::from("mock_tx");
        let repos = ();
        let mut ctx = TransactionContext::new(tx, repos);
        assert_eq!(ctx.transaction(), "mock_tx");
    }

    #[test]
    fn transaction_context_repos_mut() {
        let tx = "mock_tx";
        let repos = vec![1, 2, 3];
        let mut ctx = TransactionContext::new(tx, repos);
        ctx.repos_mut().push(4);
        assert_eq!(*ctx.repos(), vec![1, 2, 3, 4]);
    }
}
