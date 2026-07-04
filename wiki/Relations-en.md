# Entity Relations
Define relationships between entities using `#[belongs_to]` and `#[has_many]`. Relations generate type-safe navigation methods in repositories.
## Quick Start

```rust
// Parent entity
#[derive(Entity)]
#[entity(table = "users")]
#[has_many(Post)]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,
}

// Child entity
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

## Generated Code

### For `#[has_many(Post)]` on User

```rust
#[async_trait]
impl UserRepository for PgPool {
    // ... standard CRUD methods

    /// Find all posts belonging to this user.
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

### For `#[belongs_to(User)]` on Post

```rust
#[async_trait]
impl PostRepository for PgPool {
    // ... standard CRUD methods

    /// Find the user this post belongs to.
    async fn find_user(&self, id: Uuid) -> Result<Option<User>, Self::Error> {
        // First get the post to find user_id
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

## Relation Types

### belongs_to (Many-to-One)

A child entity references a parent via foreign key.

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

**Generated methods:**
- `find_post(comment_id)` → `Option<Post>`
- `find_author(comment_id)` → `Option<User>` (note: method name derived from field name without `_id`)

### has_many (One-to-Many)

A parent entity has multiple children.

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

**Generated methods:**
- `find_posts(user_id)` → `Vec<Post>`
- `find_comments(user_id)` → `Vec<Comment>`

### has_many through (Many-to-Many)

Declare the junction table with `through` and the repository gains a JOIN-backed lookup plus link management; `migrations` emits `MIGRATION_JUNCTIONS` (composite primary key over both foreign keys, `ON DELETE CASCADE` on each side).

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

Generated methods: `find_users` (INNER JOIN), `add_user` (idempotent, `ON CONFLICT DO NOTHING`), `remove_user` (`false` when not linked), `has_user` (`SELECT EXISTS`).

## Usage Examples

### Loading Related Data

```rust
// Get user with their posts
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

// Get post with author
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

### Building Response DTOs

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

### Nested Loading with API

```rust
use axum::{extract::Path, Json};

#[derive(Serialize)]
pub struct UserProfile {
    pub user: UserResponse,
    pub posts: Vec<PostResponse>,
    pub post_count: usize,
}

async fn get_user_profile(
    Path(user_id): Path<Uuid>,
    pool: Extension<PgPool>,
) -> Result<Json<UserProfile>, AppError> {
    let user = pool.find_by_id(user_id).await?
        .ok_or(AppError::NotFound)?;

    let posts = pool.find_posts(user_id).await?;

    Ok(Json(UserProfile {
        user: UserResponse::from(&user),
        post_count: posts.len(),
        posts: posts.into_iter().map(PostResponse::from).collect(),
    }))
}
```

## Multiple Relations

An entity can have multiple relations:

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

**Generated for Organization:**
- `find_users(org_id)`
- `find_projects(org_id)`
- `find_teams(org_id)`

**Generated for Project:**
- `find_organization(project_id)`
- `find_owner(project_id)`

## Custom Joins with sql = "trait"

For complex queries with eager loading, use custom SQL:

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
        // Single query with joins
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

## With Filtering

Combine relations with query filtering:

```rust
#[derive(Entity)]
#[entity(table = "posts")]
pub struct Post {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    #[belongs_to(User)]
    #[filter]  // Enable filtering by user_id
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

**Usage:**
```rust
// Get posts by specific user with title filter
let query = PostQuery {
    user_id: Some(user_id),
    title: Some("%rust%".into()),
    limit: Some(20),
    ..Default::default()
};
let posts = pool.query(query).await?;
```

## Best Practices

1. **Avoid N+1 queries** — Use joins for eager loading when fetching multiple related entities
2. **Use pagination** — Always limit `has_many` results
3. **Consider data access patterns** — Add indexes on foreign key columns
4. **Cache when appropriate** — Cache frequently accessed related data
5. **Use projections** — Fetch only needed fields for related entities

## See Also

- [[Filtering]] — Query filtering
- [[Custom SQL]] — Complex joins and queries
- [[Best Practices]] — Performance tips
