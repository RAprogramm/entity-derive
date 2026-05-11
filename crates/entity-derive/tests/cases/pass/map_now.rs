// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Test for `#[map(now)]` attribute.

use chrono::{DateTime, Utc};
use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "sessions")]
pub struct Session {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub user_id: Uuid,

    #[field(response)]
    #[map(now)]
    pub last_active: Option<DateTime<Utc>>,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,
}

fn main() {
    // Verify generated types exist
    let _: fn(CreateSessionRequest) = |_| {};
    let _: fn(SessionResponse) = |_| {};

    // Verify repository trait exists
    fn _check_trait<T: SessionRepository>() {}
}
