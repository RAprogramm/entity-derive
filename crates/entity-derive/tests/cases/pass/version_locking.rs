// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Debug, Clone, Entity)]
#[entity(table = "orders", migrations)]
pub struct Order {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub note: String,

    #[version]
    #[field(response)]
    #[auto]
    pub version: i32,
}

async fn exercise(pool: sqlx::PgPool, order: Order) -> Result<(), sqlx::Error> {
    let patch = UpdateOrderRequest {
        note: Some("changed".into()),
        expected_version: order.version,
    };
    let _updated: Order = pool.update(order.id, patch).await?;
    Ok(())
}

fn main() {
    assert!(Order::MIGRATION_UP.contains("version INTEGER NOT NULL DEFAULT 0"));
    let _ = exercise;
}
