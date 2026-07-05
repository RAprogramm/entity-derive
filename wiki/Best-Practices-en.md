# Best Practices
Guidelines for using `entity-derive` effectively in production.
## Entity Design

### Keep Entities Focused

One entity per database table. Don't try to model complex relationships in a single entity.

```rust
// Good: Separate entities
#[derive(Entity)]
#[entity(table = "users")]
pub struct User {
    #[id]
    pub id: Uuid,
    #[field(create, update, response)]
    pub name: String,
}

#[derive(Entity)]
#[entity(table = "posts")]
pub struct Post {
    #[id]
    pub id: Uuid,
    #[field(create, response)]
    pub author_id: Uuid,  // Reference, not embed
    #[field(create, update, response)]
    pub title: String,
}

// Bad: Trying to embed relationships
pub struct User {
    pub id: Uuid,
    pub posts: Vec<Post>,  // Don't do this
}
```

### Use Meaningful Field Attributes

Be explicit about each field's purpose:

```rust
// Good: Clear intent
#[field(create, response)]      // Set once, always visible
pub email: String,

#[field(update, response)]      // Can change, always visible
pub display_name: Option<String>,

#[field(response)]              // Read-only, computed/managed elsewhere
pub post_count: i64,

#[field(skip)]                  // Never exposed
pub password_hash: String,

// Bad: Everything everywhere
#[field(create, update, response)]  // Is this really needed for all?
pub internal_id: String,
```

### Prefer Option for Nullable Fields

Match your database schema:

```rust
// Database: email VARCHAR NOT NULL
#[field(create, update, response)]
pub email: String,

// Database: bio TEXT NULL
#[field(update, response)]
pub bio: Option<String>,
```

## Security

### Always Use `#[field(skip)]` for Sensitive Data

```rust
// Passwords
#[field(skip)]
pub password_hash: String,

// API keys
#[field(skip)]
pub api_key: String,

// Internal tokens
#[field(skip)]
pub refresh_token: Option<String>,

// PII that shouldn't be in responses
#[field(skip)]
pub ssn: String,

// Internal audit data
#[field(skip)]
pub created_by_ip: String,
```

### Separate Internal and External Entities

For admin-only data, consider separate entities:

```rust
// Public entity
#[derive(Entity)]
#[entity(table = "users")]
pub struct User {
    #[id]
    pub id: Uuid,
    #[field(create, update, response)]
    pub name: String,
    #[field(skip)]
    pub admin_notes: Option<String>,
}

// Admin-only entity (same table, different view)
#[derive(Entity)]
#[entity(table = "users", sql = "trait")]
pub struct AdminUser {
    #[id]
    pub id: Uuid,
    #[field(response)]
    pub name: String,
    #[field(update, response)]  // Now visible and editable
    pub admin_notes: Option<String>,
    #[field(response)]
    pub last_login_ip: Option<String>,
}
```

## Performance

### Use `sql = "trait"` for Complex Queries

Don't fight the generated SQL. If you need joins or complex logic, implement it yourself:

```rust
// Simple CRUD - use full generation
#[entity(table = "categories", sql = "full")]

// Complex queries needed - implement yourself
#[entity(table = "posts", sql = "trait")]
```

### Batch Operations

For bulk inserts, implement custom methods:

```rust
#[entity(table = "events", sql = "trait")]
pub struct Event { /* ... */ }

pub trait EventBatchRepository {
    async fn create_batch(&self, events: Vec<CreateEventRequest>) -> Result<(), sqlx::Error>;
}

#[async_trait]
impl EventBatchRepository for PgPool {
    async fn create_batch(&self, events: Vec<CreateEventRequest>) -> Result<(), sqlx::Error> {
        let mut tx = self.begin().await?;

        for event in events {
            let entity = Event::from(event);
            let insertable = InsertableEvent::from(&entity);
            // Insert within transaction
        }

        tx.commit().await?;
        Ok(())
    }
}
```

### Avoid N+1 Queries

Use joins instead of loading related entities one by one:

```rust
// Bad: N+1 queries
let posts = pool.list(100, 0).await?;
for post in &posts {
    let author = pool.find_user_by_id(post.author_id).await?;  // N queries!
}

// Good: Single query with join
let posts_with_authors = pool.list_with_authors(100, 0).await?;  // 1 query
```

## Testing

### Use Separate Test Database

```rust
#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    async fn setup_test_db() -> PgPool {
        let url = std::env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL must be set");

        let pool = PgPool::connect(&url).await.unwrap();

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_create_user() {
        let pool = setup_test_db().await;

        let request = CreateUserRequest {
            username: "test_user".into(),
            email: "test@example.com".into(),
        };

        let user = pool.create(request).await.unwrap();
        assert_eq!(user.username, "test_user");
    }
}
```

### Test DTOs Separately

```rust
#[test]
fn test_user_response_excludes_password() {
    let user = User {
        id: Uuid::new_v4(),
        username: "test".into(),
        email: "test@example.com".into(),
        password_hash: "secret_hash".into(),
        created_at: Utc::now(),
    };

    let response = UserResponse::from(&user);

    // password_hash is not in UserResponse
    assert_eq!(response.username, "test");
    // No way to access password_hash through response
}

#[test]
fn test_update_request_is_partial() {
    let update = UpdateUserRequest {
        username: Some("new_name".into()),
        email: None,  // Not updating email
    };

    assert!(update.username.is_some());
    assert!(update.email.is_none());
}
```

## Schema Assertion

Generated SQL is correct by construction, but the *table* can drift (missed migration, manual ALTER, renamed column) — that surfaces as runtime decode errors. Every Postgres entity gets `{Entity}::SCHEMA` (declared columns: name, DDL type, nullability) and `{Entity}::assert_schema(pool)`, which compares the declaration against `information_schema.columns` and reports **all** drifts at once. Run one integration test per entity:

```rust
#[tokio::test]
async fn entities_match_database_schema() {
    let pool = test_pool().await;
    User::assert_schema(&pool).await.expect("users drifted");
    Order::assert_schema(&pool).await.expect("orders drifted");
}
```

Type comparison is family-based (`TEXT`/`VARCHAR`/`CHAR` fold together, arrays compare as arrays); Postgres enums (`USER-DEFINED`) skip the type check but keep presence + nullability guarantees.

## Project Organization

### Recommended Structure

```
src/
├── entities/           # Entity definitions
│   ├── mod.rs
│   ├── user.rs
│   ├── post.rs
│   └── comment.rs
├── repositories/       # Custom repository extensions
│   ├── mod.rs
│   └── post_search.rs
├── handlers/           # HTTP handlers
│   ├── mod.rs
│   ├── users.rs
│   └── posts.rs
├── services/           # Business logic
│   ├── mod.rs
│   └── auth.rs
└── main.rs
```

### Re-export Generated Types

```rust
// src/entities/mod.rs
mod user;
mod post;

pub use user::*;
pub use post::*;
```

### Group Related Entities

```rust
// src/entities/auth/mod.rs
mod user;
mod session;
mod api_key;

pub use user::*;
pub use session::*;
pub use api_key::*;
```

## Common Mistakes

### 1. Forgetting `#[field(skip)]` on Sensitive Fields

```rust
// Wrong: password_hash will be in Response!
pub struct User {
    pub password_hash: String,
}

// Right
#[field(skip)]
pub password_hash: String,
```

### 2. Using `sql = "full"` When You Need Joins

If you need related data, use `sql = "trait"` and implement yourself.

### 3. Not Handling Optional Updates

Remember: `UpdateRequest` fields are `Option<T>`. Check before applying:

```rust
// Generated UpdateUserRequest has Option<String> for name
// Your update logic should handle None (no change) vs Some (change)
```

### 4. Duplicating Business Logic

Put validation and business rules in a service layer, not in handlers:

```rust
// Good: Service layer
impl UserService {
    pub async fn create_user(&self, request: CreateUserRequest) -> Result<User, AppError> {
        self.validate_email(&request.email)?;
        self.check_username_available(&request.username).await?;
        self.pool.create(request).await.map_err(Into::into)
    }
}

// Bad: Logic scattered in handlers
pub async fn create_user(pool: State<PgPool>, request: Json<CreateUserRequest>) -> ... {
    // Validation here
    // Business rules here
    // Repository call here
    // All mixed together
}
```

## Checklist

Before deploying:

- [ ] All sensitive fields have `#[field(skip)]`
- [ ] DTOs match API contract expectations
- [ ] Complex queries use `sql = "trait"`
- [ ] Integration tests cover repository methods
- [ ] Error handling is consistent
- [ ] Pagination is implemented for list endpoints
- [ ] Database indexes exist for query patterns
