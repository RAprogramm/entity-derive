# Transactions

Type-safe multi-entity transactions with automatic commit/rollback.

## What are Transactions?

A **database transaction** is a way to group multiple database operations into a single atomic unit. This means:

- **All or nothing**: Either ALL operations succeed, or NONE of them are applied
- **Automatic rollback**: If any operation fails, all previous changes are automatically undone
- **Data consistency**: Your database never ends up in an inconsistent state

### Why do you need transactions?

Imagine you're building a banking app and need to transfer money between accounts:

```
1. Subtract $100 from Account A
2. Add $100 to Account B
```

**Without transactions**, if step 1 succeeds but step 2 fails (network error, database crash, etc.), you've just lost $100! The money was subtracted from A but never added to B.

**With transactions**, if step 2 fails, step 1 is automatically rolled back. The money stays in Account A as if nothing happened.

## Enabling Transactions

Add the `transactions` attribute to your entity:

```rust
use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "accounts", transactions)]  // ← Add this
pub struct Account {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub user_id: Uuid,

    #[field(create, update, response)]
    pub balance: i64,
}
```

## What Gets Generated?

For an entity `Account` with `#[entity(transactions)]`, the macro generates:

### 1. Transaction Repository Adapter

```rust
pub struct AccountTransactionRepo<'t> {
    tx: &'t mut sqlx::Transaction<'static, sqlx::Postgres>,
}
```

This is like your regular repository, but all operations happen inside the transaction.

### 2. Builder Extension Trait

```rust
pub trait TransactionWithAccount<'p> {
    fn with_accounts(self) -> Transaction<'p, PgPool, AccountTransactionRepo<'static>>;
}
```

This adds the `with_accounts()` method to the transaction builder.

## Available Methods

Inside a transaction, you have access to these methods:

| Method | Signature | Description |
|--------|-----------|-------------|
| `create` | `create(dto) -> Result<Entity, Error>` | Insert a new record |
| `find_by_id` | `find_by_id(id) -> Result<Option<Entity>, Error>` | Find by primary key |
| `update` | `update(id, dto) -> Result<Entity, Error>` | Update existing record |
| `delete` | `delete(id) -> Result<bool, Error>` | Delete record (or soft-delete) |
| `list` | `list(limit, offset) -> Result<Vec<Entity>, Error>` | Paginated list |

## Basic Example

```rust
use entity_core::prelude::*;

async fn create_account(pool: &PgPool, user_id: Uuid) -> Result<Account, AppError> {
    Transaction::new(pool)           // 1. Start building transaction
        .with_accounts()              // 2. Add Account repository
        .run(|mut ctx| async move {   // 3. Execute operations
            let account = ctx.accounts().create(CreateAccountRequest {
                user_id,
                balance: 0,
            }).await?;

            Ok(account)               // 4. Return result (auto-commits)
        })
        .await
}
```

### Step by Step:

1. **`Transaction::new(pool)`** — Creates a new transaction builder with your database pool
2. **`.with_accounts()`** — Adds the Account repository to the transaction context
3. **`.run(|mut ctx| async move { ... })`** — Executes your operations inside the transaction
4. **`Ok(account)`** — Returning `Ok` commits the transaction. Returning `Err` rolls it back.

## Complete Example: Money Transfer

This example shows the full power of transactions:

```rust
use entity_core::prelude::*;
use uuid::Uuid;

#[derive(Debug)]
pub enum TransferError {
    Database(sqlx::Error),
    AccountNotFound(Uuid),
    InsufficientFunds { available: i64, requested: i64 },
}

impl From<sqlx::Error> for TransferError {
    fn from(e: sqlx::Error) -> Self {
        TransferError::Database(e)
    }
}

impl From<TransactionError<sqlx::Error>> for TransferError {
    fn from(e: TransactionError<sqlx::Error>) -> Self {
        TransferError::Database(e.into_inner())
    }
}

/// Transfer money between two accounts atomically.
///
/// If ANY step fails, all changes are rolled back automatically.
pub async fn transfer(
    pool: &PgPool,
    from_id: Uuid,
    to_id: Uuid,
    amount: i64,
) -> Result<(), TransferError> {
    Transaction::new(pool)
        .with_accounts()
        .run(|mut ctx| async move {
            // Step 1: Get source account
            let from = ctx.accounts()
                .find_by_id(from_id)
                .await?
                .ok_or(TransferError::AccountNotFound(from_id))?;

            // Step 2: Check if source has enough money
            if from.balance < amount {
                return Err(TransferError::InsufficientFunds {
                    available: from.balance,
                    requested: amount,
                });
            }

            // Step 3: Get destination account
            let to = ctx.accounts()
                .find_by_id(to_id)
                .await?
                .ok_or(TransferError::AccountNotFound(to_id))?;

            // Step 4: Subtract from source
            // If this succeeds but step 5 fails, this will be ROLLED BACK
            ctx.accounts().update(from_id, UpdateAccountRequest {
                balance: Some(from.balance - amount),
                user_id: None,  // Don't change user_id
            }).await?;

            // Step 5: Add to destination
            ctx.accounts().update(to_id, UpdateAccountRequest {
                balance: Some(to.balance + amount),
                user_id: None,
            }).await?;

            // All operations succeeded - transaction will COMMIT
            Ok(())
        })
        .await
}
```

### What happens in different scenarios:

| Scenario | Result |
|----------|--------|
| Both updates succeed | Transaction commits, money transferred |
| Source account not found | Transaction rolls back (no changes) |
| Insufficient funds | Transaction rolls back (no changes) |
| First update succeeds, second fails | Transaction rolls back (first update undone!) |
| Network error mid-transaction | Transaction rolls back (no partial changes) |

## Multiple Entities in One Transaction

You can operate on multiple entities atomically:

```rust
#[derive(Entity)]
#[entity(table = "accounts", transactions)]
pub struct Account {
    #[id]
    pub id: Uuid,
    #[field(create, update, response)]
    pub balance: i64,
}

#[derive(Entity)]
#[entity(table = "transfer_logs", transactions)]
pub struct TransferLog {
    #[id]
    pub id: Uuid,
    #[field(create, response)]
    pub from_account_id: Uuid,
    #[field(create, response)]
    pub to_account_id: Uuid,
    #[field(create, response)]
    pub amount: i64,
    #[auto]
    #[field(response)]
    pub created_at: DateTime<Utc>,
}

async fn transfer_with_logging(
    pool: &PgPool,
    from_id: Uuid,
    to_id: Uuid,
    amount: i64,
) -> Result<TransferLog, AppError> {
    Transaction::new(pool)
        .with_accounts()      // Add Account repo
        .with_transfer_logs() // Add TransferLog repo
        .run(|mut ctx| async move {
            // Update balances
            let from = ctx.accounts().find_by_id(from_id).await?
                .ok_or(AppError::NotFound)?;

            ctx.accounts().update(from_id, UpdateAccountRequest {
                balance: Some(from.balance - amount),
            }).await?;

            let to = ctx.accounts().find_by_id(to_id).await?
                .ok_or(AppError::NotFound)?;

            ctx.accounts().update(to_id, UpdateAccountRequest {
                balance: Some(to.balance + amount),
            }).await?;

            // Create log entry - all in same transaction!
            let log = ctx.transfer_logs().create(CreateTransferLogRequest {
                from_account_id: from_id,
                to_account_id: to_id,
                amount,
            }).await?;

            Ok(log)
        })
        .await
}
```

If the log creation fails, both account updates are rolled back!

## Error Handling

### Automatic Rollback

Any error returned from the closure triggers a rollback:

```rust
Transaction::new(pool)
    .with_accounts()
    .run(|mut ctx| async move {
        ctx.accounts().update(id, dto).await?;  // Succeeds

        // Some validation fails
        if amount < 0 {
            return Err(AppError::InvalidAmount);  // ← Triggers rollback!
        }

        // This never executes, and the update above is undone
        ctx.accounts().update(other_id, other_dto).await?;

        Ok(())
    })
    .await
```

### Transaction Error Types

The `TransactionError` enum tells you what went wrong:

```rust
use entity_core::transaction::TransactionError;

let result = Transaction::new(pool)
    .with_accounts()
    .run(|mut ctx| async move { /* ... */ })
    .await;

match result {
    Ok(value) => {
        println!("Success: {:?}", value);
    }
    Err(e) => {
        // Check what kind of error
        if e.is_begin() {
            println!("Failed to start transaction");
        } else if e.is_operation() {
            println!("Operation failed: {}", e);
        } else if e.is_commit() {
            println!("Failed to commit");
        } else if e.is_rollback() {
            println!("Failed to rollback");
        }

        // Get the inner database error
        let db_error: sqlx::Error = e.into_inner();
    }
}
```

## With Soft Delete

Transactions respect the `soft_delete` attribute:

```rust
#[derive(Entity)]
#[entity(table = "documents", transactions, soft_delete)]
pub struct Document {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub title: String,

    #[field(skip)]
    pub deleted_at: Option<DateTime<Utc>>,  // Required for soft_delete
}

async fn archive_document(pool: &PgPool, id: Uuid) -> Result<bool, AppError> {
    Transaction::new(pool)
        .with_documents()
        .run(|mut ctx| async move {
            // This sets deleted_at = NOW() instead of DELETE
            let deleted = ctx.documents().delete(id).await?;
            Ok(deleted)
        })
        .await
}
```

## Best Practices

### 1. Keep Transactions Short

❌ **Bad**: Long-running transactions

```rust
Transaction::new(pool)
    .with_accounts()
    .run(|mut ctx| async move {
        let account = ctx.accounts().find_by_id(id).await?;

        // DON'T: Call external APIs inside transactions
        let rate = external_api.get_exchange_rate().await?;  // ← SLOW!

        ctx.accounts().update(id, dto).await?;
        Ok(())
    })
    .await
```

✅ **Good**: Do slow operations outside

```rust
// Fetch external data BEFORE starting transaction
let rate = external_api.get_exchange_rate().await?;

Transaction::new(pool)
    .with_accounts()
    .run(|mut ctx| async move {
        ctx.accounts().update(id, UpdateAccountRequest {
            balance: Some(calculate_new_balance(rate)),
        }).await?;
        Ok(())
    })
    .await
```

### 2. Don't Use Transactions for Single Operations

❌ **Unnecessary**:
```rust
Transaction::new(pool)
    .with_users()
    .run(|mut ctx| async move {
        ctx.users().find_by_id(id).await  // Just one operation!
    })
    .await
```

✅ **Better**: Use regular repository
```rust
pool.find_by_id(id).await  // No transaction needed
```

### 3. Handle All Errors Properly

Always make sure errors are propagated with `?`:

```rust
Transaction::new(pool)
    .with_accounts()
    .run(|mut ctx| async move {
        let result = ctx.accounts().update(id, dto).await;

        // DON'T: Swallow errors
        if let Err(e) = result {
            log::error!("Update failed: {}", e);
            // Transaction won't rollback properly!
        }

        // DO: Propagate errors
        ctx.accounts().update(id, dto).await?;  // ← Use ?

        Ok(())
    })
    .await
```

## Common Patterns

### Check-Then-Update

```rust
Transaction::new(pool)
    .with_products()
    .run(|mut ctx| async move {
        let product = ctx.products().find_by_id(id).await?
            .ok_or(AppError::NotFound)?;

        if product.stock < quantity {
            return Err(AppError::OutOfStock);
        }

        ctx.products().update(id, UpdateProductRequest {
            stock: Some(product.stock - quantity),
            ..Default::default()
        }).await?;

        Ok(product)
    })
    .await
```

### Create Multiple Related Records

```rust
Transaction::new(pool)
    .with_orders()
    .with_order_items()
    .run(|mut ctx| async move {
        // Create parent
        let order = ctx.orders().create(CreateOrderRequest {
            customer_id,
            status: "pending".to_string(),
        }).await?;

        // Create children
        for item in items {
            ctx.order_items().create(CreateOrderItemRequest {
                order_id: order.id,
                product_id: item.product_id,
                quantity: item.quantity,
            }).await?;
        }

        Ok(order)
    })
    .await
```

## See Also

- [[Attributes-en|Attributes Reference]] — Complete attribute documentation
- [[Hooks-en|Lifecycle Hooks]] — Run code before/after operations
- [[Commands-en|Commands]] — CQRS command pattern
- [[Events-en|Events]] — Track entity changes
