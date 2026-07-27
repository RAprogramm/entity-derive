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

    #[tokio::test]
    async fn soft_deleted_rows_stay_out_of_every_read() {
        let Some(db) = pg::provision("softreads", &[Note::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let kept = pool
            .create(CreateNoteRequest {
                title: "kept".to_owned()
            })
            .await
            .expect("create failed");
        let removed = pool
            .create(CreateNoteRequest {
                title: "removed".to_owned()
            })
            .await
            .expect("create failed");

        assert!(pool.delete(removed.id).await.expect("soft delete failed"));

        let visible = pool.list(10, 0).await.expect("list failed");
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].id, kept.id);

        let everything = pool
            .list_with_deleted(10, 0)
            .await
            .expect("list_with_deleted failed");
        assert_eq!(
            everything.len(),
            2,
            "the deleted row must still be listable"
        );

        let by_ids = pool
            .find_by_ids(vec![kept.id, removed.id])
            .await
            .expect("find_by_ids failed");
        assert_eq!(
            by_ids.len(),
            1,
            "a bulk read must respect the soft delete as well"
        );

        let deleted_count = pool
            .delete_many(vec![kept.id])
            .await
            .expect("delete_many failed");
        assert_eq!(deleted_count, 1);
        assert!(
            pool.list(10, 0).await.expect("list failed").is_empty(),
            "the bulk delete must apply the soft delete too"
        );
        assert_eq!(
            pool.list_with_deleted(10, 0)
                .await
                .expect("list_with_deleted failed")
                .len(),
            2,
            "a soft bulk delete must not remove rows physically"
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

/// Relations: parent lookups, child lookups and the junction table
/// behind a many-to-many link.
mod relations {
    use entity_derive::Entity;
    use uuid::Uuid;

    use crate::pg;

    #[derive(Debug, Clone, Entity)]
    #[entity(table = "authors", migrations)]
    #[has_many(Book)]
    #[has_many(Genre, through = "author_genres")]
    pub struct Author {
        #[id]
        pub id: Uuid,

        #[field(create, update, response)]
        pub name: String
    }

    #[derive(Debug, Clone, Entity)]
    #[entity(table = "books", migrations)]
    pub struct Book {
        #[id]
        pub id: Uuid,

        #[belongs_to(Author)]
        #[field(create, response)]
        pub author_id: Uuid,

        #[field(create, update, response)]
        pub title: String
    }

    #[derive(Debug, Clone, Entity)]
    #[entity(table = "genres", migrations)]
    pub struct Genre {
        #[id]
        pub id: Uuid,

        #[field(create, update, response)]
        pub label: String
    }

    /// Every table plus the junction DDL the many-to-many link needs.
    fn migrations() -> Vec<&'static str> {
        let mut scripts = vec![
            Author::MIGRATION_UP,
            Genre::MIGRATION_UP,
            Book::MIGRATION_UP,
        ];
        scripts.extend_from_slice(Author::MIGRATION_JUNCTIONS);
        scripts
    }

    #[tokio::test]
    async fn parent_and_child_lookups() {
        let Some(db) = pg::provision("relations", &migrations()).await else {
            return;
        };
        let pool = db.pool();

        let author = AuthorRepository::create(
            pool,
            CreateAuthorRequest {
                name: "Ada".to_owned()
            }
        )
        .await
        .expect("create author failed");
        let book = BookRepository::create(
            pool,
            CreateBookRequest {
                author_id: author.id,
                title:     "Notes".to_owned()
            }
        )
        .await
        .expect("create book failed");

        let books = pool
            .find_books(author.id)
            .await
            .expect("has_many lookup failed");
        assert_eq!(books.len(), 1, "the author must own exactly one book");
        assert_eq!(books[0].id, book.id);

        let parent = pool
            .find_author(book.id)
            .await
            .expect("belongs_to lookup failed")
            .expect("the book must resolve its author");
        assert_eq!(parent.id, author.id);

        db.teardown().await;
    }

    #[tokio::test]
    async fn many_to_many_link_lifecycle() {
        let Some(db) = pg::provision("junction", &migrations()).await else {
            return;
        };
        let pool = db.pool();

        let author = AuthorRepository::create(
            pool,
            CreateAuthorRequest {
                name: "Grace".to_owned()
            }
        )
        .await
        .expect("create author failed");
        let genre = GenreRepository::create(
            pool,
            CreateGenreRequest {
                label: "essays".to_owned()
            }
        )
        .await
        .expect("create genre failed");

        assert!(
            !pool
                .has_genre(author.id, genre.id)
                .await
                .expect("has_ lookup failed"),
            "no link exists yet"
        );

        pool.add_genre(author.id, genre.id)
            .await
            .expect("add_ failed");
        assert!(
            pool.has_genre(author.id, genre.id)
                .await
                .expect("has_ lookup failed")
        );

        let linked = pool
            .find_genres(author.id)
            .await
            .expect("through lookup failed");
        assert_eq!(linked.len(), 1);
        assert_eq!(linked[0].id, genre.id);

        assert!(
            pool.remove_genre(author.id, genre.id)
                .await
                .expect("remove_ failed")
        );
        assert!(
            pool.find_genres(author.id)
                .await
                .expect("through lookup failed")
                .is_empty()
        );

        db.teardown().await;
    }
}

/// Ownership scoping: every scoped method must refuse rows owned by
/// somebody else.
mod scoping {
    use chrono::{DateTime, Utc};
    use entity_derive::Entity;
    use uuid::Uuid;

    use crate::pg;

    #[derive(Debug, Clone, Entity)]
    #[entity(table = "tickets", soft_delete, migrations)]
    pub struct Ticket {
        #[id]
        pub id: Uuid,

        #[owner]
        #[field(create, response)]
        pub owner_id: Uuid,

        #[field(create, update, response)]
        pub subject: String,

        #[field(skip)]
        pub deleted_at: Option<DateTime<Utc>>
    }

    #[tokio::test]
    async fn scoped_methods_refuse_another_owner() {
        let Some(db) = pg::provision("scoped", &[Ticket::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let mine = Uuid::now_v7();
        let theirs = Uuid::now_v7();

        let ticket = pool
            .create(CreateTicketRequest {
                owner_id: mine,
                subject:  "printer".to_owned()
            })
            .await
            .expect("create failed");

        assert!(
            pool.find_by_id_scoped(ticket.id, mine)
                .await
                .expect("scoped read failed")
                .is_some()
        );
        assert!(
            pool.find_by_id_scoped(ticket.id, theirs)
                .await
                .expect("scoped read failed")
                .is_none(),
            "another owner must not see the row"
        );

        let mine_only = pool
            .list_by_owner(mine, 10, 0)
            .await
            .expect("list_by_owner failed");
        assert_eq!(mine_only.len(), 1);
        assert!(
            pool.list_by_owner(theirs, 10, 0)
                .await
                .expect("list_by_owner failed")
                .is_empty()
        );

        assert!(
            pool.update_scoped(
                ticket.id,
                theirs,
                UpdateTicketRequest {
                    subject: Some("hijacked".to_owned())
                }
            )
            .await
            .expect("scoped update failed")
            .is_none(),
            "another owner must not update the row"
        );
        let updated = pool
            .update_scoped(
                ticket.id,
                mine,
                UpdateTicketRequest {
                    subject: Some("scanner".to_owned())
                }
            )
            .await
            .expect("scoped update failed")
            .expect("the owner must update the row");
        assert_eq!(updated.subject, "scanner");

        assert!(
            !pool
                .delete_scoped(ticket.id, theirs)
                .await
                .expect("scoped delete failed"),
            "another owner must not delete the row"
        );
        assert!(
            pool.delete_scoped(ticket.id, mine)
                .await
                .expect("scoped delete failed")
        );
        assert!(
            pool.find_by_id(ticket.id)
                .await
                .expect("read failed")
                .is_none(),
            "the scoped delete must apply the soft delete"
        );

        db.teardown().await;
    }
}

/// The transaction adapter and the aggregate-root `save()`, including
/// what a rollback must undo.
mod transactional {
    use entity_derive::Entity;
    use uuid::Uuid;

    use crate::pg;

    #[derive(Debug, Clone, Entity)]
    #[entity(
        table = "wallets",
        migrations,
        transactions,
        aggregate_root,
        upsert(conflict = "holder")
    )]
    pub struct Wallet {
        #[id]
        pub id: Uuid,

        #[field(create, response)]
        #[column(unique)]
        pub holder: String,

        #[field(create, update, response)]
        pub balance: i64
    }

    #[tokio::test]
    async fn adapter_writes_inside_one_transaction() {
        let Some(db) = pg::provision("tx", &[Wallet::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let mut tx = pool.begin().await.expect("begin failed");
        let mut repo = WalletTransactionRepo::new(&mut tx);
        let created = repo
            .create(CreateWalletRequest {
                holder:  "ada".to_owned(),
                balance: 100
            })
            .await
            .expect("transactional create failed");
        let updated = repo
            .update(
                created.id,
                UpdateWalletRequest {
                    balance: Some(250)
                }
            )
            .await
            .expect("transactional update failed");
        assert_eq!(updated.balance, 250);
        let merged = repo
            .upsert(CreateWalletRequest {
                holder:  "ada".to_owned(),
                balance: 400
            })
            .await
            .expect("transactional upsert failed");
        assert_eq!(merged.id, created.id, "the upsert must hit the same row");
        tx.commit().await.expect("commit failed");

        let stored = pool
            .find_by_id(created.id)
            .await
            .expect("read failed")
            .expect("the committed row must be visible");
        assert_eq!(stored.balance, 400);

        db.teardown().await;
    }

    #[tokio::test]
    async fn rollback_undoes_the_whole_unit() {
        let Some(db) = pg::provision("rollback", &[Wallet::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let mut tx = pool.begin().await.expect("begin failed");
        let mut repo = WalletTransactionRepo::new(&mut tx);
        let created = repo
            .create(CreateWalletRequest {
                holder:  "grace".to_owned(),
                balance: 10
            })
            .await
            .expect("transactional create failed");
        tx.rollback().await.expect("rollback failed");

        assert!(
            pool.find_by_id(created.id)
                .await
                .expect("read failed")
                .is_none(),
            "a rolled back write must leave nothing behind"
        );

        db.teardown().await;
    }

    #[tokio::test]
    async fn row_lock_blocks_a_concurrent_writer() {
        let Some(db) = pg::provision("rowlock", &[Wallet::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let wallet = pool
            .create(CreateWalletRequest {
                holder:  "linus".to_owned(),
                balance: 1
            })
            .await
            .expect("create failed");

        let mut holder = pool.begin().await.expect("begin failed");
        let mut repo = WalletTransactionRepo::new(&mut holder);
        let locked = repo
            .find_by_id_for_update(wallet.id)
            .await
            .expect("row lock failed")
            .expect("the row must be there to lock");
        assert_eq!(locked.id, wallet.id);

        let contender = pool.clone();
        let blocked = tokio::time::timeout(std::time::Duration::from_millis(300), async move {
            let mut tx = contender.begin().await.expect("begin failed");
            let mut repo = WalletTransactionRepo::new(&mut tx);
            repo.find_by_id_for_update(wallet.id).await
        })
        .await;
        assert!(
            blocked.is_err(),
            "a second FOR UPDATE must wait while the first transaction holds the row"
        );

        holder.rollback().await.expect("rollback failed");

        db.teardown().await;
    }

    #[tokio::test]
    async fn aggregate_root_save_persists() {
        let Some(db) = pg::provision("save", &[Wallet::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let saved = pool
            .save(NewWallet {
                holder:  "hopper".to_owned(),
                balance: 7
            })
            .await
            .expect("save failed");

        let stored = pool
            .find_by_id(saved.id)
            .await
            .expect("read failed")
            .expect("save must persist the row");
        assert_eq!(stored.holder, "hopper");
        assert_eq!(stored.balance, 7);

        db.teardown().await;
    }
}

/// Migration extras: extensions, triggers, indexes, checks and foreign
/// keys have to survive contact with the server, not just string
/// assertions.
mod migration_extras {
    use chrono::{DateTime, Utc};
    use entity_derive::Entity;
    use uuid::Uuid;

    use crate::pg;

    #[derive(Debug, Clone, Entity)]
    #[entity(
        table = "posts",
        migrations(touch_updated_at, audit),
        unique_index(space_id, slug)
    )]
    pub struct Post {
        #[id]
        pub id: Uuid,

        #[field(create, response)]
        pub space_id: Uuid,

        #[field(create, response)]
        pub slug: String,

        #[field(create, update, response)]
        #[filter(search)]
        pub title: String,

        #[field(create, update, response)]
        #[column(check = "score >= 0")]
        pub score: i32,

        #[field(create, update, response)]
        #[column(index = "gin")]
        pub tags: Vec<String>,

        #[field(response)]
        #[auto]
        pub updated_at: DateTime<Utc>
    }

    /// Extensions, then enum types, then the table, then triggers — the
    /// order a migration runner has to apply them in.
    fn migrations() -> Vec<&'static str> {
        let mut scripts = Vec::new();
        scripts.extend_from_slice(Post::MIGRATION_EXTENSIONS);
        scripts.push(Post::MIGRATION_UP);
        scripts.extend_from_slice(Post::MIGRATION_TRIGGERS);
        scripts
    }

    fn draft(slug: &str, title: &str, score: i32) -> CreatePostRequest {
        CreatePostRequest {
            space_id: Uuid::nil(),
            slug: slug.to_owned(),
            title: title.to_owned(),
            score,
            tags: vec!["rust".to_owned()]
        }
    }

    #[tokio::test]
    async fn declared_ddl_applies_and_holds() {
        let Some(db) = pg::provision("ddl", &migrations()).await else {
            return;
        };
        let pool = db.pool();

        let post = pool
            .create(draft("first", "Postgres in anger", 3))
            .await
            .expect("create failed");

        assert!(
            pool.create(draft("first", "Duplicate slug", 1))
                .await
                .is_err(),
            "the composite unique index must reject the duplicate pair"
        );

        assert!(
            pool.create(draft("second", "Negative", -1)).await.is_err(),
            "the check constraint must reject a negative score"
        );

        let hits = pool
            .query(PostQuery {
                title: Some("anger".to_owned()),
                ..Default::default()
            })
            .await
            .expect("trigram search failed");
        assert_eq!(hits.len(), 1, "the search filter must find the substring");

        let before = post.updated_at;
        pool.update(
            post.id,
            UpdatePostRequest {
                title: Some("Postgres, calmly".to_owned()),
                score: Some(4),
                tags:  Some(vec!["rust".to_owned(), "sql".to_owned()])
            }
        )
        .await
        .expect("update failed");
        let after = pool
            .find_by_id(post.id)
            .await
            .expect("read failed")
            .expect("row missing")
            .updated_at;
        assert!(
            after > before,
            "the touch_updated_at trigger must move the timestamp: {before} -> {after}"
        );

        let audited: i64 =
            sqlx::query_scalar("SELECT count(*) FROM entity_audit_log WHERE table_name = 'posts'")
                .fetch_one(pool)
                .await
                .expect("audit table missing");
        assert!(
            audited >= 2,
            "the audit trigger must have recorded the insert and the update, got {audited}"
        );

        db.teardown().await;
    }
}

/// Typed constraint errors: a violation has to arrive as the declared
/// error type with the field named, not as an opaque database error.
mod typed_constraints {
    use entity_derive::{ConstraintError, Entity};
    use uuid::Uuid;

    use crate::pg;

    #[derive(Debug)]
    pub enum ShopError {
        Database(sqlx::Error),
        Constraint(ConstraintError)
    }

    impl std::fmt::Display for ShopError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Database(e) => write!(f, "database error: {e}"),
                Self::Constraint(e) => write!(f, "constraint violation: {e}")
            }
        }
    }

    impl std::error::Error for ShopError {}

    impl From<sqlx::Error> for ShopError {
        fn from(e: sqlx::Error) -> Self {
            Self::Database(e)
        }
    }

    impl From<ConstraintError> for ShopError {
        fn from(e: ConstraintError) -> Self {
            Self::Constraint(e)
        }
    }

    #[derive(Debug, Clone, Entity)]
    #[entity(
        table = "customers",
        migrations,
        typed_constraints,
        error = "ShopError"
    )]
    pub struct Customer {
        #[id]
        pub id: Uuid,

        #[field(create, response)]
        #[column(unique)]
        pub email: String,

        #[field(create, update, response)]
        pub name: String
    }

    #[tokio::test]
    async fn unique_violation_arrives_typed() {
        let Some(db) = pg::provision("constraints", &[Customer::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        pool.create(CreateCustomerRequest {
            email: "ada@example.com".to_owned(),
            name:  "Ada".to_owned()
        })
        .await
        .expect("create failed");

        let duplicate = pool
            .create(CreateCustomerRequest {
                email: "ada@example.com".to_owned(),
                name:  "Impostor".to_owned()
            })
            .await;

        match duplicate {
            Err(ShopError::Constraint(violation)) => {
                assert_eq!(
                    violation.field,
                    Some("email"),
                    "the violation must name the column that collided"
                );
            }
            Err(other) => panic!("expected a typed constraint violation, got {other}"),
            Ok(_) => panic!("the unique index must reject the duplicate")
        }

        db.teardown().await;
    }
}

/// Postgres enum columns and embedded value objects: both change what
/// the DDL and the row mapping look like.
mod column_shapes {
    use entity_derive::{Entity, ValueObject};
    use uuid::Uuid;

    use crate::pg;

    #[derive(
        ValueObject,
        Debug,
        Clone,
        PartialEq,
        Eq,
        utoipa::ToSchema,
        serde::Serialize,
        serde::Deserialize,
    )]
    #[value_object(pg_type = "shipment_status", sqlx)]
    pub enum ShipmentStatus {
        Pending,
        Shipped,
        Delivered
    }

    #[derive(Debug, Clone, PartialEq, utoipa::ToSchema, serde::Serialize, serde::Deserialize)]
    pub struct Money {
        pub amount_cents: i64,
        pub currency:     String
    }

    #[derive(Debug, Clone, Entity)]
    #[entity(table = "shipments", migrations)]
    pub struct Shipment {
        #[id]
        pub id: Uuid,

        #[field(create, update, response)]
        #[column(pg_enum = "shipment_status")]
        pub status: ShipmentStatus,

        #[field(create, update, response)]
        #[embed(prefix = "cost_", fields(amount_cents: i64, currency: String))]
        pub cost: Money
    }

    fn migrations() -> Vec<&'static str> {
        let mut scripts = Vec::new();
        scripts.extend_from_slice(Shipment::MIGRATION_TYPES);
        scripts.push(Shipment::MIGRATION_UP);
        scripts
    }

    #[tokio::test]
    async fn enum_and_embedded_columns_round_trip() {
        let Some(db) = pg::provision("shapes", &migrations()).await else {
            return;
        };
        let pool = db.pool();

        let created = pool
            .create(CreateShipmentRequest {
                status: ShipmentStatus::Pending,
                cost:   Money {
                    amount_cents: 1999,
                    currency:     "EUR".to_owned()
                }
            })
            .await
            .expect("create failed");
        assert_eq!(created.status, ShipmentStatus::Pending);
        assert_eq!(created.cost.amount_cents, 1999);

        let updated = pool
            .update(
                created.id,
                UpdateShipmentRequest {
                    status: Some(ShipmentStatus::Delivered),
                    cost:   Some(Money {
                        amount_cents: 2500,
                        currency:     "USD".to_owned()
                    })
                }
            )
            .await
            .expect("update failed");
        assert_eq!(updated.status, ShipmentStatus::Delivered);
        assert_eq!(updated.cost.currency, "USD");

        let stored = pool
            .find_by_id(created.id)
            .await
            .expect("read failed")
            .expect("row missing");
        assert_eq!(stored.status, ShipmentStatus::Delivered);
        assert_eq!(stored.cost.amount_cents, 2500);

        let native: String =
            sqlx::query_scalar("SELECT status::text FROM shipments WHERE id = $1")
                .bind(created.id)
                .fetch_one(pool)
                .await
                .expect("the column must be the declared enum type");
        assert_eq!(native, "delivered");

        db.teardown().await;
    }
}

/// The transactional outbox: a write and its outbox row must land in
/// the same transaction.
mod outbox {
    use entity_derive::Entity;
    use uuid::Uuid;

    use crate::pg;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Entity)]
    #[entity(table = "invoices", migrations, events(outbox))]
    pub struct Invoice {
        #[id]
        pub id: Uuid,

        #[field(create, update, response)]
        pub number: String
    }

    #[tokio::test]
    async fn writes_enqueue_their_event() {
        let Some(db) = pg::provision(
            "outbox",
            &[Invoice::MIGRATION_UP, Invoice::MIGRATION_OUTBOX]
        )
        .await
        else {
            return;
        };
        let pool = db.pool();

        let invoice = pool
            .create(CreateInvoiceRequest {
                number: "INV-1".to_owned()
            })
            .await
            .expect("create failed");

        let (kind, entity_id): (String, String) = sqlx::query_as(
            "SELECT kind, entity_id FROM entity_outbox WHERE entity = 'invoices' ORDER BY id"
        )
        .fetch_one(pool)
        .await
        .expect("the create must have enqueued an outbox row");
        assert_eq!(entity_id, invoice.id.to_string());
        assert_eq!(kind.to_lowercase(), "created");

        db.teardown().await;
    }

    /// Records what it was handed, and fails on demand.
    struct Recorder {
        seen: std::sync::Mutex<Vec<String>>,
        fail: bool
    }

    #[entity_derive::async_trait]
    impl entity_derive::outbox::OutboxHandler for Recorder {
        type Error = String;

        async fn handle(&self, row: &entity_derive::outbox::OutboxRow) -> Result<(), Self::Error> {
            self.seen
                .lock()
                .expect("the recorder lock is never poisoned")
                .push(row.entity_id.clone());
            if self.fail {
                Err("handler refused".to_owned())
            } else {
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn the_drainer_claims_delivers_and_retries() {
        let Some(db) = pg::provision(
            "drainer",
            &[Invoice::MIGRATION_UP, Invoice::MIGRATION_OUTBOX]
        )
        .await
        else {
            return;
        };
        let pool = db.pool();

        let invoice = pool
            .create(CreateInvoiceRequest {
                number: "INV-2".to_owned()
            })
            .await
            .expect("create failed");

        let failing = entity_derive::outbox::OutboxDrainer::new(
            pool.clone(),
            Recorder {
                seen: std::sync::Mutex::new(Vec::new()),
                fail: true
            }
        );
        let claimed = failing.drain_once().await.expect("drain failed");
        assert_eq!(claimed, 1, "the pending row must be claimed");

        let (attempts, processed): (i32, Option<chrono::DateTime<chrono::Utc>>) =
            sqlx::query_as("SELECT attempts, processed_at FROM entity_outbox")
                .fetch_one(pool)
                .await
                .expect("read failed");
        assert_eq!(attempts, 1, "a failed delivery must count an attempt");
        assert!(
            processed.is_none(),
            "a failed delivery must stay pending for the retry"
        );

        sqlx::query("UPDATE entity_outbox SET next_attempt_at = NOW()")
            .execute(pool)
            .await
            .expect("rescheduling for the test failed");

        let recorder = Recorder {
            seen: std::sync::Mutex::new(Vec::new()),
            fail: false
        };
        let succeeding = entity_derive::outbox::OutboxDrainer::new(pool.clone(), recorder);
        assert_eq!(
            succeeding.drain_once().await.expect("drain failed"),
            1,
            "the retry must claim the row again"
        );

        let processed: Option<chrono::DateTime<chrono::Utc>> =
            sqlx::query_scalar("SELECT processed_at FROM entity_outbox")
                .fetch_one(pool)
                .await
                .expect("read failed");
        assert!(
            processed.is_some(),
            "a successful delivery must mark the row processed"
        );

        assert_eq!(
            entity_derive::outbox::OutboxDrainer::new(
                pool.clone(),
                Recorder {
                    seen: std::sync::Mutex::new(Vec::new()),
                    fail: false
                }
            )
            .drain_once()
            .await
            .expect("drain failed"),
            0,
            "a processed row must not be claimed twice"
        );

        let _ = invoice;
        db.teardown().await;
    }
}

/// Streams: a write has to publish its event on the entity channel, in
/// the same transaction that wrote the row.
mod streams {
    use entity_derive::Entity;
    use sqlx::postgres::PgListener;
    use uuid::Uuid;

    use crate::pg;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Entity)]
    #[entity(table = "alerts", migrations, events, streams)]
    pub struct Alert {
        #[id]
        pub id: Uuid,

        #[field(create, update, response)]
        pub message: String
    }

    #[tokio::test]
    async fn writes_notify_the_channel() {
        let Some(db) = pg::provision("streams", &[Alert::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let mut listener = PgListener::connect_with(pool)
            .await
            .expect("listener connect failed");
        listener
            .listen(Alert::CHANNEL)
            .await
            .expect("LISTEN failed");

        let alert = pool
            .create(CreateAlertRequest {
                message: "disk full".to_owned()
            })
            .await
            .expect("create failed");

        let notification =
            tokio::time::timeout(std::time::Duration::from_secs(5), listener.recv())
                .await
                .expect("no notification arrived within five seconds")
                .expect("listener failed");

        let event: AlertEvent = serde_json::from_str(notification.payload())
            .expect("the payload must deserialize into the generated event");
        match event {
            AlertEvent::Created(created) => assert_eq!(created.id, alert.id),
            other => panic!("expected a Created event, got {other:?}")
        }

        // The listener holds a pooled connection; teardown closes the
        // pool and would wait for it.
        drop(listener);
        db.teardown().await;
    }

    #[tokio::test]
    async fn the_generated_subscriber_receives_events() {
        let Some(db) = pg::provision("subscriber", &[Alert::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let mut subscriber = AlertSubscriber::new(pool)
            .await
            .expect("subscriber connect failed");

        let alert = pool
            .create(CreateAlertRequest {
                message: "battery low".to_owned()
            })
            .await
            .expect("create failed");

        let event = tokio::time::timeout(std::time::Duration::from_secs(5), subscriber.recv())
            .await
            .expect("no event arrived within five seconds")
            .expect("the subscriber must decode the payload");
        match event {
            AlertEvent::Created(created) => assert_eq!(created.id, alert.id),
            other => panic!("expected a Created event, got {other:?}")
        }

        drop(subscriber);
        db.teardown().await;
    }
}

/// Returning modes decide what comes back from a write, and each mode
/// builds a different statement.
mod returning_modes {
    use entity_derive::Entity;
    use uuid::Uuid;

    use crate::pg;

    #[derive(Debug, Clone, Entity)]
    #[entity(table = "id_only", migrations, returning = "id")]
    pub struct IdOnly {
        #[id]
        pub id: Uuid,

        #[field(create, update, response)]
        pub label: String
    }

    #[derive(Debug, Clone, Entity)]
    #[entity(table = "no_returning", migrations, returning = "none")]
    pub struct NoReturning {
        #[id]
        pub id: Uuid,

        #[field(create, update, response)]
        pub label: String
    }

    #[tokio::test]
    async fn id_mode_returns_the_row_it_wrote() {
        let Some(db) = pg::provision("ret_id", &[IdOnly::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let created = IdOnlyRepository::create(
            pool,
            CreateIdOnlyRequest {
                label: "alpha".to_owned()
            }
        )
        .await
        .expect("create failed");

        let stored = IdOnlyRepository::find_by_id(pool, created.id)
            .await
            .expect("read failed")
            .expect("the write must have landed");
        assert_eq!(stored.label, "alpha");

        db.teardown().await;
    }

    #[tokio::test]
    async fn none_mode_still_writes() {
        let Some(db) = pg::provision("ret_none", &[NoReturning::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let created = NoReturningRepository::create(
            pool,
            CreateNoReturningRequest {
                label: "beta".to_owned()
            }
        )
        .await
        .expect("create failed");

        let stored = NoReturningRepository::find_by_id(pool, created.id)
            .await
            .expect("read failed")
            .expect("a write with no RETURNING must still persist the row");
        assert_eq!(stored.label, "beta");

        db.teardown().await;
    }
}

/// Joined read models: the generated `SELECT` spans several tables, so
/// a wrong alias or a missing column only shows up when it runs.
mod join_views {
    use entity_derive::Entity;
    use uuid::Uuid;

    use crate::pg;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Entity)]
    #[join(airports as origin, on = origin_iata = iata, fields(
        city as origin_city: String
    ))]
    #[join(airports as dest, on = destination_iata = iata, fields(
        city as destination_city: String
    ))]
    #[entity(table = "tickets", migrations)]
    pub struct Ticket {
        #[id]
        pub id: Uuid,

        #[field(create, response)]
        pub origin_iata: String,

        #[field(create, response)]
        pub destination_iata: String
    }

    /// The joined table is not an entity, so its DDL is written by hand
    /// exactly as a user would write it.
    const AIRPORTS: &str = "CREATE TABLE airports (iata TEXT PRIMARY KEY, city TEXT NOT NULL);\n\
                            INSERT INTO airports (iata, city) VALUES \
                            ('TLL', 'Tallinn'), ('HEL', 'Helsinki');";

    #[tokio::test]
    async fn view_joins_both_sides() {
        let Some(db) = pg::provision("joins", &[AIRPORTS, Ticket::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let ticket = pool
            .create(CreateTicketRequest {
                origin_iata:      "TLL".to_owned(),
                destination_iata: "HEL".to_owned()
            })
            .await
            .expect("create failed");

        let view = TicketView::find_by_id(pool, ticket.id)
            .await
            .expect("the joined SELECT must execute")
            .expect("the row must resolve through both joins");
        assert_eq!(view.origin_city, "Tallinn");
        assert_eq!(view.destination_city, "Helsinki");
        assert_eq!(view.origin_iata, "TLL");

        let page = TicketView::list(pool, 10, 0)
            .await
            .expect("the joined list must execute");
        assert_eq!(page.len(), 1);
        assert_eq!(page[0].destination_city, "Helsinki");

        db.teardown().await;
    }

    #[tokio::test]
    async fn inner_join_drops_rows_without_a_match() {
        let Some(db) = pg::provision("joinmiss", &[AIRPORTS, Ticket::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let orphan = pool
            .create(CreateTicketRequest {
                origin_iata:      "TLL".to_owned(),
                destination_iata: "XXX".to_owned()
            })
            .await
            .expect("create failed");

        assert!(
            TicketView::find_by_id(pool, orphan.id)
                .await
                .expect("the joined SELECT must execute")
                .is_none(),
            "an INNER JOIN must drop the row whose destination has no airport"
        );

        db.teardown().await;
    }
}

/// The policy wrapper is an authorization boundary: a denial has to
/// stop the write, not just be recorded.
mod policy {
    use entity_derive::{Entity, async_trait};
    use uuid::Uuid;

    use crate::pg;

    #[derive(Debug, Clone, Entity)]
    #[entity(table = "documents", migrations, policy)]
    pub struct Document {
        #[id]
        pub id: Uuid,

        #[field(create, response)]
        pub owner_id: Uuid,

        #[field(create, update, response)]
        pub title: String
    }

    /// Denies everything an admin is not allowed to do.
    struct OwnerOnly;

    /// Who is asking.
    struct Caller {
        user_id: Uuid
    }

    #[derive(Debug)]
    struct Denied;

    impl std::fmt::Display for Denied {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("not allowed")
        }
    }

    impl std::error::Error for Denied {}

    #[async_trait]
    impl DocumentPolicy for OwnerOnly {
        type Context = Caller;
        type Error = Denied;

        async fn can_create(
            &self,
            dto: &CreateDocumentRequest,
            ctx: &Self::Context
        ) -> Result<(), Self::Error> {
            if dto.owner_id == ctx.user_id {
                Ok(())
            } else {
                Err(Denied)
            }
        }

        async fn can_read(&self, _id: &Uuid, _ctx: &Self::Context) -> Result<(), Self::Error> {
            Ok(())
        }

        async fn can_update(
            &self,
            _id: &Uuid,
            _dto: &UpdateDocumentRequest,
            _ctx: &Self::Context
        ) -> Result<(), Self::Error> {
            Err(Denied)
        }

        async fn can_delete(&self, _id: &Uuid, _ctx: &Self::Context) -> Result<(), Self::Error> {
            Err(Denied)
        }

        async fn can_list(&self, _ctx: &Self::Context) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn denied_operations_never_reach_the_database() {
        let Some(db) = pg::provision("policy", &[Document::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let owner = Uuid::now_v7();
        let guarded = DocumentPolicyRepository::new(pool.clone(), OwnerOnly);
        let ctx = Caller {
            user_id: owner
        };

        let refused = guarded
            .create(
                CreateDocumentRequest {
                    owner_id: Uuid::now_v7(),
                    title:    "somebody else's".to_owned()
                },
                &ctx
            )
            .await;
        assert!(refused.is_err(), "the policy must refuse the create");
        assert!(
            pool.list(10, 0).await.expect("list failed").is_empty(),
            "a refused create must not have written anything"
        );

        let allowed = guarded
            .create(
                CreateDocumentRequest {
                    owner_id: owner,
                    title:    "mine".to_owned()
                },
                &ctx
            )
            .await
            .expect("the policy must allow the owner's create");
        assert_eq!(pool.list(10, 0).await.expect("list failed").len(), 1);

        assert!(
            guarded
                .update(
                    allowed.id,
                    UpdateDocumentRequest {
                        title: Some("renamed".to_owned())
                    },
                    &ctx
                )
                .await
                .is_err(),
            "the policy must refuse the update"
        );
        let unchanged = pool
            .find_by_id(allowed.id)
            .await
            .expect("read failed")
            .expect("row missing");
        assert_eq!(
            unchanged.title, "mine",
            "a refused update must leave the row alone"
        );

        assert!(
            guarded.delete(allowed.id, &ctx).await.is_err(),
            "the policy must refuse the delete"
        );
        assert!(
            pool.find_by_id(allowed.id)
                .await
                .expect("read failed")
                .is_some(),
            "a refused delete must leave the row in place"
        );

        db.teardown().await;
    }
}

/// The generated HTTP layer, driven over the generated router with a
/// real repository behind it.
mod http {
    use std::sync::Arc;

    use axum::{
        body::Body,
        http::{Request, StatusCode}
    };
    use entity_derive::Entity;
    use tower::ServiceExt;
    use uuid::Uuid;

    use crate::pg;

    #[derive(Debug, Clone, Entity)]
    #[entity(table = "gadgets", migrations, api(tag = "Gadgets", handlers))]
    pub struct Gadget {
        #[id]
        pub id: Uuid,

        #[field(create, update, response)]
        pub name: String,

        #[field(create, update, response)]
        pub weight: i32
    }

    /// Send one request through the generated router and return the
    /// status together with the body.
    async fn call(pool: &sqlx::PgPool, request: Request<Body>) -> (StatusCode, serde_json::Value) {
        let app = gadget_router::<sqlx::PgPool>().with_state(Arc::new(pool.clone()));
        let response = app.oneshot(request).await.expect("the router must respond");
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), 1 << 20)
            .await
            .expect("body read failed");
        let body = if bytes.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_slice(&bytes).expect("the response must be JSON")
        };
        (status, body)
    }

    fn json_request(method: &str, uri: &str, body: serde_json::Value) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("request build failed")
    }

    #[tokio::test]
    async fn crud_endpoints_answer_over_http() {
        let Some(db) = pg::provision("http", &[Gadget::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let (status, created) = call(
            pool,
            json_request(
                "POST",
                "/gadgets",
                serde_json::json!({
                    "name": "spanner",
                    "weight": 3
                })
            )
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "a create must answer 201");
        assert_eq!(created["name"], "spanner");
        let id = created["id"].as_str().expect("the response carries the id");

        let (status, fetched) = call(
            pool,
            Request::builder()
                .uri(format!("/gadgets/{id}"))
                .body(Body::empty())
                .expect("request build failed")
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(fetched["weight"], 3);

        let (status, listed) = call(
            pool,
            Request::builder()
                .uri("/gadgets?limit=10&offset=0")
                .body(Body::empty())
                .expect("request build failed")
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            listed.as_array().map(Vec::len),
            Some(1),
            "the list endpoint must honour its pagination parameters"
        );

        let (status, updated) = call(
            pool,
            json_request(
                "PATCH",
                &format!("/gadgets/{id}"),
                serde_json::json!({ "weight": 5 })
            )
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(updated["weight"], 5);
        assert_eq!(
            updated["name"], "spanner",
            "a PATCH must leave the omitted field alone"
        );

        let (status, _) = call(
            pool,
            Request::builder()
                .method("DELETE")
                .uri(format!("/gadgets/{id}"))
                .body(Body::empty())
                .expect("request build failed")
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT, "a delete must answer 204");

        let (status, _) = call(
            pool,
            Request::builder()
                .uri(format!("/gadgets/{id}"))
                .body(Body::empty())
                .expect("request build failed")
        )
        .await;
        assert_eq!(
            status,
            StatusCode::NOT_FOUND,
            "reading a deleted row must answer 404"
        );

        db.teardown().await;
    }

    #[tokio::test]
    async fn unknown_id_answers_not_found() {
        let Some(db) = pg::provision("http404", &[Gadget::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let (status, _) = call(
            pool,
            Request::builder()
                .uri(format!("/gadgets/{}", Uuid::now_v7()))
                .body(Body::empty())
                .expect("request build failed")
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        db.teardown().await;
    }
}

/// Declarative state transitions: the guard is SQL plus a status check
/// under a row lock, so only a real run proves it.
mod transitions {
    use entity_derive::{Entity, ValueObject};
    use uuid::Uuid;

    use crate::pg;

    #[derive(
        ValueObject,
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        utoipa::ToSchema,
        serde::Serialize,
        serde::Deserialize,
    )]
    #[value_object(pg_type = "parcel_status", sqlx)]
    pub enum ParcelStatus {
        Created,
        Accepted,
        Cancelled
    }

    /// The transition guard reports a typed failure, so the entity has
    /// to declare an error type that can carry it.
    #[derive(Debug)]
    pub enum ParcelError {
        Database(sqlx::Error),
        Transition(entity_derive::TransitionError)
    }

    impl std::fmt::Display for ParcelError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Database(e) => write!(f, "database error: {e}"),
                Self::Transition(e) => write!(f, "transition refused: {e}")
            }
        }
    }

    impl std::error::Error for ParcelError {}

    impl From<sqlx::Error> for ParcelError {
        fn from(e: sqlx::Error) -> Self {
            Self::Database(e)
        }
    }

    impl From<entity_derive::TransitionError> for ParcelError {
        fn from(e: entity_derive::TransitionError) -> Self {
            Self::Transition(e)
        }
    }

    #[derive(Debug, Clone, Entity)]
    #[entity(table = "parcels", migrations, transactions, error = "ParcelError")]
    #[transition(created -> accepted, sets(courier_id))]
    #[transition(created | accepted -> cancelled)]
    pub struct Parcel {
        #[id]
        pub id: Uuid,

        #[field(create, update, response)]
        #[column(pg_enum = "parcel_status")]
        pub status: ParcelStatus,

        #[field(create, update, response)]
        pub courier_id: Option<Uuid>
    }

    fn migrations() -> Vec<&'static str> {
        let mut scripts = Vec::new();
        scripts.extend_from_slice(Parcel::MIGRATION_TYPES);
        scripts.push(Parcel::MIGRATION_UP);
        scripts
    }

    #[tokio::test]
    async fn allowed_transition_patches_declared_columns() {
        let Some(db) = pg::provision("transition", &migrations()).await else {
            return;
        };
        let pool = db.pool();

        let parcel = pool
            .create(CreateParcelRequest {
                status:     ParcelStatus::Created,
                courier_id: None
            })
            .await
            .expect("create failed");

        let courier = Uuid::now_v7();
        let mut tx = pool.begin().await.expect("begin failed");
        let mut repo = ParcelTransactionRepo::new(&mut tx);
        let accepted = repo
            .transition_to_accepted(parcel.id, courier)
            .await
            .expect("the declared transition must be allowed")
            .expect("the row must exist");
        tx.commit().await.expect("commit failed");

        assert_eq!(accepted.status, ParcelStatus::Accepted);
        assert_eq!(
            accepted.courier_id,
            Some(courier),
            "the transition must patch the columns it declares"
        );

        db.teardown().await;
    }

    #[tokio::test]
    async fn disallowed_source_status_is_refused() {
        let Some(db) = pg::provision("transbad", &migrations()).await else {
            return;
        };
        let pool = db.pool();

        let parcel = pool
            .create(CreateParcelRequest {
                status:     ParcelStatus::Cancelled,
                courier_id: None
            })
            .await
            .expect("create failed");

        let mut tx = pool.begin().await.expect("begin failed");
        let mut repo = ParcelTransactionRepo::new(&mut tx);
        let refused = repo.transition_to_accepted(parcel.id, Uuid::now_v7()).await;
        assert!(
            refused.is_err(),
            "a cancelled parcel must not become accepted"
        );
        tx.rollback().await.expect("rollback failed");

        let unchanged = pool
            .find_by_id(parcel.id)
            .await
            .expect("read failed")
            .expect("row missing");
        assert_eq!(
            unchanged.status,
            ParcelStatus::Cancelled,
            "a refused transition must leave the status alone"
        );

        db.teardown().await;
    }

    #[tokio::test]
    async fn a_missing_row_is_not_an_error() {
        let Some(db) = pg::provision("transnone", &migrations()).await else {
            return;
        };
        let pool = db.pool();

        let mut tx = pool.begin().await.expect("begin failed");
        let mut repo = ParcelTransactionRepo::new(&mut tx);
        let outcome = repo
            .transition_to_cancelled(Uuid::now_v7())
            .await
            .expect("a missing row must not be reported as a transition failure");
        assert!(outcome.is_none());
        tx.rollback().await.expect("rollback failed");

        db.teardown().await;
    }
}

/// HTTP guards and the generated OpenAPI document.
mod http_guard {
    use std::sync::Arc;

    use axum::{
        body::Body,
        extract::FromRequestParts,
        http::{Request, StatusCode, request::Parts}
    };
    use entity_derive::Entity;
    use tower::ServiceExt;
    use uuid::Uuid;

    use crate::pg;

    /// Accepts a request only when it carries an authorization header.
    pub struct RequireAuth;

    impl<S> FromRequestParts<S> for RequireAuth
    where
        S: Send + Sync
    {
        type Rejection = StatusCode;

        fn from_request_parts(
            parts: &mut Parts,
            _state: &S
        ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
            let authenticated = parts.headers.contains_key("authorization");
            async move {
                if authenticated {
                    Ok(Self)
                } else {
                    Err(StatusCode::UNAUTHORIZED)
                }
            }
        }
    }

    #[derive(Debug, Clone, Entity)]
    #[entity(
        table = "vaults",
        migrations,
        api(tag = "Vaults", handlers, guard = "RequireAuth", guard(list = "none"))
    )]
    pub struct Vault {
        #[id]
        pub id: Uuid,

        #[field(create, update, response)]
        pub label: String
    }

    async fn status_of(pool: &sqlx::PgPool, request: Request<Body>) -> StatusCode {
        let app = vault_router::<sqlx::PgPool>().with_state(Arc::new(pool.clone()));
        app.oneshot(request)
            .await
            .expect("the router must respond")
            .status()
    }

    #[tokio::test]
    async fn the_guard_rejects_and_the_exempt_route_stays_open() {
        let Some(db) = pg::provision("guard", &[Vault::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let anonymous_create = Request::builder()
            .method("POST")
            .uri("/vaults")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"label":"secrets"}"#))
            .expect("request build failed");
        assert_eq!(
            status_of(pool, anonymous_create).await,
            StatusCode::UNAUTHORIZED,
            "the guard must reject a request without credentials"
        );
        assert!(
            pool.list(10, 0).await.expect("list failed").is_empty(),
            "a rejected request must not have written anything"
        );

        let authenticated_create = Request::builder()
            .method("POST")
            .uri("/vaults")
            .header("content-type", "application/json")
            .header("authorization", "Bearer token")
            .body(Body::from(r#"{"label":"secrets"}"#))
            .expect("request build failed");
        assert_eq!(
            status_of(pool, authenticated_create).await,
            StatusCode::CREATED,
            "the guard must let an authenticated request through"
        );

        let anonymous_list = Request::builder()
            .uri("/vaults")
            .body(Body::empty())
            .expect("request build failed");
        assert_eq!(
            status_of(pool, anonymous_list).await,
            StatusCode::OK,
            "the route exempted with guard(list = \"none\") must stay open"
        );

        db.teardown().await;
    }

    #[test]
    fn the_openapi_document_describes_the_routes() {
        use utoipa::OpenApi;

        let document = VaultApi::openapi();
        let json = serde_json::to_value(&document).expect("the document must serialize");

        let paths = json["paths"]
            .as_object()
            .expect("the document must declare paths");
        assert!(
            paths.contains_key("/vaults"),
            "the collection path must be documented, got {:?}",
            paths.keys().collect::<Vec<_>>()
        );
        assert!(
            paths.contains_key("/vaults/{id}"),
            "the item path must be documented"
        );
        assert!(
            paths["/vaults"].get("post").is_some(),
            "the create operation must be documented"
        );

        let schemas = json["components"]["schemas"]
            .as_object()
            .expect("the document must declare schemas");
        for expected in ["VaultResponse", "CreateVaultRequest", "UpdateVaultRequest"] {
            assert!(
                schemas.contains_key(expected),
                "{expected} must be in the document, got {:?}",
                schemas.keys().collect::<Vec<_>>()
            );
        }
    }
}

/// CQRS commands: the dispatcher routes a variant to its handler, and
/// the generated route carries a command over HTTP.
mod commands {
    use std::sync::Arc;

    use axum::{
        body::Body,
        http::{Request, StatusCode}
    };
    use entity_derive::{Entity, async_trait};
    use tower::ServiceExt;
    use uuid::Uuid;

    use crate::pg;

    #[derive(Debug, Clone, Entity)]
    #[entity(
        table = "members",
        migrations,
        commands,
        api(tag = "Members", handlers)
    )]
    #[command(Register)]
    #[command(Rename: nickname, requires_id)]
    pub struct Member {
        #[id]
        pub id: Uuid,

        #[field(create, update, response)]
        pub nickname: String
    }

    /// Handles commands by writing through the generated repository.
    struct Handlers {
        pool: sqlx::PgPool
    }

    #[async_trait]
    impl MemberCommandHandler for Handlers {
        type Context = ();
        type Error = sqlx::Error;

        async fn handle_register(
            &self,
            cmd: RegisterMember,
            _ctx: &Self::Context
        ) -> Result<Member, Self::Error> {
            self.pool
                .create(CreateMemberRequest {
                    nickname: cmd.nickname
                })
                .await
        }

        async fn handle_rename(
            &self,
            cmd: RenameMember,
            _ctx: &Self::Context
        ) -> Result<Member, Self::Error> {
            self.pool
                .update(
                    cmd.id,
                    UpdateMemberRequest {
                        nickname: Some(cmd.nickname)
                    }
                )
                .await
        }
    }

    #[tokio::test]
    async fn the_dispatcher_routes_each_variant() {
        let Some(db) = pg::provision("commands", &[Member::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();
        let handlers = Handlers {
            pool: pool.clone()
        };

        let registered = handlers
            .handle(
                MemberCommand::Register(RegisterMember {
                    nickname: "ada".to_owned()
                }),
                &()
            )
            .await
            .expect("the dispatcher must route Register");
        let MemberCommandResult::Register(member) = registered else {
            panic!("the result variant must match the command")
        };
        assert_eq!(member.nickname, "ada");

        let renamed = handlers
            .handle(
                MemberCommand::Rename(RenameMember {
                    id:       member.id,
                    nickname: "grace".to_owned()
                }),
                &()
            )
            .await
            .expect("the dispatcher must route Rename");
        let MemberCommandResult::Rename(updated) = renamed else {
            panic!("the result variant must match the command")
        };
        assert_eq!(updated.nickname, "grace");

        let stored = pool
            .find_by_id(member.id)
            .await
            .expect("read failed")
            .expect("row missing");
        assert_eq!(
            stored.nickname, "grace",
            "the handler's write must have reached the database"
        );

        db.teardown().await;
    }

    #[tokio::test]
    async fn a_command_route_answers_over_http() {
        let Some(db) = pg::provision("cmdhttp", &[Member::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        // Command handlers take their dependency as an extension, not
        // as router state.
        let app =
            member_commands_router::<Handlers>().layer(axum::Extension(Arc::new(Handlers {
                pool: pool.clone()
            })));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/members/register")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"nickname":"hopper"}"#))
                    .expect("request build failed")
            )
            .await
            .expect("the router must respond");
        assert_eq!(response.status(), StatusCode::OK);

        assert_eq!(
            pool.list(10, 0).await.expect("list failed").len(),
            1,
            "the command handler must have written the row"
        );

        let body = axum::body::to_bytes(response.into_body(), 1 << 20)
            .await
            .expect("body read failed");
        let json: serde_json::Value =
            serde_json::from_slice(&body).expect("the response must be JSON");
        assert_eq!(
            json["nickname"], "hopper",
            "a command route answers with the response shape, not the raw entity"
        );

        db.teardown().await;
    }
}

/// Update-DTO setters have to produce the same patch a struct literal
/// does, including asking for NULL.
mod update_builders {
    use entity_derive::Entity;
    use uuid::Uuid;

    use crate::pg;

    #[derive(Debug, Clone, Entity)]
    #[entity(table = "shipments_b", migrations)]
    pub struct Shipment {
        #[id]
        pub id: Uuid,

        #[field(create, update, response)]
        pub status: String,

        #[field(create, update, response)]
        pub courier_id: Option<Uuid>
    }

    #[tokio::test]
    async fn setters_and_clear_reach_the_row() {
        let Some(db) = pg::provision("builders", &[Shipment::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let courier = Uuid::now_v7();
        let shipment = pool
            .create(CreateShipmentRequest {
                status:     "created".to_owned(),
                courier_id: Some(courier)
            })
            .await
            .expect("create failed");

        let patched = pool
            .update(
                shipment.id,
                UpdateShipmentRequest::default().set_status("accepted".to_owned())
            )
            .await
            .expect("update failed");
        assert_eq!(patched.status, "accepted");
        assert_eq!(
            patched.courier_id,
            Some(courier),
            "an untouched field must keep its stored value"
        );

        let cleared = pool
            .update(
                shipment.id,
                UpdateShipmentRequest::default().clear_courier_id()
            )
            .await
            .expect("update failed");
        assert_eq!(
            cleared.courier_id, None,
            "clear_ must write NULL, not leave the column alone"
        );
        assert_eq!(
            cleared.status, "accepted",
            "clearing one column must not touch another"
        );

        let reassigned = pool
            .update(
                shipment.id,
                UpdateShipmentRequest::default().set_courier_id(courier)
            )
            .await
            .expect("update failed");
        assert_eq!(reassigned.courier_id, Some(courier));

        db.teardown().await;
    }
}

/// Participant scopes: one value matched against several roles, with
/// and without narrowing to a parent row.
mod scopes {
    use entity_derive::Entity;
    use uuid::Uuid;

    use crate::pg;

    #[derive(Debug, Clone, Entity)]
    #[entity(table = "disputes", migrations)]
    #[scope(involving: requester_id | subject_id)]
    #[scope(handled: requester_id | subject_id, within = parcel_id)]
    pub struct Dispute {
        #[id]
        pub id: Uuid,

        #[field(create, response)]
        pub parcel_id: Uuid,

        #[field(create, response)]
        pub requester_id: Uuid,

        #[field(create, response)]
        pub subject_id: Uuid
    }

    #[tokio::test]
    async fn a_scope_matches_every_declared_role() {
        let Some(db) = pg::provision("scopes", &[Dispute::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let ada = Uuid::now_v7();
        let grace = Uuid::now_v7();
        let stranger = Uuid::now_v7();
        let parcel = Uuid::now_v7();
        let other_parcel = Uuid::now_v7();

        let requested = pool
            .create(CreateDisputeRequest {
                parcel_id:    parcel,
                requester_id: ada,
                subject_id:   grace
            })
            .await
            .expect("create failed");
        let subjected = pool
            .create(CreateDisputeRequest {
                parcel_id:    other_parcel,
                requester_id: grace,
                subject_id:   ada
            })
            .await
            .expect("create failed");
        pool.create(CreateDisputeRequest {
            parcel_id:    parcel,
            requester_id: grace,
            subject_id:   stranger
        })
        .await
        .expect("create failed");

        let ada_rows = pool
            .list_involving(ada, 10, 0)
            .await
            .expect("scope query failed");
        let ada_ids: Vec<Uuid> = ada_rows.iter().map(|d| d.id).collect();
        assert_eq!(ada_ids.len(), 2, "both roles must match the same principal");
        assert!(ada_ids.contains(&requested.id) && ada_ids.contains(&subjected.id));

        let narrowed = pool
            .list_handled(parcel, ada, 10, 0)
            .await
            .expect("narrowed scope query failed");
        assert_eq!(
            narrowed.len(),
            1,
            "narrowing must drop the row belonging to another parcel"
        );
        assert_eq!(narrowed[0].id, requested.id);

        assert!(
            pool.list_involving(Uuid::now_v7(), 10, 0)
                .await
                .expect("scope query failed")
                .is_empty(),
            "an uninvolved principal matches nothing"
        );

        let page = pool
            .list_involving(ada, 1, 0)
            .await
            .expect("scope query failed");
        assert_eq!(page.len(), 1, "the scope honours its pagination");

        db.teardown().await;
    }
}

/// Domain operations write named columns that the public patch DTO
/// deliberately does not carry.
mod domain_operations {
    use chrono::{DateTime, Utc};
    use entity_derive::Entity;
    use uuid::Uuid;

    use crate::pg;

    #[derive(Debug, Clone, Entity)]
    #[entity(table = "citizens", migrations, commands, transactions)]
    #[command(
        VerifyPassport,
        payload(passport_provider),
        sets(passport_verified = "true", passport_verified_at = "NOW()")
    )]
    pub struct Citizen {
        #[id]
        pub id: Uuid,

        #[field(create, update, response)]
        pub name: String,

        #[field(response)]
        #[column(default = "false")]
        pub passport_verified: bool,

        #[field(response)]
        pub passport_provider: Option<String>,

        #[field(response)]
        pub passport_verified_at: Option<DateTime<Utc>>
    }

    #[tokio::test]
    async fn the_operation_writes_exactly_its_columns() {
        let Some(db) = pg::provision("domainop", &[Citizen::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let citizen = pool
            .create(CreateCitizenRequest {
                name: "Ada".to_owned()
            })
            .await
            .expect("create failed");
        assert!(!citizen.passport_verified);
        assert!(citizen.passport_verified_at.is_none());

        let verified = pool
            .verify_passport(VerifyPassportCitizen {
                id:                citizen.id,
                passport_provider: Some("gov".to_owned())
            })
            .await
            .expect("the domain operation must apply");

        assert!(
            verified.passport_verified,
            "the fixed expression must apply"
        );
        assert_eq!(
            verified.passport_provider.as_deref(),
            Some("gov"),
            "the payload column must be bound"
        );
        assert!(
            verified.passport_verified_at.is_some(),
            "the second fixed expression must apply too"
        );
        assert_eq!(
            verified.name, "Ada",
            "a column the operation does not name must stay untouched"
        );

        let missing = pool
            .verify_passport(VerifyPassportCitizen {
                id:                Uuid::now_v7(),
                passport_provider: None
            })
            .await;
        assert!(missing.is_err(), "an unknown id must not report success");

        db.teardown().await;
    }

    #[tokio::test]
    async fn the_operation_runs_inside_a_transaction() {
        let Some(db) = pg::provision("domainoptx", &[Citizen::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let citizen = pool
            .create(CreateCitizenRequest {
                name: "Grace".to_owned()
            })
            .await
            .expect("create failed");

        let mut tx = pool.begin().await.expect("begin failed");
        let verified = CitizenTransactionRepo::new(&mut tx)
            .verify_passport(VerifyPassportCitizen {
                id:                citizen.id,
                passport_provider: Some("gov".to_owned())
            })
            .await
            .expect("the operation must apply")
            .expect("the row exists");
        assert!(verified.passport_verified);
        tx.rollback().await.expect("rollback failed");

        let after_rollback = pool
            .find_by_id(citizen.id)
            .await
            .expect("lookup failed")
            .expect("the row exists");
        assert!(
            !after_rollback.passport_verified,
            "the operation must take part in the transaction, not commit on its own"
        );

        let mut tx = pool.begin().await.expect("begin failed");
        let missing = CitizenTransactionRepo::new(&mut tx)
            .verify_passport(VerifyPassportCitizen {
                id:                Uuid::now_v7(),
                passport_provider: None
            })
            .await
            .expect("an unknown id is not an error here");
        assert!(missing.is_none(), "an unknown id must report no row");
        tx.commit().await.expect("commit failed");

        db.teardown().await;
    }
}

/// The hook-invoking wrapper: order of calls, and what a refusing
/// `before_*` must prevent.
mod hooks {
    use std::sync::{Arc, Mutex};

    use entity_derive::{Entity, async_trait};
    use uuid::Uuid;

    use crate::pg;

    #[derive(Debug, Clone, Entity)]
    #[entity(table = "accounts_h", migrations, soft_delete, hooks)]
    pub struct Account {
        #[id]
        pub id: Uuid,

        #[field(create, update, response)]
        pub label: String,

        #[field(skip)]
        pub deleted_at: Option<chrono::DateTime<chrono::Utc>>
    }

    /// Records the calls it receives, and can refuse one of them.
    #[derive(Clone)]
    struct Recorder {
        calls:  Arc<Mutex<Vec<&'static str>>>,
        refuse: Option<&'static str>
    }

    impl Recorder {
        fn new(refuse: Option<&'static str>) -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
                refuse
            }
        }

        fn note(&self, call: &'static str) -> Result<(), sqlx::Error> {
            self.calls
                .lock()
                .expect("the recorder lock is never poisoned")
                .push(call);
            if self.refuse == Some(call) {
                return Err(sqlx::Error::RowNotFound);
            }
            Ok(())
        }

        fn calls(&self) -> Vec<&'static str> {
            self.calls
                .lock()
                .expect("the recorder lock is never poisoned")
                .clone()
        }
    }

    #[async_trait]
    impl AccountHooks for Recorder {
        type Error = sqlx::Error;

        async fn before_create(&self, dto: &mut CreateAccountRequest) -> Result<(), Self::Error> {
            let trimmed = dto.label.trim().to_owned();
            dto.label = trimmed;
            self.note("before_create")
        }

        async fn after_create(&self, _entity: &Account) -> Result<(), Self::Error> {
            self.note("after_create")
        }

        async fn before_update(
            &self,
            _id: &Uuid,
            _dto: &mut UpdateAccountRequest
        ) -> Result<(), Self::Error> {
            self.note("before_update")
        }

        async fn after_update(&self, _entity: &Account) -> Result<(), Self::Error> {
            self.note("after_update")
        }

        async fn before_delete(&self, _id: &Uuid) -> Result<(), Self::Error> {
            self.note("before_delete")
        }

        async fn after_delete(&self, _id: &Uuid) -> Result<(), Self::Error> {
            self.note("after_delete")
        }

        async fn before_restore(&self, _id: &Uuid) -> Result<(), Self::Error> {
            self.note("before_restore")
        }

        async fn after_restore(&self, _id: &Uuid) -> Result<(), Self::Error> {
            self.note("after_restore")
        }
    }

    #[tokio::test]
    async fn hooks_run_around_every_mutation() {
        let Some(db) = pg::provision("hooks", &[Account::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let recorder = Recorder::new(None);
        let repo = AccountRepo::new(pool.clone(), recorder.clone());

        let created = repo
            .create(CreateAccountRequest {
                label: "  ledger  ".to_owned()
            })
            .await
            .expect("create failed");
        assert_eq!(
            created.label, "ledger",
            "before_create must be able to rewrite the DTO before the INSERT"
        );

        repo.update(
            created.id,
            UpdateAccountRequest {
                label: Some("cashbook".to_owned())
            }
        )
        .await
        .expect("update failed");

        assert!(repo.delete(created.id).await.expect("delete failed"));
        assert!(repo.restore(created.id).await.expect("restore failed"));

        assert_eq!(
            recorder.calls(),
            vec![
                "before_create",
                "after_create",
                "before_update",
                "after_update",
                "before_delete",
                "after_delete",
                "before_restore",
                "after_restore",
            ]
        );

        db.teardown().await;
    }

    #[tokio::test]
    async fn a_refusing_before_hook_writes_nothing() {
        let Some(db) = pg::provision("hooksrefuse", &[Account::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let recorder = Recorder::new(Some("before_create"));
        let repo = AccountRepo::new(pool.clone(), recorder.clone());

        assert!(
            repo.create(CreateAccountRequest {
                label: "ledger".to_owned()
            })
            .await
            .is_err(),
            "a refusing before_create must fail the call"
        );
        assert!(
            pool.list(10, 0).await.expect("list failed").is_empty(),
            "a refused create must not have written a row"
        );
        assert_eq!(
            recorder.calls(),
            vec!["before_create"],
            "the after hook must not run when the before hook refused"
        );

        db.teardown().await;
    }

    #[tokio::test]
    async fn a_refusing_delete_hook_leaves_the_row() {
        let Some(db) = pg::provision("hooksdel", &[Account::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let stored = pool
            .create(CreateAccountRequest {
                label: "ledger".to_owned()
            })
            .await
            .expect("create failed");

        let repo = AccountRepo::new(pool.clone(), Recorder::new(Some("before_delete")));
        assert!(repo.delete(stored.id).await.is_err());
        assert!(
            pool.find_by_id(stored.id)
                .await
                .expect("read failed")
                .is_some(),
            "a refused delete must leave the row in place"
        );

        db.teardown().await;
    }

    #[tokio::test]
    async fn reads_reach_the_pool_through_the_wrapper() {
        let Some(db) = pg::provision("hooksread", &[Account::MIGRATION_UP]).await else {
            return;
        };
        let pool = db.pool();

        let recorder = Recorder::new(None);
        let repo = AccountRepo::new(pool.clone(), recorder.clone());
        let created = repo
            .create(CreateAccountRequest {
                label: "ledger".to_owned()
            })
            .await
            .expect("create failed");

        let found = repo
            .find_by_id(created.id)
            .await
            .expect("read through the wrapper failed")
            .expect("row missing");
        assert_eq!(found.id, created.id);
        assert_eq!(
            recorder.calls(),
            vec!["before_create", "after_create"],
            "a read must not invoke any hook"
        );

        db.teardown().await;
    }
}
