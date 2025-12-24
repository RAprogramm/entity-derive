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
- [Generated Code](#generated-code)
- [Architecture](#architecture)
- [Comparison](#comparison)
- [Code Coverage](#code-coverage)
- [Documentation](#documentation)
- [MSRV](#msrv)
- [License](#license)
- [Contributing](#contributing)

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

<div align="right"><a href="#top">⬆ back to top</a></div>

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
entity-derive = "0.1"

# Required peer dependencies
uuid = { version = "1", features = ["v7"] }
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
async-trait = "0.1"

# For database support
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres"] }
```

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

#### SQL Levels

| Level | Repository Trait | PgPool Impl | Use Case |
|-------|-----------------|-------------|----------|
| `full` | Yes | Yes | Simple entities with standard CRUD |
| `trait` | Yes | No | Custom queries (joins, CTEs, full-text search) |
| `none` | No | No | DTOs only, no database layer |

### Field-Level Attributes

| Attribute | Effect |
|-----------|--------|
| `#[id]` | Primary key, auto-generated UUID (v7), always in response |
| `#[auto]` | Auto-generated field (timestamps), excluded from create/update |
| `#[field(create)]` | Include in `CreateRequest` |
| `#[field(update)]` | Include in `UpdateRequest` (wrapped in `Option` if not already) |
| `#[field(response)]` | Include in `Response` |
| `#[field(skip)]` | Exclude from all DTOs (for sensitive data) |

Combine multiple: `#[field(create, update, response)]`

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
