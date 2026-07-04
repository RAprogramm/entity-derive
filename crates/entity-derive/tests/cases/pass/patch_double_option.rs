// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Debug, Clone, Entity)]
#[entity(table = "profiles")]
pub struct Profile {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,

    #[field(create, update, response)]
    pub nickname: Option<String>,
}

async fn exercise(pool: sqlx::PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    let patch: UpdateProfileRequest = serde_json::from_str(r#"{"nickname": null}"#).unwrap();
    let _updated: Profile = pool.update(id, patch).await?;
    Ok(())
}

fn main() {
    let absent: UpdateProfileRequest = serde_json::from_str("{}").unwrap();
    assert_eq!(absent.name, None);
    assert_eq!(absent.nickname, None);

    let null_nick: UpdateProfileRequest = serde_json::from_str(r#"{"nickname": null}"#).unwrap();
    assert_eq!(null_nick.nickname, Some(None));

    let set_nick: UpdateProfileRequest =
        serde_json::from_str(r#"{"nickname": "neo", "name": "Thomas"}"#).unwrap();
    assert_eq!(set_nick.nickname, Some(Some("neo".to_string())));
    assert_eq!(set_nick.name, Some("Thomas".to_string()));

    let json = serde_json::to_string(&UpdateProfileRequest {
        name: None,
        nickname: Some(None),
    })
    .unwrap();
    assert_eq!(json, r#"{"name":null,"nickname":null}"#);

    let _ = exercise;
}
