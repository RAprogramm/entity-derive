// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Live-Postgres harness for the generated-SQL integration suite.
//!
//! The rest of the test suite proves that the macro *compiles*. This
//! harness lets a test prove that what the macro emitted is also valid
//! SQL: every case provisions a throwaway database, applies the
//! entity's generated `MIGRATION_UP`, exercises the generated
//! repository methods against a real server, and drops the database
//! again.
//!
//! # Running
//!
//! Point one of the environment variables below at a Postgres server —
//! the URL names the maintenance database used to create and drop the
//! per-test ones:
//!
//! | Variable | Precedence |
//! |----------|------------|
//! | `ENTITY_DERIVE_TEST_DATABASE_URL` | preferred |
//! | `DATABASE_URL` | fallback |
//!
//! ```text
//! ENTITY_DERIVE_TEST_DATABASE_URL=postgres://postgres@localhost/postgres \
//!   cargo test -p entity-derive --all-features --test postgres
//! ```
//!
//! With neither variable set the cases print a notice and pass, so a
//! contributor without a local server is not blocked. Under CI the
//! absence is a hard error instead — coverage must not be lost by a
//! forgotten variable.
//!
//! # Isolation
//!
//! Table names are compile-time constants, so isolation comes from a
//! database per test rather than a schema per test. Names carry their
//! creation timestamp (`ed_t_<millis>_<random>_<label>`), which lets
//! every run sweep databases left behind by an earlier *aborted* run
//! without ever touching one belonging to a test running right now.

use std::{
    env,
    time::{SystemTime, UNIX_EPOCH}
};

use sqlx::{
    AssertSqlSafe, Connection, PgConnection, PgPool,
    postgres::{PgConnectOptions, PgPoolOptions}
};
use uuid::Uuid;

/// Environment variables consulted for the maintenance connection, in
/// order of precedence.
const URL_VARS: [&str; 2] = ["ENTITY_DERIVE_TEST_DATABASE_URL", "DATABASE_URL"];

/// Prefix shared by every database this harness creates.
const DB_PREFIX: &str = "ed_t_";

/// Age past which a leftover database is treated as abandoned by an
/// aborted run and swept.
const STALE_AFTER_MS: u128 = 30 * 60 * 1000;

/// A provisioned database, owned by exactly one test.
pub struct TestDb {
    /// Name of the database created for this test.
    name:  String,
    /// Pool connected to that database.
    pool:  PgPool,
    /// Connection options of the maintenance database, used to create
    /// and drop `name`.
    admin: PgConnectOptions
}

impl TestDb {
    /// Pool connected to the throwaway database.
    pub const fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Execute arbitrary SQL against the throwaway database.
    ///
    /// Accepts multi-statement scripts, which is what the generated
    /// `MIGRATION_UP` and `MIGRATION_DOWN` constants are.
    pub async fn run(&self, sql: &str) {
        sqlx::raw_sql(AssertSqlSafe(sql.to_owned()))
            .execute(&self.pool)
            .await
            .unwrap_or_else(|e| panic!("failed to run SQL against {}: {e}\n{sql}", self.name));
    }

    /// Close the pool and drop the database.
    ///
    /// Call this at the end of a test. A test that panics earlier
    /// leaves the database behind on purpose — it can be inspected,
    /// and the next run sweeps it once it ages out.
    pub async fn teardown(self) {
        self.pool.close().await;
        let Ok(mut conn) = PgConnection::connect_with(&self.admin).await else {
            return;
        };
        let drop_sql = format!("DROP DATABASE IF EXISTS \"{}\" WITH (FORCE)", self.name);
        let _ = sqlx::raw_sql(AssertSqlSafe(drop_sql))
            .execute(&mut conn)
            .await;
    }
}

/// Provision a database for one test and apply the given migration
/// scripts, or return `None` when no server is configured.
///
/// `label` is embedded in the database name to make a leftover
/// identifiable.
pub async fn provision(label: &str, migrations: &[&str]) -> Option<TestDb> {
    let base = maintenance_url()?;
    let admin: PgConnectOptions = base
        .parse()
        .unwrap_or_else(|e| panic!("invalid Postgres URL in the environment: {e}"));

    sweep_abandoned(&admin).await;

    let name = database_name(label);
    let mut conn = PgConnection::connect_with(&admin)
        .await
        .unwrap_or_else(|e| panic!("cannot reach the Postgres server: {e}"));
    let create_sql = format!("CREATE DATABASE \"{name}\"");
    sqlx::raw_sql(AssertSqlSafe(create_sql))
        .execute(&mut conn)
        .await
        .unwrap_or_else(|e| panic!("cannot create database {name}: {e}"));
    let _ = conn.close().await;

    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect_with(admin.clone().database(&name))
        .await
        .unwrap_or_else(|e| panic!("cannot connect to {name}: {e}"));

    let db = TestDb {
        name,
        pool,
        admin
    };
    for script in migrations {
        db.run(script).await;
    }
    Some(db)
}

/// Resolve the maintenance URL, deciding between skipping and failing
/// when none is configured.
fn maintenance_url() -> Option<String> {
    for key in URL_VARS {
        if let Ok(value) = env::var(key)
            && !value.trim().is_empty()
        {
            return Some(value);
        }
    }

    assert!(
        env::var("CI").is_err(),
        "the live-Postgres suite requires {} under CI; the job must provide a server",
        URL_VARS.join(" or ")
    );

    eprintln!(
        "skipping: no live Postgres configured, set {} to run this test",
        URL_VARS.join(" or ")
    );
    None
}

/// Build a unique, timestamped database name for one test.
fn database_name(label: &str) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let slug: String = label
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .take(16)
        .collect();
    let salt = Uuid::new_v4().simple().to_string();
    format!("{DB_PREFIX}{millis}_{}_{slug}", &salt[..8])
}

/// Drop databases this harness created that are older than
/// [`STALE_AFTER_MS`], i.e. left behind by an aborted run.
///
/// Every failure is ignored: a sweep is a courtesy, never a reason to
/// fail the test that triggered it.
async fn sweep_abandoned(admin: &PgConnectOptions) {
    let Ok(mut conn) = PgConnection::connect_with(admin).await else {
        return;
    };
    let listed: Result<Vec<(String,)>, _> =
        sqlx::query_as("SELECT datname FROM pg_database WHERE datname LIKE $1")
            .bind(format!("{DB_PREFIX}%"))
            .fetch_all(&mut conn)
            .await;
    let Ok(rows) = listed else {
        return;
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    for (name,) in rows {
        let abandoned = created_at_millis(&name)
            .is_some_and(|created| now.saturating_sub(created) > STALE_AFTER_MS);
        if abandoned {
            let drop_sql = format!("DROP DATABASE IF EXISTS \"{name}\" WITH (FORCE)");
            let _ = sqlx::raw_sql(AssertSqlSafe(drop_sql))
                .execute(&mut conn)
                .await;
        }
    }
    let _ = conn.close().await;
}

/// Recover the creation timestamp encoded in a harness database name.
fn created_at_millis(name: &str) -> Option<u128> {
    name.strip_prefix(DB_PREFIX)?
        .split('_')
        .next()?
        .parse()
        .ok()
}
