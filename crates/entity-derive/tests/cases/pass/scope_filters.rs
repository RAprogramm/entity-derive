// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! `#[scope(...)]` generates a listing over an OR group of columns.

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Debug, Clone, Entity)]
#[entity(table = "disputes")]
#[scope(involving: requester_id | subject_id)]
#[scope(handled: requester_id | subject_id, within = parcel_id)]
pub struct Dispute {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub parcel_id: Uuid,

    #[field(create, response)]
    pub requester_id: Uuid,

    #[field(create, response)]
    pub subject_id: Uuid,
}

async fn exercise(pool: sqlx::PgPool, user: Uuid, parcel: Uuid) -> Result<(), sqlx::Error> {
    let _mine: Vec<Dispute> = pool.list_involving(user, 20, 0).await?;
    let _here: Vec<Dispute> = pool.list_handled(parcel, user, 20, 0).await?;
    Ok(())
}

fn main() {
    let _ = exercise;
}
