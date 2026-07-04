<a id="top"></a>

<p align="center">
  <img src="logo.png" alt="entity-derive logo" width="200"/>
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
  <a href="https://github.com/RAprogramm/entity-derive/actions">
    <img src="https://img.shields.io/github/actions/workflow/status/RAprogramm/entity-derive/ci.yml?style=for-the-badge" alt="CI Status"/>
  </a>
</p>

<p align="center">
  <a href="https://codecov.io/gh/RAprogramm/entity-derive">
    <img src="https://img.shields.io/codecov/c/github/RAprogramm/entity-derive?style=for-the-badge&token=HGuwZf0REV" alt="Coverage"/>
  </a>
  <a href="https://github.com/RAprogramm/entity-derive/blob/main/LICENSES/MIT.txt">
    <img src="https://img.shields.io/badge/license-MIT-blue.svg?style=for-the-badge" alt="License: MIT"/>
  </a>
  <a href="https://api.reuse.software/info/github.com/RAprogramm/entity-derive">
    <img src="https://img.shields.io/reuse/compliance/github.com%2FRAprogramm%2Fentity-derive?style=for-the-badge" alt="REUSE Compliant"/>
  </a>
  <a href="https://github.com/RAprogramm/entity-derive/wiki">
    <img src="https://img.shields.io/badge/Wiki-Documentation-green?style=for-the-badge&logo=github" alt="Wiki"/>
  </a>
</p>

---

## The Problem

Building a typical CRUD application requires writing the same boilerplate over and over: entity struct, create DTO, update DTO, response DTO, row struct, repository trait, SQL implementation, and 6+ From implementations.

**That's 200+ lines of boilerplate for a single entity.**

## The Solution

```rust,ignore
#[derive(Entity)]
#[entity(table = "users")]
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

---

## Installation

```toml
[dependencies]
entity-derive = { version = "0.22", features = ["postgres", "api"] }
```

### Feature flags

| Feature | Default | What it does |
|---------|:-------:|--------------|
| `postgres` | ✓ | Generate `sqlx::PgPool`-backed repository implementations |
| `events` | ✓ | Generate `{Entity}Event` enum (`Created` / `Updated` / `Deleted` variants) |
| `commands` | ✓ | CQRS command pattern: command structs + dispatcher (`#[entity(commands)]`, `#[command(...)]`) |
| `hooks` | ✓ | `{Entity}Hooks` trait with before/after lifecycle methods |
| `transactions` | ✓ | `{Entity}TransactionRepo` adapter + transaction builder helpers (`#[entity(transactions)]`) |
| `aggregate_root` | ✓ | `New{Entity}` constructor type and transactional `save()` (`#[entity(aggregate_root)]`) |
| `migrations` | ✓ | Compile-time `MIGRATION_UP` / `MIGRATION_DOWN` SQL constants (`#[entity(migrations)]`) |
| `projections` | ✓ | Projection structs and `find_by_id_<projection>` lookups (`#[projection(...)]`) |
| `clickhouse` |   | Generate ClickHouse-backed repositories *(planned)* |
| `mongodb` |   | Generate MongoDB-backed repositories *(planned)* |
| `streams` |   | `{Entity}Subscriber` using Postgres `LISTEN`/`NOTIFY` (pulls in `events`) |
| `outbox` |   | Transactional-outbox enqueue in generated writes + `OutboxDrainer` runtime (pulls in `events`) |
| `api` |   | Generate HTTP handlers (`axum`) and `utoipa` OpenAPI schemas |
| `validate` |   | Wire up `validator::Validate` on generated DTOs |
| `tracing` |   | Wrap every generated async method in `#[tracing::instrument]` carrying `entity` + `op` span fields |

Default features cover the full entity-attribute surface so existing projects work without changes. For lean builds, opt out of what you don't need:

```toml
[dependencies]
# Just repositories — no events, hooks, commands, etc.
entity-derive = { version = "0.22", default-features = false, features = ["postgres"] }
```

If you use an entity attribute whose feature is disabled (e.g. `#[entity(commands)]` without `features = ["commands"]`), the macro emits a `compile_error!` at the attribute pointing to the missing feature.

Enable extras alongside the defaults:

```toml
[dependencies]
entity-derive = { version = "0.22", features = ["postgres", "api", "tracing", "streams"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

---

## Features

| Feature | Description |
|---------|-------------|
| **Zero Runtime Cost** | All code generation at compile time |
| **Type Safe** | Change a field once, everything updates |
| **Auto HTTP Handlers** | `api(handlers)` generates CRUD endpoints + router |
| **`OpenAPI` Docs** | Auto-generated Swagger/OpenAPI documentation |
| **Query Filtering** | Type-safe `#[filter]`, `#[filter(like)]`, `#[filter(range)]`, `#[filter(search)]` |
| **Sorting & Keyset** | `#[sort]` whitelisted ORDER BY + `list_after` cursor pagination |
| **Bulk Operations** | `find_by_ids`, atomic `create_many`, soft-delete-aware `delete_many` |
| **PATCH Semantics** | Dynamic UPDATE SET; double-`Option` distinguishes "leave" from "set NULL" |
| **Optimistic Locking** | `#[version]` — guarded, auto-incremented version column |
| **Typed Constraint Errors** | `typed_constraints` — violations resolved to `ConstraintError` with field info |
| **Embedded Value Objects** | `#[embed(prefix, fields(...))]` — structs flattened to prefixed columns |
| **Relations** | `#[belongs_to]`, `#[has_many]` and many-to-many via `through = "junction"` |
| **Ownership Scoping** | `#[owner]` generates `find_by_id_scoped` / `list_by_owner` / `update_scoped` / `delete_scoped` |
| **Upsert** | `upsert(conflict = "…")` generates `INSERT ... ON CONFLICT DO UPDATE / DO NOTHING` |
| **Aggregate Roots** | `#[entity(aggregate_root)]` with `New{T}` DTOs and transactional `save` |
| **Transactions** | Multi-entity atomic operations |
| **Lifecycle Events** | `Created`, `Updated`, `Deleted` events |
| **Real-Time Streams** | Postgres LISTEN/NOTIFY integration |
| **Transactional Outbox** | `events(outbox)` — durable at-least-once event delivery with retry/backoff |
| **Lifecycle Hook Traits** | `{Entity}Hooks` trait emitted with `before_create` / `after_update` / etc.; invocation is currently manual at your service layer (tracking auto-invocation: [#127](https://github.com/RAprogramm/entity-derive/issues/127)) |
| **CQRS Commands** | Business-oriented command pattern |
| **Soft Delete** | `deleted_at` timestamp support |
| **Structured Logging** | Opt-in `tracing` feature wraps every generated async method in `#[tracing::instrument]` with `entity` + `op` fields |

---

## Documentation

| Topic | Languages |
|-------|:---------:|
| **Getting Started** ||
| Attributes | [🇬🇧](https://github.com/RAprogramm/entity-derive/wiki/Attributes-en) [🇷🇺](https://github.com/RAprogramm/entity-derive/wiki/Attributes-ru) [🇰🇷](https://github.com/RAprogramm/entity-derive/wiki/Attributes-ko) [🇪🇸](https://github.com/RAprogramm/entity-derive/wiki/Attributes-es) [🇨🇳](https://github.com/RAprogramm/entity-derive/wiki/Attributes-zh) |
| Examples | [🇬🇧](https://github.com/RAprogramm/entity-derive/wiki/Examples-en) [🇷🇺](https://github.com/RAprogramm/entity-derive/wiki/Examples-ru) [🇰🇷](https://github.com/RAprogramm/entity-derive/wiki/Examples-ko) [🇪🇸](https://github.com/RAprogramm/entity-derive/wiki/Examples-es) [🇨🇳](https://github.com/RAprogramm/entity-derive/wiki/Examples-zh) |
| **Features** ||
| Filtering | [🇬🇧](https://github.com/RAprogramm/entity-derive/wiki/Filtering-en) [🇷🇺](https://github.com/RAprogramm/entity-derive/wiki/Filtering-ru) [🇰🇷](https://github.com/RAprogramm/entity-derive/wiki/Filtering-ko) [🇪🇸](https://github.com/RAprogramm/entity-derive/wiki/Filtering-es) [🇨🇳](https://github.com/RAprogramm/entity-derive/wiki/Filtering-zh) |
| Relations | [🇬🇧](https://github.com/RAprogramm/entity-derive/wiki/Relations-en) [🇷🇺](https://github.com/RAprogramm/entity-derive/wiki/Relations-ru) [🇰🇷](https://github.com/RAprogramm/entity-derive/wiki/Relations-ko) [🇪🇸](https://github.com/RAprogramm/entity-derive/wiki/Relations-es) [🇨🇳](https://github.com/RAprogramm/entity-derive/wiki/Relations-zh) |
| Events | [🇬🇧](https://github.com/RAprogramm/entity-derive/wiki/Events-en) [🇷🇺](https://github.com/RAprogramm/entity-derive/wiki/Events-ru) [🇰🇷](https://github.com/RAprogramm/entity-derive/wiki/Events-ko) [🇪🇸](https://github.com/RAprogramm/entity-derive/wiki/Events-es) [🇨🇳](https://github.com/RAprogramm/entity-derive/wiki/Events-zh) |
| Streams | [🇬🇧](https://github.com/RAprogramm/entity-derive/wiki/Streams-en) [🇷🇺](https://github.com/RAprogramm/entity-derive/wiki/Streams-ru) [🇰🇷](https://github.com/RAprogramm/entity-derive/wiki/Streams-ko) [🇪🇸](https://github.com/RAprogramm/entity-derive/wiki/Streams-es) [🇨🇳](https://github.com/RAprogramm/entity-derive/wiki/Streams-zh) |
| Hooks | [🇬🇧](https://github.com/RAprogramm/entity-derive/wiki/Hooks-en) [🇷🇺](https://github.com/RAprogramm/entity-derive/wiki/Hooks-ru) [🇰🇷](https://github.com/RAprogramm/entity-derive/wiki/Hooks-ko) [🇪🇸](https://github.com/RAprogramm/entity-derive/wiki/Hooks-es) [🇨🇳](https://github.com/RAprogramm/entity-derive/wiki/Hooks-zh) |
| Commands | [🇬🇧](https://github.com/RAprogramm/entity-derive/wiki/Commands-en) [🇷🇺](https://github.com/RAprogramm/entity-derive/wiki/Commands-ru) [🇰🇷](https://github.com/RAprogramm/entity-derive/wiki/Commands-ko) [🇪🇸](https://github.com/RAprogramm/entity-derive/wiki/Commands-es) [🇨🇳](https://github.com/RAprogramm/entity-derive/wiki/Commands-zh) |
| **Advanced** ||
| Custom SQL | [🇬🇧](https://github.com/RAprogramm/entity-derive/wiki/Custom-SQL-en) [🇷🇺](https://github.com/RAprogramm/entity-derive/wiki/Custom-SQL-ru) [🇰🇷](https://github.com/RAprogramm/entity-derive/wiki/Custom-SQL-ko) [🇪🇸](https://github.com/RAprogramm/entity-derive/wiki/Custom-SQL-es) [🇨🇳](https://github.com/RAprogramm/entity-derive/wiki/Custom-SQL-zh) |
| Web Frameworks | [🇬🇧](https://github.com/RAprogramm/entity-derive/wiki/Web-Frameworks-en) [🇷🇺](https://github.com/RAprogramm/entity-derive/wiki/Web-Frameworks-ru) [🇰🇷](https://github.com/RAprogramm/entity-derive/wiki/Web-Frameworks-ko) [🇪🇸](https://github.com/RAprogramm/entity-derive/wiki/Web-Frameworks-es) [🇨🇳](https://github.com/RAprogramm/entity-derive/wiki/Web-Frameworks-zh) |
| Best Practices | [🇬🇧](https://github.com/RAprogramm/entity-derive/wiki/Best-Practices-en) [🇷🇺](https://github.com/RAprogramm/entity-derive/wiki/Best-Practices-ru) [🇰🇷](https://github.com/RAprogramm/entity-derive/wiki/Best-Practices-ko) [🇪🇸](https://github.com/RAprogramm/entity-derive/wiki/Best-Practices-es) [🇨🇳](https://github.com/RAprogramm/entity-derive/wiki/Best-Practices-zh) |

---

## Quick Reference

### Entity Attributes

```rust,ignore
#[entity(
    table = "users",           // Required: table name
    schema = "public",         // Optional: schema (default: omitted)
    dialect = "postgres",      // Optional: database dialect
    aggregate_root,            // Optional: New{T} DTOs + transactional save
    soft_delete,               // Optional: use deleted_at instead of DELETE
    migrations(                // Optional: extra DDL alongside the table
        touch_updated_at,      // updated_at trigger (shared function)
        audit,                 // JSONB audit-log table + trigger
        extensions = "pg_trgm",// CREATE EXTENSION statements
    ),
    typed_constraints,         // Optional: constraint violations as typed errors
    upsert(                    // Optional: INSERT ... ON CONFLICT method
        conflict = "email",    // #[column(unique)] field(s) or unique_index columns
        action = "update",     // "update" (default) or "nothing"
    ),
    events,                    // Optional: generate lifecycle events
    events(outbox),            // Optional: + durable transactional outbox
    streams,                   // Optional: real-time Postgres NOTIFY
    hooks,                     // Optional: before/after lifecycle hooks
    commands,                  // Optional: CQRS command pattern
    transactions,              // Optional: multi-entity transaction support
    api(                       // Optional: generate HTTP handlers + OpenAPI
        tag = "Users",
        handlers,              // All CRUD, or handlers(get, list, create)
        security = "bearer",   // cookie, bearer, api_key, or none
        guard = "RequireAuth", // FromRequestParts extractor enforced in handlers
        guard(list = "none"),  // per-op override: create/get/update/delete/list/commands
        title = "My API",
        api_version = "1.0.0",
    ),
)]
```

### Field Attributes

```rust,ignore
#[id]                          // Primary key (auto-generated UUID)
#[auto]                        // Auto-generated (timestamps)
#[owner]                       // Ownership column: adds *_scoped methods
#[version]                     // Optimistic locking: guarded, auto-bumped
#[embed(prefix, fields(...))]  // Flatten a value object to prefixed columns
#[field(create)]               // Include in CreateRequest
#[field(update)]               // Include in UpdateRequest
#[field(response)]             // Include in Response
#[field(skip)]                 // Exclude from all DTOs
#[filter]                      // Exact match filter
#[filter(like)]                // ILIKE pattern filter
#[filter(range)]               // Range filter (from/to)
#[filter(search)]              // Trigram substring search (pg_trgm)
#[sort]                        // Whitelisted dynamic ORDER BY
#[belongs_to(Entity)]          // Foreign key relation
#[has_many(Entity)]            // One-to-many relation
#[has_many(E, through = "t")]  // Many-to-many via junction table
#[projection(Name: fields)]    // Partial view
```

### Transactional Outbox

LISTEN/NOTIFY (`streams`) is fire-and-forget: events are lost if no
subscriber is listening. `events(outbox)` makes delivery durable — every
generated write inserts the serialized event into the `entity_outbox`
table in the same transaction as the DML, and a drainer delivers rows
with retry and exponential backoff:

```rust,ignore
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

Rows are claimed with `FOR UPDATE SKIP LOCKED` (multiple drainers
cooperate), retried with exponential backoff and parked after
`max_attempts` for manual inspection. Delivery is at-least-once —
handlers must be idempotent. Composes with `streams`: NOTIFY wakes
subscribers instantly, the outbox guarantees nothing is lost. Requires
the `outbox` feature and `serde_json` in your crate.

### Ownership Scoping

Mark the column carrying the owning principal's id with `#[owner]` and the
repository gains row-level scoped methods — "only this user's rows" without
hand-written predicates:

```rust,ignore
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

let mine: Vec<Order> = pool.list_by_owner(user_id, 20, 0).await?;
let order = pool.find_by_id_scoped(id, user_id).await?;          // None if not theirs
let updated = pool.update_scoped(id, user_id, patch).await?;     // None if not theirs
let removed = pool.delete_scoped(id, user_id).await?;            // false if not theirs
```

Scoped reads and writes never reveal whether a row exists for another
owner, and all of them respect `soft_delete`.

### Handler Guards

`security = "..."` only documents authentication in the OpenAPI spec. To
actually enforce it, pass a `guard` — any type implementing axum's
`FromRequestParts`. It is injected as a leading argument of every generated
handler, so a failed extraction rejects the request before the handler body
runs:

```rust,ignore
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

Per-operation overrides accept `create`, `get`, `update`, `delete`, `list`
and `commands`; the literal `"none"` disables the guard for that operation.
Commands listed in `public = [...]` never receive a guard.

### Embedded Value Objects

Keep money-style value objects as real structs on the entity while
storing them as flat scalar columns — no JSONB detour:

```rust,ignore
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

DDL, Row struct, CRUD SQL and dynamic updates operate on
`price_amount_cents` / `price_currency`; DTOs and the entity carry
`Money` itself. The declared shape is destructured against the real
struct at compile time — a renamed, retyped, missing or extra field
fails the build. `Option<Money>` parents are not supported yet.

### Typed Constraint Errors

The macro knows every constraint it creates. With `typed_constraints`,
write methods resolve violated constraint names and surface
`entity_core::ConstraintError { kind, constraint, field }` instead of a
raw driver error — no string-matching constraint names in handlers:

```rust,ignore
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

Covers unique columns, `belongs_to` foreign keys, column checks and
`unique_index` names. The custom `error` type must implement
`From<ConstraintError>`; without the flag behavior is unchanged.

### Trigram Search

`#[filter(search)]` on a text column gives the Query struct a fuzzy
substring filter (`col ILIKE '%' || $n || '%'`), while `migrations`
emit the matching `gin_trgm_ops` index and add `pg_trgm` to
`MIGRATION_EXTENSIONS` automatically:

```rust,ignore
#[field(create, update, response)]
#[filter(search)]
pub title: String,

let hits = pool.query(ArticleQuery { title: Some("rust".into()), ..Default::default() }).await?;
```

### Optimistic Locking

Mark an integer column with `#[version]` and concurrent updates stop
silently overwriting each other. The Update DTO gains a required
`expected_version`; the generated UPDATE bumps the column and only
applies when the stored version still matches — a stale write fails
with a conflict error instead of clobbering newer data:

```rust,ignore
#[derive(Entity)]
#[entity(table = "orders", migrations)]
pub struct Order {
    #[id] pub id: Uuid,
    #[field(create, update, response)] pub note: String,
    #[version] #[field(response)] #[auto] pub version: i32,
}

let patch = UpdateOrderRequest { note: Some("v2".into()), expected_version: order.version };
let updated = pool.update(order.id, patch).await?;   // version = version + 1
```

DDL gets `INTEGER NOT NULL DEFAULT 0` automatically. Works in plain,
scoped and transactional updates.

### Migration Triggers & Extensions

`migrations(...)` options emit the DDL production tables actually carry:

```rust,ignore
#[entity(table = "articles", migrations(touch_updated_at, audit, extensions = "pg_trgm"))]
pub struct Article { /* ... */ }

for ddl in Article::MIGRATION_EXTENSIONS { sqlx::query(ddl).execute(&pool).await?; }
sqlx::query(Article::MIGRATION_UP).execute(&pool).await?;
for ddl in Article::MIGRATION_TRIGGERS { sqlx::query(ddl).execute(&pool).await?; }
```

- `touch_updated_at` — shared `entity_touch_updated_at()` function + per-table
  BEFORE UPDATE trigger keeping `updated_at` fresh DB-side (requires an
  `updated_at` field, checked at compile time)
- `audit` — `entity_audit_log` table + trigger recording `to_jsonb(OLD/NEW)`
  diffs for INSERT/UPDATE/DELETE
- `extensions = "..."` — idempotent `CREATE EXTENSION` statements

### PATCH Semantics

Update DTOs are true partial patches. Fields absent from the payload are
left untouched — the generated UPDATE only includes columns actually
present. Nullable columns use double-`Option`, so "leave unchanged" and
"set NULL" are finally distinguishable:

```rust,ignore
// {}                          → nothing changes (fetch-and-return)
// {"nickname": null}          → nickname = NULL
// {"nickname": "neo"}         → nickname = 'neo'
let patch: UpdateProfileRequest = serde_json::from_str(body)?;
let profile = pool.update(id, patch).await?;
```

In Rust code: `None` = leave, `Some(None)` = SET NULL,
`Some(Some(v))` = SET v.

### Bulk Operations

Every repository ships batch primitives — one round-trip reads and
atomic writes:

```rust,ignore
let posts: Vec<Post> = pool.find_by_ids(ids).await?;          // WHERE id = ANY($1)
let created: Vec<Post> = pool.create_many(dtos).await?;       // one transaction
let removed: u64 = pool.delete_many(stale_ids).await?;        // soft-delete aware
```

`create_many` rolls the whole batch back if any row fails; `delete_many`
returns the number of rows actually affected. With `events` delivery
(streams or outbox) per-row events are emitted inside the same
transaction.

### Sorting & Keyset Pagination

Mark sortable columns with `#[sort]` — the Query struct gains a
whitelisted sort selector (`{Entity}SortField`, one `Asc`/`Desc` variant
per column, JSON-friendly), so user input can never inject SQL. Every
repository also gets `list_after` keyset pagination that stays fast on
deep pages, unlike OFFSET:

```rust,ignore
let query = PostQuery {
    sort: Some(PostSortField::ViewsDesc),
    limit: Some(20),
    ..Default::default()
};
let top: Vec<Post> = pool.query(query).await?;

let page: Vec<Post> = pool.list_after(None, 20).await?;
let next: Vec<Post> = pool.list_after(page.last().map(|p| p.id), 20).await?;
```

With the default UUIDv7 ids, the id-ordered keyset walk is
chronologically stable.

### Many-to-Many Relations

Declare the junction table with `through` and the repository gains a
JOIN-backed lookup plus link management, while `migrations` emits the
junction DDL (composite primary key, cascading foreign keys):

```rust,ignore
#[derive(Entity)]
#[entity(table = "teams", migrations)]
#[has_many(User, through = "team_members")]
pub struct Team { /* ... */ }

for ddl in Team::MIGRATION_JUNCTIONS {
    sqlx::query(ddl).execute(&pool).await?;
}

pool.add_user(team_id, user_id).await?;          // idempotent link
let members: Vec<User> = pool.find_users(team_id).await?;
let linked = pool.has_user(team_id, user_id).await?;
let removed = pool.remove_user(team_id, user_id).await?;
```

### Postgres Enums

Derive `ValueObject` on a status-style enum and reference it from entities
with `#[column(pg_enum = "...")]`:

```rust,ignore
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

- `ValueObject` generates `PG_TYPE` and idempotent `PG_CREATE_TYPE` constants;
  the opt-in `sqlx` flag additionally emits `sqlx::Type` / `Encode` / `Decode`
  impls so the enum binds and decodes without hand-written glue (omit it if
  you already derive `sqlx::Type` yourself).
- `#[column(pg_enum = "...")]` sets the DDL column type and registers the
  enum's DDL in `{Entity}::MIGRATION_TYPES` — run those before `MIGRATION_UP`.
- The declared name is checked against the enum's `pg_type` at compile time;
  a typo fails the build.

### Upsert

Declare a conflict target with a uniqueness guarantee (`#[id]`,
`#[column(unique)]` or `unique_index(...)`) and the repository gains an
`upsert` method backed by `INSERT ... ON CONFLICT`:

```rust,ignore
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

let user = pool.upsert(CreateUserRequest { email, name }).await?;
```

`action = "update"` (default) overwrites all non-conflict columns with the
incoming values (`DO UPDATE SET col = EXCLUDED.col`) and returns the
persisted row. `action = "nothing"` keeps the existing row (`DO NOTHING`)
and returns `Option<Entity>` — `None` when a conflicting row already
existed. Requires `returning = "full"` (the default). With `streams`
enabled, upsert publishes a `Created` notification for every row it
returns.

### Transactions

Mark each participating entity with `#[entity(table = "…", transactions)]` and
drive a multi-entity transaction through `Transaction::run`. The closure
receives `&mut TransactionContext`; `run` commits on `Ok` and rolls back on
`Err` (or any panic) automatically:

```rust,ignore
use entity_core::transaction::Transaction;

Transaction::new(&pool)
    .run(async |ctx| {
        let user = ctx.users().create(create_user).await?;
        ctx.orders().create(order_for(user.id)).await?;
        Ok::<_, sqlx::Error>(user)
    })
    .await?;
```

Need conditional commit/rollback inside the closure? Use
[`run_with_commit`](https://docs.rs/entity-core/latest/entity_core/transaction/struct.Transaction.html#method.run_with_commit)
— it takes `TransactionContext` by value so the closure can call
`ctx.commit().await` (or `ctx.rollback().await`) itself.

### Tracing

Opt-in with the `tracing` feature. Every generated async method
(`create`, `find_by_id`, `update`, `delete`, `list`, `find_by_<field>`,
projections, transaction adapters, stream subscribers) is wrapped in
`#[tracing::instrument(skip_all, fields(entity, op), err(Debug))]`.

```toml
entity-derive = { version = "0.22", features = ["postgres", "tracing"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

With a subscriber initialized, a failed `User::create` surfaces as:

```text
ERROR entity.User.create: error=database error: duplicate key value violates unique constraint
  in entity.User.create with entity="User" op="create"
```

When the feature is off, generated code is byte-for-byte identical to a
build without the attribute — zero runtime cost.

---

## Code Coverage

<p align="center">
  <a href="https://codecov.io/gh/RAprogramm/entity-derive">
    <img src="https://codecov.io/gh/RAprogramm/entity-derive/graphs/sunburst.svg?token=HGuwZf0REV" alt="Coverage Sunburst"/>
  </a>
</p>

