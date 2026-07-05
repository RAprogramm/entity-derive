// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Debug, Clone, Entity, serde::Serialize, serde::Deserialize)]
#[join(airports as origin, on = origin_iata = iata, fields(
    lat as origin_lat: f64,
    city as origin_city: String
))]
#[join(airports as dest, on = destination_iata = iata, fields(
    lat as destination_lat: f64
))]
#[entity(table = "tickets")]
pub struct Ticket {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub origin_iata: String,

    #[field(create, response)]
    pub destination_iata: String,
}

fn assert_schema<T: utoipa::ToSchema>() {}

fn main() {
    let _select: &str = TicketView::SELECT;
    assert_schema::<TicketView>();
}
