// SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
// SPDX-License-Identifier: MIT

//! Test all command variants for full coverage.

use async_trait::async_trait;
use entity_derive::{Entity, EntityCommand};
use uuid::Uuid;

/// Custom transform payload.
#[derive(Debug, Clone)]
pub struct TransformPayload {
    pub factor: i64,
}

#[derive(Entity, Debug, Clone)]
#[entity(table = "products", commands)]
#[command(Create)]
#[command(Modify, source = "update")]
#[command(Delete, requires_id, kind = "delete")]
#[command(Archive, requires_id, kind = "custom")]
#[command(Restore, source = "none")]
#[command(Process, kind = "custom")]
#[command(Purge, kind = "delete")]
#[command(Transform, payload = "TransformPayload")]
pub struct Product {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,

    #[field(create, update, response)]
    pub price: i64,
}

struct MockHandler;
struct Context;

#[async_trait]
impl ProductCommandHandler for MockHandler {
    type Error = std::io::Error;
    type Context = Context;

    async fn handle_create(
        &self,
        cmd: CreateProduct,
        _ctx: &Self::Context,
    ) -> Result<Product, Self::Error> {
        Ok(Product {
            id: Uuid::new_v4(),
            name: cmd.name,
            price: cmd.price,
        })
    }

    async fn handle_modify(
        &self,
        cmd: ModifyProduct,
        _ctx: &Self::Context,
    ) -> Result<Product, Self::Error> {
        Ok(Product {
            id: cmd.id,
            name: cmd.name.unwrap_or_default(),
            price: cmd.price.unwrap_or_default(),
        })
    }

    async fn handle_delete(
        &self,
        _cmd: DeleteProduct,
        _ctx: &Self::Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn handle_archive(
        &self,
        _cmd: ArchiveProduct,
        _ctx: &Self::Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn handle_restore(
        &self,
        _cmd: RestoreProduct,
        _ctx: &Self::Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn handle_process(
        &self,
        cmd: ProcessProduct,
        _ctx: &Self::Context,
    ) -> Result<Product, Self::Error> {
        Ok(Product {
            id: Uuid::new_v4(),
            name: cmd.name,
            price: cmd.price,
        })
    }

    async fn handle_purge(
        &self,
        _cmd: PurgeProduct,
        _ctx: &Self::Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn handle_transform(
        &self,
        _cmd: TransformPayload,
        _ctx: &Self::Context,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

fn main() {
    // Verify Create command (source = create fields)
    let create = CreateProduct {
        name: String::from("Widget"),
        price: 1000,
    };
    let cmd = ProductCommand::Create(create);
    assert_eq!(cmd.name(), "Create");
    assert!(matches!(cmd.kind(), entity_derive::CommandKind::Create));

    // Verify Modify command (source = update, optional fields)
    let modify = ModifyProduct {
        id: Uuid::new_v4(),
        name: Some(String::from("Updated")),
        price: None,
    };
    let cmd = ProductCommand::Modify(modify);
    assert_eq!(cmd.name(), "Modify");
    assert!(matches!(cmd.kind(), entity_derive::CommandKind::Update));

    // Verify Delete command (kind = delete, unit result)
    let delete = DeleteProduct { id: Uuid::new_v4() };
    let cmd = ProductCommand::Delete(delete);
    assert_eq!(cmd.name(), "Delete");
    assert!(matches!(cmd.kind(), entity_derive::CommandKind::Delete));

    // Verify Archive command (kind = custom with requires_id)
    let archive = ArchiveProduct { id: Uuid::new_v4() };
    let cmd = ProductCommand::Archive(archive);
    assert_eq!(cmd.name(), "Archive");
    assert!(matches!(cmd.kind(), entity_derive::CommandKind::Custom));

    // Verify Restore command (source = none)
    let restore = RestoreProduct {};
    let cmd = ProductCommand::Restore(restore);
    assert_eq!(cmd.name(), "Restore");

    // Verify Process command (kind = custom with create source, returns entity)
    let process = ProcessProduct {
        name: String::from("Process"),
        price: 500,
    };
    let cmd = ProductCommand::Process(process);
    assert_eq!(cmd.name(), "Process");
    assert!(matches!(cmd.kind(), entity_derive::CommandKind::Custom));

    // Verify result variants (unit results for delete/archive/restore)
    let _result: ProductCommandResult = ProductCommandResult::Delete;
    let _result: ProductCommandResult = ProductCommandResult::Archive;
    let _result: ProductCommandResult = ProductCommandResult::Restore;

    // Process returns Product (custom kind with non-custom source)
    let _result: ProductCommandResult = ProductCommandResult::Process(Product {
        id: Uuid::new_v4(),
        name: String::new(),
        price: 0,
    });

    // Purge: kind = delete with create source (hits CommandKindHint::Delete branch)
    let purge = PurgeProduct {
        name: String::from("Purge"),
        price: 0,
    };
    let cmd = ProductCommand::Purge(purge);
    assert_eq!(cmd.name(), "Purge");
    assert!(matches!(cmd.kind(), entity_derive::CommandKind::Delete));
    let _result: ProductCommandResult = ProductCommandResult::Purge;

    // Transform: custom payload without result (hits Custom source branch)
    let transform = TransformPayload { factor: 2 };
    let cmd = ProductCommand::Transform(transform);
    assert_eq!(cmd.name(), "Transform");
    assert!(matches!(cmd.kind(), entity_derive::CommandKind::Custom));
    let _result: ProductCommandResult = ProductCommandResult::Transform;
}
