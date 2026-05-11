// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::ValueObject;

#[derive(ValueObject)]
#[value_object(pg_type = "status")]
pub struct NotAnEnum {
    field: String,
}

fn main() {}
