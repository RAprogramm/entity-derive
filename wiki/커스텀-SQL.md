자동 생성된 SQL이 충분하지 않을 때 `sql = "trait"`를 사용하여 완전한 제어를 합니다.

## 커스텀 SQL을 사용해야 할 때

- **JOIN** — 단일 쿼리로 관련 엔티티
- **CTE** — 복잡한 재귀 또는 다단계 쿼리
- **전문 검색** — PostgreSQL `tsvector`/`tsquery`
- **집계** — `GROUP BY`, `HAVING`, 윈도우 함수
- **파티션 테이블** — 시간 기반 또는 범위 파티션
- **대량 작업** — 배치 삽입/업데이트
- **소프트 삭제** — 커스텀 삭제 로직
- **낙관적 잠금** — 버전 기반 동시성 제어

## 기본 설정

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

생성되는 것:
- 모든 DTO (`CreatePostRequest`, `UpdatePostRequest`, `PostResponse`)
- `PostRow`와 `InsertablePost`
- `PostRepository` 트레이트
- 모든 `From` 구현

하지만 `impl PostRepository for PgPool`은 **아님**.

## 리포지토리 구현

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
        // 커스텀 업데이트 로직
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

## 예제: 작성자 JOIN이 있는 게시물

```rust
// 작성자 데이터가 있는 확장 응답
pub struct PostWithAuthor {
    pub post: Post,
    pub author: User,
}

// 커스텀 리포지토리 확장
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
        // 페이지네이션이 있는 유사한 JOIN 쿼리
        todo!()
    }
}
```

## 예제: 전문 검색

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

## 예제: 소프트 삭제

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
    // ... 다른 메서드

    async fn delete(&self, id: Uuid) -> Result<bool, Self::Error> {
        // 하드 삭제 대신 소프트 삭제
        let result = sqlx::query(
            "UPDATE blog.posts SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL"
        )
        .bind(&id)
        .execute(self)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Post>, Self::Error> {
        // 소프트 삭제된 항목 제외
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

// 관리자용 추가 메서드
pub trait PostAdminRepository {
    async fn restore(&self, id: Uuid) -> Result<bool, sqlx::Error>;
    async fn hard_delete(&self, id: Uuid) -> Result<bool, sqlx::Error>;
    async fn list_deleted(&self, limit: i64, offset: i64) -> Result<Vec<Post>, sqlx::Error>;
}
```

## 예제: 낙관적 잠금

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
        // 낙관적 잠금을 위해 현재 버전 필요
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

    // ... 다른 메서드
}
```

## 커스텀 SQL 모범 사례

1. **Row 구조체로 `query_as` 사용** — 타입 안전 매핑
2. **모든 파라미터 바인딩** — 문자열 보간 절대 금지
3. **Row 반환, Entity로 변환** — 생성된 `From` 구현 사용
4. **대체가 아닌 확장** — `Repository`와 함께 커스텀 트레이트 추가
5. **실제 DB로 테스트** — 통합 테스트 필수
