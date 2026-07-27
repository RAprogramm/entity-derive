<!--
SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
SPDX-License-Identifier: MIT
-->

# Release Process

Releases are driven by [release-plz](https://release-plz.dev).

## Flow

1. Land commits on `main` using the repository subject format
   (`#N feat: ...`, `#N fix: ...`).
2. The `Release-plz` workflow keeps a release PR open, bumping crate
   versions and prepending `CHANGELOG.md` sections.
3. Review and squash-merge the release PR. The workflow then publishes
   `entity-core` → `entity-derive-impl` → `entity-derive` to crates.io
   in dependency order, tags `vX.Y.Z` (facade version) and creates the
   GitHub release.

Publishing has exactly one path: merging the release PR.
`release_always = false` in `release-plz.toml` makes the `release`
command act only on commits that belong to a `release-plz-*` branch, so
nothing reaches crates.io without a reviewed changelog section.

## Versioning policy

Semantic Versioning 2.0.0 with Cargo's `0.x` interpretation:

| Change | Bump while major is `0` | Bump after 1.0 |
|---|---|---|
| Breaking (API or behaviour) | minor | major |
| New capability, additive | patch | minor |
| Fix, docs, internals | patch | patch |

Two mechanisms decide the number, and neither reads the commit subject:

- `cargo semver-checks` runs on every PR and in release-plz. An
  API-incompatible diff forces the breaking bump automatically.
- A **behavioural** break — generated SQL that means something else, a
  changed default, a stricter runtime check — is invisible to
  `cargo semver-checks`. The PR that introduces it must raise the
  version of the affected crates by hand, to the next minor while the
  major is `0`. Merging that PR does not publish: the bumped,
  unpublished version flows into the next release PR.

The subject prefix (`#N feat:`) is not a conventional commit, so
release-plz treats it as a plain message. It still selects the
`CHANGELOG.md` section through the preprocessors in `release-plz.toml`,
but it never decides the version on its own. Do not rely on `feat!:` to
signal a break — bump the version instead.

## Configuration

| File | Purpose |
|------|---------|
| `release-plz.toml` | Per-package settings, changelog headings, publish gating |
| `.github/workflows/release-plz.yml` | The two-mode workflow (`release-pr` + `release`) |

Secrets: `GH_TOKEN` (PAT, so the release PR triggers CI) and
`CARGO_REGISTRY_TOKEN`.

## Supply-chain artefacts

Publishing a GitHub release triggers `release-attestations.yml`, which
repackages the workspace from the tagged commit and attaches to the
release:

| Artefact | What it is |
|---|---|
| `entity-{core,derive-impl,derive}-X.Y.Z.crate` | The archives, byte-identical to the ones on crates.io |
| `entity-derive-sbom.cdx.json` | CycloneDX bill of materials for the workspace |
| `entity-derive-provenance.sigstore.json` | Sigstore bundle covering every artefact above |

A consumer verifies an archive against the provenance without trusting
this repository's word for it:

```bash
gh attestation verify entity-derive-0.22.7.crate --repo RAprogramm/entity-derive
```

## Withdrawing a release

Run the `Yank` workflow with the crate, the version, and optionally the
un-yank flag. It uses the same registry token as publishing, so nothing
has to happen on a maintainer machine. Yanking keeps the version
resolvable for existing lockfiles and stops new dependants from selecting
it; record the reason in the next `CHANGELOG.md` section.

## Verifying a release

Within a few minutes of the release PR merging:

- <https://crates.io/crates/entity-derive> shows the new version.
- <https://docs.rs/entity-derive> rebuilds.
- The tag `vX.Y.Z` and a GitHub release whose body is the `CHANGELOG.md`
  section for that version both exist.

If any of these is missing, inspect the most recent `Release-plz` run on
`main`; `release-plz release` is idempotent and can be re-run through
the workflow's manual dispatch.
