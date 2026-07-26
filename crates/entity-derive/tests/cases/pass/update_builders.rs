// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Chainable setters on the update DTO say what a struct literal spells
//! out with nested `Option`s.

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Debug, Clone, Entity)]
#[entity(table = "parcels")]
pub struct Parcel {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub status: String,

    #[field(create, update, response)]
    pub courier_id: Option<Uuid>,

    #[version]
    #[field(response)]
    #[auto]
    pub version: i32,
}

fn main() {
    let courier = Uuid::nil();

    let built = UpdateParcelRequest::default()
        .set_status("accepted".to_owned())
        .set_courier_id(courier)
        .expecting_version(3);

    assert_eq!(built.status.as_deref(), Some("accepted"));
    assert_eq!(built.courier_id, Some(Some(courier)));
    assert_eq!(built.expected_version, 3);

    let cleared = UpdateParcelRequest::default().clear_courier_id();
    assert_eq!(cleared.courier_id, Some(None), "clear_ asks for NULL");
    assert_eq!(cleared.status, None, "an untouched field stays absent");

    // The struct literal keeps working.
    let literal = UpdateParcelRequest {
        status: Some("cancelled".to_owned()),
        courier_id: None,
        expected_version: 1,
    };
    assert_eq!(literal.status.as_deref(), Some("cancelled"));
}
