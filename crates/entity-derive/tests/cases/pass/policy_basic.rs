// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity, Debug, Clone)]
#[entity(table = "orders", policy)]
pub struct Order {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub user_id: Uuid,

    #[field(create, update, response)]
    pub status: String,
}

/// Custom authorization error.
#[derive(Debug)]
pub struct AuthError(String);

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for AuthError {}

/// Request context for authorization.
pub struct RequestContext {
    pub user_id: Uuid,
    pub is_admin: bool,
}

/// Custom policy implementation.
struct OwnerPolicy;

#[async_trait]
impl OrderPolicy for OwnerPolicy {
    type Context = RequestContext;
    type Error = AuthError;

    async fn can_create(&self, dto: &CreateOrderRequest, ctx: &Self::Context) -> Result<(), Self::Error> {
        if dto.user_id == ctx.user_id || ctx.is_admin {
            Ok(())
        } else {
            Err(AuthError("cannot create order for another user".into()))
        }
    }

    async fn can_read(&self, _id: &Uuid, _ctx: &Self::Context) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn can_update(&self, _id: &Uuid, _dto: &UpdateOrderRequest, ctx: &Self::Context) -> Result<(), Self::Error> {
        if ctx.is_admin {
            Ok(())
        } else {
            Err(AuthError("only admins can update".into()))
        }
    }

    async fn can_delete(&self, _id: &Uuid, ctx: &Self::Context) -> Result<(), Self::Error> {
        if ctx.is_admin {
            Ok(())
        } else {
            Err(AuthError("only admins can delete".into()))
        }
    }

    async fn can_list(&self, _ctx: &Self::Context) -> Result<(), Self::Error> {
        Ok(())
    }
}

fn main() {
    // Verify generated types exist
    let _: OrderAllowAllPolicy = OrderAllowAllPolicy;

    // Verify policy trait can be implemented
    let _policy: &dyn OrderPolicy<Context = RequestContext, Error = AuthError> = &OwnerPolicy;
}
