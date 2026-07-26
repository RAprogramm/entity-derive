// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Generated SQL executed against a live Postgres server.
//!
//! The trybuild cases in `tests/cases` prove that the macro emits code
//! that compiles; nothing there proves that the emitted *SQL* is valid.
//! This target closes that gap: the generated DDL creates real tables
//! and the generated repository methods run real statements against
//! them.
//!
//! Each entity lives in its own module so that the repository traits —
//! all of which are implemented for `sqlx::PgPool` — never collide in
//! method resolution.
//!
//! See [`pg`] for how to point the suite at a server; without one every
//! case reports a skip and passes.

mod pg;

/// CRUD, bulk, keyset pagination, upsert, projections, filtering and
/// schema assertion over a single richly annotated entity.
mod articles {
    use chrono::{DateTime, Utc};
    use entity_derive::Entity;
    use uuid::Uuid;

    use crate::pg;

    #[derive(Debug, Clone, Entity)]
    #[entity(table = "articles", migrations, upsert(conflict = "slug"))]
    #[projection(Card: id, title, views)]
    pub struct Article {
        #[id]
        pub id: Uuid,

        #[field(create, update, response)]
        #[sort]
        #[filter(like)]
        pub title: String,

        #[field(create, response)]
        #[column(unique)]
        pub slug: String,

        #[field(create, response)]
        pub body: String,

        #[field(create, update, response)]
        #[sort]
        #[filter(range)]
        pub views: i64,

        /// Populated by the database: the generated INSERT skips
        /// `#[auto]` columns, so the DDL supplies the value.
        #[field(response)]
        #[auto]
        pub created_at: DateTime<Utc>
    }

    /// Build a create request with distinct values per call.
    fn draft(slug: &str, title: &str, views: i64) -> CreateArticleRequest {
        CreateArticleRequest {
            title: title.to_owned(),
            slug: slug.to_owned(),
            body: format!("body of {slug}"),
            views
        }
    }

    #[tokio::test]
    async fn crud_roundtrip() {
        let Some(db) = pg::provision("crud", &[Article::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let created = pool
            .create(draft("hello", "Hello", 1))
            .await
            .expect("create failed");
        assert_eq!(created.title, "Hello");
        assert_eq!(created.views, 1);

        let found = pool
            .find_by_id(created.id)
            .await
            .expect("find_by_id failed")
            .expect("row missing right after create");
        assert_eq!(found.slug, "hello");
        assert_eq!(
            found.created_at, created.created_at,
            "the auto column must come back from the database, not from the DTO"
        );

        let updated = pool
            .update(
                created.id,
                UpdateArticleRequest {
                    title: Some("Hello again".to_owned()),
                    views: Some(7)
                }
            )
            .await
            .expect("update failed");
        assert_eq!(updated.title, "Hello again");
        assert_eq!(updated.views, 7);
        assert_eq!(updated.body, created.body, "update must not touch body");

        let listed = pool.list(10, 0).await.expect("list failed");
        assert_eq!(listed.len(), 1);

        assert!(pool.delete(created.id).await.expect("delete failed"));
        assert!(
            pool.find_by_id(created.id)
                .await
                .expect("find_by_id failed")
                .is_none()
        );

        db.teardown().await;
    }

    #[tokio::test]
    async fn bulk_and_keyset_pagination() {
        let Some(db) = pg::provision("bulk", &[Article::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let created = pool
            .create_many(vec![
                draft("a", "A", 10),
                draft("b", "B", 20),
                draft("c", "C", 30),
            ])
            .await
            .expect("create_many failed");
        assert_eq!(created.len(), 3);

        let ids: Vec<Uuid> = created.iter().map(|a| a.id).collect();
        let fetched = pool
            .find_by_ids(ids.clone())
            .await
            .expect("find_by_ids failed");
        assert_eq!(fetched.len(), 3);

        let first_page = pool.list_after(None, 2).await.expect("list_after failed");
        assert_eq!(first_page.len(), 2);
        let cursor = first_page.last().map(|a| a.id);
        let second_page = pool
            .list_after(cursor, 2)
            .await
            .expect("list_after with cursor failed");
        assert_eq!(second_page.len(), 1);
        assert!(
            !second_page
                .iter()
                .any(|a| first_page.iter().any(|p| p.id == a.id)),
            "keyset pages must not overlap"
        );

        let removed = pool.delete_many(ids).await.expect("delete_many failed");
        assert_eq!(removed, 3);

        db.teardown().await;
    }

    #[tokio::test]
    async fn upsert_touches_only_update_columns() {
        let Some(db) = pg::provision("upsert", &[Article::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let original = pool
            .upsert(draft("guide", "Guide", 5))
            .await
            .expect("first upsert failed");

        let mut conflicting = draft("guide", "Guide v2", 9);
        conflicting.body = "rewritten body".to_owned();
        let merged = pool
            .upsert(conflicting)
            .await
            .expect("second upsert failed");

        assert_eq!(
            merged.id, original.id,
            "conflict must reuse the existing row"
        );
        assert_eq!(merged.title, "Guide v2", "update-marked column must change");
        assert_eq!(merged.views, 9, "update-marked column must change");
        assert_eq!(
            merged.body, original.body,
            "column without #[field(update)] must survive the conflict"
        );

        db.teardown().await;
    }

    #[tokio::test]
    async fn query_filters_and_sorts() {
        let Some(db) = pg::provision("query", &[Article::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        pool.create_many(vec![
            draft("rust-1", "Rust patterns", 100),
            draft("rust-2", "Rust internals", 50),
            draft("go-1", "Go patterns", 300),
            draft("pct", "100% coverage", 1),
        ])
        .await
        .expect("create_many failed");

        let matched = pool
            .query(ArticleQuery {
                title: Some("rust".to_owned()),
                sort: Some(ArticleSortField::ViewsDesc),
                limit: Some(10),
                ..Default::default()
            })
            .await
            .expect("query failed");
        assert_eq!(matched.len(), 2, "ILIKE filter must exclude the Go article");
        assert_eq!(matched[0].views, 100, "sort must order by views descending");

        let literal = pool
            .query(ArticleQuery {
                title: Some("100%".to_owned()),
                ..Default::default()
            })
            .await
            .expect("query with a wildcard character failed");
        assert_eq!(
            literal.len(),
            1,
            "a % inside the filter value must match literally, not as a wildcard"
        );

        let ranged = pool
            .query(ArticleQuery {
                views_from: Some(60),
                ..Default::default()
            })
            .await
            .expect("range query failed");
        assert_eq!(ranged.len(), 2, "range filter must keep views >= 60");

        let windowed = pool
            .query(ArticleQuery {
                views_from: Some(40),
                views_to: Some(150),
                ..Default::default()
            })
            .await
            .expect("bounded range query failed");
        assert_eq!(windowed.len(), 2, "both range bounds must apply");

        db.teardown().await;
    }

    #[tokio::test]
    async fn projection_returns_subset() {
        let Some(db) = pg::provision("projection", &[Article::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let created = pool
            .create(draft("card", "Card", 3))
            .await
            .expect("create failed");

        let card: ArticleCard = pool
            .find_by_id_card(created.id)
            .await
            .expect("find_by_id_card failed")
            .expect("projection row missing");
        assert_eq!(card.id, created.id);
        assert_eq!(card.title, "Card");
        assert_eq!(card.views, 3);

        db.teardown().await;
    }

    #[tokio::test]
    async fn schema_assertion_detects_drift() {
        let Some(db) = pg::provision("drift", &[Article::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        Article::assert_schema(pool)
            .await
            .expect("freshly migrated table must match the entity");

        db.run("ALTER TABLE articles DROP COLUMN body").await;
        let drift = Article::assert_schema(pool)
            .await
            .expect_err("dropping a column must be reported as drift");
        assert!(
            drift.to_string().contains("body"),
            "drift report must name the missing column, got: {drift}"
        );

        db.teardown().await;
    }

    #[tokio::test]
    async fn migration_down_reverses_up() {
        let Some(db) = pg::provision("migration", &[Article::MIGRATION_UP]).await else {
            return;
        };

        Article::assert_schema(db.pool())
            .await
            .expect("MIGRATION_UP must create the declared table");

        db.run(Article::MIGRATION_DOWN).await;
        Article::assert_schema(db.pool())
            .await
            .expect_err("MIGRATION_DOWN must remove the table");

        db.run(Article::MIGRATION_UP).await;
        Article::assert_schema(db.pool())
            .await
            .expect("MIGRATION_UP must be repeatable after a down migration");

        db.teardown().await;
    }
}

/// Unique and case-insensitive lookup methods.
mod accounts {
    use entity_derive::Entity;
    use uuid::Uuid;

    use crate::pg;

    #[derive(Debug, Clone, Entity)]
    #[entity(table = "accounts", migrations)]
    pub struct Account {
        #[id]
        pub id: Uuid,

        #[field(create, response)]
        #[column(unique, ci)]
        pub email: String,

        #[field(create, update, response)]
        pub name: String
    }

    #[tokio::test]
    async fn lookups_are_case_insensitive_when_declared() {
        let Some(db) = pg::provision("lookup", &[Account::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let created = pool
            .create(CreateAccountRequest {
                email: "Ada@Example.COM".to_owned(),
                name:  "Ada".to_owned()
            })
            .await
            .expect("create failed");

        let exact = pool
            .find_by_email("Ada@Example.COM".to_owned())
            .await
            .expect("find_by_email failed")
            .expect("row missing for the exact spelling");
        assert_eq!(exact.id, created.id);

        let folded = pool
            .find_by_email("ada@example.com".to_owned())
            .await
            .expect("find_by_email failed")
            .expect("ci column must match a differently cased spelling");
        assert_eq!(folded.id, created.id);

        assert!(
            pool.exists_by_email("ADA@EXAMPLE.COM".to_owned())
                .await
                .expect("exists_by_email failed")
        );

        let duplicate = pool
            .create(CreateAccountRequest {
                email: "ADA@example.com".to_owned(),
                name:  "Impostor".to_owned()
            })
            .await;
        assert!(
            duplicate.is_err(),
            "the LOWER() unique index must reject a case variant"
        );

        db.teardown().await;
    }
}

/// Soft-delete lifecycle: hidden reads, restore, hard delete.
mod notes {
    use chrono::{DateTime, Utc};
    use entity_derive::Entity;
    use uuid::Uuid;

    use crate::pg;

    #[derive(Debug, Clone, Entity)]
    #[entity(table = "notes", soft_delete, migrations)]
    pub struct Note {
        #[id]
        pub id: Uuid,

        #[field(create, update, response)]
        pub title: String,

        #[field(skip)]
        pub deleted_at: Option<DateTime<Utc>>
    }

    #[tokio::test]
    async fn soft_delete_lifecycle() {
        let Some(db) = pg::provision("softdelete", &[Note::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let note = pool
            .create(CreateNoteRequest {
                title: "Draft".to_owned()
            })
            .await
            .expect("create failed");

        assert!(pool.delete(note.id).await.expect("soft delete failed"));
        assert!(
            pool.find_by_id(note.id)
                .await
                .expect("find_by_id failed")
                .is_none(),
            "soft-deleted rows must be hidden from find_by_id"
        );
        assert!(
            pool.list(10, 0).await.expect("list failed").is_empty(),
            "soft-deleted rows must be hidden from list"
        );
        assert!(
            pool.find_by_id_with_deleted(note.id)
                .await
                .expect("find_by_id_with_deleted failed")
                .is_some(),
            "the row must still exist physically"
        );

        assert!(pool.restore(note.id).await.expect("restore failed"));
        assert!(
            pool.find_by_id(note.id)
                .await
                .expect("find_by_id failed")
                .is_some(),
            "restore must bring the row back"
        );

        assert!(pool.hard_delete(note.id).await.expect("hard_delete failed"));
        assert!(
            pool.find_by_id_with_deleted(note.id)
                .await
                .expect("find_by_id_with_deleted failed")
                .is_none(),
            "hard_delete must remove the row"
        );

        db.teardown().await;
    }
}

/// Optimistic locking via the `#[version]` column.
mod orders {
    use entity_derive::Entity;
    use uuid::Uuid;

    use crate::pg;

    #[derive(Debug, Clone, Entity)]
    #[entity(table = "orders", migrations)]
    pub struct Order {
        #[id]
        pub id: Uuid,

        #[field(create, update, response)]
        pub note: String,

        #[version]
        #[field(response)]
        #[auto]
        pub version: i32
    }

    #[tokio::test]
    async fn stale_version_is_rejected() {
        let Some(db) = pg::provision("version", &[Order::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let order = pool
            .create(CreateOrderRequest {
                note: "first".to_owned()
            })
            .await
            .expect("create failed");
        assert_eq!(order.version, 0, "a fresh row starts at version 0");

        let bumped = pool
            .update(
                order.id,
                UpdateOrderRequest {
                    note:             Some("second".to_owned()),
                    expected_version: order.version
                }
            )
            .await
            .expect("update with the current version must succeed");
        assert_eq!(bumped.version, 1, "a successful update bumps the version");

        let stale = pool
            .update(
                order.id,
                UpdateOrderRequest {
                    note:             Some("third".to_owned()),
                    expected_version: order.version
                }
            )
            .await;
        assert!(
            stale.is_err(),
            "an update carrying a stale version must be rejected"
        );

        db.teardown().await;
    }
}
