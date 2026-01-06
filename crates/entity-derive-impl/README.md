# entity-derive-impl

[![Crates.io](https://img.shields.io/crates/v/entity-derive-impl.svg)](https://crates.io/crates/entity-derive-impl)
[![Docs.rs](https://docs.rs/entity-derive-impl/badge.svg)](https://docs.rs/entity-derive-impl)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

Procedural macro implementation for [`entity-derive`](https://crates.io/crates/entity-derive).

**This is an internal crate. Use [`entity-derive`](https://crates.io/crates/entity-derive) directly.**

## Overview

This crate contains the proc-macro implementation for the `#[derive(Entity)]` macro.
It generates:

- DTOs (`CreateRequest`, `UpdateRequest`, `Response`)
- Repository traits and implementations
- Type-safe SQL queries
- Projections and filters

## Documentation

For complete documentation, examples, and usage guide, see:

- **[entity-derive documentation](https://docs.rs/entity-derive)** - Full API reference
- **[GitHub repository](https://github.com/RAprogramm/entity-derive)** - Source code and examples
- **[Wiki](https://github.com/RAprogramm/entity-derive/wiki)** - Comprehensive guides

## License

MIT License - see [LICENSE](https://github.com/RAprogramm/entity-derive/blob/main/LICENSES/MIT.txt)
