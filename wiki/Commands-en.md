# CQRS Commands
Define business-oriented commands instead of generic CRUD. Commands bring domain language to your API and enable the Command Query Responsibility Segregation (CQRS) pattern.
## Quick Start

```rust
#[derive(Entity)]
#[entity(table = "users", commands)]
#[command(Register)]
#[command(UpdateEmail: email)]
#[command(Deactivate, requires_id)]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub email: String,

    #[field(create, response)]
    pub name: String,

    #[field(response)]
    pub active: bool,
}
```

## Generated Code

### Command Structs

```rust
/// Command payload for Register operation on User.
#[derive(Debug, Clone)]
pub struct RegisterUser {
    pub email: String,
    pub name: String,
}

/// Command payload for UpdateEmail operation on User.
#[derive(Debug, Clone)]
pub struct UpdateEmailUser {
    pub id: Uuid,
    pub email: String,
}

/// Command payload for Deactivate operation on User.
#[derive(Debug, Clone)]
pub struct DeactivateUser {
    pub id: Uuid,
}
```

### Command Enum

```rust
/// Command enum for User entity.
#[derive(Debug, Clone)]
pub enum UserCommand {
    Register(RegisterUser),
    UpdateEmail(UpdateEmailUser),
    Deactivate(DeactivateUser),
}

impl EntityCommand for UserCommand {
    fn kind(&self) -> CommandKind {
        match self {
            UserCommand::Register(_) => CommandKind::Create,
            UserCommand::UpdateEmail(_) => CommandKind::Update,
            UserCommand::Deactivate(_) => CommandKind::Custom,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            UserCommand::Register(_) => "Register",
            UserCommand::UpdateEmail(_) => "UpdateEmail",
            UserCommand::Deactivate(_) => "Deactivate",
        }
    }
}
```

### Result Enum

```rust
/// Result enum for User command execution.
#[derive(Debug, Clone)]
pub enum UserCommandResult {
    Register(User),
    UpdateEmail(User),
    Deactivate,
}
```

### Handler Trait

```rust
/// Async trait for handling User commands.
#[async_trait]
pub trait UserCommandHandler: Send + Sync {
    type Error: std::error::Error + Send + Sync;
    type Context: Send + Sync;

    /// Dispatch command to appropriate handler.
    async fn handle(&self, cmd: UserCommand, ctx: &Self::Context)
        -> Result<UserCommandResult, Self::Error>;

    /// Handle Register command.
    async fn handle_register(&self, cmd: RegisterUser, ctx: &Self::Context)
        -> Result<User, Self::Error>;

    /// Handle UpdateEmail command.
    async fn handle_update_email(&self, cmd: UpdateEmailUser, ctx: &Self::Context)
        -> Result<User, Self::Error>;

    /// Handle Deactivate command.
    async fn handle_deactivate(&self, cmd: DeactivateUser, ctx: &Self::Context)
        -> Result<(), Self::Error>;
}
```

## Command Syntax Reference

### Basic Command

Uses all `#[field(create)]` fields:

```rust
#[command(Register)]
// Generated: RegisterUser { email, name }
```

### Specific Fields

Uses only specified fields (adds `requires_id` automatically):

```rust
#[command(UpdateEmail: email)]
// Generated: UpdateEmailUser { id, email }

#[command(UpdateProfile: name, bio, avatar)]
// Generated: UpdateProfileUser { id, name, bio, avatar }
```

### Domain Operation

A command declaring `sets(...)` writes named columns directly, without them being `#[field(update)]` — so they stay out of the public patch DTO and out of the upsert SET list:

```rust
#[command(VerifyPassport, payload(passport_provider), sets(
    passport_verified = "true",
    passport_verified_at = "NOW()"
))]
// Generated: VerifyPassportUser { id, passport_provider }
//            pool.verify_passport(command) -> User
```

One UPDATE writes the fixed expressions plus the payload columns and nothing else. The expressions land in the statement verbatim, like `#[column(default = "...")]`; the column names are checked against the entity at compile time.

With `transactions` on, the same operation is available on the adapter, so it can land together with other writes; there a missing row is `Ok(None)`, like the adapter's other methods.

```rust
let mut tx = pool.begin().await?;
let verified = UserTransactionRepo::new(&mut tx)
    .verify_passport(VerifyPassportUser { id, passport_provider: Some("gov".into()) })
    .await?;
tx.commit().await?;
```


### ID-Only Command

Adds only the ID field:

```rust
#[command(Deactivate, requires_id)]
// Generated: DeactivateUser { id }

#[command(Delete, requires_id, kind = "delete")]
// Generated: DeleteUser { id }, returns ()
```

### Custom Payload

Uses an external struct:

```rust
pub struct TransferPayload {
    pub from_account: Uuid,
    pub to_account: Uuid,
    pub amount: i64,
}

#[command(Transfer, payload = "TransferPayload")]
// Uses TransferPayload directly
```

### Custom Result

Uses a custom result type:

```rust
pub struct TransferResult {
    pub transaction_id: Uuid,
    pub success: bool,
}

#[command(Transfer, payload = "TransferPayload", result = "TransferResult")]
// Returns TransferResult instead of entity
```

### Source Options

Control which fields are used:

```rust
#[command(Create, source = "create")]     // Uses #[field(create)] fields (default)
#[command(Modify, source = "update")]     // Uses #[field(update)] fields (optional)
#[command(Ping, source = "none")]         // No payload fields
```

### Kind Hints

Affect result type inference:

```rust
#[command(Create, kind = "create")]   // Returns entity (default)
#[command(Update, kind = "update")]   // Returns entity
#[command(Remove, kind = "delete")]   // Returns ()
#[command(Process, kind = "custom")]  // Inferred from source
```

## Implementation Example

```rust
use async_trait::async_trait;

struct UserHandler {
    pool: PgPool,
    email_service: EmailService,
}

struct RequestContext {
    user_id: Option<Uuid>,
    correlation_id: Uuid,
}

#[async_trait]
impl UserCommandHandler for UserHandler {
    type Error = AppError;
    type Context = RequestContext;

    async fn handle(&self, cmd: UserCommand, ctx: &Self::Context)
        -> Result<UserCommandResult, Self::Error>
    {
        match cmd {
            UserCommand::Register(c) => {
                let user = self.handle_register(c, ctx).await?;
                Ok(UserCommandResult::Register(user))
            }
            UserCommand::UpdateEmail(c) => {
                let user = self.handle_update_email(c, ctx).await?;
                Ok(UserCommandResult::UpdateEmail(user))
            }
            UserCommand::Deactivate(c) => {
                self.handle_deactivate(c, ctx).await?;
                Ok(UserCommandResult::Deactivate)
            }
        }
    }

    async fn handle_register(&self, cmd: RegisterUser, ctx: &Self::Context)
        -> Result<User, Self::Error>
    {
        // Validate
        if cmd.email.is_empty() {
            return Err(AppError::Validation("Email required".into()));
        }

        // Create user
        let user = User {
            id: Uuid::now_v7(),
            email: cmd.email.to_lowercase(),
            name: cmd.name,
            active: true,
        };

        // Persist
        sqlx::query(
            "INSERT INTO users (id, email, name, active) VALUES ($1, $2, $3, $4)"
        )
        .bind(user.id)
        .bind(&user.email)
        .bind(&user.name)
        .bind(user.active)
        .execute(&self.pool)
        .await?;

        // Side effects
        self.email_service.send_welcome(&user.email).await?;

        Ok(user)
    }

    async fn handle_update_email(&self, cmd: UpdateEmailUser, ctx: &Self::Context)
        -> Result<User, Self::Error>
    {
        // Authorization check
        if ctx.user_id != Some(cmd.id) {
            return Err(AppError::Forbidden("Cannot update other user's email".into()));
        }

        // Update
        let user: User = sqlx::query_as(
            "UPDATE users SET email = $1 WHERE id = $2 RETURNING *"
        )
        .bind(&cmd.email.to_lowercase())
        .bind(cmd.id)
        .fetch_one(&self.pool)
        .await?;

        // Send verification
        self.email_service.send_verification(&user.email).await?;

        Ok(user)
    }

    async fn handle_deactivate(&self, cmd: DeactivateUser, ctx: &Self::Context)
        -> Result<(), Self::Error>
    {
        sqlx::query("UPDATE users SET active = false WHERE id = $1")
            .bind(cmd.id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
```

## Using Commands

```rust
async fn register_user(
    handler: &impl UserCommandHandler,
    email: String,
    name: String,
) -> Result<User, AppError> {
    let cmd = RegisterUser { email, name };
    let ctx = RequestContext {
        user_id: None,
        correlation_id: Uuid::new_v4(),
    };

    match handler.handle(UserCommand::Register(cmd), &ctx).await? {
        UserCommandResult::Register(user) => Ok(user),
        _ => unreachable!(),
    }
}

// Or call specific handler directly
async fn update_email(
    handler: &impl UserCommandHandler,
    user_id: Uuid,
    new_email: String,
    ctx: &RequestContext,
) -> Result<User, AppError> {
    let cmd = UpdateEmailUser {
        id: user_id,
        email: new_email,
    };

    handler.handle_update_email(cmd, ctx).await
}
```

## EntityCommand Trait

All command enums implement the `EntityCommand` trait:

```rust
use entity_derive::{EntityCommand, CommandKind};

let cmd = UserCommand::Register(register_data);

// Get command metadata
assert_eq!(cmd.name(), "Register");
assert!(matches!(cmd.kind(), CommandKind::Create));

// Pattern matching
match cmd.kind() {
    CommandKind::Create => println!("Creating entity"),
    CommandKind::Update => println!("Updating entity"),
    CommandKind::Delete => println!("Deleting entity"),
    CommandKind::Custom => println!("Custom operation"),
}
```

## Command Hooks

When `commands` and `hooks` are both enabled:

```rust
#[derive(Entity)]
#[entity(table = "orders", commands, hooks)]
#[command(Place)]
#[command(Cancel, requires_id)]
pub struct Order { /* ... */ }
```

**Generated hooks:**
```rust
#[async_trait]
pub trait OrderHooks: Send + Sync {
    type Error: std::error::Error + Send + Sync;

    // Standard CRUD hooks...

    // Command-specific hooks
    async fn before_command(&self, cmd: &OrderCommand) -> Result<(), Self::Error>;
    async fn after_command(&self, cmd: &OrderCommand, result: &OrderCommandResult) -> Result<(), Self::Error>;
}
```

**Usage:**
```rust
async fn before_command(&self, cmd: &OrderCommand) -> Result<(), Self::Error> {
    // Log command
    tracing::info!(command = cmd.name(), "Processing command");

    // Authorize
    match cmd {
        OrderCommand::Cancel(c) => {
            let order = self.find_order(c.id).await?;
            if order.status == "shipped" {
                return Err(AppError::Forbidden("Cannot cancel shipped order".into()));
            }
        }
        _ => {}
    }

    Ok(())
}

async fn after_command(&self, cmd: &OrderCommand, result: &OrderCommandResult) -> Result<(), Self::Error> {
    // Audit log
    match (cmd, result) {
        (OrderCommand::Place(c), OrderCommandResult::Place(order)) => {
            self.audit_log("order_placed", order.id).await?;
        }
        (OrderCommand::Cancel(c), OrderCommandResult::Cancel) => {
            self.audit_log("order_cancelled", c.id).await?;
        }
    }

    Ok(())
}
```

## Best Practices

1. **Domain language** — Use business terms: `RegisterUser` not `CreateUser`
2. **Single responsibility** — One command = one business operation
3. **Explicit intent** — Command names should describe the action
4. **Validation in handlers** — Keep validation logic in command handlers
5. **Idempotent when possible** — Design commands to be safely retried
6. **Use context** — Pass request metadata (user, correlation ID) via context

## CQRS Pattern

Commands are one half of CQRS. Combine with projections for the query side:

```rust
#[derive(Entity)]
#[entity(table = "orders", commands)]
#[projection(Summary: id, status, total_cents, created_at)]
#[projection(Details: id, status, items, shipping_address, total_cents)]
#[command(Place)]
#[command(Ship, requires_id)]
#[command(Cancel, requires_id)]
pub struct Order { /* ... */ }

// Commands (write side)
let result = handler.handle(OrderCommand::Place(place_order), &ctx).await?;

// Queries (read side)
let summary = repo.find_by_id_summary(order_id).await?;
let details = repo.find_by_id_details(order_id).await?;
```

## See Also

- [[Hooks]] — Lifecycle hooks including command hooks
- [[Events]] — Event generation for audit logging
- [[Attributes]] — Complete attribute reference
