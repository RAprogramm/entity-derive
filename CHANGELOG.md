<!--
SPDX-FileCopyrightText: 2025 RAprogramm <andrey.rozanov.vl@gmail.com>

SPDX-License-Identifier: MIT
-->

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased](https://github.com/RAprogramm/entity-derive/compare/v0.1.1...HEAD)

### 🐛 Bug Fixes

- **ci:** Add continue-on-error to cache steps for trybuild compatibility ([18ae752](https://github.com/RAprogramm/entity-derive/commit/18ae7529637c225126c964c6d66b705fd98bfa47)) by [@RAprogramm](https://github.com/RAprogramm)
- **ci:** Update deprecated codecov test-results-action to v5 ([c1dea6e](https://github.com/RAprogramm/entity-derive/commit/c1dea6e06ba570ac7ef3f0518e737eb84a88e6b1)) by [@RAprogramm](https://github.com/RAprogramm)
- **ci:** Simplify fallback release notes ([02b0533](https://github.com/RAprogramm/entity-derive/commit/02b053312a1a625b2efc570cbd4418158ca19e90)) by [@RAprogramm](https://github.com/RAprogramm)
- **ci:** Require all checks to pass before release ([d5bbd2a](https://github.com/RAprogramm/entity-derive/commit/d5bbd2a10bc375b8e2dd7a78f00488cd35129213)) by [@RAprogramm](https://github.com/RAprogramm)
- **ci:** Ignore RUSTSEC-2023-0071 (rsa timing side-channel) ([d05b283](https://github.com/RAprogramm/entity-derive/commit/d05b283e06353c80edb3a2618526c4dfdb9c1980)) by [@RAprogramm](https://github.com/RAprogramm)

## [0.1.1](https://github.com/RAprogramm/entity-derive/releases/tag/v0.1.1) — 2025-12-24

### 🐛 Bug Fixes

- Mark architecture diagram as text, fix LICENSE link ([05bd3da](https://github.com/RAprogramm/entity-derive/commit/05bd3da7fb829e76c8531b340c5a6149bd65408d)) by [@RAprogramm](https://github.com/RAprogramm)
- Handle already published version gracefully with semver guide ([3965c6a](https://github.com/RAprogramm/entity-derive/commit/3965c6a7e2087d384fc873d0b2b6418f359a3d37)) by [@RAprogramm](https://github.com/RAprogramm)


### 📚 Documentation

- Improve changelog formatting with emojis and links ([76e3d04](https://github.com/RAprogramm/entity-derive/commit/76e3d04af57a0d0c51c457001a512ca1990449f1)) by [@RAprogramm](https://github.com/RAprogramm)
- Comprehensive docs.rs documentation ([c71c50a](https://github.com/RAprogramm/entity-derive/commit/c71c50ad6bd287a8aa1e8890594c7d45913ecec2)) by [@RAprogramm](https://github.com/RAprogramm)
- Add CHANGELOG.md ([34f12f0](https://github.com/RAprogramm/entity-derive/commit/34f12f0a5b9b1ca3066e7ecc5be78bce6e74c3c2)) by [@RAprogramm](https://github.com/RAprogramm)


### 🔧 Miscellaneous

- Configure publish exclude/include for crates.io ([67bc341](https://github.com/RAprogramm/entity-derive/commit/67bc3413029ed72f253503b354b4b47e0360a79b)) by [@RAprogramm](https://github.com/RAprogramm)
- Add REUSE header to CHANGELOG.md ([5d141ae](https://github.com/RAprogramm/entity-derive/commit/5d141ae5e017ab15471f0d905af27eac882f6488)) by [@RAprogramm](https://github.com/RAprogramm)
- Upload test results to Codecov ([581f595](https://github.com/RAprogramm/entity-derive/commit/581f5952f0b15cbcdffa0668a0b4bc5f237f676b)) by [@RAprogramm](https://github.com/RAprogramm)
- **deps:** Bump actions/checkout from 5 to 6 ([620b8f8](https://github.com/RAprogramm/entity-derive/commit/620b8f8e1f60ad9c5fc018df2161946992016c37)) by [@dependabot[bot]](https://github.com/dependabot[bot])
- **deps:** Bump actions/upload-artifact from 4 to 6 ([fe5bed5](https://github.com/RAprogramm/entity-derive/commit/fe5bed534ab1b1cb4251c7373794ef36247cdf3a)) by [@dependabot[bot]](https://github.com/dependabot[bot])


### 🧪 Testing

- Add comprehensive tests with 95%+ coverage ([2ec3701](https://github.com/RAprogramm/entity-derive/commit/2ec37011aaafe51ee9ace07d7071ebc1a4ae6d8b)) by [@RAprogramm](https://github.com/RAprogramm)


### 👋 New Contributors

- [@github-actions[bot]](https://github.com/github-actions[bot]) made their first contribution
- [@dependabot[bot]](https://github.com/dependabot[bot]) made their first contribution

**Full Changelog**: [`v0.1.0...v0.1.1`](https://github.com/RAprogramm/entity-derive/compare/v0.1.0...v0.1.1)
## [0.1.0](https://github.com/RAprogramm/entity-derive/releases/tag/v0.1.0) — 2025-12-24

### ✨ Features

- Entity derive macro for domain code generation ([1c2ab02](https://github.com/RAprogramm/entity-derive/commit/1c2ab024d4ada3c88dac01a796ebfe1e84c65014)) by [@RAprogramm](https://github.com/RAprogramm)


### 🐛 Bug Fixes

- Make GH_TOKEN optional with GITHUB_TOKEN fallback ([0dfb8bf](https://github.com/RAprogramm/entity-derive/commit/0dfb8bfd1534ef89a28cae1ae2574df022aee348)) by [@RAprogramm](https://github.com/RAprogramm)
- Clean advisory-db before cargo audit ([c32c799](https://github.com/RAprogramm/entity-derive/commit/c32c799c75a2a7331403b8b8afc40b5e2503ff51)) by [@RAprogramm](https://github.com/RAprogramm)
- Update deny.toml to v2 format ([f0cc505](https://github.com/RAprogramm/entity-derive/commit/f0cc505e53fd3ae99c2c480d85b070e25d446b1a)) by [@RAprogramm](https://github.com/RAprogramm)


### 📚 Documentation

- Add table of contents and back-to-top links ([3048954](https://github.com/RAprogramm/entity-derive/commit/3048954b1f5510735ff747b18db7dc66b6ffbcc4)) by [@RAprogramm](https://github.com/RAprogramm)
- Add code coverage section with graphs ([1afe033](https://github.com/RAprogramm/entity-derive/commit/1afe0337dd4c3a6dd082c6d962a8664d2c3bdd14)) by [@RAprogramm](https://github.com/RAprogramm)
- Prepare for crates.io publication ([f764e77](https://github.com/RAprogramm/entity-derive/commit/f764e77692e5d4b686414c23738eabf68df8c7f8)) by [@RAprogramm](https://github.com/RAprogramm)
- Add comprehensive documentation, refactor utils ([c2a8079](https://github.com/RAprogramm/entity-derive/commit/c2a80793caa9f055965eaa94fefa0f6a3751367f)) by [@RAprogramm](https://github.com/RAprogramm)


### 🔧 Miscellaneous

- Run automerge only for dependabot PRs ([d991b99](https://github.com/RAprogramm/entity-derive/commit/d991b9910c94fa3d938d8162b902db00bd565de8)) by [@RAprogramm](https://github.com/RAprogramm)
- Add dependabot with auto-merge and grouping ([acbcf71](https://github.com/RAprogramm/entity-derive/commit/acbcf71a1d074e77a6c4291332857da6eec4b417)) by [@RAprogramm](https://github.com/RAprogramm)
- Add dependabot for dependency updates ([d977530](https://github.com/RAprogramm/entity-derive/commit/d977530030761dcce50809b3988743f65656c018)) by [@RAprogramm](https://github.com/RAprogramm)
- Add comprehensive CI workflow ([3844cc1](https://github.com/RAprogramm/entity-derive/commit/3844cc1fc5adac5b2e28f9268eb69df9b1dcbb0c)) by [@RAprogramm](https://github.com/RAprogramm)


### 👋 New Contributors

- [@RAprogramm](https://github.com/RAprogramm) made their first contribution

**Full Changelog**: [`...v0.1.0`](https://github.com/RAprogramm/entity-derive/compare/...v0.1.0)
---

<div align="center">
<sub>Generated with <a href="https://git-cliff.org">git-cliff</a></sub>
</div>
