// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct SizeCm {
    pub length: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Entity, serde::Serialize, serde::Deserialize)]
#[join(airports as origin, on = origin_iata = iata, fields(
    lat as origin_lat: f64
))]
#[entity(table = "parcels")]
pub struct Parcel {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub origin_iata: String,

    /// A JSONB column documents as a free-form object without the
    /// override; with it the schema names the shape it really carries.
    #[field(create, response)]
    #[schema(value_type = Option<SizeCm>)]
    pub size_cm: Option<serde_json::Value>,
}

fn assert_schema<T: utoipa::ToSchema>() {}

fn main() {
    assert_schema::<CreateParcelRequest>();
    assert_schema::<ParcelResponse>();
    assert_schema::<ParcelView>();

    let _size: Option<serde_json::Value> = None::<serde_json::Value>;
}
