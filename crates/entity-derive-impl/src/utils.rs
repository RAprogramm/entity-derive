// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Shared utilities for code generation.
//!
//! This module contains helper functions used across multiple generators.
//!
//! # Submodules
//!
//! - [`fields`] — Field assignment generation for `From` implementations
//! - [`marker`] — Generated code marker comments
//! - [`sql`] — SQL query building utilities (minimal, most SQL in dialect.rs)

pub mod fields;
pub mod marker;
pub mod sql;
