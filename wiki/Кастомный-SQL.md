Когда автоматически сгенерированного SQL недостаточно, используйте `sql = "trait"` для полного контроля.

## Когда использовать пользовательский SQL

- **JOIN'ы** — Связанные сущности в одном запросе
- **CTE** — Сложные рекурсивные или многоэтапные запросы
- **Полнотекстовый поиск** — PostgreSQL `tsvector`/`tsquery`
- **Агрегации** — `GROUP BY`, `HAVING`, оконные функции
- **Партиционированные таблицы** — Временное или диапазонное партиционирование
- **Пакетные операции** — Массовые вставки/обновления
- **Мягкое удаление** — Пользовательская логика удаления
- **Оптимистичная блокировка** — Контроль конкурентности на основе версий

## Базовая настройка

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

Это генерирует:
- Все DTO (`CreatePostRequest`, `UpdatePostRequest`, `PostResponse`)
- `PostRow` и `InsertablePost`
- Трейт `PostRepository`
- Все реализации `From`

Но **не** `impl PostRepository for PgPool`.

## Реализация репозитория

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
        // Ваша пользовательская логика обновления
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

## Пример: Посты с JOIN автора

```rust
// Расширенный response с данными автора
pub struct PostWithAuthor {
    pub post: Post,
    pub author: User,
}

// Расширение пользовательского репозитория
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
        // Аналогичный запрос с JOIN и пагинацией
        todo!()
    }
}
```

## Пример: Полнотекстовый поиск

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

## Пример: Мягкое удаление

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
    // ... другие методы

    async fn delete(&self, id: Uuid) -> Result<bool, Self::Error> {
        // Мягкое удаление вместо полного
        let result = sqlx::query(
            "UPDATE blog.posts SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL"
        )
        .bind(&id)
        .execute(self)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Post>, Self::Error> {
        // Исключаем мягко удалённые
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

// Дополнительные методы для администраторов
pub trait PostAdminRepository {
    async fn restore(&self, id: Uuid) -> Result<bool, sqlx::Error>;
    async fn hard_delete(&self, id: Uuid) -> Result<bool, sqlx::Error>;
    async fn list_deleted(&self, limit: i64, offset: i64) -> Result<Vec<Post>, sqlx::Error>;
}
```

## Пример: Оптимистичная блокировка

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
        // Требуется текущая версия для оптимистичной блокировки
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

    // ... другие методы
}
```

## Лучшие практики для пользовательского SQL

1. **Используйте `query_as` с Row-структурами** — Типобезопасное отображение
2. **Привязывайте все параметры** — Никогда не интерполируйте строки
3. **Возвращайте Row, преобразуйте в Entity** — Используйте сгенерированные `From` реализации
4. **Расширяйте, не заменяйте** — Добавляйте пользовательские трейты рядом с `Repository`
5. **Тестируйте с реальной БД** — Интеграционные тесты необходимы
