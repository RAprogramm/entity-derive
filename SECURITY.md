<!--
SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
SPDX-License-Identifier: MIT
-->

# Security Policy

## Supported versions

Fixes land on the latest published minor of each crate. Older minors are
not patched — upgrade instead.

| Crate | Supported |
|---|---|
| `entity-derive` | 0.22.x |
| `entity-derive-impl` | 0.20.x |
| `entity-core` | 0.10.x |

## Reporting a vulnerability

**Do not open a public issue.**

| Channel | Link |
|---|---|
| Private advisory | <https://github.com/RAprogramm/entity-derive/security/advisories/new> |
| Email | andrey.rozanov.vl@gmail.com |

Include the entity definition that reproduces the problem, the feature
set it was built with, the generated code or SQL if you have it, and the
impact you see.

| Stage | Timeline |
|---|---|
| Acknowledgement | 48 hours |
| Assessment | 7 days |
| Fix | by severity; a patch release follows the same automated pipeline as any other release |

Disclosure is coordinated: the advisory becomes public once a fixed
version is on crates.io.

## What is in scope

The macro turns an entity definition into SQL, DTOs, HTTP handlers and
migrations. In scope:

- Generated SQL that interpolates a value instead of binding it, or that
  lets a caller-supplied string change the statement's shape.
- A filter, scope, projection or policy wrapper that returns rows it was
  declared to exclude — soft-deleted rows, other tenants' rows, columns
  left out of a response DTO.
- Generated HTTP handlers that serialize more than the response DTO
  declares, or that skip a declared guard.
- Identifier validation that accepts a name the generated statement
  cannot safely carry.
- Anything in the published crates that executes code at build time
  beyond expanding the macro.

Out of scope: vulnerabilities in dependencies (report them upstream;
`cargo deny check` runs here and tracks advisories), and misuse of the
generated API by a consumer crate — for example passing an unvalidated
string where the documentation requires a bound parameter.

## Supply chain

Releases are published only by the `Release-plz` workflow from a merged
release PR, never from a maintainer machine. Dependencies are checked
against the RustSec advisory database, a licence allow-list and a source
allow-list on every pull request; see `deny.toml`.
