// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Test for multiple `#[map(...)]` types on the same entity.

use chrono::{DateTime, Utc};
use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "events")]
pub struct Event {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,

    #[field(response)]
    #[map(empty_to_none)]
    pub description: Option<String>,

    #[field(response)]
    #[map(unwrap_default)]
    pub views: Option<i64>,

    #[field(response)]
    #[map(now)]
    pub last_viewed: Option<DateTime<Utc>>,

    #[field(response)]
    pub score: i32,

    #[field(response)]
    pub plain_field: String,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,
}

fn main() {
    // Verify generated types exist
    let _: fn(CreateEventRequest) = |_| {};
    let _: fn(EventResponse) = |_| {};

    // Verify repository trait exists
    fn _check_trait<T: EventRepository>() {}
}
