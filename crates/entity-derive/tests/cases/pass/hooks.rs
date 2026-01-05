// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity, Debug, Clone)]
#[entity(table = "customers", hooks)]
pub struct Customer {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,

    #[field(create, update, response)]
    pub email: String,
}

/// Mock repository implementing hooks.
struct MockRepo;

#[async_trait]
impl CustomerHooks for MockRepo {
    type Error = std::io::Error;

    async fn before_create(&self, dto: &mut CreateCustomerRequest) -> Result<(), Self::Error> {
        dto.email = dto.email.to_lowercase();
        Ok(())
    }

    async fn after_create(&self, customer: &Customer) -> Result<(), Self::Error> {
        assert!(!customer.email.chars().any(|c| c.is_uppercase()));
        Ok(())
    }

    async fn before_update(
        &self,
        _id: &Uuid,
        dto: &mut UpdateCustomerRequest,
    ) -> Result<(), Self::Error> {
        if let Some(ref mut email) = dto.email {
            *email = email.to_lowercase();
        }
        Ok(())
    }

    async fn after_update(&self, _customer: &Customer) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn before_delete(&self, _id: &Uuid) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn after_delete(&self, _id: &Uuid) -> Result<(), Self::Error> {
        Ok(())
    }
}

fn main() {
    let repo = MockRepo;

    // Verify hooks trait exists and can be implemented
    let _: &dyn CustomerHooks<Error = std::io::Error> = &repo;
}
