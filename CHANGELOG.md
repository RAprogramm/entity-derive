<!--
SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>

SPDX-License-Identifier: MIT
-->

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased](https://github.com/RAprogramm/entity-derive/compare/v0.8.2...HEAD)

### 🐛 Bug Fixes

- **commands:** Surface parse errors instead of silently dropping invalid #[command(...)] (#130) ([8801315](https://github.com/RAprogramm/entity-derive/commit/8801315da095f84a7a66a1e6314a3e1999793375)) by [@RAprogramm](https://github.com/RAprogramm) in [#130](https://github.com/RAprogramm/entity-derive/pull/130)


### 📚 Documentation

- **hooks:** Clarify manual-wiring requirement; track auto-invocation in #127 (#128) ([61300c6](https://github.com/RAprogramm/entity-derive/commit/61300c6bad3da19c64880372170e2c0ba62da8dc)) by [@RAprogramm](https://github.com/RAprogramm) in [#128](https://github.com/RAprogramm/entity-derive/pull/128)

## [0.8.2](https://github.com/RAprogramm/entity-derive/releases/tag/v0.8.2) — 2026-05-11

### 🐛 Bug Fixes

- **atomicity:** Wrap streams CRUD + pg_notify in one transaction (#125) ([b36292f](https://github.com/RAprogramm/entity-derive/commit/b36292f248a1d365ee35f27a2f627827490d0393)) by [@RAprogramm](https://github.com/RAprogramm) in [#125](https://github.com/RAprogramm/entity-derive/pull/125)

**Full Changelog**: [`v0.8.1...v0.8.2`](https://github.com/RAprogramm/entity-derive/compare/v0.8.1...v0.8.2)
## [0.8.1](https://github.com/RAprogramm/entity-derive/releases/tag/v0.8.1) — 2026-05-11

### 📚 Documentation

- **readme:** Refresh install pin, document tracing, show transactions example (#121) ([5c064ff](https://github.com/RAprogramm/entity-derive/commit/5c064ffff1e41e10bb3178f3b6e44be3bfbdb7fc)) by [@RAprogramm](https://github.com/RAprogramm) in [#121](https://github.com/RAprogramm/entity-derive/pull/121)

**Full Changelog**: [`v0.8.0...v0.8.1`](https://github.com/RAprogramm/entity-derive/compare/v0.8.0...v0.8.1)
## [0.8.0](https://github.com/RAprogramm/entity-derive/releases/tag/v0.8.0) — 2026-05-11

### ✨ Features

- Opt-in tracing instrumentation on generated entity methods (#119) ([1b4b1b6](https://github.com/RAprogramm/entity-derive/commit/1b4b1b6ddaa893ab73b032e24eb9445fa05c6a4a)) by [@RAprogramm](https://github.com/RAprogramm) in [#119](https://github.com/RAprogramm/entity-derive/pull/119)

**Full Changelog**: [`v0.7.3...v0.8.0`](https://github.com/RAprogramm/entity-derive/compare/v0.7.3...v0.8.0)
## [0.7.3](https://github.com/RAprogramm/entity-derive/releases/tag/v0.7.3) — 2026-05-11

### 🐛 Bug Fixes

- **ci:** Enrich release notes with author/PR details (#115) ([7aa50a6](https://github.com/RAprogramm/entity-derive/commit/7aa50a64da7982845421238b73a783db18b968f8)) by [@RAprogramm](https://github.com/RAprogramm) in [#115](https://github.com/RAprogramm/entity-derive/pull/115)


### 📚 Documentation

- **transactions:** Make run_with_commit's silent-rollback contract impossible to miss (#113) ([8da735a](https://github.com/RAprogramm/entity-derive/commit/8da735a7536c248e7f8cebea80600b0263d0a6d5)) by [@RAprogramm](https://github.com/RAprogramm) in [#113](https://github.com/RAprogramm/entity-derive/pull/113)

**Full Changelog**: [`v0.7.2...v0.7.3`](https://github.com/RAprogramm/entity-derive/compare/v0.7.2...v0.7.3)
## [0.7.2](https://github.com/RAprogramm/entity-derive/releases/tag/v0.7.2) — 2026-05-11

### 🐛 Bug Fixes

- **transactions:** Deprecate generated with_*() no-op builder + release 0.7.2 (#111) ([159876d](https://github.com/RAprogramm/entity-derive/commit/159876d34920fddcf235a0d7fcf25c47b812d1f3)) by [@RAprogramm](https://github.com/RAprogramm) in [#111](https://github.com/RAprogramm/entity-derive/pull/111)
- **map:** Emit compile_error for invalid expr and nested-meta failures (#109) ([c6c5557](https://github.com/RAprogramm/entity-derive/commit/c6c555727ae422078616321d81fac7d1f03348d7)) by [@RAprogramm](https://github.com/RAprogramm) in [#109](https://github.com/RAprogramm/entity-derive/pull/109)

**Full Changelog**: [`v0.7.1...v0.7.2`](https://github.com/RAprogramm/entity-derive/compare/v0.7.1...v0.7.2)
## [0.7.1](https://github.com/RAprogramm/entity-derive/releases/tag/v0.7.1) — 2026-05-11

### 🧪 Testing

- Cover Transaction::run commit logic with backend-free unit tests (#105) ([556370f](https://github.com/RAprogramm/entity-derive/commit/556370f3d7baf2f862e0120211af5ec69595a2de)) by [@RAprogramm](https://github.com/RAprogramm) in [#105](https://github.com/RAprogramm/entity-derive/pull/105)

**Full Changelog**: [`v0.7.0...v0.7.1`](https://github.com/RAprogramm/entity-derive/compare/v0.7.0...v0.7.1)
## [0.7.0](https://github.com/RAprogramm/entity-derive/releases/tag/v0.7.0) — 2026-05-11

### 🐛 Bug Fixes

- Transaction::run() now commits explicitly on Ok (#103) ([21ede5e](https://github.com/RAprogramm/entity-derive/commit/21ede5e61ce033aee496305dbd8d6e725887f709)) by [@RAprogramm](https://github.com/RAprogramm) in [#103](https://github.com/RAprogramm/entity-derive/pull/103)

**Full Changelog**: [`v0.6.0...v0.7.0`](https://github.com/RAprogramm/entity-derive/compare/v0.6.0...v0.7.0)
## [0.5.0](https://github.com/RAprogramm/entity-derive/releases/tag/v0.5.0) — 2026-01-08

### ✨ Features

- **migrations:** Add compile-time migration generation (#96) ([ac35092](https://github.com/RAprogramm/entity-derive/commit/ac350921b48f9e8e31cdd5cc040ab3b19da9f626)) by [@RAprogramm](https://github.com/RAprogramm) in [#96](https://github.com/RAprogramm/entity-derive/pull/96)


### 🐛 Bug Fixes

- **ci:** Replace heredoc with echo to fix yaml parsing ([6befde2](https://github.com/RAprogramm/entity-derive/commit/6befde27f2a00e10bdf5f0688e738bf7f67984b0)) by [@RAprogramm](https://github.com/RAprogramm)
- **ci:** Fix yaml heredoc syntax error ([94dc054](https://github.com/RAprogramm/entity-derive/commit/94dc0546b29476456cee0f7c910c8ec589e70ced)) by [@RAprogramm](https://github.com/RAprogramm)
- **changelog:** Add parsers for issue-prefixed commits ([a008ca8](https://github.com/RAprogramm/entity-derive/commit/a008ca83b3fe097093873ee05249adb3ac537fac)) by [@RAprogramm](https://github.com/RAprogramm)
- **ci:** Extract release notes from PR commits when git-cliff fails ([86503ed](https://github.com/RAprogramm/entity-derive/commit/86503eddcb43ff8c5659c94e55e39a0bf31df37c)) by [@RAprogramm](https://github.com/RAprogramm)


### 📚 Documentation

- Add v0.4.0 changelog entries ([f6e00e7](https://github.com/RAprogramm/entity-derive/commit/f6e00e74e310b8caca13d57062986d0e2b989bde)) by [@RAprogramm](https://github.com/RAprogramm)

**Full Changelog**: [`v0.4.0...v0.5.0`](https://github.com/RAprogramm/entity-derive/compare/v0.4.0...v0.5.0)
## [0.3.3](https://github.com/RAprogramm/entity-derive/releases/tag/v0.3.3) — 2026-01-07

### ✨ Features

- Implement SQL methods in TransactionRepo ([6691df5](https://github.com/RAprogramm/entity-derive/commit/6691df539bc6fc617b045227369dcfa5d372cabc)) by [@RAprogramm](https://github.com/RAprogramm)
- **transactions:** Add type-safe transaction scripts support ([3a38be3](https://github.com/RAprogramm/entity-derive/commit/3a38be39f7996ba0a08ed1584b8fe7831da85c0f)) by [@RAprogramm](https://github.com/RAprogramm)


### 🐛 Bug Fixes

- Resolve doc link and formatting issues ([eae9d6e](https://github.com/RAprogramm/entity-derive/commit/eae9d6e7e7a1191932550fde85a92f407bf97c99)) by [@RAprogramm](https://github.com/RAprogramm)


### 🔧 Miscellaneous

- Exclude postgres_impl from coverage ([ca58557](https://github.com/RAprogramm/entity-derive/commit/ca585577acb61d26ce674738a01e2a68a41ce7fe)) by [@RAprogramm](https://github.com/RAprogramm)


### 🧪 Testing

- Add transaction test cases, bump versions ([c36460d](https://github.com/RAprogramm/entity-derive/commit/c36460d7867c0ab85c231bd3985e609aed47feba)) by [@RAprogramm](https://github.com/RAprogramm)
- Add comprehensive tests for transaction module ([0ad2310](https://github.com/RAprogramm/entity-derive/commit/0ad2310aebcfb84e75f2f998aed8725e62939b3c)) by [@RAprogramm](https://github.com/RAprogramm)

**Full Changelog**: [`v0.3.2...v0.3.3`](https://github.com/RAprogramm/entity-derive/compare/v0.3.2...v0.3.3)
## [0.3.2](https://github.com/RAprogramm/entity-derive/releases/tag/v0.3.2) — 2026-01-06

### 🐛 Bug Fixes

- **docs:** Resolve docs.rs build failure for published crates ([22e8cf9](https://github.com/RAprogramm/entity-derive/commit/22e8cf9bf3f313230d850d746b73d88a21e4a89f)) by [@RAprogramm](https://github.com/RAprogramm)


### 🔧 Miscellaneous

- Add SPDX headers to README files ([5fce943](https://github.com/RAprogramm/entity-derive/commit/5fce9437f7243ba0397d3b0b5b67a697e60ad92a)) by [@RAprogramm](https://github.com/RAprogramm)

**Full Changelog**: [`v0.3.1...v0.3.2`](https://github.com/RAprogramm/entity-derive/compare/v0.3.1...v0.3.2)
## [0.3.1](https://github.com/RAprogramm/entity-derive/releases/tag/v0.3.1) — 2026-01-06

### ♻️ Refactor

- Remove dead code and unused methods ([911e724](https://github.com/RAprogramm/entity-derive/commit/911e724f0d9ab91b1a446ef7aa24a0e2582432fa)) by [@RAprogramm](https://github.com/RAprogramm)
- Validate #[id] at parse time, remove all panic! ([f5ccc49](https://github.com/RAprogramm/entity-derive/commit/f5ccc49b6c61a5d1bc396eb4a03d83ec79430758)) by [@RAprogramm](https://github.com/RAprogramm)
- Professional crate structure (sqlx/axum style) ([4090ad5](https://github.com/RAprogramm/entity-derive/commit/4090ad56f9c52ddc8e119a1c665447b82f95ca84)) by [@RAprogramm](https://github.com/RAprogramm)
- Split large files into logical submodules ([859ebda](https://github.com/RAprogramm/entity-derive/commit/859ebdae8e6268f717ee3875856af74a63213d26)) by [@RAprogramm](https://github.com/RAprogramm)
- Separate field semantics from database metadata ([b58e35b](https://github.com/RAprogramm/entity-derive/commit/b58e35bed227b3d5da849e76a65ef924ef27c0f3)) by [@RAprogramm](https://github.com/RAprogramm)
- Extract SQL generation into dialect-specific modules ([530455b](https://github.com/RAprogramm/entity-derive/commit/530455b43371dd3aaecec31fc05045fb762c81f0)) by [@RAprogramm](https://github.com/RAprogramm)


### ✨ Features

- Integrate pg_notify into CRUD operations ([758bdc3](https://github.com/RAprogramm/entity-derive/commit/758bdc3431a2f3dac13e70b02fa1c62bf2a1c323)) by [@RAprogramm](https://github.com/RAprogramm)
- Add streams module with Subscriber generation ([c092a9a](https://github.com/RAprogramm/entity-derive/commit/c092a9a14265d5fa66827f1d36534e7c2901ffd2)) by [@RAprogramm](https://github.com/RAprogramm)
- Add serde derives to events when streams enabled ([9ffb5f8](https://github.com/RAprogramm/entity-derive/commit/9ffb5f8453ec342a156285b5604fcf2bdb88f6c2)) by [@RAprogramm](https://github.com/RAprogramm)
- Add streams attribute to entity parsing ([3a6b524](https://github.com/RAprogramm/entity-derive/commit/3a6b524cfb5c7aa6b12037b5cef6565c3134addd)) by [@RAprogramm](https://github.com/RAprogramm)
- Add StreamError and streams feature to entity-core ([381fe4b](https://github.com/RAprogramm/entity-derive/commit/381fe4b981a7c74daab4bac2d03e025d4479af8e)) by [@RAprogramm](https://github.com/RAprogramm)
- Add policy code generation module ([6ae604d](https://github.com/RAprogramm/entity-derive/commit/6ae604d330affd13f36fb19ea99757d57920f8ef)) by [@RAprogramm](https://github.com/RAprogramm)
- Add policy attribute to entity parsing ([47dba44](https://github.com/RAprogramm/entity-derive/commit/47dba44286f8f7821bee1f91e430572f02cedfce)) by [@RAprogramm](https://github.com/RAprogramm)
- Add PolicyError and PolicyOperation to entity-core ([18d34af](https://github.com/RAprogramm/entity-derive/commit/18d34af9b9522d5fb5dd90452458504cbe21699c)) by [@RAprogramm](https://github.com/RAprogramm)
- Add command hooks to lifecycle hooks trait ([d70f4d6](https://github.com/RAprogramm/entity-derive/commit/d70f4d6e85b676b284b47dc9250ee23d5680aec6)) by [@RAprogramm](https://github.com/RAprogramm)
- Add CQRS command code generation ([4487d56](https://github.com/RAprogramm/entity-derive/commit/4487d567155a458f4923d44ce26b440ba833ab68)) by [@RAprogramm](https://github.com/RAprogramm)
- Add command parsing for CQRS pattern ([ced450e](https://github.com/RAprogramm/entity-derive/commit/ced450ee523aee389a58737c8d4744746981da29)) by [@RAprogramm](https://github.com/RAprogramm)
- Add CommandKind and EntityCommand to entity-core ([ff8ffb7](https://github.com/RAprogramm/entity-derive/commit/ff8ffb7fbe593ba4760a6f859a3450ede0209bd8)) by [@RAprogramm](https://github.com/RAprogramm)
- Add lifecycle hooks for entities ([342b122](https://github.com/RAprogramm/entity-derive/commit/342b122b1766fc3727c237e6ff0a323fb09c7884)) by [@RAprogramm](https://github.com/RAprogramm)
- Add lifecycle events for entities ([0e5883a](https://github.com/RAprogramm/entity-derive/commit/0e5883abc7da87c2205a6c89e8eef01be4804336)) by [@RAprogramm](https://github.com/RAprogramm)
- Extract entity-core crate from monolith ([a6bd8c3](https://github.com/RAprogramm/entity-derive/commit/a6bd8c3b9a741677dfd815da1b5a707c72a6047a)) by [@RAprogramm](https://github.com/RAprogramm)
- Add query filtering with #[filter] attribute ([5da116f](https://github.com/RAprogramm/entity-derive/commit/5da116feb9ea9167456aa08369e771aadb4f52b1)) by [@RAprogramm](https://github.com/RAprogramm)
- Add custom columns support for RETURNING ([db7a666](https://github.com/RAprogramm/entity-derive/commit/db7a6663105f3e453efac3a97c0b376ede1dddcf)) by [@RAprogramm](https://github.com/RAprogramm)
- Add flexible RETURNING clause options ([e4f7071](https://github.com/RAprogramm/entity-derive/commit/e4f70713313a6d95e8b3d778a357db71e232278a)) by [@RAprogramm](https://github.com/RAprogramm)
- Add soft delete support ([cebb6e7](https://github.com/RAprogramm/entity-derive/commit/cebb6e7aaa939e8924856bc3325d165f21c8d58f)) by [@RAprogramm](https://github.com/RAprogramm)
- Add entity projections for partial selects ([cc48d21](https://github.com/RAprogramm/entity-derive/commit/cc48d21b57d7ea285d09c32a6cc2f68b11cf2827)) by [@RAprogramm](https://github.com/RAprogramm)
- Add belongs_to and has_many relation support ([746a77c](https://github.com/RAprogramm/entity-derive/commit/746a77c238979c048a93d645d748d50fed03a1be)) by [@RAprogramm](https://github.com/RAprogramm)
- Add generated code marker comments ([a572926](https://github.com/RAprogramm/entity-derive/commit/a572926066f0b77d61daf61b6647a29f6d4b3209)) by [@RAprogramm](https://github.com/RAprogramm)
- Add pool accessor to repository trait ([f712fd4](https://github.com/RAprogramm/entity-derive/commit/f712fd48e41a43dc9f272ba09521b83f3628a0e1)) by [@RAprogramm](https://github.com/RAprogramm)
- Add custom error type support ([c69b659](https://github.com/RAprogramm/entity-derive/commit/c69b65912506a0790c4d2238bcd8a94f2d801dad)) by [@RAprogramm](https://github.com/RAprogramm)


### 🐛 Bug Fixes

- **ci:** Correct YAML syntax in release notes generation ([397438c](https://github.com/RAprogramm/entity-derive/commit/397438c3bff63c767d01efce6f483ebd93eea972)) by [@RAprogramm](https://github.com/RAprogramm)
- Improve CI release detection and docs.rs configuration ([1977a2c](https://github.com/RAprogramm/entity-derive/commit/1977a2c3bf458341b9e2b0a321f76ccb542cb094)) by [@RAprogramm](https://github.com/RAprogramm)
- Align PolicyRepository list signature with repository trait ([b1ca80f](https://github.com/RAprogramm/entity-derive/commit/b1ca80ff59835830091c2570067639a07b22a5c3)) by [@RAprogramm](https://github.com/RAprogramm)
- Remove unused test helper function ([8abf8b9](https://github.com/RAprogramm/entity-derive/commit/8abf8b9ba90a1a0f2c5cbe54a19f8f1ff70af106)) by [@RAprogramm](https://github.com/RAprogramm)
- Remove unused methods and use derive for Default ([486572c](https://github.com/RAprogramm/entity-derive/commit/486572cc853f309249d139aed1bde8c4a5c5fa85)) by [@RAprogramm](https://github.com/RAprogramm)
- Escape SQL wildcards in LIKE patterns ([995c99a](https://github.com/RAprogramm/entity-derive/commit/995c99a1aaf279c059ebcec0eaa7f23cf204892b)) by [@RAprogramm](https://github.com/RAprogramm)
- Use entity schema for related tables instead of hardcoded 'public' ([bc0e9ac](https://github.com/RAprogramm/entity-derive/commit/bc0e9ac537facfe72fb26d4b64607890fb16a877)) by [@RAprogramm](https://github.com/RAprogramm)
- Use actual id field name in events instead of hardcoded 'id' ([43406f6](https://github.com/RAprogramm/entity-derive/commit/43406f6b3e516371eed998d0eaa9de40daef0ccc)) by [@RAprogramm](https://github.com/RAprogramm)
- Replace unreachable!() with graceful handling ([3938169](https://github.com/RAprogramm/entity-derive/commit/3938169a10b62502a56ff5aedc8ddffe44331c30)) by [@RAprogramm](https://github.com/RAprogramm)
- Use compile-time parse_quote! for default error type ([5a70738](https://github.com/RAprogramm/entity-derive/commit/5a7073871d76bfcc85ad0a605d6d17957eb693b7)) by [@RAprogramm](https://github.com/RAprogramm)
- Replace expect() with proper error handling in field parsing ([c7243df](https://github.com/RAprogramm/entity-derive/commit/c7243df5d6e68482dbcef6b5a498ffb078a5d38e)) by [@RAprogramm](https://github.com/RAprogramm)
- Remove memory leak from .leak() in update_method ([7629b43](https://github.com/RAprogramm/entity-derive/commit/7629b43f5cc5f5368d242a05cc817fe8a874ac4c)) by [@RAprogramm](https://github.com/RAprogramm)
- Use fully qualified syntax to avoid find_by_id ambiguity ([901b036](https://github.com/RAprogramm/entity-derive/commit/901b03651903b6cec573985c2c95a0299051c8b1)) by [@RAprogramm](https://github.com/RAprogramm)
- Use PostgreSQL 18 ([789f2c3](https://github.com/RAprogramm/entity-derive/commit/789f2c36ec27ca238d692235bf5c9ad3b52e0fcf)) by [@RAprogramm](https://github.com/RAprogramm)
- Remove invalid crates-io registry from dependabot ([38f27e3](https://github.com/RAprogramm/entity-derive/commit/38f27e3045da206c1538158e74467a003c9f7430)) by [@RAprogramm](https://github.com/RAprogramm)


### 📚 Documentation

- Compact documentation table with flag links ([7868334](https://github.com/RAprogramm/entity-derive/commit/786833495c0ee384de678b9117d4898295d85aa4)) by [@RAprogramm](https://github.com/RAprogramm)
- Simplify README with wiki links ([48e84c5](https://github.com/RAprogramm/entity-derive/commit/48e84c58c6d920d44bd84fd57655ede9790b8b84)) by [@RAprogramm](https://github.com/RAprogramm)
- Complete README with error attribute, command options, EntityCommand trait ([956e7cc](https://github.com/RAprogramm/entity-derive/commit/956e7cc3b8922c78f1049a1c770414e3fc140070)) by [@RAprogramm](https://github.com/RAprogramm)
- Add Events, Hooks, Commands documentation to README ([ad7af5b](https://github.com/RAprogramm/entity-derive/commit/ad7af5bb7bbdf3b4fa123947a4a5b176319c7361)) by [@RAprogramm](https://github.com/RAprogramm)
- Update README with new features ([17231d8](https://github.com/RAprogramm/entity-derive/commit/17231d8ba4d53932b2a7aece4f02d6d1f77187f0)) by [@RAprogramm](https://github.com/RAprogramm)
- Add Axum CRUD example ([874a3b8](https://github.com/RAprogramm/entity-derive/commit/874a3b88350c892a1fcbff82e0d54e28db8aeb91)) by [@RAprogramm](https://github.com/RAprogramm)
- Add stability policy and semver guarantees ([dc97205](https://github.com/RAprogramm/entity-derive/commit/dc97205661ee4415bf841d4848dab840ec97aca4)) by [@RAprogramm](https://github.com/RAprogramm)
- Enhance module documentation ([f01ce8a](https://github.com/RAprogramm/entity-derive/commit/f01ce8af0ee470c87b9259d53cb625b0a952754e)) by [@RAprogramm](https://github.com/RAprogramm)
- Remove regular comments, use doc comments only ([109119f](https://github.com/RAprogramm/entity-derive/commit/109119ff1778f080fc36d3d79ae1fed49db439ac)) by [@RAprogramm](https://github.com/RAprogramm)


### 🔧 Miscellaneous

- Implement cascading crate publication ([4b254ea](https://github.com/RAprogramm/entity-derive/commit/4b254ea27ecc3cabef2f477ad332af5a0c9274f1)) by [@RAprogramm](https://github.com/RAprogramm)
- Add SPDX header to FUNDING.yml ([b8ad87f](https://github.com/RAprogramm/entity-derive/commit/b8ad87fd2918f85b036d69cccc7276ca7318db03)) by [@RAprogramm](https://github.com/RAprogramm)
- Sync crate versions to 0.3.0 ([c77ac34](https://github.com/RAprogramm/entity-derive/commit/c77ac34e71fa459f15c70ac140a09cc18d00f316)) by [@RAprogramm](https://github.com/RAprogramm)


### 🧪 Testing

- Improve coverage for streams module ([3385fe2](https://github.com/RAprogramm/entity-derive/commit/3385fe2da44e394e96b5c30830c4e50172b61427)) by [@RAprogramm](https://github.com/RAprogramm)
- Add coverage for PolicyError and PolicyOperation ([c56cc3f](https://github.com/RAprogramm/entity-derive/commit/c56cc3f448267f92ef7822ba56bd64c329cc89aa)) by [@RAprogramm](https://github.com/RAprogramm)
- Improve coverage for command pattern ([5559dcb](https://github.com/RAprogramm/entity-derive/commit/5559dcb65d2a4ad29e73349840e35d13e422862a)) by [@RAprogramm](https://github.com/RAprogramm)
- Add trybuild tests for command pattern ([5cb539f](https://github.com/RAprogramm/entity-derive/commit/5cb539f9429056b760b6b49db8da5c1169bd8987)) by [@RAprogramm](https://github.com/RAprogramm)
- Add coverage for hooks without CRUD fields ([cc53389](https://github.com/RAprogramm/entity-derive/commit/cc53389af9bec1a5d13b735d0d13dfe76863adeb)) by [@RAprogramm](https://github.com/RAprogramm)
- Improve coverage with FilterConfig tests ([fc07ff3](https://github.com/RAprogramm/entity-derive/commit/fc07ff3a94056342b432db954f3c7d08734508f1)) by [@RAprogramm](https://github.com/RAprogramm)
- Add returning_id test for coverage ([60f76e6](https://github.com/RAprogramm/entity-derive/commit/60f76e6a6305055a14e79fb55aca98c68b7b4bec)) by [@RAprogramm](https://github.com/RAprogramm)
- Add unit tests for error type parsing ([877ce40](https://github.com/RAprogramm/entity-derive/commit/877ce409ff5afa39e0d388371376e8accb32c9b5)) by [@RAprogramm](https://github.com/RAprogramm)

**Full Changelog**: [`v0.2.0...v0.3.1`](https://github.com/RAprogramm/entity-derive/compare/v0.2.0...v0.3.1)
## [0.2.0](https://github.com/RAprogramm/entity-derive/releases/tag/v0.2.0) — 2025-12-24

### ✨ Features

- Add UUID version selection (v4/v7) ([dfff15b](https://github.com/RAprogramm/entity-derive/commit/dfff15b593872b231dbc8dc3d34314dc4ac28727)) by [@RAprogramm](https://github.com/RAprogramm)
- Add multi-database dialect support ([d26b4be](https://github.com/RAprogramm/entity-derive/commit/d26b4be79f7bcf221b8616f7a1af85623f4c5f86)) by [@RAprogramm](https://github.com/RAprogramm)


### 📚 Documentation

- Fix license link to LICENSES/MIT.txt ([19a33bb](https://github.com/RAprogramm/entity-derive/commit/19a33bb498471639ea6cd11f69916c45c416fb88)) by [@RAprogramm](https://github.com/RAprogramm)
- Add Wiki badge and organize badges by category ([9824162](https://github.com/RAprogramm/entity-derive/commit/98241629475331c208fb69553d177adb44865e47)) by [@RAprogramm](https://github.com/RAprogramm)
- Add SPDX license header to CONTRIBUTING.md ([f376485](https://github.com/RAprogramm/entity-derive/commit/f3764858e990bef8525d63a58b13fb85e2677058)) by [@RAprogramm](https://github.com/RAprogramm)

**Full Changelog**: [`v0.1.6...v0.2.0`](https://github.com/RAprogramm/entity-derive/compare/v0.1.6...v0.2.0)
## [0.1.6](https://github.com/RAprogramm/entity-derive/releases/tag/v0.1.6) — 2025-12-24

### 🎨 Styling

- Update badges to for-the-badge style ([51c78f7](https://github.com/RAprogramm/entity-derive/commit/51c78f7e8dcd051d0b6722684c1c320f2ccdb4b2)) by [@RAprogramm](https://github.com/RAprogramm)


### 🐛 Bug Fixes

- **ci:** Generate release notes before creating tag ([4c1b824](https://github.com/RAprogramm/entity-derive/commit/4c1b824a7c896014db6a90a3dc2cb0b94b862c4e)) by [@RAprogramm](https://github.com/RAprogramm)

**Full Changelog**: [`v0.1.5...v0.1.6`](https://github.com/RAprogramm/entity-derive/compare/v0.1.5...v0.1.6)
## [0.1.5](https://github.com/RAprogramm/entity-derive/releases/tag/v0.1.5) — 2025-12-24

### 🐛 Bug Fixes

- **ci:** Generate release notes directly with git-cliff ([cafbbb9](https://github.com/RAprogramm/entity-derive/commit/cafbbb9e908042fef17aed2bcff14012836204b6)) by [@RAprogramm](https://github.com/RAprogramm)
- **ci:** Fix changelog extraction pattern to handle URL in version header ([8e533f7](https://github.com/RAprogramm/entity-derive/commit/8e533f72bdc8be83f6655d88761e9384251d9b63)) by [@RAprogramm](https://github.com/RAprogramm)

**Full Changelog**: [`v0.1.4...v0.1.5`](https://github.com/RAprogramm/entity-derive/compare/v0.1.4...v0.1.5)
## [0.1.4](https://github.com/RAprogramm/entity-derive/releases/tag/v0.1.4) — 2025-12-24

### 🐛 Bug Fixes

- **ci:** Simplify publish logic with proper exit code handling ([0b42664](https://github.com/RAprogramm/entity-derive/commit/0b42664aff525d6965e5f5a2840f4286ee498534)) by [@RAprogramm](https://github.com/RAprogramm)
- **ci:** Simplify codecov upload to match working configuration ([af65596](https://github.com/RAprogramm/entity-derive/commit/af65596760b6074f606f6011bcde1fab113bced0)) by [@RAprogramm](https://github.com/RAprogramm)

**Full Changelog**: [`v0.1.3...v0.1.4`](https://github.com/RAprogramm/entity-derive/compare/v0.1.3...v0.1.4)
## [0.1.3](https://github.com/RAprogramm/entity-derive/releases/tag/v0.1.3) — 2025-12-24

### ♻️ Refactor

- **ci:** Remove redundant tag trigger ([d99a837](https://github.com/RAprogramm/entity-derive/commit/d99a8375c273f44f338f7f318a465c0ba5750912)) by [@RAprogramm](https://github.com/RAprogramm)


### 🐛 Bug Fixes

- Remove deprecated doc_auto_cfg feature (merged into doc_cfg in 1.92) ([88f10d8](https://github.com/RAprogramm/entity-derive/commit/88f10d835137463d3c5345a7b2f9bd628d18b758)) by [@RAprogramm](https://github.com/RAprogramm)
- **ci:** Extract release notes from CHANGELOG.md correctly ([5e56b1e](https://github.com/RAprogramm/entity-derive/commit/5e56b1eb6e5b9b68d578f051fa05029410585279)) by [@RAprogramm](https://github.com/RAprogramm)

**Full Changelog**: [`v0.1.2...v0.1.3`](https://github.com/RAprogramm/entity-derive/compare/v0.1.2...v0.1.3)
## [0.1.2](https://github.com/RAprogramm/entity-derive/releases/tag/v0.1.2) — 2025-12-24

### 🐛 Bug Fixes

- **ci:** Improve coverage reporting and add docs.rs metadata ([0df17cf](https://github.com/RAprogramm/entity-derive/commit/0df17cf64de32a42f10841b1869df8a5512fc7ab)) by [@RAprogramm](https://github.com/RAprogramm)
- **ci:** Add continue-on-error to cache steps for trybuild compatibility ([18ae752](https://github.com/RAprogramm/entity-derive/commit/18ae7529637c225126c964c6d66b705fd98bfa47)) by [@RAprogramm](https://github.com/RAprogramm)
- **ci:** Update deprecated codecov test-results-action to v5 ([c1dea6e](https://github.com/RAprogramm/entity-derive/commit/c1dea6e06ba570ac7ef3f0518e737eb84a88e6b1)) by [@RAprogramm](https://github.com/RAprogramm)
- **ci:** Simplify fallback release notes ([02b0533](https://github.com/RAprogramm/entity-derive/commit/02b053312a1a625b2efc570cbd4418158ca19e90)) by [@RAprogramm](https://github.com/RAprogramm)
- **ci:** Require all checks to pass before release ([d5bbd2a](https://github.com/RAprogramm/entity-derive/commit/d5bbd2a10bc375b8e2dd7a78f00488cd35129213)) by [@RAprogramm](https://github.com/RAprogramm)
- **ci:** Ignore RUSTSEC-2023-0071 (rsa timing side-channel) ([d05b283](https://github.com/RAprogramm/entity-derive/commit/d05b283e06353c80edb3a2618526c4dfdb9c1980)) by [@RAprogramm](https://github.com/RAprogramm)

**Full Changelog**: [`v0.1.1...v0.1.2`](https://github.com/RAprogramm/entity-derive/compare/v0.1.1...v0.1.2)
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
