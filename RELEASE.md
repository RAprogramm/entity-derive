<!--
SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
SPDX-License-Identifier: MIT
-->

# Release Process

Releases are fully automated by [release-plz](https://release-plz.dev).

## Flow

1. Land conventional commits on `main` (`#N feat: ...`, `#N fix: ...`).
2. The `Release-plz` workflow keeps a release PR open, bumping crate
   versions from the commit history and prepending `CHANGELOG.md`
   sections. Do not bump versions by hand.
3. Review and squash-merge the release PR. The workflow then publishes
   `entity-core` → `entity-derive-impl` → `entity-derive` to crates.io
   in dependency order, tags `vX.Y.Z` (facade version) and creates the
   GitHub release.

## Configuration

| File | Purpose |
|------|---------|
| `release-plz.toml` | Per-package settings, changelog headings |
| `.github/workflows/release-plz.yml` | The two-mode workflow (`release-pr` + `release`) |

Secrets: `GH_TOKEN` (PAT, so the release PR triggers CI) and
`CARGO_REGISTRY_TOKEN`.
