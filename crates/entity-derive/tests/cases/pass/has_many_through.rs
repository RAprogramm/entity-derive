// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Debug, Clone, Entity)]
#[entity(table = "users")]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,
}

#[derive(Debug, Clone, Entity)]
#[entity(table = "teams", migrations)]
#[has_many(User, through = "team_members")]
pub struct Team {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,
}

async fn exercise(pool: sqlx::PgPool, team: Uuid, user: Uuid) -> Result<(), sqlx::Error> {
    pool.add_user(team, user).await?;
    let members: Vec<User> = pool.find_users(team).await?;
    let linked: bool = pool.has_user(team, user).await?;
    let removed: bool = pool.remove_user(team, user).await?;
    let _ = (members, linked, removed);
    Ok(())
}

fn main() {
    assert_eq!(Team::MIGRATION_JUNCTIONS.len(), 1);
    assert!(Team::MIGRATION_JUNCTIONS[0].contains("team_members"));
    let _ = exercise;
}
