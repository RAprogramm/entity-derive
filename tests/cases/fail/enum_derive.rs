// SPDX-FileCopyrightText: 2025 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use entity_derive::Entity;

#[derive(Entity)]
#[entity(table = "statuses")]
pub enum Status {
    Active,
    Inactive,
}

fn main() {}
