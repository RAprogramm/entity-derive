// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity, Debug, Clone)]
#[entity(table = "posts", hooks, soft_delete)]
pub struct Post {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub title: String,

    #[field(response)]
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Mock repository with soft delete hooks.
struct MockRepo;

#[async_trait]
impl PostHooks for MockRepo {
    type Error = std::io::Error;

    async fn before_create(&self, _dto: &mut CreatePostRequest) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn after_create(&self, _post: &Post) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn before_update(
        &self,
        _id: &Uuid,
        _dto: &mut UpdatePostRequest,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn after_update(&self, _post: &Post) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn before_delete(&self, _id: &Uuid) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn after_delete(&self, _id: &Uuid) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn before_hard_delete(&self, _id: &Uuid) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn after_hard_delete(&self, _id: &Uuid) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn before_restore(&self, _id: &Uuid) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn after_restore(&self, _id: &Uuid) -> Result<(), Self::Error> {
        Ok(())
    }
}

fn main() {
    let repo = MockRepo;

    // Verify hooks trait exists with soft delete methods
    let _: &dyn PostHooks<Error = std::io::Error> = &repo;
}
