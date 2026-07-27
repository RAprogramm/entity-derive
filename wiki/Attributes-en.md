# Attributes Reference
Complete guide to all attributes supported by `entity-derive`.
## Entity-Level Attributes

Applied to the struct with `#[entity(...)]`:

```rust
#[derive(Entity)]
#[entity(
    table = "users",
    schema = "core",
    sql = "full",
    dialect = "postgres",
    uuid = "v7",
    soft_delete,
    returning = "full",
    error = "AppError",
    events,
    hooks,
    commands,
    transactions
)]
pub struct User { /* ... */ }
```

### Quick Reference

| Attribute | Required | Default | Description |
|-----------|----------|---------|-------------|
| `table` | Yes | — | Database table name |
| `schema` | No | `"public"` | Database schema |
| `sql` | No | `"full"` | SQL generation level |
| `dialect` | No | `"postgres"` | Database dialect |
| `uuid` | No | `"v7"` | UUID version for ID generation |
| `soft_delete` | No | `false` | Enable soft delete |
| `returning` | No | `"full"` | RETURNING clause mode |
| `upsert(...)` | No | — | Generate `INSERT ... ON CONFLICT` upsert method |
| `api(guard = "...")` | No | — | Enforced `FromRequestParts` guard in generated handlers |
| `error` | No | `sqlx::Error` | Custom error type |
| `events` | No | `false` | Generate lifecycle events |
| `hooks` | No | `false` | Generate lifecycle hooks trait |
| `commands` | No | `false` | Enable CQRS command pattern |
| `transactions` | No | `false` | Generate transaction repository adapter |

### `table` (required)

Database table name.

```rust
#[entity(table = "users")]           // → FROM users
#[entity(table = "user_profiles")]   // → FROM user_profiles
```

### `schema` (optional)

Database schema. Default: `"public"`.

```rust
#[entity(table = "users")]                    // → FROM public.users
#[entity(table = "users", schema = "core")]   // → FROM core.users
#[entity(table = "users", schema = "auth")]   // → FROM auth.users
```

### `sql` (optional)

SQL generation level. Default: `"full"`.

| Value | Repository Trait | PgPool Implementation | Use Case |
|-------|-----------------|----------------------|----------|
| `"full"` | Yes | Yes | Standard CRUD entities |
| `"trait"` | Yes | No | Custom queries (joins, CTEs) |
| `"none"` | No | No | DTOs only, no database |

```rust
#[entity(table = "users", sql = "full")]   // Full automation (default)
#[entity(table = "users", sql = "trait")]  // Only trait, implement SQL yourself
#[entity(table = "users", sql = "none")]   // No database layer at all
```

### `dialect` (optional)

Database dialect for SQL generation. Default: `"postgres"`.

| Dialect | Aliases | Client Type | Status |
|---------|---------|-------------|--------|
| `"postgres"` | `"pg"`, `"postgresql"` | `sqlx::PgPool` | Stable |
| `"clickhouse"` | `"ch"` | `clickhouse::Client` | Not implemented — compile error |
| `"mongodb"` | `"mongo"` | `mongodb::Client` | Not implemented — compile error |

### `uuid` (optional)

UUID version for auto-generated primary keys. Default: `"v7"`.

| Version | Method | Properties |
|---------|--------|------------|
| `"v7"` | `Uuid::now_v7()` | Time-ordered, sortable (recommended) |
| `"v4"` | `Uuid::new_v4()` | Random, widely compatible |

```rust
#[entity(table = "users", uuid = "v7")]     // Time-ordered (default)
#[entity(table = "sessions", uuid = "v4")]  // Random UUID
```

**Why UUID v7?**
- Time-ordered: natural sorting by creation time
- Better database index performance
- No coordination required (unlike sequences)
- Globally unique across distributed systems

### `soft_delete` (optional)

Enable soft delete to mark records as deleted instead of removing them.

```rust
#[derive(Entity)]
#[entity(table = "documents", soft_delete)]
pub struct Document {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub title: String,

    #[field(skip)]
    pub deleted_at: Option<DateTime<Utc>>,  // Required field
}
```

**Generated methods:**
- `delete()` — Sets `deleted_at = NOW()` instead of DELETE
- `hard_delete()` — Permanently removes the record
- `restore()` — Sets `deleted_at = NULL`
- `find_by_id()` / `list()` — Automatically filter deleted records
- `find_by_id_with_deleted()` / `list_with_deleted()` — Include deleted records

### `returning` (optional)

Control what data is fetched back after INSERT/UPDATE. Default: `"full"`.

| Mode | SQL Clause | Use Case |
|------|------------|----------|
| `"full"` | `RETURNING *` | Get all fields including DB-generated (default) |
| `"id"` | `RETURNING id` | Confirm insert, return pre-built entity |
| `"none"` | (no RETURNING) | Fire-and-forget, fastest option |
| `"col1, col2"` | `RETURNING col1, col2` | Return specific columns |

```rust
#[entity(table = "logs", returning = "none")]              // Fastest
#[entity(table = "users", returning = "full")]             // Get DB-generated values
#[entity(table = "events", returning = "id, created_at")]  // Custom columns
```

### `events(outbox)` (optional)

Durable event delivery via a transactional outbox. Plain `events` only generates the enum; with `streams`, NOTIFY is fire-and-forget. `events(outbox)` makes every generated write insert the serialized event into the `entity_outbox` table in the same transaction, and the `OutboxDrainer` runtime (entity-core, feature `outbox`) delivers rows with `FOR UPDATE SKIP LOCKED`, exponential backoff and parking after `max_attempts`. At-least-once — handlers must be idempotent. Composes with `streams`.

```rust
#[derive(Entity, Serialize, Deserialize)]
#[entity(table = "orders", events(outbox), migrations)]
pub struct Order { /* ... */ }

sqlx::query(Order::MIGRATION_OUTBOX).execute(&pool).await?;

struct Notifier;

#[async_trait::async_trait]
impl entity_core::outbox::OutboxHandler for Notifier {
    type Error = anyhow::Error;
    async fn handle(&self, row: &OutboxRow) -> Result<(), Self::Error> {
        deliver(&row.entity, &row.payload).await
    }
}

entity_core::outbox::OutboxDrainer::new(pool, Notifier).run().await;
```

### `upsert(...)` (optional)

Generate an `upsert` repository method backed by `INSERT ... ON CONFLICT`.

```rust
#[derive(Entity)]
#[entity(table = "users", upsert(conflict = "email"))]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    #[column(unique)]
    pub email: String,

    #[field(create, update, response)]
    pub name: String,
}
```

| Option | Required | Default | Description |
|--------|----------|---------|-------------|
| `conflict` | Yes | — | Comma-separated conflict target columns |
| `action` | No | `"update"` | `"update"` (DO UPDATE) or `"nothing"` (DO NOTHING) |

**Generated:**
- `action = "update"` → `async fn upsert(&self, dto: CreateUserRequest) -> Result<User, Error>` — overwrites the non-conflict `#[field(update)]` columns (`DO UPDATE SET col = EXCLUDED.col`) and returns the persisted row; columns not marked updatable keep their stored values on conflict
- `action = "nothing"` → `async fn upsert(&self, dto: CreateUserRequest) -> Result<Option<User>, Error>` — keeps the existing row untouched; `None` means a conflicting row already existed

**Compile-time validation:**
- conflict columns must exist and carry a uniqueness guarantee (`#[id]`, `#[column(unique)]` or a matching `unique_index(...)`)
- requires `returning = "full"` (the default)
- `action = "update"` needs at least one non-conflict `#[field(update)]` column

With `streams` enabled, upsert publishes a `Created` notification for every row it returns.

### `api(guard = "...")` (optional)

Enforce authentication in generated handlers. `security = "..."` only documents auth in OpenAPI; `guard` injects an axum `FromRequestParts` extractor as a leading argument of every generated CRUD and command handler, so a failed extraction rejects the request before the handler body runs.

```rust
pub struct RequireAuth;

impl<S: Send + Sync> FromRequestParts<S> for RequireAuth {
    type Rejection = StatusCode;
    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        parts.headers.contains_key("authorization")
            .then_some(Self)
            .ok_or(StatusCode::UNAUTHORIZED)
    }
}

#[derive(Entity)]
#[entity(table = "users", api(tag = "Users", handlers, guard = "RequireAuth", guard(list = "none")))]
pub struct User { /* ... */ }
```

Per-operation overrides: `guard(create = "Admin", list = "none", ...)` with operations `create`, `get`, `update`, `delete`, `list`, `commands`; the literal `"none"` disables the guard. Commands listed in `public = [...]` never receive a guard.

### `error` (optional)

Custom error type for repository. Default: `sqlx::Error`.

```rust
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
pub struct User { /* ... */ }

// Generated repository uses AppError:
// impl UserRepository for PgPool {
//     type Error = AppError;
//     ...
// }
```

### `events` (optional)

Generate lifecycle events enum. See [[Events]] for details.

```rust
#[entity(table = "orders", events)]
```

**Generated:**
```rust
pub enum OrderEvent {
    Created(Order),
    Updated { id: Uuid, changes: UpdateOrderRequest },
    Deleted(Uuid),
}
```

### `hooks` (optional)

Generate lifecycle hooks trait. See [[Hooks]] for details.

```rust
#[entity(table = "users", hooks)]
```

**Generated:**
```rust
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
```

### `commands` (optional)

Enable CQRS command pattern. See [[Commands]] for details.

```rust
#[entity(table = "users", commands)]
#[command(Register)]
#[command(Deactivate, requires_id)]
```

### `transactions` (optional)

Generate transaction repository adapter for type-safe multi-entity transactions.

```rust
#[derive(Entity)]
#[entity(table = "accounts", transactions)]
pub struct Account {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub balance: i64,
}
```

**Generated:**
- `AccountTransactionRepo<'t>` — Repository adapter for transaction context
- `TransactionWithAccount` trait with `with_accounts()` method

**Usage:**
```rust
use entity_core::prelude::*;

async fn transfer(pool: &PgPool, from: Uuid, to: Uuid, amount: i64) -> Result<(), AppError> {
    Transaction::new(pool)
        .with_accounts()
        .run(|mut ctx| async move {
            let from_acc = ctx.accounts().find_by_id(from).await?
                .ok_or(AppError::NotFound)?;

            ctx.accounts().update(from, UpdateAccountRequest {
                balance: Some(from_acc.balance - amount),
            }).await?;

            ctx.accounts().update(to, UpdateAccountRequest {
                balance: Some(to_acc.balance + amount),
            }).await?;

            Ok(())
        })
        .await
}
```

**Transaction methods:**
- `create(dto)` — Create entity within transaction
- `find_by_id(id)` — Find entity by ID
- `update(id, dto)` — Update entity
- `delete(id)` — Delete entity (respects `soft_delete`)
- `list(limit, offset)` — List entities

**Features:**
- Automatic rollback on error or panic
- Type-safe builder pattern
- Full CRUD support within transaction

## Field-Level Attributes

Applied to individual fields.

### `#[id]`

Marks the primary key field.

**Behavior:**
- Auto-generates UUID (v7 by default, configurable with `uuid` attribute)
- Always included in `Response` DTO
- Excluded from `CreateRequest` and `UpdateRequest`

```rust
#[id]
pub id: Uuid,
```

### `#[auto]`

Marks auto-generated fields (timestamps, sequences).

**Behavior:**
- Gets `Default::default()` in `From<CreateRequest>`
- Excluded from `CreateRequest` and `UpdateRequest`
- Can be included in `Response` with `#[field(response)]`
- Excluded from the generated `INSERT`, so with `migrations` a
  non-nullable temporal column receives a matching database default:
  `NOW()` for `TIMESTAMPTZ` and `TIMESTAMP`, `CURRENT_DATE` for `DATE`,
  `CURRENT_TIME` for `TIME`. An explicit `#[column(default = "...")]`
  overrides it

```rust
#[auto]
#[field(response)]
pub created_at: DateTime<Utc>,
```

### `#[field(...)]`

Controls DTO inclusion. Combine multiple options:

```rust
#[field(create)]                    // Only in CreateRequest
#[field(update)]                    // Only in UpdateRequest
#[field(response)]                  // Only in Response
#[field(create, response)]          // In Create and Response
#[field(create, update, response)]  // In all three
#[field(skip)]                      // Excluded from all DTOs
```

#### `create`

Include field in `CreateRequest` DTO.

```rust
#[field(create)]
pub email: String,

// Generated:
pub struct CreateUserRequest {
    pub email: String,
}
```

#### `update`

Include field in `UpdateRequest` DTO.

**Important:** Non-optional fields are automatically wrapped in `Option<T>` for partial updates.

```rust
#[field(update)]
pub name: String,  // Not Option

// Generated:
pub struct UpdateUserRequest {
    pub name: Option<String>,  // Wrapped automatically
}
```

#### `response`

Include field in `Response` DTO.

```rust
#[field(response)]
pub email: String,

// Generated:
pub struct UserResponse {
    pub id: Uuid,        // Always included (has #[id])
    pub email: String,   // Included
}
```

#### `skip`

Exclude field from **all** DTOs. Use for sensitive data.

```rust
#[field(skip)]
pub password_hash: String,
```

**Important:** `skip` overrides all other field options. The field will only exist in:
- Original entity struct
- `Row` struct (for database reads)
- `Insertable` struct (for database writes)

### `#[column(pg_enum = "...")]`

Wire a `ValueObject` Postgres enum into DDL generation.

```rust
#[derive(ValueObject, Debug, Clone, Serialize, Deserialize)]
#[value_object(pg_type = "order_status", sqlx)]
pub enum OrderStatus { Pending, Shipped, Delivered }

#[derive(Entity)]
#[entity(table = "orders", migrations)]
pub struct Order {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    #[column(pg_enum = "order_status")]
    pub status: OrderStatus,
}

for ddl in Order::MIGRATION_TYPES {
    sqlx::query(ddl).execute(&pool).await?;
}
sqlx::query(Order::MIGRATION_UP).execute(&pool).await?;
```

- Sets the DDL column type (enum fields otherwise fall back to TEXT)
- Registers the enum's idempotent `PG_CREATE_TYPE` DDL in `{Entity}::MIGRATION_TYPES` — run those before `MIGRATION_UP`
- The declared name is checked against the enum's `PG_TYPE` constant at compile time; a mismatch fails the build
- The `ValueObject`'s opt-in `sqlx` flag emits `sqlx::Type` / `Encode` / `Decode` impls; omit it if you already derive `sqlx::Type`

### `#[column(ci)]`

Case-insensitive text column. Generated `find_by_{field}` / `exists_by_{field}` compare via `LOWER(col) = LOWER($1)`, and `Option<T>` fields unwrap to `T` in the lookup signature (a NULL column never matches a probe).

```rust
#[field(create, update, response)]
#[column(unique, ci)]
pub username: Option<String>,

let user = pool.find_by_username(handle).await?;   // handle: String
let taken = pool.exists_by_username(handle).await?;
```

- With `migrations`, `unique + ci` emits `CREATE UNIQUE INDEX {table}_{column}_lower_key ON {table} (LOWER({column}))` instead of an inline `UNIQUE` constraint
- With `typed_constraints`, violations of that index resolve to the field like any other unique constraint

### `#[scope(...)]`

Entity-level. Declares a listing over an OR group of columns holding the same kind of value — "rows where this principal takes part in any role".

```rust
#[derive(Entity)]
#[entity(table = "disputes")]
#[scope(involving: requester_id | subject_id)]
#[scope(handled: requester_id | subject_id, within = parcel_id)]
pub struct Dispute { /* ... */ }
```

Generated: `list_involving(value, limit, offset)` and `list_handled(parcel_id, value, limit, offset)`. The value is bound once and matched against every declared column; `within` narrows the group to one parent first. At least two columns are required, they must exist and must agree on their type — all checked at compile time. Soft-delete aware, ordered by id descending.

### `#[owner]`

Row-level ownership scoping. Marks the column carrying the owning principal's id; the repository gains scoped methods that never reveal whether a row exists for another owner and respect `soft_delete`.

```rust
#[derive(Entity)]
#[entity(table = "orders")]
pub struct Order {
    #[id]
    pub id: Uuid,

    #[owner]
    pub user_id: Uuid,

    #[field(create, update, response)]
    pub note: String,
}

let mine = pool.list_by_owner(user_id, 20, 0).await?;
let order = pool.find_by_id_scoped(id, user_id).await?;
let updated = pool.update_scoped(id, user_id, patch).await?;
let removed = pool.delete_scoped(id, user_id).await?;
```

Generated: `find_by_id_scoped`, `list_by_owner`, `update_scoped` (when update fields exist, returns `None` if the row isn't theirs), `delete_scoped`. At most one `#[owner]` field; combining with `#[id]` is rejected at compile time.

### `#[filter]` / `#[filter(...)]`

Generate query filter fields. See [[Filtering]] for details.

```rust
#[filter]              // Exact match: WHERE field = $n
#[filter(eq)]          // Same as above
#[filter(like)]        // Pattern match: WHERE field ILIKE $n
#[filter(range)]       // Range: WHERE field >= $n AND field <= $m
```

### `#[belongs_to(Entity)]`

Foreign key relation. See [[Relations]] for details.

```rust
#[belongs_to(User)]
pub user_id: Uuid,
```

**Generated:** `find_user()` method in repository.

### `#[join(...)]` — joined read models

Declares an `INNER JOIN` contributing columns to a generated `{Entity}View` read model. Repeatable; the joined columns' Rust types are part of the declaration (the macro cannot see the foreign table's schema).

```rust
#[derive(Entity)]
#[entity(table = "tickets")]
#[join(airports as origin, on = origin_iata = iata, fields(
    lat as origin_lat: f64,
    lon as origin_lon: f64,
    city as origin_city: String
))]
#[join(airports as dest, on = destination_iata = iata, fields(
    lat as destination_lat: f64,
    lon as destination_lon: f64,
    city as destination_city: String
))]
pub struct Ticket { /* ... */ }
```

**Generated:**
- `TicketView` — flat struct with every entity column plus the joined columns; derives `sqlx::FromRow` and `serde::Serialize`
- `TicketView::SELECT` — the canonical `SELECT ... FROM ... JOIN ...` fragment (no WHERE) for custom filters:
  ```rust
  let rows: Vec<TicketView> = sqlx::query_as(::sqlx::AssertSqlSafe(format!(
      "{} WHERE tickets.verified = true ORDER BY tickets.departs_at ASC",
      TicketView::SELECT
  ))).fetch_all(&pool).await?;
  ```
- `TicketView::find_by_id(pool, id)` and `TicketView::list(pool, limit, offset)`

Base-table columns are qualified with the table name; a `join` column that does not match an entity column fails the build.

### `#[has_many(Entity)]`

One-to-many relation (entity-level). See [[Relations]] for details.

```rust
#[has_many(Post)]
pub struct User { /* ... */ }
```

**Generated:** `find_posts()` method in repository.

### `#[schema(...)]`

Overrides how a field documents itself in OpenAPI. The token list is
handed to utoipa verbatim and lands on every generated struct that
derives `ToSchema` — the create and response DTOs and the joined view.

A JSONB column typed as `serde_json::Value` documents as a free-form
object; the override names the shape it actually carries:

```rust
#[field(create, response)]
#[schema(value_type = Option<SizeCm>)]
pub size_cm: Option<serde_json::Value>,
```

Without the `api` feature nothing generated derives `ToSchema`, and the
attribute is dropped rather than emitted onto a struct that cannot
interpret it. Update DTOs do not carry it: the macro rewrites their
field types for PATCH semantics, so a declared `value_type` would
contradict the type actually generated.

### `#[projection(Name: fields)]`

Generate partial view struct (entity-level).

```rust
#[projection(Public: id, name, avatar)]
#[projection(Admin: id, name, email, role)]
pub struct User { /* ... */ }
```

**Generated:**
- `UserPublic { id, name, avatar }`
- `UserAdmin { id, name, email, role }`
- `From<User>` implementations
- `find_by_id_public()`, `find_by_id_admin()` methods

## Command Attributes

Applied at entity level with `#[command(...)]`.

### Quick Reference

| Syntax | Effect |
|--------|--------|
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

See [[Commands]] for detailed documentation.

## Complete Example

```rust
#[derive(Entity)]
#[entity(
    table = "posts",
    schema = "blog",
    sql = "full",
    dialect = "postgres",
    uuid = "v7",
    soft_delete,
    returning = "full",
    events,
    hooks,
    commands
)]
#[has_many(Comment)]
#[projection(Summary: id, title, author_id, created_at)]
#[command(Publish)]
#[command(Archive, requires_id)]
pub struct Post {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    #[filter(like)]
    pub title: String,

    #[field(create, update, response)]
    pub content: String,

    #[field(create, response)]
    #[belongs_to(User)]
    #[filter]
    pub author_id: Uuid,

    #[field(update, response)]
    pub published: bool,

    #[field(response)]
    #[filter(range)]
    pub view_count: i64,

    #[field(skip)]
    pub moderation_notes: String,

    #[field(skip)]
    pub deleted_at: Option<DateTime<Utc>>,

    #[auto]
    #[field(response)]
    #[filter(range)]
    pub created_at: DateTime<Utc>,

    #[auto]
    #[field(response)]
    pub updated_at: DateTime<Utc>,
}
```

## Decision Matrix

| I want to... | Attributes |
|-------------|------------|
| Auto-generate primary key | `#[id]` |
| Use random UUID | `uuid = "v4"` on entity |
| Use time-ordered UUID | `uuid = "v7"` (default) |
| Accept in POST body | `#[field(create)]` |
| Accept in PATCH body | `#[field(update)]` |
| Return in API response | `#[field(response)]` |
| Accept and return | `#[field(create, update, response)]` |
| Hide from all APIs | `#[field(skip)]` |
| Auto-generate timestamp | `#[auto]` + `#[field(response)]` |
| Read-only (DB managed) | `#[field(response)]` only |
| Write-only (no return) | `#[field(create)]` only |
| Custom SQL queries | `sql = "trait"` |
| DTOs only, no DB | `sql = "none"` |
| Soft delete records | `soft_delete` on entity |
| Custom error type | `error = "MyError"` on entity |
| Filter by exact value | `#[filter]` on field |
| Filter by pattern | `#[filter(like)]` on field |
| Filter by range | `#[filter(range)]` on field |
| Track entity changes | `events` on entity |
| Run code on lifecycle | `hooks` on entity |
| Use domain commands | `commands` on entity + `#[command(...)]` |
| Use multi-entity transactions | `transactions` on entity |
| Define relationship | `#[belongs_to(Entity)]` or `#[has_many(Entity)]` |
| Partial entity view | `#[projection(Name: fields)]` |
| Override an OpenAPI field type | `#[schema(value_type = T)]` |

### Update DTOs: PATCH semantics

Generated updates are true partial patches: the UPDATE SET clause is built at runtime from the fields actually present, so omitted fields stay untouched. Nullable columns use double-`Option` (`None` = leave, `Some(None)` = SET NULL, `Some(Some(v))` = SET v) via `entity_core::serde_helpers::double_option`.

Chainable setters express the same patch without the nesting: `set_{field}` for every updatable column, `clear_{field}` for the nullable ones, and `expecting_version` where `#[version]` is declared.

```rust
let patch = UpdateUserRequest::default()
    .set_name("Neo".into())
    .clear_nickname();
```

Struct-literal construction keeps working; the setters are additive.

```rust
// {}                   → nothing changes
// {"nickname": null}   → nickname = NULL
// {"nickname": "neo"}  → nickname = 'neo'
let patch: UpdateProfileRequest = serde_json::from_str(body)?;
let profile = pool.update(id, patch).await?;
```

### `migrations(...)` options

Beyond the plain flag, `migrations` accepts DDL options: `touch_updated_at` (shared plpgsql function + per-table BEFORE UPDATE trigger keeping `updated_at` fresh; requires an `updated_at` field, checked at compile time), `audit` (`entity_audit_log` table + trigger recording `to_jsonb(OLD/NEW)` diffs) and `extensions = "pg_trgm, pgcrypto"` (idempotent `CREATE EXTENSION`). New constants: `MIGRATION_TRIGGERS` (run after `MIGRATION_UP`) and `MIGRATION_EXTENSIONS` (run before).

```rust
#[entity(table = "articles", migrations(touch_updated_at, audit, extensions = "pg_trgm"))]
pub struct Article { /* ... */ }

for ddl in Article::MIGRATION_EXTENSIONS { sqlx::query(ddl).execute(&pool).await?; }
sqlx::query(Article::MIGRATION_UP).execute(&pool).await?;
for ddl in Article::MIGRATION_TRIGGERS { sqlx::query(ddl).execute(&pool).await?; }
```

### `#[version]`

Optimistic locking. Marks an integer column (`i16`/`i32`/`i64`); the Update DTO gains a required `expected_version`, the generated UPDATE bumps the column and only applies while the stored version still matches — a stale write fails with a conflict error instead of overwriting newer data. DDL defaults to `INTEGER NOT NULL DEFAULT 0`. Applies to plain, scoped and transactional updates.

```rust
#[derive(Entity)]
#[entity(table = "orders", migrations)]
pub struct Order {
    #[id] pub id: Uuid,
    #[field(create, update, response)] pub note: String,
    #[version] #[field(response)] #[auto] pub version: i32,
}

let patch = UpdateOrderRequest { note: Some("v2".into()), expected_version: order.version };
let updated = pool.update(order.id, patch).await?;
```

### `typed_constraints` (optional)

The macro knows every constraint it creates. With this flag, generated write methods resolve violated constraint names (unique columns, `belongs_to` foreign keys, column checks, `unique_index` names) and surface `entity_core::ConstraintError { kind, constraint, field }` instead of a raw driver error. Requires a custom `error` type implementing `From<ConstraintError>`; without the flag nothing changes.

```rust
#[entity(table = "users", typed_constraints, error = "AppError")]
pub struct User {
    #[id] pub id: Uuid,
    #[field(create, response)] #[column(unique)] pub email: String,
}

match pool.create(dto).await {
    Err(AppError::Constraint(v)) if v.field == Some("email") => conflict_409(),
    other => other?,
}
```

### `#[embed(prefix = "...", fields(...))]`

Flatten a value object to prefixed scalar columns. DDL, Row struct, CRUD SQL and dynamic PATCH updates operate on `price_amount_cents` / `price_currency`, while DTOs and the entity carry the struct itself. The declared shape is destructured against the real struct at compile time — a renamed, retyped, missing or extra field fails the build. `Option<T>` parents are not supported yet.

```rust
pub struct Money { pub amount_cents: i64, pub currency: String }

#[derive(Entity)]
#[entity(table = "products", migrations)]
pub struct Product {
    #[id] pub id: Uuid,
    #[field(create, update, response)]
    #[embed(prefix = "price_", fields(amount_cents: i64, currency: String))]
    pub price: Money,
}
```

### `garde` feature (validation backend)

Enables `garde::Validate` on generated DTOs as a maintained alternative to `validator`. Field `#[validate(...)]` rules (`length`, `range`, `email`, `url`, `pattern`) are translated to garde syntax; unconstrained fields get `garde(skip)`; Option-wrapped Update DTO fields validate the inner value via `inner(...)`. When both `validate` and `garde` are enabled, `validate` takes precedence.

```rust
#[field(create, update, response)]
#[validate(length(min = 3, max = 8))]
pub name: String,

let dto: CreateUserRequest = serde_json::from_str(body)?;
garde::Validate::validate(&dto)?;
```

### `constraint(...)` (optional, with `typed_constraints`)

Declare constraints the macro cannot infer — foreign keys over natural keys, custom-named CHECK constraints, indexes from hand-written migrations — so violations resolve to `ConstraintError` with the declared field. Kinds: `unique`, `foreign_key`, `check`. Custom entries take precedence over derived entries with the same name. Requires `typed_constraints`.

```rust
#[entity(
    table = "orders",
    typed_constraints,
    constraint(name = "orders_currency_fkey", kind = "foreign_key", field = "currency"),
    constraint(name = "orders_window_check", kind = "check"),
)]
```

### Composite lookups from `unique_index(...)`

Every multi-column `unique_index(a, b)` declaration generates a lookup pair on the repository:

```rust
#[derive(Entity)]
#[entity(table = "kyc_sessions", unique_index(provider, external_id))]
pub struct KycSession {
    #[id] pub id: Uuid,
    #[field(create, response)] pub provider: String,
    #[field(create, response)] pub external_id: String,
}

let session = pool.find_by_provider_and_external_id(provider, external_id).await?;
let taken = pool.exists_by_provider_and_external_id(provider, external_id).await?;
```

- `find_by_{a}_and_{b}(a, b) -> Option<Entity>` and `exists_by_{a}_and_{b}(a, b) -> bool`
- Parameter types resolve from the matching entity fields; an index column that does not match any entity column fails the build (same rule as upsert conflict columns)
- Non-unique `index(...)` declarations generate no lookups

### Transactional upsert

With both `transactions` and `upsert(...)`, the `{Entity}TransactionRepo` adapter exposes `upsert` with the same SQL and action semantics as the pool method, executed on the transaction handle — for flows where the upsert must share atomicity with adjacent statements.

```rust
let mut tx = pool.begin().await?;
sqlx::query("UPDATE users SET username = NULL WHERE ...").execute(&mut *tx).await?;
let user = UserTransactionRepo::new(&mut tx).upsert(dto).await?;
tx.commit().await?;
```
