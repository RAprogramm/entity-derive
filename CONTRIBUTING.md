<!--
SPDX-FileCopyrightText: 2025-2026 RA <contact@revaprogramm.com>
SPDX-License-Identifier: MIT
-->

# Contributing

## Workflow

### 1. Create branch from issue number

```bash
git checkout -b 123
```

Branch name = issue number only.

### 2. Commit format

```bash
git commit -m "#123 feat: add custom class support"
```

Format: `#<issue> <type>: <description>`

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`

### 3. Create PR

- Title: same format as the commit — `#123 feat: add custom class support`.
  A squash merge takes the PR title as the commit subject, and the
  changelog groups releases by the type in that subject; a bare number
  lands the change in no section.
- Description must include: `Closes #123`

### 4. Bump the version only for a behavioural break

Versions are otherwise raised by the release PR, never by hand. The one
exception is a change that breaks behaviour without breaking the API —
`cargo semver-checks` cannot see it, so the PR itself raises the version
of the affected crates to the next minor. See [RELEASE.md](RELEASE.md).

## Before commit

Run locally:

```bash
cargo +nightly fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

`--all-features` is not optional for the test suite: the compile-pass
fixtures under `crates/entity-derive/tests/cases/pass/` declare entities
that use the HTTP, stream and validation attributes, and trybuild
compiles them against whichever features are active. CI runs the same
commands.

## Live Postgres suite

`crates/entity-derive/tests/postgres.rs` runs the *generated* SQL —
migrations, CRUD, upsert, keyset pagination, soft delete, lookups,
projections, optimistic locking — against a real server. Point it at
one and it provisions a throwaway database per test:

```bash
ENTITY_DERIVE_TEST_DATABASE_URL=postgres://postgres@localhost/postgres \
  cargo test -p entity-derive --all-features --test postgres
```

`DATABASE_URL` works as a fallback. Without either variable the tests
print a notice and pass, so no local server is required to contribute;
CI always provides one, and there the missing variable is a hard error.

A test that panics leaves its database behind for inspection — the next
run drops it once it is half an hour old.

## CI checks

| Check | Command |
|-------|---------|
| Format | `cargo +nightly fmt --check` |
| Lint | `cargo clippy -- -D warnings` |
| Test | `cargo test` |
| Live Postgres | `cargo nextest run -p entity-derive --all-features --test postgres` (matrix: Postgres 18, 17) |
| Examples | `cargo check --manifest-path examples/<name>/Cargo.toml --all-targets` |
| Feature combinations | `cargo hack check --workspace --feature-powerset --depth 2 --no-dev-deps` |
| Dependency policy | `cargo deny check` |
| Workflow audit | `zizmor --offline .github/workflows/` — actions must be pinned to a commit SHA |
| Semver | `cargo semver-checks check-release --workspace --exclude entity-derive-impl` |
| Coverage | `cargo llvm-cov` (95%+ required) |

## Code standards

| Rule | Example |
|------|---------|
| No `unwrap()` / `expect()` | Use `?` or `.ok_or()` |
| No unnecessary `clone()` | Pass references |
| `::` only in imports | `use foo::bar` ok, `foo::bar()` bad |
| Doc comments on public items | `/// Description` |
| Max line width | 99 chars |

## Full guidelines

See [RustManifest](https://github.com/RAprogramm/RustManifest)

## Wiki

The GitHub wiki is generated from the `wiki/` directory in this
repository — do not edit wiki pages directly on GitHub. Change the
Markdown files under `wiki/` in the same PR as your code change; on
merge to `main`, the `Publish Wiki` workflow force-syncs the directory
to the wiki. Pages exist in five languages (English plus Spanish,
Russian, Korean and Chinese translations); update all of them when you
touch a documented feature.
