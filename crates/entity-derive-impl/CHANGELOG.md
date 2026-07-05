# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
