// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Shared utilities for code generation.
//!
//! This module contains helper functions used across multiple generators.
//!
//! # Submodules
//!
//! - [`docs`] — Documentation extraction from attributes
//! - [`fields`] — Field assignment generation for `From` implementations
//! - [`marker`] — Generated code marker comments

pub mod docs;
pub mod fields;
pub mod marker;
