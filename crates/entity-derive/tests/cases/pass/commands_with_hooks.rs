// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use entity_derive::{Entity, EntityCommand};
use uuid::Uuid;

#[derive(Entity, Debug, Clone)]
#[entity(table = "orders", commands, hooks)]
#[command(Place)]
#[command(Cancel, requires_id)]
pub struct Order {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub customer_id: Uuid,

    #[field(create, response)]
    pub total: i64,
}

/// Mock repository with hooks.
struct MockRepo;

#[async_trait]
impl OrderHooks for MockRepo {
    type Error = std::io::Error;

    async fn before_create(&self, _dto: &mut CreateOrderRequest) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn after_create(&self, _entity: &Order) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn before_delete(&self, _id: &Uuid) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn after_delete(&self, _id: &Uuid) -> Result<(), Self::Error> {
        Ok(())
    }

    // Command hooks
    async fn before_command(&self, cmd: &OrderCommand) -> Result<(), Self::Error> {
        // Log or authorize command
        match cmd {
            OrderCommand::Place(_) => println!("Placing order..."),
            OrderCommand::Cancel(_) => println!("Canceling order..."),
        }
        Ok(())
    }

    async fn after_command(
        &self,
        cmd: &OrderCommand,
        result: &OrderCommandResult,
    ) -> Result<(), Self::Error> {
        // Audit log after command
        match (cmd, result) {
            (OrderCommand::Place(_), OrderCommandResult::Place(order)) => {
                println!("Order {} placed", order.id);
            }
            (OrderCommand::Cancel(_), OrderCommandResult::Cancel) => {
                println!("Order canceled");
            }
            _ => {}
        }
        Ok(())
    }
}

/// Mock command handler.
struct MockHandler;

/// Context for command execution.
struct Context;

#[async_trait]
impl OrderCommandHandler for MockHandler {
    type Error = std::io::Error;
    type Context = Context;

    async fn handle_place(
        &self,
        cmd: PlaceOrder,
        _ctx: &Self::Context,
    ) -> Result<Order, Self::Error> {
        Ok(Order {
            id: Uuid::new_v4(),
            customer_id: cmd.customer_id,
            total: cmd.total,
        })
    }

    async fn handle_cancel(
        &self,
        _cmd: CancelOrder,
        _ctx: &Self::Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

fn main() {
    let _repo = MockRepo;
    let _handler = MockHandler;

    // Verify both hooks and commands work together
    let place = PlaceOrder {
        customer_id: Uuid::new_v4(),
        total: 9900,
    };

    let cmd = OrderCommand::Place(place);
    assert_eq!(cmd.name(), "Place");
}
