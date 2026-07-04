// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Debug, Clone, Entity, serde::Serialize, serde::Deserialize)]
#[entity(table = "orders", events(outbox), migrations)]
pub struct Order {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub note: String,
}

async fn exercise(pool: sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(Order::MIGRATION_OUTBOX).execute(&pool).await?;
    sqlx::query(Order::MIGRATION_UP).execute(&pool).await?;
    let order = pool
        .create(CreateOrderRequest {
            note: "hello".into(),
        })
        .await?;
    let _ = pool.delete(order.id).await?;
    Ok(())
}

fn main() {
    assert!(Order::MIGRATION_OUTBOX.contains("CREATE TABLE IF NOT EXISTS entity_outbox"));
    let _ = exercise;
    let _event = OrderEvent::created(Order {
        id: Uuid::nil(),
        note: String::new(),
    });
    let _json = serde_json::to_value(&_event).unwrap();
}
