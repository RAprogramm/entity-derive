<!--
SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
SPDX-License-Identifier: MIT
-->

# Stability Policy

This document describes the stability guarantees for `entity-derive` and how
semantic versioning is applied to the crate.

## Versioning

This crate follows [Semantic Versioning 2.0.0](https://semver.org/):

- **MAJOR** (1.0.0 → 2.0.0): Breaking changes to stable APIs
- **MINOR** (1.0.0 → 1.1.0): New features, backward-compatible
- **PATCH** (1.0.0 → 1.0.1): Bug fixes, backward-compatible

### Pre-1.0 Policy

While the crate is below version 1.0.0:

- **MINOR** bumps may include breaking changes
- **PATCH** bumps are always backward-compatible
- Breaking changes are documented in CHANGELOG.md

## Stable Guarantees

The following are considered stable and follow semver:

### Attribute Syntax

```rust
#[derive(Entity)]
#[entity(table = "...", schema = "...", sql = "...", dialect = "...", uuid = "...")]
pub struct Entity {
    #[id]
    pub id: Uuid,

    #[field(create, update, response, skip)]
    pub field: T,

    #[auto]
    pub timestamp: DateTime<Utc>,
}
```

Attribute names and their accepted values are stable. New attributes may be
added in minor versions but existing attributes will not change behavior.

### Generated Type Names

For an entity `User`, these type names are stable:

| Type | Name Pattern |
|------|--------------|
| Create DTO | `Create{Name}Request` |
| Update DTO | `Update{Name}Request` |
| Response DTO | `{Name}Response` |
| Repository trait | `{Name}Repository` |
| Row struct | `{Name}Row` |
| Insertable struct | `Insertable{Name}` |

### Repository Trait Methods

The following trait methods and their signatures are stable:

```rust
trait {Name}Repository: Send + Sync {
    type Error: std::error::Error + Send + Sync;
    type Pool;

    fn pool(&self) -> &Self::Pool;
    async fn create(&self, dto: Create{Name}Request) -> Result<{Name}, Self::Error>;
    async fn find_by_id(&self, id: IdType) -> Result<Option<{Name}>, Self::Error>;
    async fn update(&self, id: IdType, dto: Update{Name}Request) -> Result<{Name}, Self::Error>;
    async fn delete(&self, id: IdType) -> Result<bool, Self::Error>;
    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<{Name}>, Self::Error>;
}
```

### DTO Structure

- `CreateRequest` includes fields marked with `#[field(create)]`
- `UpdateRequest` includes fields marked with `#[field(update)]`, wrapped in `Option<T>`
- `Response` includes fields marked with `#[field(response)]` and `#[id]`
- All DTOs derive `Debug`, `Clone`, `Serialize`, `Deserialize`

### Feature Flags

These feature flags are stable:

| Flag | Purpose |
|------|---------|
| `postgres` | PostgreSQL support via sqlx |
| `api` | OpenAPI schema generation (utoipa) |
| `validate` | Validation derive (validator) |

## Unstable / Experimental

The following may change without a major version bump:

### Generated SQL

The exact SQL queries generated are not part of the public API. While they
will remain functionally equivalent, the specific query text may change.

### Internal Modules

Anything under `entity::parse` is internal and may change. Only re-exported
types in the public API are stable.

### Planned Features (Not Yet Stable)

These features are planned but not yet implemented:

- `clickhouse` dialect
- `mongodb` dialect
- Relations (`#[belongs_to]`, `#[has_many]`)
- Soft delete (`#[soft_delete]`)
- Lifecycle hooks

### Generated Code Internals

- Helper structs and their internal fields
- Implementation details of `From` conversions
- Order of generated items

## What Constitutes a Breaking Change

### Breaking (requires MAJOR bump post-1.0)

- Removing or renaming attributes
- Changing attribute behavior
- Removing generated types or methods
- Changing method signatures in generated traits
- Removing feature flags

### Not Breaking (allowed in MINOR/PATCH)

- Adding new attributes
- Adding new methods to generated traits (with default impls)
- Adding new feature flags
- Changing generated SQL (if functionally equivalent)
- Performance improvements
- Bug fixes that correct clearly wrong behavior
- Adding new derive macros to generated types

## Minimum Supported Rust Version (MSRV)

- Current MSRV: **1.92.0**
- MSRV bumps are considered breaking changes pre-1.0
- Post-1.0, MSRV bumps require at least a minor version bump

## Reporting Issues

If you believe a change violated this policy, please open an issue with:

1. Version before and after the change
2. Code that worked before but broke after
3. Expected vs actual behavior

We take backward compatibility seriously and will issue patch releases to
fix accidental breakage.
