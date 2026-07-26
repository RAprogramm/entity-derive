Define relaciones entre entidades usando `#[belongs_to]` y `#[has_many]`. Las relaciones generan métodos de navegación type-safe en repositorios.

## Inicio Rápido

```rust
// Entidad padre
#[derive(Entity)]
#[entity(table = "users")]
#[has_many(Post)]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,
}

// Entidad hija
#[derive(Entity)]
#[entity(table = "posts")]
pub struct Post {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    #[belongs_to(User)]
    pub user_id: Uuid,

    #[field(create, update, response)]
    pub title: String,
}
```

## Código Generado

### Para `#[has_many(Post)]` en User

```rust
#[async_trait]
impl UserRepository for PgPool {
    // ... métodos CRUD estándar

    /// Encontrar todos los posts de este usuario.
    async fn find_posts(&self, user_id: Uuid) -> Result<Vec<Post>, Self::Error> {
        let rows: Vec<PostRow> = sqlx::query_as(
            "SELECT * FROM posts WHERE user_id = $1 ORDER BY created_at DESC"
        )
        .bind(&user_id)
        .fetch_all(self)
        .await?;

        Ok(rows.into_iter().map(Post::from).collect())
    }
}
```

### Para `#[belongs_to(User)]` en Post

```rust
#[async_trait]
impl PostRepository for PgPool {
    // ... métodos CRUD estándar

    /// Encontrar el usuario al que pertenece este post.
    async fn find_user(&self, id: Uuid) -> Result<Option<User>, Self::Error> {
        // Primero obtener el post para encontrar user_id
        let post = self.find_by_id(id).await?;

        if let Some(post) = post {
            let row: Option<UserRow> = sqlx::query_as(
                "SELECT * FROM users WHERE id = $1"
            )
            .bind(&post.user_id)
            .fetch_optional(self)
            .await?;

            Ok(row.map(User::from))
        } else {
            Ok(None)
        }
    }
}
```

## Tipos de Relación

### belongs_to (Muchos-a-Uno)

Una entidad hija referencia a un padre via clave foránea.

```rust
#[derive(Entity)]
#[entity(table = "comments")]
pub struct Comment {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    #[belongs_to(Post)]
    pub post_id: Uuid,

    #[field(create, response)]
    #[belongs_to(User)]
    pub author_id: Uuid,

    #[field(create, response)]
    pub content: String,
}
```

**Métodos generados:**
- `find_post(comment_id)` → `Option<Post>`
- `find_author(comment_id)` → `Option<User>` (nota: nombre derivado del campo sin `_id`)

### has_many (Uno-a-Muchos)

Una entidad padre tiene múltiples hijos.

```rust
#[derive(Entity)]
#[entity(table = "users")]
#[has_many(Post)]
#[has_many(Comment)]
pub struct User {
    #[id]
    pub id: Uuid,
    // ...
}
```

**Métodos generados:**
- `find_posts(user_id)` → `Vec<Post>`
- `find_comments(user_id)` → `Vec<Comment>`

## Ejemplos de Uso

### Cargando Datos Relacionados

```rust
// Obtener usuario con sus posts
async fn get_user_with_posts(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Option<(User, Vec<Post>)>, sqlx::Error> {
    let user = pool.find_by_id(user_id).await?;

    if let Some(user) = user {
        let posts = pool.find_posts(user_id).await?;
        Ok(Some((user, posts)))
    } else {
        Ok(None)
    }
}

// Obtener post con autor
async fn get_post_with_author(
    pool: &PgPool,
    post_id: Uuid,
) -> Result<Option<(Post, User)>, sqlx::Error> {
    let post = pool.find_by_id(post_id).await?;

    if let Some(post) = post {
        let user = pool.find_user(post.id).await?;
        if let Some(user) = user {
            return Ok(Some((post, user)));
        }
    }

    Ok(None)
}
```

### Construyendo DTOs de Respuesta

```rust
#[derive(Serialize)]
pub struct PostWithAuthor {
    #[serde(flatten)]
    pub post: PostResponse,
    pub author: UserResponse,
}

async fn get_posts_with_authors(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<PostWithAuthor>, sqlx::Error> {
    let posts = pool.list(limit, 0).await?;

    let mut results = Vec::with_capacity(posts.len());

    for post in posts {
        if let Some(user) = pool.find_user(post.id).await? {
            results.push(PostWithAuthor {
                post: PostResponse::from(&post),
                author: UserResponse::from(&user),
            });
        }
    }

    Ok(results)
}
```

## Relaciones Múltiples

Una entidad puede tener múltiples relaciones:

```rust
#[derive(Entity)]
#[entity(table = "organizations")]
#[has_many(User)]
#[has_many(Project)]
#[has_many(Team)]
pub struct Organization {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,
}

#[derive(Entity)]
#[entity(table = "projects")]
pub struct Project {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    #[belongs_to(Organization)]
    pub organization_id: Uuid,

    #[field(create, response)]
    #[belongs_to(User)]
    pub owner_id: Uuid,

    #[field(create, update, response)]
    pub name: String,
}
```

**Generado para Organization:**
- `find_users(org_id)`
- `find_projects(org_id)`
- `find_teams(org_id)`

**Generado para Project:**
- `find_organization(project_id)`
- `find_owner(project_id)`

## Joins Personalizados con sql = "trait"

Para consultas complejas con eager loading, usa SQL personalizado:

```rust
#[derive(Entity)]
#[entity(table = "posts", sql = "trait")]
pub struct Post { /* ... */ }

pub struct PostWithRelations {
    pub post: Post,
    pub author: User,
    pub comments: Vec<Comment>,
}

pub trait PostRepositoryExt {
    async fn find_with_relations(&self, id: Uuid) -> Result<Option<PostWithRelations>, sqlx::Error>;
    async fn list_with_authors(&self, limit: i64) -> Result<Vec<(Post, User)>, sqlx::Error>;
}

#[async_trait]
impl PostRepositoryExt for PgPool {
    async fn find_with_relations(&self, id: Uuid) -> Result<Option<PostWithRelations>, sqlx::Error> {
        // Consulta única con joins
        let row = sqlx::query_as::<_, (PostRow, UserRow)>(
            r#"
            SELECT p.*, u.*
            FROM posts p
            JOIN users u ON u.id = p.user_id
            WHERE p.id = $1
            "#
        )
        .bind(&id)
        .fetch_optional(self)
        .await?;

        if let Some((post_row, user_row)) = row {
            let comments: Vec<CommentRow> = sqlx::query_as(
                "SELECT * FROM comments WHERE post_id = $1 ORDER BY created_at"
            )
            .bind(&id)
            .fetch_all(self)
            .await?;

            Ok(Some(PostWithRelations {
                post: Post::from(post_row),
                author: User::from(user_row),
                comments: comments.into_iter().map(Comment::from).collect(),
            }))
        } else {
            Ok(None)
        }
    }

    async fn list_with_authors(&self, limit: i64) -> Result<Vec<(Post, User)>, sqlx::Error> {
        let rows = sqlx::query_as::<_, (PostRow, UserRow)>(
            r#"
            SELECT p.*, u.*
            FROM posts p
            JOIN users u ON u.id = p.user_id
            ORDER BY p.created_at DESC
            LIMIT $1
            "#
        )
        .bind(limit)
        .fetch_all(self)
        .await?;

        Ok(rows.into_iter()
            .map(|(p, u)| (Post::from(p), User::from(u)))
            .collect())
    }
}
```

## Con Filtrado

Combina relaciones con filtrado de consultas:

```rust
#[derive(Entity)]
#[entity(table = "posts")]
pub struct Post {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    #[belongs_to(User)]
    #[filter]  // Habilitar filtrado por user_id
    pub user_id: Uuid,

    #[field(create, update, response)]
    #[filter(like)]
    pub title: String,

    #[field(response)]
    #[auto]
    #[filter(range)]
    pub created_at: DateTime<Utc>,
}
```

**Uso:**
```rust
// Obtener posts de un usuario específico con filtro de título
let query = PostQuery {
    user_id: Some(user_id),
    title: Some("rust".into()),
    limit: Some(20),
    ..Default::default()
};
let posts = pool.query(query).await?;
```

## Mejores Prácticas

1. **Evitar consultas N+1** — Usa joins para eager loading al obtener múltiples entidades relacionadas
2. **Usar paginación** — Siempre limita resultados de `has_many`
3. **Considerar patrones de acceso a datos** — Añade índices en columnas de clave foránea
4. **Cachear cuando sea apropiado** — Cachea datos relacionados accedidos frecuentemente
5. **Usar proyecciones** — Obtén solo campos necesarios para entidades relacionadas

## Ver También

- [[Filtrado|Filtrado]] — Filtrado de consultas
- [[SQL Personalizado|SQL-Personalizado]] — Joins y consultas complejas
- [[Mejores Prácticas|Mejores-Prácticas]] — Consejos de rendimiento

### has_many through (muchos-a-muchos)

Declara la tabla de unión con `through` y el repositorio obtiene una consulta con JOIN más gestión de vínculos; `migrations` emite `MIGRATION_JUNCTIONS` (clave primaria compuesta sobre ambas claves foráneas, `ON DELETE CASCADE` en ambos lados).

```rust
#[derive(Entity)]
#[entity(table = "teams", migrations)]
#[has_many(User, through = "team_members")]
pub struct Team { /* ... */ }

for ddl in Team::MIGRATION_JUNCTIONS {
    sqlx::query(ddl).execute(&pool).await?;
}

pool.add_user(team_id, user_id).await?;
let members: Vec<User> = pool.find_users(team_id).await?;
let linked = pool.has_user(team_id, user_id).await?;
let removed = pool.remove_user(team_id, user_id).await?;
```

Métodos generados: `find_users` (INNER JOIN), `add_user` (idempotente, `ON CONFLICT DO NOTHING`), `remove_user` (`false` si no estaba vinculado), `has_user` (`SELECT EXISTS`).
