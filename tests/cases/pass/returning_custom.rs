// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Test for `#[entity(returning = "id, created_at")]` custom columns.

use chrono::{DateTime, Utc};
use entity_derive::Entity;
use uuid::Uuid;

#[derive(Debug, Clone, Entity)]
#[entity(table = "events", returning = "id, created_at")]
pub struct Event {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,

    #[field(response)]
    #[auto]
    pub updated_at: DateTime<Utc>,
}

fn main() {
    // Verify generated types exist
    let _: fn(CreateEventRequest) = |_| {};
    let _: fn(EventResponse) = |_| {};

    // Verify repository trait exists
    fn _check_trait<T: EventRepository>() {}
}
