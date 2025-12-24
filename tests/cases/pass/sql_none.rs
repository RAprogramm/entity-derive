// SPDX-FileCopyrightText: 2025 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "events", sql = "none")]
pub struct Event {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub name: String,

    #[field(create, update, response)]
    pub description: Option<String>,
}

fn main() {
    let create = CreateEventRequest {
        name: "Conference".to_string(),
        description: Some("Annual tech conference".to_string()),
    };
    assert_eq!(create.name, "Conference");

    let response = EventResponse {
        id: Uuid::nil(),
        name: "Conference".to_string(),
        description: None,
    };
    assert_eq!(response.name, "Conference");
}
