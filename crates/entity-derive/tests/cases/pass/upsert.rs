// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "users", transactions, upsert(conflict = "email"))]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    #[column(unique)]
    pub email: String,

    #[field(create, update, response)]
    pub name: String,
}

#[derive(Entity)]
#[entity(
    table = "members",
    unique_index(tenant_id, email),
    upsert(conflict = "tenant_id, email", action = "nothing")
)]
pub struct Member {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub tenant_id: Uuid,

    #[field(create, response)]
    pub email: String,

    #[field(create, update, response)]
    pub role: String,
}

fn assert_upsert_signatures() {
    fn takes_update_style<R>()
    where
        R: UserRepository + ?Sized
    {
    }

    fn takes_nothing_style<R>()
    where
        R: MemberRepository + ?Sized
    {
    }

    takes_update_style::<sqlx::PgPool>();
    takes_nothing_style::<sqlx::PgPool>();
}

async fn exercise_tx_upsert(pool: sqlx::PgPool, dto: CreateUserRequest) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    let mut repo = UserTransactionRepo::new(&mut tx);
    let _user: User = repo.upsert(dto).await?;
    tx.commit().await?;
    Ok(())
}

fn main() {
    assert_upsert_signatures();
    let _ = exercise_tx_upsert;
}
