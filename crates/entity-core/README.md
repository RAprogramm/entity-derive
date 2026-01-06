<!--
SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
SPDX-License-Identifier: MIT
-->

# entity-core

[![Crates.io](https://img.shields.io/crates/v/entity-core.svg)](https://crates.io/crates/entity-core)
[![Docs.rs](https://docs.rs/entity-core/badge.svg)](https://docs.rs/entity-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

Core traits and types for [`entity-derive`](https://crates.io/crates/entity-derive).

**This is an internal crate. Use [`entity-derive`](https://crates.io/crates/entity-derive) directly.**

## Overview

This crate provides the foundational types used by entity-derive:

- `Repository<T>` - Base repository trait for CRUD operations
- `Pagination` - Type-safe pagination with offset/limit
- `SortDirection` - Enum for ASC/DESC ordering

## Usage

These types are re-exported from `entity-derive`, so you typically don't need to depend on this crate directly:

```rust
use entity_derive::{Entity, Pagination, SortDirection};

let page = Pagination::page(0, 25);
```

## Documentation

For complete documentation, examples, and usage guide, see:

- **[entity-derive documentation](https://docs.rs/entity-derive)** - Full API reference
- **[GitHub repository](https://github.com/RAprogramm/entity-derive)** - Source code and examples
- **[Wiki](https://github.com/RAprogramm/entity-derive/wiki)** - Comprehensive guides

## License

MIT License - see [LICENSE](https://github.com/RAprogramm/entity-derive/blob/main/LICENSES/MIT.txt)
