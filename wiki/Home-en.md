<div align="center">

# entity-derive

> One macro to rule them all

[![English](https://img.shields.io/badge/🇬🇧_English-blue?style=for-the-badge)](#)
[![Русский](https://img.shields.io/badge/🇷🇺_Русский-gray?style=for-the-badge)](Главная)
[![한국어](https://img.shields.io/badge/🇰🇷_한국어-gray?style=for-the-badge)](홈)
[![Español](https://img.shields.io/badge/🇪🇸_Español-gray?style=for-the-badge)](Inicio)
[![中文](https://img.shields.io/badge/🇨🇳_中文-gray?style=for-the-badge)](首页)

</div>

---

## What is it?

**entity-derive** is a Rust procedural macro that generates a complete domain layer from a single entity definition. Not just CRUD — an architectural framework with events, hooks, commands, and type-safe filtering.

---

## The Problem

A typical Rust backend with ~10 entities means:

| Component | Lines of Code | Problems |
|-----------|---------------|----------|
| DTOs (Create, Update, Response) | ~60 per entity | Manual sync, forgotten fields |
| Repository trait + impl | ~150 per entity | Runtime SQL errors, copy-paste |
| Entity ↔ DTO mapping | ~40 per entity | Data leaks (`password_hash` in Response) |
| Validation and hooks | Scattered across services | Duplication, no single source |
| Events/audit | Missing or ad-hoc | No change history |

**Total: ~2500 lines of boilerplate** for 10 entities. And every schema change requires manual edits in 5+ places.

---

## The Solution

```rust
#[derive(Entity)]
#[entity(table = "users", events, hooks, commands)]
#[command(Register)]
#[command(Deactivate, requires_id)]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    #[filter(like)]
    pub email: String,

    #[field(skip)]  // Never leaks to API
    pub password_hash: String,

    #[auto]
    #[field(response)]
    pub created_at: DateTime<Utc>,
}
```

**15 lines → complete domain layer:**
- `CreateUserRequest`, `UpdateUserRequest`, `UserResponse`
- `UserRepository` with type-safe SQL
- `UserEvent::Created`, `Updated`, `Deleted`
- `UserHooks` for business logic
- `RegisterUser`, `DeactivateUser` commands
- `UserQuery` for filtering

---

## Simple for Beginners

**Minimal example — 10 lines:**

```rust
#[derive(Entity)]
#[entity(table = "posts")]
pub struct Post {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub title: String,

    #[field(create, update, response)]
    pub content: String,
}
```

**Done.** You have:
- `CreatePostRequest`, `UpdatePostRequest`, `PostResponse`
- `PostRepository` with `create()`, `find_by_id()`, `update()`, `delete()`, `list()`
- Type-safe SQL for PostgreSQL
- Everything works out of the box

**No magic.** Run `cargo expand` — you'll see exactly the code you would write yourself. Just without errors and in seconds.

---

## Why Events?

**Problem:** CRUD applications have no history. Who changed the record? When? What was there before? Audit requires separate infrastructure and discipline.

**Solution:** `#[entity(events)]` generates typed events:

```rust
pub enum UserEvent {
    Created(User),
    Updated { id: Uuid, changes: UpdateUserRequest },
    Deleted(Uuid),
}
```

**Benefits:**
- **Audit out of the box** — subscribe to events, save to log
- **Event Sourcing** — can restore state from event history
- **Integrations** — Kafka, WebSocket notifications, cache invalidation
- **Debugging** — complete change history for every entity

---

## Why Hooks?

**Problem:** Business logic is scattered. Email validation — in controller. Password hashing — in service. Sending email — in separate worker. Where to look for user creation logic?

**Solution:** `#[entity(hooks)]` centralizes lifecycle:

```rust
impl UserHooks for MyHooks {
    async fn before_create(&self, dto: &mut CreateUserRequest) -> Result<(), Error> {
        dto.email = dto.email.to_lowercase();  // Normalization
        validate_email(&dto.email)?;            // Validation
        Ok(())
    }

    async fn after_create(&self, user: &User) -> Result<(), Error> {
        self.mailer.send_welcome(user).await?;  // Business action
        Ok(())
    }
}
```

**Benefits:**
- **Single place** — all entity logic next to definition
- **Predictability** — clear when what executes
- **Testability** — hooks can be mocked and tested in isolation
- **Composition** — different implementations for different contexts

---

## Why Commands?

**Problem:** REST API hides intent. `POST /users` — is it registration? Admin creation? CSV import? `PATCH /users/123` — deactivation? Email change? Ban?

**Solution:** `#[command(...)]` expresses business domain:

```rust
#[command(Register)]           // Self-registration
#[command(Invite)]             // Admin invitation
#[command(Deactivate, requires_id)]  // Account deactivation
#[command(Ban, requires_id)]   // Ban for violations
```

**Compare:**
```rust
// CRUD (what's happening?)
pool.update(user_id, UpdateUserRequest { active: Some(false), ..default() }).await?;

// Commands (clear intent)
handler.handle(DeactivateUser { id: user_id }).await?;
```

**Benefits:**
- **Self-documenting API** — command names = business vocabulary
- **Different logic** — `Deactivate` and `Ban` can have different side-effects
- **CQRS ready** — commands easy to route, log, retry
- **Type safety** — compiler verifies command exists

---

## Why Type-Safe Filtering?

**Problem:** String query params are runtime error sources:
```
GET /users?stauts=active  // Typo — silently ignored
GET /users?created_at=tomorrow  // Invalid date — runtime panic
```

**Solution:** `#[filter]` generates typed struct:

```rust
let query = UserQuery {
    email: Some("@company.com".into()),  // ILIKE '%@company.com%'
    created_at_min: Some(week_ago),       // >= week_ago
    created_at_max: Some(now),            // <= now
    ..Default::default()
};

let users = pool.list_filtered(&query, 100, 0).await?;
```

**Benefits:**
- **Compile-time check** — field name typo = compilation error
- **Type safety** — can't compare `DateTime` with `String`
- **Autocomplete** — IDE suggests available filters
- **SQL injection protection** — parameters are bound, not concatenated

---

## Transparency

The macro doesn't hide logic. Everything generated is regular Rust code you can:

- **Read** — `cargo expand` shows all generated code
- **Understand** — no runtime reflection, just structs and traits
- **Override** — `sql = "trait"` and write your own SQL
- **Debug** — compiler errors point to your code, not macro internals

```rust
// Want to understand what's generated?
cargo expand --lib | grep -A 50 "impl UserRepository"
```

**Zero magic.** If the macro breaks — you can always write code by hand. No lock-in.

---

## Full Power of Rust

### Compile-time Guarantees

```rust
#[field(skip)]
pub password_hash: String,
```

This isn't a runtime check "don't serialize this field". It's **physical absence** of the field in `UserResponse` struct. Can't accidentally return it — the field simply doesn't exist.

### Zero-cost Abstractions

Generated code is:
- Regular `struct` without Box/dyn
- Direct sqlx calls without intermediate layers
- `#[inline]` on hot paths
- No allocations beyond necessary

**Benchmark:** generated repository runs at the same speed as hand-written. Because it is the same code.

### Async Out of the Box

```rust
// Everything async, everything Send + Sync
let user = pool.find_by_id(id).await?;
let users = pool.list(100, 0).await?;
```

Full compatibility with tokio, async-std, any async runtime.

### Strict Typing

```rust
// Compilation error: no such field
let query = UserQuery { naem: "test".into(), ..default() };
                        ^^^^ unknown field

// Compilation error: wrong type
let query = UserQuery { created_at_min: "yesterday".into(), ..default() };
                                        ^^^^^^^^^^^^ expected DateTime<Utc>
```

If code compiles — it works correctly.

---

## Professional Architecture

### Clean Architecture Ready

```
Domain Layer (entity-derive)
├── Entities — #[derive(Entity)]
├── DTOs — CreateRequest, UpdateRequest, Response
├── Repository Trait — storage abstraction
├── Events — domain events
├── Commands — business operations
└── Hooks — lifecycle logic

Infrastructure Layer (your code)
├── Repository Impl — PgPool automatic or custom
├── Event Handlers — event subscriptions
├── Command Handlers — business logic implementation
└── External Services — integrations
```

Clean separation. Domain knows nothing about HTTP, database, Kafka. Those are implementation details.

### CQRS/Event Sourcing Ready

```rust
// Command side
handler.handle(RegisterUser { email, name }).await?;

// Query side
let users = pool.list_filtered(&query, 100, 0).await?;

// Event side
match event {
    UserEvent::Created(user) => kafka.send("user.created", &user).await?,
    UserEvent::Updated { id, changes } => audit_log.record(id, changes).await?,
    _ => {}
}
```

Want simple CRUD? You got it. Want full CQRS? Enable `commands` and `events`. Architecture grows with project.

### Extensibility

**Level 1: Basic CRUD**
```rust
#[entity(table = "users")]
```

**Level 2: + Filtering**
```rust
#[entity(table = "users")]
// + #[filter] on fields
```

**Level 3: + Events and Hooks**
```rust
#[entity(table = "users", events, hooks)]
```

**Level 4: + CQRS Commands**
```rust
#[entity(table = "users", events, hooks, commands)]
#[command(Register)]
#[command(Deactivate, requires_id)]
```

**Level 5: Full Control**
```rust
#[entity(table = "users", sql = "trait", events, hooks, commands)]
// Your SQL, your logic, but keeping all DTOs and types
```

Start simple. Add features as you grow. Don't rewrite — extend.

---

## Security

```rust
#[field(skip)]
pub password_hash: String,
```

`skip` means: this field will **never** appear in:
- `CreateUserRequest` (can't pass from outside)
- `UpdateUserRequest` (can't modify via API)
- `UserResponse` (can't accidentally return to client)

The only way to work with `password_hash` — directly through entity in code. Leak impossible by design.

---

## Why This is Awesome

| Aspect | What You Get |
|--------|--------------|
| **Development Speed** | 10 entities in an hour instead of a day |
| **Reliability** | Compile-time verification of everything |
| **Security** | Impossible to accidentally leak data |
| **Performance** | Zero-cost, like hand-written code |
| **Clarity** | Transparent generation, no magic |
| **Flexibility** | From simple CRUD to CQRS with one attribute |
| **Scalability** | Architecture grows with project |
| **Maintainability** | Single source of truth, fewer bugs |

---

## Documentation

| Topic | Description |
|-------|-------------|
| [[Attributes\|Attributes-en]] | Complete attribute reference |
| [[Filtering\|Filtering-en]] | Type-safe query filtering |
| [[Relations\|Relations-en]] | `belongs_to` and `has_many` |
| [[Events\|Events-en]] | Lifecycle events |
| [[Hooks\|Hooks-en]] | Before/after hooks |
| [[Commands\|Commands-en]] | CQRS pattern |
| [[Custom SQL\|Custom-SQL-en]] | Complex queries |
| [[Examples\|Examples-en]] | Real-world use cases |
| [[Web Frameworks\|Web-Frameworks-en]] | Axum, Actix integration |
| [[Best Practices\|Best-Practices-en]] | Production guidelines |

---

<div align="center">

**This is not a framework that dictates how to live.**
**This is a tool that removes routine and lets you build right.**

---

</div>
