// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Test for `#[entity(returning = "none")]` attribute.

use chrono::{DateTime, Utc};
use entity_derive::Entity;
use uuid::Uuid;

#[derive(Debug, Clone, Entity)]
#[entity(table = "logs", returning = "none")]
pub struct Log {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub message: String,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,
}

fn main() {
    // Verify generated types exist
    let _: fn(CreateLogRequest) = |_| {};
    let _: fn(LogResponse) = |_| {};

    // Verify repository trait exists
    fn _check_trait<T: LogRepository>() {}
}
