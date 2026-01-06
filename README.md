<a id="top"></a>

<p align="center">
  <h1 align="center">entity-derive</h1>
  <p align="center">
    <strong>One macro to rule them all</strong>
  </p>
  <p align="center">
    Generate DTOs, repositories, mappers, and SQL from a single entity definition
  </p>
</p>

<p align="center">
  <a href="https://crates.io/crates/entity-derive">
    <img src="https://img.shields.io/crates/v/entity-derive.svg?style=for-the-badge" alt="Crates.io"/>
  </a>
  <a href="https://docs.rs/entity-derive">
    <img src="https://img.shields.io/docsrs/entity-derive?style=for-the-badge" alt="Documentation"/>
  </a>
</p>

<p align="center">
  <a href="https://github.com/RAprogramm/entity-derive/actions">
    <img src="https://img.shields.io/github/actions/workflow/status/RAprogramm/entity-derive/ci.yml?style=for-the-badge" alt="CI Status"/>
  </a>
  <a href="https://codecov.io/gh/RAprogramm/entity-derive">
    <img src="https://img.shields.io/codecov/c/github/RAprogramm/entity-derive?style=for-the-badge&token=HGuwZf0REV" alt="Coverage"/>
  </a>
</p>

<p align="center">
  <a href="https://github.com/RAprogramm/entity-derive/blob/main/LICENSES/MIT.txt">
    <img src="https://img.shields.io/badge/license-MIT-blue.svg?style=for-the-badge" alt="License: MIT"/>
  </a>
  <a href="https://api.reuse.software/info/github.com/RAprogramm/entity-derive">
    <img src="https://img.shields.io/reuse/compliance/github.com%2FRAprogramm%2Fentity-derive?style=for-the-badge" alt="REUSE Compliant"/>
  </a>
  <a href="https://github.com/RAprogramm/entity-derive/wiki">
    <img src="https://img.shields.io/badge/Wiki-Documentation-purple?style=for-the-badge&logo=github" alt="Wiki"/>
  </a>
</p>

---

## Table of Contents

- [The Problem](#the-problem)
- [The Solution](#the-solution)
- [Features](#features)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Attribute Reference](#attribute-reference)
  - [Custom Error Type](#custom-error-type)
  - [Soft Delete](#soft-delete)
  - [RETURNING Modes](#returning-modes)
  - [Query Filtering](#query-filtering)
  - [Projections](#projections)
  - [Relations](#relations)
  - [Lifecycle Events](#lifecycle-events)
  - [Lifecycle Hooks](#lifecycle-hooks)
  - [CQRS Commands](#cqrs-commands)
- [Generated Code](#generated-code)
- [Architecture](#architecture)
- [Comparison](#comparison)
- [Code Coverage](#code-coverage)

---

## The Problem

Building a typical CRUD application requires writing the same boilerplate over and over:

```rust,ignore
// 1. Your domain entity
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

// 2. DTO for creating (without id, without auto-generated fields)
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
}

// 3. DTO for updating (all fields optional for partial updates)
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
}

// 4. DTO for API response (without sensitive fields)
pub struct UserResponse {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

// 5. Database row struct
pub struct UserRow { /* ... */ }

// 6. Insertable struct
pub struct InsertableUser { /* ... */ }

// 7. Repository trait
pub trait UserRepository { /* ... */ }

// 8. SQL implementation
impl UserRepository for PgPool { /* ... */ }

// 9. Six From implementations for mapping between types
impl From<UserRow> for User { /* ... */ }
impl From<User> for UserResponse { /* ... */ }
// ... and more
```

**That's 200+ lines of boilerplate for a single entity.**

<div align="right"><a href="#top">⬆ back to top</a></div>

## The Solution

```rust,ignore
use entity_derive::Entity;

#[derive(Entity)]
#[entity(table = "users", schema = "core")]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,

    #[field(create, update, response)]
    pub email: String,

    #[field(skip)]
    pub password_hash: String,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,
}
```

**Done.** The macro generates everything else.

<div align="right"><a href="#top">⬆ back to top</a></div>

## Features

- **Zero Runtime Cost** — All code generation happens at compile time
- **Type Safe** — Change a field type once, everything updates automatically
- **Flexible Attributes** — Fine-grained control over what goes where
- **SQL Generation** — Complete CRUD operations for PostgreSQL (via sqlx)
- **Partial Updates** — Non-optional fields automatically wrapped in `Option` for updates
- **Security by Default** — `#[field(skip)]` ensures sensitive data never leaks to responses
- **Soft Delete** — Optional `deleted_at` timestamp instead of hard delete
- **Query Filtering** — Type-safe query objects with `#[filter]`, `#[filter(like)]`, `#[filter(range)]`
- **Projections** — Partial entity views with optimized SELECT queries
- **RETURNING Control** — Configure what data comes back from INSERT/UPDATE
- **Relations** — `#[belongs_to]` and `#[has_many]` for entity relationships
- **Lifecycle Events** — `Created`, `Updated`, `Deleted` events for audit logging
- **Lifecycle Hooks** — `before_create`, `after_update`, etc. for validation and side effects
- **CQRS Commands** — Business-oriented commands instead of generic CRUD

<div align="right"><a href="#top">⬆ back to top</a></div>

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
entity-derive = { version = "0.3", features = ["postgres"] }

# Required peer dependencies
uuid = { version = "1", features = ["v4", "v7"] }
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
async-trait = "0.1"

# For PostgreSQL support
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres"] }
```

### Available Features

| Feature | Description |
|---------|-------------|
| `postgres` | PostgreSQL support via sqlx (stable) |
| `clickhouse` | ClickHouse support (planned) |
| `mongodb` | MongoDB support (planned) |
| `api` | OpenAPI schema generation via utoipa |
| `validate` | Validation derives via validator |

<div align="right"><a href="#top">⬆ back to top</a></div>

## Quick Start

```rust,ignore
use entity_derive::Entity;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Entity)]
#[entity(table = "posts", schema = "blog")]
pub struct Post {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub title: String,

    #[field(create, update, response)]
    pub content: String,

    #[field(create, response)]
    pub author_id: Uuid,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,

    #[field(response)]
    #[auto]
    pub updated_at: DateTime<Utc>,
}

// Now you have:
// - CreatePostRequest { title, content, author_id }
// - UpdatePostRequest { title?, content? }
// - PostResponse { id, title, content, author_id, created_at, updated_at }
// - PostRow, InsertablePost
// - PostRepository trait
// - impl PostRepository for sqlx::PgPool
```

<div align="right"><a href="#top">⬆ back to top</a></div>

## Attribute Reference

### Entity-Level: `#[entity(...)]`

| Attribute | Required | Default | Description |
|-----------|----------|---------|-------------|
| `table` | Yes | — | Database table name |
| `schema` | No | `"public"` | Database schema |
| `sql` | No | `"full"` | SQL generation level |
| `dialect` | No | `"postgres"` | Database dialect |
| `uuid` | No | `"v7"` | UUID version for ID generation |
| `soft_delete` | No | `false` | Enable soft delete (uses `deleted_at` timestamp) |
| `returning` | No | `"full"` | RETURNING clause mode (`full`, `id`, `none`, or custom columns) |
| `error` | No | `sqlx::Error` | Custom error type for repository (must impl `From<sqlx::Error>`) |
| `events` | No | `false` | Generate lifecycle events enum |
| `hooks` | No | `false` | Generate lifecycle hooks trait |
| `commands` | No | `false` | Enable CQRS command pattern (use with `#[command(...)]`) |

#### Database Dialects

| Dialect | Alias | Client | Status |
|---------|-------|--------|--------|
| `postgres` | `pg`, `postgresql` | `sqlx::PgPool` | Stable |
| `clickhouse` | `ch` | `clickhouse::Client` | Planned |
| `mongodb` | `mongo` | `mongodb::Client` | Planned |

#### UUID Versions

| Version | Method | Properties |
|---------|--------|------------|
| `v7` | `Uuid::now_v7()` | Time-ordered, sortable (recommended for databases) |
| `v4` | `Uuid::new_v4()` | Random, widely compatible |

#### SQL Levels

| Level | Repository Trait | PgPool Impl | Use Case |
|-------|-----------------|-------------|----------|
| `full` | Yes | Yes | Simple entities with standard CRUD |
| `trait` | Yes | No | Custom queries (joins, CTEs, full-text search) |
| `none` | No | No | DTOs only, no database layer |

#### Custom Error Type

Use a custom error type instead of `sqlx::Error`:

```rust,ignore
#[derive(Debug)]
pub enum AppError {
    Database(sqlx::Error),
    NotFound,
    Validation(String),
}

impl std::error::Error for AppError {}
impl std::fmt::Display for AppError { /* ... */ }

// Required: convert from sqlx::Error
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err)
    }
}

#[derive(Entity)]
#[entity(table = "users", error = "AppError")]
pub struct User {
    #[id]
    pub id: Uuid,
    // ...
}

// Generated repository uses AppError:
// impl UserRepository for PgPool {
//     type Error = AppError;
//     ...
// }
```

#### Soft Delete

Enable soft delete to mark records as deleted instead of removing them:

```rust,ignore
#[derive(Entity)]
#[entity(table = "documents", soft_delete)]
pub struct Document {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub title: String,

    #[field(skip)]
    pub deleted_at: Option<DateTime<Utc>>,  // Required for soft delete
}

// Generated methods:
// - delete() sets deleted_at = NOW() instead of DELETE
// - find_by_id() and list() automatically filter deleted records
// - hard_delete() permanently removes the record
// - restore() sets deleted_at = NULL
// - find_by_id_with_deleted() and list_with_deleted() include deleted records
```

#### RETURNING Modes

Control what data is fetched back after INSERT/UPDATE:

| Mode | Clause | Use Case |
|------|--------|----------|
| `full` | `RETURNING *` | Get all fields including DB-generated (default) |
| `id` | `RETURNING id` | Confirm insert, return pre-built entity |
| `none` | (no RETURNING) | Fire-and-forget, fastest option |
| `"col1, col2"` | `RETURNING col1, col2` | Return specific columns |

```rust,ignore
#[entity(table = "logs", returning = "none")]      // Fastest
#[entity(table = "users", returning = "full")]     // Get DB-generated values
#[entity(table = "events", returning = "id, created_at")]  // Custom columns
```

#### Query Filtering

Generate type-safe query structs for filtering entities:

```rust,ignore
#[derive(Entity)]
#[entity(table = "products")]
pub struct Product {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    #[filter]                          // Exact match: WHERE name = $n
    pub name: String,

    #[field(create, update, response)]
    #[filter(like)]                    // Pattern match: WHERE description ILIKE $n
    pub description: String,

    #[field(create, update, response)]
    #[filter(range)]                   // Range: WHERE price >= $n AND price <= $m
    pub price: i64,

    #[field(response)]
    #[auto]
    #[filter(range)]                   // Date range filtering
    pub created_at: DateTime<Utc>,
}

// Generated ProductQuery struct:
// pub struct ProductQuery {
//     pub name: Option<String>,
//     pub description: Option<String>,
//     pub price_from: Option<i64>,
//     pub price_to: Option<i64>,
//     pub created_at_from: Option<DateTime<Utc>>,
//     pub created_at_to: Option<DateTime<Utc>>,
//     pub limit: Option<i64>,
//     pub offset: Option<i64>,
// }

// Usage:
let query = ProductQuery {
    name: Some("Widget".to_string()),
    price_from: Some(100),
    price_to: Some(500),
    limit: Some(20),
    ..Default::default()
};
let products = repo.query(query).await?;
```

| Filter Type | Attribute | SQL Generated |
|-------------|-----------|---------------|
| Exact match | `#[filter]` or `#[filter(eq)]` | `WHERE field = $n` |
| Pattern match | `#[filter(like)]` | `WHERE field ILIKE $n` |
| Range | `#[filter(range)]` | `WHERE field >= $n AND field <= $m` |

### Field-Level Attributes

| Attribute | Effect |
|-----------|--------|
| `#[id]` | Primary key, auto-generated UUID (v7 by default, configurable with `uuid` attribute), always in response |
| `#[auto]` | Auto-generated field (timestamps), excluded from create/update |
| `#[field(create)]` | Include in `CreateRequest` |
| `#[field(update)]` | Include in `UpdateRequest` (wrapped in `Option` if not already) |
| `#[field(response)]` | Include in `Response` |
| `#[field(skip)]` | Exclude from all DTOs (for sensitive data) |
| `#[belongs_to(Entity)]` | Foreign key relation, generates `find_{entity}` method in repository |
| `#[has_many(Entity)]` | One-to-many relation (entity-level), generates `find_{entities}` method |
| `#[projection(Name: fields)]` | Generate partial view struct (entity-level) |
| `#[filter]` | Exact match filter, generates field in Query struct |
| `#[filter(like)]` | ILIKE pattern filter for text search |
| `#[filter(range)]` | Range filter, generates `field_from` and `field_to` fields |
| `#[command(Name)]` | Define a command (entity-level, requires `commands` in entity) |

Combine multiple: `#[field(create, update, response)]`

### Projections

Define partial views of your entity for optimized queries:

```rust,ignore
#[derive(Entity)]
#[entity(table = "users")]
#[projection(Public: id, name, avatar)]
#[projection(Admin: id, name, email, role, created_at)]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,

    #[field(create, response)]
    pub email: String,

    #[field(update, response)]
    pub avatar: Option<String>,

    #[field(response)]
    pub role: String,

    #[field(skip)]
    pub password_hash: String,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,
}

// Generated:
// - UserPublic { id, name, avatar }
// - UserAdmin { id, name, email, role, created_at }
// - From<User> for UserPublic
// - From<User> for UserAdmin
// - find_by_id_public() - optimized SELECT with only needed columns
// - find_by_id_admin() - optimized SELECT with only needed columns
```

### Example with All Options

```rust,ignore
#[derive(Entity)]
#[entity(
    table = "sessions",
    schema = "auth",
    sql = "full",
    dialect = "postgres",
    uuid = "v4"  // Use random UUID instead of time-ordered
)]
pub struct Session {
    #[id]
    pub id: Uuid,
    // ...
}
```

### Relations

Use `#[belongs_to]` for foreign keys and `#[has_many]` for one-to-many relations:

```rust,ignore
// Parent entity with has_many
#[derive(Entity)]
#[entity(table = "users")]
#[has_many(Post)]  // One-to-many: User has many Posts
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub name: String,
}

// Child entity with belongs_to
#[derive(Entity)]
#[entity(table = "posts")]
pub struct Post {
    #[id]
    pub id: Uuid,

    #[belongs_to(User)]  // Foreign key to User
    pub user_id: Uuid,

    #[field(create, update, response)]
    pub title: String,
}

// Generated UserRepository includes:
// async fn find_posts(&self, user_id: Uuid) -> Result<Vec<Post>, Self::Error>;

// Generated PostRepository includes:
// async fn find_user(&self, id: Uuid) -> Result<Option<User>, Self::Error>;
```

<div align="right"><a href="#top">⬆ back to top</a></div>

### Lifecycle Events

Generate domain events for entity lifecycle changes:

```rust,ignore
#[derive(Entity)]
#[entity(table = "orders", events)]
pub struct Order {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub customer_id: Uuid,

    #[field(create, update, response)]
    pub status: String,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,
}

// Generated OrderEvent enum:
// pub enum OrderEvent {
//     Created(Order),
//     Updated { id: Uuid, changes: UpdateOrderRequest },
//     Deleted(Uuid),
// }

// Usage with event bus:
async fn create_order(repo: &impl OrderRepository, bus: &impl EventBus) {
    let order = repo.create(dto).await?;
    bus.publish(OrderEvent::Created(order.clone())).await;
}
```

Events enable:
- **Audit logging** — Track all entity changes
- **Event sourcing** — Reconstruct state from events
- **Integration** — Publish to message queues (Kafka, RabbitMQ)

<div align="right"><a href="#top">⬆ back to top</a></div>

### Lifecycle Hooks

Execute custom logic before/after entity operations:

```rust,ignore
#[derive(Entity)]
#[entity(table = "users", hooks)]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub email: String,

    #[field(create, response)]
    pub name: String,
}

// Generated UserHooks trait:
#[async_trait]
pub trait UserHooks: Send + Sync {
    type Error: std::error::Error + Send + Sync;

    async fn before_create(&self, dto: &mut CreateUserRequest) -> Result<(), Self::Error>;
    async fn after_create(&self, entity: &User) -> Result<(), Self::Error>;
    async fn before_update(&self, id: &Uuid, dto: &mut UpdateUserRequest) -> Result<(), Self::Error>;
    async fn after_update(&self, entity: &User) -> Result<(), Self::Error>;
    async fn before_delete(&self, id: &Uuid) -> Result<(), Self::Error>;
    async fn after_delete(&self, id: &Uuid) -> Result<(), Self::Error>;
}

// Implementation example:
struct UserService { db: PgPool, cache: Redis }

#[async_trait]
impl UserHooks for UserService {
    type Error = AppError;

    async fn before_create(&self, dto: &mut CreateUserRequest) -> Result<(), Self::Error> {
        // Normalize email
        dto.email = dto.email.to_lowercase();
        Ok(())
    }

    async fn after_delete(&self, id: &Uuid) -> Result<(), Self::Error> {
        // Invalidate cache
        self.cache.del(&format!("user:{}", id)).await?;
        Ok(())
    }
    // ... other hooks with default Ok(()) implementations
}
```

Hooks enable:
- **Validation** — Reject invalid data before persistence
- **Normalization** — Transform data (lowercase email, trim whitespace)
- **Side effects** — Cache invalidation, notifications, audit logs
- **Authorization** — Check permissions before operations

<div align="right"><a href="#top">⬆ back to top</a></div>

### CQRS Commands

Define business-oriented commands instead of generic CRUD:

```rust,ignore
#[derive(Entity)]
#[entity(table = "users", commands)]
#[command(Register)]                                // Uses create fields
#[command(UpdateEmail: email)]                      // Specific fields only
#[command(Deactivate, requires_id)]                 // ID-only command
#[command(Transfer, payload = "TransferPayload", result = "TransferResult")]  // Custom types
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub email: String,

    #[field(create, response)]
    pub name: String,
}

// Generated command structs:
pub struct RegisterUser { pub email: String, pub name: String }
pub struct UpdateEmailUser { pub id: Uuid, pub email: String }
pub struct DeactivateUser { pub id: Uuid }

// Generated command enum:
pub enum UserCommand {
    Register(RegisterUser),
    UpdateEmail(UpdateEmailUser),
    Deactivate(DeactivateUser),
    Transfer(TransferPayload),
}

// Generated result enum:
pub enum UserCommandResult {
    Register(User),
    UpdateEmail(User),
    Deactivate,
    Transfer(TransferResult),
}

// Generated handler trait:
#[async_trait]
pub trait UserCommandHandler: Send + Sync {
    type Error: std::error::Error + Send + Sync;
    type Context: Send + Sync;

    async fn handle(&self, cmd: UserCommand, ctx: &Self::Context)
        -> Result<UserCommandResult, Self::Error>;

    async fn handle_register(&self, cmd: RegisterUser, ctx: &Self::Context)
        -> Result<User, Self::Error>;
    async fn handle_update_email(&self, cmd: UpdateEmailUser, ctx: &Self::Context)
        -> Result<User, Self::Error>;
    async fn handle_deactivate(&self, cmd: DeactivateUser, ctx: &Self::Context)
        -> Result<(), Self::Error>;
}
```

| Command Syntax | Effect |
|----------------|--------|
| `#[command(Name)]` | Uses all `#[field(create)]` fields |
| `#[command(Name: field1, field2)]` | Uses only specified fields (adds `requires_id`) |
| `#[command(Name, requires_id)]` | Adds ID field, no other fields |
| `#[command(Name, source = "create")]` | Explicitly use create fields (default) |
| `#[command(Name, source = "update")]` | Use update fields (optional, adds `requires_id`) |
| `#[command(Name, source = "none")]` | No payload fields |
| `#[command(Name, payload = "Type")]` | Uses custom payload struct |
| `#[command(Name, result = "Type")]` | Uses custom result type |
| `#[command(Name, kind = "create")]` | Hint: creates entity (default) |
| `#[command(Name, kind = "update")]` | Hint: modifies entity |
| `#[command(Name, kind = "delete")]` | Hint: removes entity (returns `()`) |
| `#[command(Name, kind = "custom")]` | Hint: custom operation |

#### EntityCommand Trait

All command enums implement the `EntityCommand` trait:

```rust,ignore
use entity_derive::{EntityCommand, CommandKind};

// Check command metadata
let cmd = UserCommand::Register(register_data);
assert_eq!(cmd.name(), "Register");
assert!(matches!(cmd.kind(), CommandKind::Create));
```

#### Command Hooks

When `commands` and `hooks` are both enabled, additional hooks are generated:

```rust,ignore
#[derive(Entity)]
#[entity(table = "orders", commands, hooks)]
#[command(Place)]
#[command(Cancel, requires_id)]
pub struct Order { ... }

// Generated OrderHooks trait includes:
#[async_trait]
pub trait OrderHooks: Send + Sync {
    type Error: std::error::Error + Send + Sync;

    // Standard CRUD hooks...
    async fn before_create(&self, dto: &mut CreateOrderRequest) -> Result<(), Self::Error>;
    async fn after_create(&self, entity: &Order) -> Result<(), Self::Error>;
    // ...

    // Command-specific hooks
    async fn before_command(&self, cmd: &OrderCommand) -> Result<(), Self::Error>;
    async fn after_command(&self, cmd: &OrderCommand, result: &OrderCommandResult) -> Result<(), Self::Error>;
}
```

Commands enable:
- **Domain language** — `RegisterUser` instead of `CreateUserRequest`
- **Explicit intent** — Each command has a clear business purpose
- **CQRS pattern** — Separate read and write models
- **Command hooks** — `before_command` / `after_command` when combined with `hooks`

<div align="right"><a href="#top">⬆ back to top</a></div>

## Generated Code

For a `User` entity, the macro generates:

### DTOs

```rust,ignore
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
}
```

### Repository Trait

```rust,ignore
#[async_trait]
pub trait UserRepository: Send + Sync {
    type Error: std::error::Error + Send + Sync;

    async fn create(&self, dto: CreateUserRequest) -> Result<User, Self::Error>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, Self::Error>;
    async fn update(&self, id: Uuid, dto: UpdateUserRequest) -> Result<User, Self::Error>;
    async fn delete(&self, id: Uuid) -> Result<bool, Self::Error>;
    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<User>, Self::Error>;
}
```

### SQL Implementation

```rust,ignore
#[async_trait]
impl UserRepository for sqlx::PgPool {
    type Error = sqlx::Error;

    async fn create(&self, dto: CreateUserRequest) -> Result<User, Self::Error> {
        let entity = User::from(dto);
        let insertable = InsertableUser::from(&entity);
        sqlx::query(
            "INSERT INTO core.users (id, name, email, password_hash, created_at) \
             VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(insertable.id)
        .bind(&insertable.name)
        .bind(&insertable.email)
        .bind(&insertable.password_hash)
        .bind(insertable.created_at)
        .execute(self)
        .await?;
        Ok(entity)
    }

    // ... find_by_id, update, delete, list
}
```

### Mappers

```rust,ignore
impl From<UserRow> for User { /* ... */ }
impl From<CreateUserRequest> for User { /* ... */ }
impl From<User> for UserResponse { /* ... */ }
impl From<&User> for InsertableUser { /* ... */ }
// ... and more
```

<div align="right"><a href="#top">⬆ back to top</a></div>

## Architecture

```text
┌─────────────────────────────────────────────────────────────┐
│                     Your Code                               │
│  #[derive(Entity)]                                          │
│  pub struct User { ... }                                    │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   entity-derive                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Parser    │  │ Generators  │  │      Output         │  │
│  │             │  │             │  │                     │  │
│  │ EntityDef   │─>│ dto.rs      │─>│ CreateRequest       │  │
│  │ FieldDef    │  │ row.rs      │  │ UpdateRequest       │  │
│  │ SqlLevel    │  │ repository  │  │ Response            │  │
│  │             │  │ sql.rs      │  │ Row, Insertable     │  │
│  │             │  │ mappers.rs  │  │ Repository trait    │  │
│  │             │  │             │  │ PgPool impl         │  │
│  │             │  │             │  │ From impls          │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

<div align="right"><a href="#top">⬆ back to top</a></div>

## Comparison

| Aspect | Without entity-derive | With entity-derive |
|--------|----------------------|-------------------|
| Lines of code | 200+ per entity | ~15 per entity |
| Type safety | Manual sync required | Automatic |
| Sensitive data leaks | Possible | Prevented by `#[field(skip)]` |
| Partial updates | Manual wrapping | Automatic |
| SQL bindings | Error-prone | Always in sync |
| Refactoring | Update 8+ places | Update 1 place |

<div align="right"><a href="#top">⬆ back to top</a></div>

## Code Coverage

We maintain high test coverage to ensure reliability. Below are visual representations of our codebase coverage:

### Sunburst

The inner circle represents the entire project. Moving outward: folders, then individual files. Size = number of statements, color = coverage percentage.

<p align="center">
  <a href="https://codecov.io/gh/RAprogramm/entity-derive">
    <img src="https://codecov.io/gh/RAprogramm/entity-derive/graphs/sunburst.svg?token=HGuwZf0REV" alt="Coverage Sunburst"/>
  </a>
</p>

### Grid

Each block represents a file. Size = number of statements, color = coverage level (green = high, red = low).

<p align="center">
  <a href="https://codecov.io/gh/RAprogramm/entity-derive">
    <img src="https://codecov.io/gh/RAprogramm/entity-derive/graphs/tree.svg?token=HGuwZf0REV" alt="Coverage Grid"/>
  </a>
</p>

### Icicle

Hierarchical view: top = entire project, descending through folders to individual files. Size and color represent statements and coverage.

<p align="center">
  <a href="https://codecov.io/gh/RAprogramm/entity-derive">
    <img src="https://codecov.io/gh/RAprogramm/entity-derive/graphs/icicle.svg?token=HGuwZf0REV" alt="Coverage Icicle"/>
  </a>
</p>

<div align="right"><a href="#top">⬆ back to top</a></div>
