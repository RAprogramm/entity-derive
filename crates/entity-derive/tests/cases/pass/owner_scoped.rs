// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "orders", soft_delete)]
pub struct Order {
    #[id]
    pub id: Uuid,

    #[owner]
    pub user_id: Uuid,

    #[field(create, update, response)]
    pub note: String,

    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
}

fn assert_scoped_signatures() {
    fn requires_scoped<R>()
    where
        R: OrderRepository + ?Sized
    {
    }

    requires_scoped::<sqlx::PgPool>();
}

async fn exercise(pool: sqlx::PgPool, id: Uuid, user: Uuid) -> Result<(), sqlx::Error> {
    let _found: Option<Order> = pool.find_by_id_scoped(id, user).await?;
    let _mine: Vec<Order> = pool.list_by_owner(user, 10, 0).await?;
    let _updated: Option<Order> = pool
        .update_scoped(id, user, UpdateOrderRequest::default())
        .await?;
    let _gone: bool = pool.delete_scoped(id, user).await?;
    Ok(())
}

fn main() {
    assert_scoped_signatures();
    let _ = exercise;
}
