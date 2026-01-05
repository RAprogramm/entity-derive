// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::{Entity, EntityEvent, EventKind};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Entity, Debug, Clone)]
#[entity(table = "articles", events, soft_delete)]
pub struct Article {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub title: String,

    #[field(response)]
    pub deleted_at: Option<DateTime<Utc>>,
}

fn main() {
    let id = Uuid::nil();

    // Test SoftDeleted event
    let event = ArticleEvent::SoftDeleted { id };
    assert_eq!(event.kind(), EventKind::SoftDeleted);
    assert!(event.kind().is_delete());

    // Test Restored event
    let event = ArticleEvent::Restored { id };
    assert_eq!(event.kind(), EventKind::Restored);
    assert!(!event.kind().is_mutation());

    // Test HardDeleted event
    let event = ArticleEvent::hard_deleted(id);
    assert_eq!(event.kind(), EventKind::HardDeleted);
    assert!(event.kind().is_delete());
}
