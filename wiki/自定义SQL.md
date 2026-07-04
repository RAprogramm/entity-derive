当自动生成的SQL不够用时，使用 `sql = "trait"` 获得完全控制。

## 何时使用自定义SQL

- **Join连接** — 单次查询获取关联实体
- **CTE** — 复杂递归或多步查询
- **全文搜索** — PostgreSQL `tsvector`/`tsquery`
- **聚合** — `GROUP BY`、`HAVING`、窗口函数
- **分区表** — 基于时间或范围的分区
- **批量操作** — 批量插入/更新
- **软删除** — 自定义删除逻辑
- **乐观锁** — 基于版本的并发控制

## 基本配置

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

这会生成：
- 所有DTO（`CreatePostRequest`、`UpdatePostRequest`、`PostResponse`）
- `PostRow` 和 `InsertablePost`
- `PostRepository` trait
- 所有 `From` 实现

但**不会**生成 `impl PostRepository for PgPool`。

## 实现Repository

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
        // 你的自定义更新逻辑
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

## 示例：带作者Join的文章

```rust
// 带作者数据的扩展响应
pub struct PostWithAuthor {
    pub post: Post,
    pub author: User,
}

// 自定义repository扩展
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
        // 类似的带join和分页的查询
        todo!()
    }
}
```

## 示例：全文搜索

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

## 示例：软删除

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
    // ... 其他方法

    async fn delete(&self, id: Uuid) -> Result<bool, Self::Error> {
        // 软删除而不是硬删除
        let result = sqlx::query(
            "UPDATE blog.posts SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL"
        )
        .bind(&id)
        .execute(self)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Post>, Self::Error> {
        // 排除已软删除的记录
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

// 管理员的额外方法
pub trait PostAdminRepository {
    async fn restore(&self, id: Uuid) -> Result<bool, sqlx::Error>;
    async fn hard_delete(&self, id: Uuid) -> Result<bool, sqlx::Error>;
    async fn list_deleted(&self, limit: i64, offset: i64) -> Result<Vec<Post>, sqlx::Error>;
}
```

## 示例：乐观锁

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
        // 需要当前版本用于乐观锁
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

    // ... 其他方法
}
```

## 自定义SQL最佳实践

1. **使用 `query_as` 配合Row结构体** — 类型安全映射
2. **绑定所有参数** — 永远不要插值字符串
3. **返回Row，转换为Entity** — 使用生成的 `From` 实现
4. **扩展，而不是替换** — 在 `Repository` 旁边添加自定义trait
5. **使用真实数据库测试** — 集成测试是必不可少的

## 另见

- [[属性|属性]] — 完整属性参考
- [[案例|案例]] — 实际案例
- [[最佳实践|最佳实践]] — 性能技巧
