// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Test entity with commands enabled but no commands defined.

use entity_derive::Entity;
use uuid::Uuid;

/// Entity with commands flag but no actual command definitions.
/// This should compile without generating any command infrastructure.
#[derive(Entity, Debug, Clone)]
#[entity(table = "empty_commands", commands)]
pub struct EmptyCommands {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub name: String,
}

fn main() {
    // Entity should work normally without command types
    let _entity = EmptyCommands {
        id: Uuid::new_v4(),
        name: String::from("test"),
    };
}
