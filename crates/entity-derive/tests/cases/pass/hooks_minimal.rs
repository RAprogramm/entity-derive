// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use entity_derive::Entity;
use uuid::Uuid;

/// Entity with hooks but no create/update fields.
/// Tests that hooks trait is generated without create/update methods.
#[derive(Entity, Debug, Clone)]
#[entity(table = "logs", hooks)]
pub struct Log {
    #[id]
    pub id: Uuid,

    #[field(response)]
    pub message: String,
}

struct MockRepo;

#[async_trait]
impl LogHooks for MockRepo {
    type Error = std::io::Error;

    async fn before_delete(&self, _id: &Uuid) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn after_delete(&self, _id: &Uuid) -> Result<(), Self::Error> {
        Ok(())
    }
}

fn main() {
    let repo = MockRepo;
    let _: &dyn LogHooks<Error = std::io::Error> = &repo;
}
