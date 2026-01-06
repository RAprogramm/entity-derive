// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Streams feature integration tests.
//!
//! Run with: `cargo test --features streams -p entity-derive --test streams`

#![cfg(feature = "streams")]

use entity_derive::{Entity, EntityEvent, EventKind};
use uuid::Uuid;

#[derive(Entity, Debug, Clone, serde::Serialize, serde::Deserialize)]
#[entity(table = "orders", events, streams)]
pub struct Order {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub status: String
}

#[test]
fn channel_constant_exists() {
    assert_eq!(Order::CHANNEL, "entity_orders");
}

#[test]
fn subscriber_type_exists() {
    // Compile-time check that OrderSubscriber exists
    fn _check_subscriber_new(
        pool: &sqlx::PgPool
    ) -> impl std::future::Future<Output = Result<OrderSubscriber, sqlx::Error>> {
        OrderSubscriber::new(pool)
    }
}

#[test]
fn event_serialization() {
    let order = Order {
        id:     Uuid::nil(),
        status: "pending".to_string()
    };

    let event = OrderEvent::created(order.clone());
    let json = serde_json::to_string(&event).expect("serialize");
    let parsed: OrderEvent = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(parsed.kind(), EventKind::Created);
    assert_eq!(parsed.entity_id(), &Uuid::nil());
}

#[test]
fn event_updated_serialization() {
    let old_order = Order {
        id:     Uuid::nil(),
        status: "pending".to_string()
    };
    let new_order = Order {
        id:     Uuid::nil(),
        status: "completed".to_string()
    };

    let event = OrderEvent::updated(old_order, new_order);
    let json = serde_json::to_string(&event).expect("serialize");
    let parsed: OrderEvent = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(parsed.kind(), EventKind::Updated);
}

#[test]
fn event_hard_deleted_serialization() {
    let event = OrderEvent::hard_deleted(Uuid::nil());
    let json = serde_json::to_string(&event).expect("serialize");
    let parsed: OrderEvent = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(parsed.kind(), EventKind::HardDeleted);
    assert_eq!(parsed.entity_id(), &Uuid::nil());
}

// Test streams with soft_delete to cover notify_soft_deleted
#[derive(Entity, Debug, Clone, serde::Serialize, serde::Deserialize)]
#[entity(table = "documents", events, streams, soft_delete)]
pub struct Document {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub title: String,

    #[field(skip)]
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>
}

#[test]
fn soft_delete_channel_exists() {
    assert_eq!(Document::CHANNEL, "entity_documents");
}

#[test]
fn soft_delete_subscriber_exists() {
    fn _check(
        pool: &sqlx::PgPool
    ) -> impl std::future::Future<Output = Result<DocumentSubscriber, sqlx::Error>> {
        DocumentSubscriber::new(pool)
    }
}

#[test]
fn soft_delete_event_serialization() {
    let event = DocumentEvent::SoftDeleted {
        id: Uuid::nil()
    };
    let json = serde_json::to_string(&event).expect("serialize");
    let parsed: DocumentEvent = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(parsed.kind(), EventKind::SoftDeleted);
}

#[test]
fn restored_event_serialization() {
    let event = DocumentEvent::Restored {
        id: Uuid::nil()
    };
    let json = serde_json::to_string(&event).expect("serialize");
    let parsed: DocumentEvent = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(parsed.kind(), EventKind::Restored);
}
