# Custom SQL Queries
When the auto-generated SQL isn't enough, use `sql = "trait"` for full control.
## When to Use Custom SQL

- **Joins** — Related entities in single query
- **CTEs** — Complex recursive or multi-step queries
- **Full-text search** — PostgreSQL `tsvector`/`tsquery`
- **Aggregations** — `GROUP BY`, `HAVING`, window functions
- **Partitioned tables** — Time-based or range partitions
- **Bulk operations** — Batch inserts/updates
- **Soft deletes** — Custom delete logic
- **Optimistic locking** — Version-based concurrency

## Basic Setup

```rust
#[derive(Entity)]
#[entity(table = "posts", schema = "blog", sql = "trait")]
pub struct Post {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub title: String,

    #[field(create, response)]
    pub author_id: Uuid,

    #[auto]
    #[field(response)]
    pub created_at: DateTime<Utc>,
}
```

This generates:
- All DTOs (`CreatePostRequest`, `UpdatePostRequest`, `PostResponse`)
- `PostRow` and `InsertablePost`
- `PostRepository` trait
- All `From` implementations

But **not** the `impl PostRepository for PgPool`.

## Implementing the Repository

```rust
use async_trait::async_trait;
use sqlx::PgPool;

#[async_trait]
impl PostRepository for PgPool {
    type Error = sqlx::Error;

    async fn create(&self, dto: CreatePostRequest) -> Result<Post, Self::Error> {
        let entity = Post::from(dto);
        let insertable = InsertablePost::from(&entity);

        sqlx::query(
            r#"
            INSERT INTO blog.posts (id, title, author_id, created_at)
            VALUES ($1, $2, $3, $4)
            "#
        )
        .bind(insertable.id)
        .bind(&insertable.title)
        .bind(insertable.author_id)
        .bind(insertable.created_at)
        .execute(self)
        .await?;

        Ok(entity)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Post>, Self::Error> {
        let row: Option<PostRow> = sqlx::query_as(
            "SELECT id, title, author_id, created_at FROM blog.posts WHERE id = $1"
        )
        .bind(&id)
        .fetch_optional(self)
        .await?;

        Ok(row.map(Post::from))
    }

    async fn update(&self, id: Uuid, dto: UpdatePostRequest) -> Result<Post, Self::Error> {
        // Your custom update logic
        todo!()
    }

    async fn delete(&self, id: Uuid) -> Result<bool, Self::Error> {
        let result = sqlx::query("DELETE FROM blog.posts WHERE id = $1")
            .bind(&id)
            .execute(self)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Post>, Self::Error> {
        let rows: Vec<PostRow> = sqlx::query_as(
            "SELECT id, title, author_id, created_at FROM blog.posts ORDER BY created_at DESC LIMIT $1 OFFSET $2"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(self)
        .await?;

        Ok(rows.into_iter().map(Post::from).collect())
    }
}
```

## Example: Posts with Author Join

```rust
// Extended response with author data
pub struct PostWithAuthor {
    pub post: Post,
    pub author: User,
}

// Custom repository extension
pub trait PostRepositoryExt: PostRepository {
    async fn find_with_author(&self, id: Uuid) -> Result<Option<PostWithAuthor>, Self::Error>;
    async fn list_with_authors(&self, limit: i64, offset: i64) -> Result<Vec<PostWithAuthor>, Self::Error>;
}

#[async_trait]
impl PostRepositoryExt for PgPool {
    async fn find_with_author(&self, id: Uuid) -> Result<Option<PostWithAuthor>, Self::Error> {
        let row = sqlx::query_as::<_, (PostRow, UserRow)>(
            r#"
            SELECT
                p.id, p.title, p.author_id, p.created_at,
                u.id, u.username, u.email, u.created_at
            FROM blog.posts p
            JOIN auth.users u ON u.id = p.author_id
            WHERE p.id = $1
            "#
        )
        .bind(&id)
        .fetch_optional(self)
        .await?;

        Ok(row.map(|(p, u)| PostWithAuthor {
            post: Post::from(p),
            author: User::from(u),
        }))
    }

    async fn list_with_authors(&self, limit: i64, offset: i64) -> Result<Vec<PostWithAuthor>, Self::Error> {
        // Similar join query with pagination
        todo!()
    }
}
```

## Example: Full-Text Search

```rust
pub trait PostSearchRepository {
    async fn search(&self, query: &str, limit: i64) -> Result<Vec<Post>, sqlx::Error>;
}

#[async_trait]
impl PostSearchRepository for PgPool {
    async fn search(&self, query: &str, limit: i64) -> Result<Vec<Post>, sqlx::Error> {
        let rows: Vec<PostRow> = sqlx::query_as(
            r#"
            SELECT id, title, author_id, created_at
            FROM blog.posts
            WHERE to_tsvector('english', title || ' ' || content) @@ plainto_tsquery('english', $1)
            ORDER BY ts_rank(to_tsvector('english', title || ' ' || content), plainto_tsquery('english', $1)) DESC
            LIMIT $2
            "#
        )
        .bind(query)
        .bind(limit)
        .fetch_all(self)
        .await?;

        Ok(rows.into_iter().map(Post::from).collect())
    }
}
```

## Example: Soft Deletes

```rust
#[derive(Entity)]
#[entity(table = "posts", sql = "trait")]
pub struct Post {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub title: String,

    #[field(response)]
    pub deleted_at: Option<DateTime<Utc>>,

    #[auto]
    #[field(response)]
    pub created_at: DateTime<Utc>,
}

#[async_trait]
impl PostRepository for PgPool {
    // ... other methods

    async fn delete(&self, id: Uuid) -> Result<bool, Self::Error> {
        // Soft delete instead of hard delete
        let result = sqlx::query(
            "UPDATE blog.posts SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL"
        )
        .bind(&id)
        .execute(self)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Post>, Self::Error> {
        // Exclude soft-deleted
        let rows: Vec<PostRow> = sqlx::query_as(
            r#"
            SELECT id, title, deleted_at, created_at
            FROM blog.posts
            WHERE deleted_at IS NULL
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(self)
        .await?;

        Ok(rows.into_iter().map(Post::from).collect())
    }
}

// Additional method for admins
pub trait PostAdminRepository {
    async fn restore(&self, id: Uuid) -> Result<bool, sqlx::Error>;
    async fn hard_delete(&self, id: Uuid) -> Result<bool, sqlx::Error>;
    async fn list_deleted(&self, limit: i64, offset: i64) -> Result<Vec<Post>, sqlx::Error>;
}
```

## Example: Optimistic Locking

```rust
#[derive(Entity)]
#[entity(table = "documents", sql = "trait")]
pub struct Document {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub content: String,

    #[field(response)]
    pub version: i64,

    #[auto]
    #[field(response)]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug)]
pub enum DocumentError {
    Sqlx(sqlx::Error),
    ConcurrentModification,
}

#[async_trait]
impl DocumentRepository for PgPool {
    type Error = DocumentError;

    async fn update(&self, id: Uuid, dto: UpdateDocumentRequest) -> Result<Document, Self::Error> {
        // Requires current version for optimistic locking
        let expected_version = dto.version.ok_or(DocumentError::ConcurrentModification)?;

        let row: Option<DocumentRow> = sqlx::query_as(
            r#"
            UPDATE documents
            SET content = COALESCE($1, content),
                version = version + 1,
                updated_at = NOW()
            WHERE id = $2 AND version = $3
            RETURNING id, content, version, updated_at
            "#
        )
        .bind(&dto.content)
        .bind(&id)
        .bind(expected_version)
        .fetch_optional(self)
        .await
        .map_err(DocumentError::Sqlx)?;

        row.map(Document::from)
            .ok_or(DocumentError::ConcurrentModification)
    }

    // ... other methods
}
```

## Best Practices for Custom SQL

1. **Use `query_as` with Row structs** — Type-safe mapping
2. **Bind all parameters** — Never interpolate strings
3. **Return Row, convert to Entity** — Use generated `From` impls
4. **Extend, don't replace** — Add custom traits alongside `Repository`
5. **Test with real database** — Integration tests are essential
