Cuando el SQL generado automáticamente no es suficiente, usa `sql = "trait"` para control total.

## Cuándo Usar SQL Personalizado

- **Joins** — Entidades relacionadas en una sola consulta
- **CTEs** — Consultas recursivas o de múltiples pasos
- **Búsqueda full-text** — PostgreSQL `tsvector`/`tsquery`
- **Agregaciones** — `GROUP BY`, `HAVING`, funciones de ventana
- **Tablas particionadas** — Particiones basadas en tiempo o rango
- **Operaciones bulk** — Inserciones/actualizaciones por lotes
- **Borrado lógico** — Lógica de eliminación personalizada
- **Bloqueo optimista** — Concurrencia basada en versión

## Configuración Básica

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

Esto genera:
- Todos los DTOs (`CreatePostRequest`, `UpdatePostRequest`, `PostResponse`)
- `PostRow` e `InsertablePost`
- Trait `PostRepository`
- Todas las implementaciones `From`

Pero **no** genera `impl PostRepository for PgPool`.

## Implementando el Repository

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
        // Tu lógica de actualización personalizada
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

## Ejemplo: Posts con Join de Autor

```rust
// Respuesta extendida con datos del autor
pub struct PostWithAuthor {
    pub post: Post,
    pub author: User,
}

// Extensión de repository personalizada
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
        // Consulta similar con join y paginación
        todo!()
    }
}
```

## Ejemplo: Búsqueda Full-Text

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

## Ejemplo: Borrado Lógico

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
    // ... otros métodos

    async fn delete(&self, id: Uuid) -> Result<bool, Self::Error> {
        // Borrado lógico en lugar de borrado físico
        let result = sqlx::query(
            "UPDATE blog.posts SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL"
        )
        .bind(&id)
        .execute(self)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Post>, Self::Error> {
        // Excluir borrados lógicos
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

// Método adicional para administradores
pub trait PostAdminRepository {
    async fn restore(&self, id: Uuid) -> Result<bool, sqlx::Error>;
    async fn hard_delete(&self, id: Uuid) -> Result<bool, sqlx::Error>;
    async fn list_deleted(&self, limit: i64, offset: i64) -> Result<Vec<Post>, sqlx::Error>;
}
```

## Ejemplo: Bloqueo Optimista

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
        // Requiere versión actual para bloqueo optimista
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

    // ... otros métodos
}
```

## Mejores Prácticas para SQL Personalizado

1. **Usa `query_as` con estructuras Row** — Mapeo type-safe
2. **Vincula todos los parámetros** — Nunca interpoles strings
3. **Retorna Row, convierte a Entity** — Usa las impl `From` generadas
4. **Extiende, no reemplaces** — Añade traits personalizados junto a `Repository`
5. **Prueba con base de datos real** — Los tests de integración son esenciales

## Ver También

- [[Atributos|Atributos]] — Referencia completa de atributos
- [[Ejemplos|Ejemplos]] — Ejemplos del mundo real
- [[Mejores Prácticas|Mejores-Prácticas]] — Consejos de rendimiento
