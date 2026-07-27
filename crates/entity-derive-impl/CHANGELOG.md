# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.20.16](https://github.com/RAprogramm/entity-derive/compare/entity-derive-impl-v0.20.15...entity-derive-impl-v0.20.16) - 2026-07-27

### ⚙️ CI

- gate feature combinations, dependency policy and semver ([#248](https://github.com/RAprogramm/entity-derive/issues/248))

### ✨ Features

- OpenAPI schema overrides carried to generated structs ([#291](https://github.com/RAprogramm/entity-derive/issues/291))
- repository wrapper invoking the generated hooks ([#267](https://github.com/RAprogramm/entity-derive/issues/267))
- domain operations writing declared columns ([#266](https://github.com/RAprogramm/entity-derive/issues/266))
- participant scopes over an OR group of columns ([#265](https://github.com/RAprogramm/entity-derive/issues/265))
- chainable setters on update DTOs ([#264](https://github.com/RAprogramm/entity-derive/issues/264))

### 🐛 Bug Fixes

- gate the token budget on the SQL generator ([#286](https://github.com/RAprogramm/entity-derive/issues/286))
- reject SQL identifiers that generated statements cannot carry ([#249](https://github.com/RAprogramm/entity-derive/issues/249))
- reach the runtime through the facade in generated code ([#247](https://github.com/RAprogramm/entity-derive/issues/247))
- give auto temporal columns a database default in migrations ([#238](https://github.com/RAprogramm/entity-derive/issues/238))

### 📚 Documentation

- mark the ClickHouse and MongoDB dialects as unimplemented ([#251](https://github.com/RAprogramm/entity-derive/issues/251))

### 🧪 Testing

- execute commands, guards, transitions and the OpenAPI document ([#263](https://github.com/RAprogramm/entity-derive/issues/263))
- execute the remaining generated SQL surfaces against Postgres ([#255](https://github.com/RAprogramm/entity-derive/issues/255))
- budget the size of the generated token stream ([#252](https://github.com/RAprogramm/entity-derive/issues/252))

## [0.20.15](https://github.com/RAprogramm/entity-derive/compare/entity-derive-impl-v0.20.14...entity-derive-impl-v0.20.15) - 2026-07-21

### ✨ Features

- derive utoipa::ToSchema on joined read models under the api feature ([#228](https://github.com/RAprogramm/entity-derive/issues/228))

## [0.20.14](https://github.com/RAprogramm/entity-derive/compare/entity-derive-impl-v0.20.13...entity-derive-impl-v0.20.14) - 2026-07-05

### ✨ Features

- runtime schema assertion for entities ([#225](https://github.com/RAprogramm/entity-derive/issues/225))

## [0.20.13](https://github.com/RAprogramm/entity-derive/compare/entity-derive-impl-v0.20.12...entity-derive-impl-v0.20.13) - 2026-07-05

### ✨ Features

- declarative state-machine transitions on the transaction adapter ([#223](https://github.com/RAprogramm/entity-derive/issues/223))

## [0.20.12](https://github.com/RAprogramm/entity-derive/compare/entity-derive-impl-v0.20.11...entity-derive-impl-v0.20.12) - 2026-07-05

### ✨ Features

- joined read models generated from join declarations ([#221](https://github.com/RAprogramm/entity-derive/issues/221))

## [0.20.11](https://github.com/RAprogramm/entity-derive/compare/entity-derive-impl-v0.20.10...entity-derive-impl-v0.20.11) - 2026-07-05

### ✨ Features

- case-insensitive unique columns via column(ci) ([#213](https://github.com/RAprogramm/entity-derive/issues/213))

## [0.20.10](https://github.com/RAprogramm/entity-derive/compare/entity-derive-impl-v0.20.9...entity-derive-impl-v0.20.10) - 2026-07-05

### 🐛 Bug Fixes

- upsert overwrites only update-marked columns on conflict ([#209](https://github.com/RAprogramm/entity-derive/issues/209))

## [0.20.9](https://github.com/RAprogramm/entity-derive/compare/entity-derive-impl-v0.20.8...entity-derive-impl-v0.20.9) - 2026-07-05

## [0.20.8](https://github.com/RAprogramm/entity-derive/compare/entity-derive-impl-v0.20.7...entity-derive-impl-v0.20.8) - 2026-07-05

### 🐛 Bug Fixes

- avoid needless_question_mark in generated transaction update ([#200](https://github.com/RAprogramm/entity-derive/issues/200))

## [0.20.7](https://github.com/RAprogramm/entity-derive/compare/entity-derive-impl-v0.20.6...entity-derive-impl-v0.20.7) - 2026-07-05

### ✨ Features

- row-locking find_by_id_for_update on the transaction adapter ([#197](https://github.com/RAprogramm/entity-derive/issues/197))

### 🐛 Bug Fixes

- honour error type and typed_constraints in the transaction adapter ([#196](https://github.com/RAprogramm/entity-derive/issues/196))

## [0.20.6](https://github.com/RAprogramm/entity-derive/compare/entity-derive-impl-v0.20.5...entity-derive-impl-v0.20.6) - 2026-07-04

### ✨ Features

- expose upsert on the transaction adapter ([#192](https://github.com/RAprogramm/entity-derive/issues/192))

## [0.20.5](https://github.com/RAprogramm/entity-derive/compare/entity-derive-impl-v0.20.4...entity-derive-impl-v0.20.5) - 2026-07-04

### ✨ Features

- declare custom constraint mappings for the typed_constraints registry ([#190](https://github.com/RAprogramm/entity-derive/issues/190))

## [0.20.4](https://github.com/RAprogramm/entity-derive/compare/entity-derive-impl-v0.20.3...entity-derive-impl-v0.20.4) - 2026-07-04

## [0.20.3](https://github.com/RAprogramm/entity-derive/compare/entity-derive-impl-v0.20.2...entity-derive-impl-v0.20.3) - 2026-07-04

### 🐛 Bug Fixes

- emit feature-dependent derives at expansion time instead of consumer cfgs ([#186](https://github.com/RAprogramm/entity-derive/issues/186))

## [0.20.2](https://github.com/RAprogramm/entity-derive/compare/entity-derive-impl-v0.20.1...entity-derive-impl-v0.20.2) - 2026-07-04

### 🐛 Bug Fixes

- exclude auto fields from INSERT so database defaults apply ([#181](https://github.com/RAprogramm/entity-derive/issues/181))

## [0.20.1](https://github.com/RAprogramm/entity-derive/compare/entity-derive-impl-v0.20.0...entity-derive-impl-v0.20.1) - 2026-07-04

### ✨ Features

- garde validation backend with translated constraints ([#176](https://github.com/RAprogramm/entity-derive/issues/176))
