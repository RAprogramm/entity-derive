// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Convenient re-exports for common usage.
//!
//! # Usage
//!
//! ```rust,ignore
//! use entity_core::prelude::*;
//! ```

#[cfg(feature = "streams")]
pub use crate::stream::StreamError;
pub use crate::{
    CommandKind, EntityCommand, EntityEvent, EventKind, Pagination, Repository, SortDirection,
    async_trait,
    policy::{PolicyError, PolicyOperation}
};
