// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Transactional-outbox drainer runtime.
//!
//! Entities declared with `#[entity(events(outbox))]` enqueue their
//! lifecycle events into the `entity_outbox` table inside the same
//! transaction as the write. This module delivers those rows:
//!
//! ```rust,ignore
//! use entity_core::outbox::{OutboxDrainer, OutboxHandler, OutboxRow};
//!
//! struct Notifier;
//!
//! #[async_trait::async_trait]
//! impl OutboxHandler for Notifier {
//!     type Error = anyhow::Error;
//!
//!     async fn handle(&self, row: &OutboxRow) -> Result<(), Self::Error> {
//!         send_somewhere(&row.entity, &row.payload).await
//!     }
//! }
//!
//! OutboxDrainer::new(pool, Notifier)
//!     .run()   // polls until the task is aborted
//!     .await;
//! ```
//!
//! # Delivery Semantics
//!
//! - Rows are claimed with `FOR UPDATE SKIP LOCKED`, so multiple drainers
//!   cooperate without double delivery.
//! - A successful `handle` marks the row processed; a failure schedules a retry
//!   with exponential backoff (`base_backoff * 2^attempts`).
//! - After `max_attempts` failures the row stays unprocessed with
//!   `next_attempt_at` in the far future — inspect and repair manually.
//! - At-least-once delivery: handlers must be idempotent.

use std::time::Duration;

use sqlx::{PgPool, Row};

/// One claimed outbox row handed to the [`OutboxHandler`].
#[derive(Debug, Clone)]
pub struct OutboxRow {
    /// Outbox row id.
    pub id: i64,

    /// Source table name of the emitting entity.
    pub entity: String,

    /// Event kind (`created`, `updated`, `soft_deleted`, `hard_deleted`).
    pub kind: String,

    /// String form of the affected row's primary key.
    pub entity_id: String,

    /// Serialized `{Entity}Event` payload.
    pub payload: serde_json::Value,

    /// Delivery attempts made so far.
    pub attempts: i32
}

/// User-provided delivery target for outbox rows.
#[async_trait::async_trait]
pub trait OutboxHandler: Send + Sync {
    /// Handler error type.
    type Error: std::fmt::Display + Send;

    /// Deliver one event. Returning `Err` schedules a retry with
    /// exponential backoff.
    async fn handle(&self, row: &OutboxRow) -> Result<(), Self::Error>;
}

/// Polling drainer delivering `entity_outbox` rows to a handler.
pub struct OutboxDrainer<H> {
    pool:          PgPool,
    handler:       H,
    batch_size:    i64,
    poll_interval: Duration,
    base_backoff:  Duration,
    max_attempts:  i32
}

impl<H: OutboxHandler> OutboxDrainer<H> {
    /// Create a drainer with defaults: batch 32, poll every second,
    /// backoff base 5s, 10 attempts.
    #[must_use]
    pub fn new(pool: PgPool, handler: H) -> Self {
        Self {
            pool,
            handler,
            batch_size: 32,
            poll_interval: Duration::from_secs(1),
            base_backoff: Duration::from_secs(5),
            max_attempts: 10
        }
    }

    /// Set the maximum rows claimed per polling cycle.
    #[must_use]
    pub fn batch_size(mut self, batch_size: i64) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Set the sleep between empty polling cycles.
    #[must_use]
    pub fn poll_interval(mut self, poll_interval: Duration) -> Self {
        self.poll_interval = poll_interval;
        self
    }

    /// Set the exponential-backoff base delay.
    #[must_use]
    pub fn base_backoff(mut self, base_backoff: Duration) -> Self {
        self.base_backoff = base_backoff;
        self
    }

    /// Set the retry ceiling before a row is parked.
    #[must_use]
    pub fn max_attempts(mut self, max_attempts: i32) -> Self {
        self.max_attempts = max_attempts;
        self
    }

    /// Drain continuously until the surrounding task is cancelled.
    pub async fn run(self) {
        loop {
            match self.drain_once().await {
                Ok(0) | Err(_) => tokio::time::sleep(self.poll_interval).await,
                Ok(_) => {}
            }
        }
    }

    /// Claim and deliver one batch.
    ///
    /// Returns the number of rows processed (successfully or scheduled
    /// for retry).
    ///
    /// # Errors
    ///
    /// Propagates database errors from claiming or updating rows.
    pub async fn drain_once(&self) -> Result<usize, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let rows = sqlx::query(
            "SELECT id, entity, kind, entity_id, payload, attempts \
             FROM entity_outbox \
             WHERE processed_at IS NULL AND next_attempt_at <= NOW() \
             ORDER BY id \
             LIMIT $1 \
             FOR UPDATE SKIP LOCKED"
        )
        .bind(self.batch_size)
        .fetch_all(&mut *tx)
        .await?;

        let mut processed = 0usize;
        for row in rows {
            let outbox_row = OutboxRow {
                id:        row.try_get("id")?,
                entity:    row.try_get("entity")?,
                kind:      row.try_get("kind")?,
                entity_id: row.try_get("entity_id")?,
                payload:   row.try_get("payload")?,
                attempts:  row.try_get("attempts")?
            };

            match self.handler.handle(&outbox_row).await {
                Ok(()) => {
                    sqlx::query("UPDATE entity_outbox SET processed_at = NOW() WHERE id = $1")
                        .bind(outbox_row.id)
                        .execute(&mut *tx)
                        .await?;
                }
                Err(_) => {
                    let delay =
                        backoff_delay(self.base_backoff, outbox_row.attempts, self.max_attempts);
                    sqlx::query(
                        "UPDATE entity_outbox \
                         SET attempts = attempts + 1, \
                             next_attempt_at = NOW() + $2 * INTERVAL '1 second' \
                         WHERE id = $1"
                    )
                    .bind(outbox_row.id)
                    .bind(delay.as_secs_f64())
                    .execute(&mut *tx)
                    .await?;
                }
            }
            processed += 1;
        }

        tx.commit().await?;
        Ok(processed)
    }
}

/// Compute the retry delay for a row that has already failed `attempts`
/// times.
///
/// Exponential (`base * 2^attempts`) capped at one hour; once
/// `max_attempts` is reached the row is parked ~30 days out for manual
/// inspection.
#[must_use]
pub fn backoff_delay(base: Duration, attempts: i32, max_attempts: i32) -> Duration {
    if attempts + 1 >= max_attempts {
        return Duration::from_secs(60 * 60 * 24 * 30);
    }
    let exp = u32::try_from(attempts.clamp(0, 20)).unwrap_or(0);
    let delay = base.saturating_mul(2u32.saturating_pow(exp));
    delay.min(Duration::from_secs(3600))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_exponentially() {
        let base = Duration::from_secs(5);
        assert_eq!(backoff_delay(base, 0, 10), Duration::from_secs(5));
        assert_eq!(backoff_delay(base, 1, 10), Duration::from_secs(10));
        assert_eq!(backoff_delay(base, 3, 10), Duration::from_secs(40));
    }

    #[test]
    fn backoff_caps_at_one_hour() {
        let base = Duration::from_secs(5);
        assert_eq!(backoff_delay(base, 15, 20), Duration::from_secs(3600));
    }

    #[test]
    fn backoff_parks_after_max_attempts() {
        let base = Duration::from_secs(5);
        let parked = backoff_delay(base, 9, 10);
        assert_eq!(parked, Duration::from_secs(60 * 60 * 24 * 30));
    }

    #[test]
    fn backoff_negative_attempts_treated_as_zero() {
        let base = Duration::from_secs(5);
        assert_eq!(backoff_delay(base, -3, 10), Duration::from_secs(5));
    }
}
