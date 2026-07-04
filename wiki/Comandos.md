Define comandos orientados al negocio en lugar de CRUD genérico. Los comandos traen el lenguaje del dominio a tu API y habilitan el patrón Command Query Responsibility Segregation (CQRS).

## Inicio Rápido

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

## Código Generado

### Estructuras de Comando

```rust
/// Payload de comando para operación Register en User.
#[derive(Debug, Clone)]
pub struct RegisterUser {
    pub email: String,
    pub name: String,
}

/// Payload de comando para operación UpdateEmail en User.
#[derive(Debug, Clone)]
pub struct UpdateEmailUser {
    pub id: Uuid,
    pub email: String,
}

/// Payload de comando para operación Deactivate en User.
#[derive(Debug, Clone)]
pub struct DeactivateUser {
    pub id: Uuid,
}
```

### Enum de Comando

```rust
/// Enum de comando para entidad User.
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

### Enum de Resultado

```rust
/// Enum de resultado para ejecución de comando User.
#[derive(Debug, Clone)]
pub enum UserCommandResult {
    Register(User),
    UpdateEmail(User),
    Deactivate,
}
```

### Trait de Handler

```rust
/// Trait async para manejar comandos User.
#[async_trait]
pub trait UserCommandHandler: Send + Sync {
    type Error: std::error::Error + Send + Sync;
    type Context: Send + Sync;

    /// Despachar comando al handler apropiado.
    async fn handle(&self, cmd: UserCommand, ctx: &Self::Context)
        -> Result<UserCommandResult, Self::Error>;

    /// Manejar comando Register.
    async fn handle_register(&self, cmd: RegisterUser, ctx: &Self::Context)
        -> Result<User, Self::Error>;

    /// Manejar comando UpdateEmail.
    async fn handle_update_email(&self, cmd: UpdateEmailUser, ctx: &Self::Context)
        -> Result<User, Self::Error>;

    /// Manejar comando Deactivate.
    async fn handle_deactivate(&self, cmd: DeactivateUser, ctx: &Self::Context)
        -> Result<(), Self::Error>;
}
```

## Referencia de Sintaxis de Comandos

### Comando Básico

Usa todos los campos `#[field(create)]`:

```rust
#[command(Register)]
// Generado: RegisterUser { email, name }
```

### Campos Específicos

Usa solo campos especificados (añade `requires_id` automáticamente):

```rust
#[command(UpdateEmail: email)]
// Generado: UpdateEmailUser { id, email }

#[command(UpdateProfile: name, bio, avatar)]
// Generado: UpdateProfileUser { id, name, bio, avatar }
```

### Comando Solo ID

Añade solo el campo ID:

```rust
#[command(Deactivate, requires_id)]
// Generado: DeactivateUser { id }

#[command(Delete, requires_id, kind = "delete")]
// Generado: DeleteUser { id }, devuelve ()
```

### Payload Personalizado

Usa una estructura externa:

```rust
pub struct TransferPayload {
    pub from_account: Uuid,
    pub to_account: Uuid,
    pub amount: i64,
}

#[command(Transfer, payload = "TransferPayload")]
// Usa TransferPayload directamente
```

### Resultado Personalizado

Usa un tipo de resultado personalizado:

```rust
pub struct TransferResult {
    pub transaction_id: Uuid,
    pub success: bool,
}

#[command(Transfer, payload = "TransferPayload", result = "TransferResult")]
// Devuelve TransferResult en lugar de entidad
```

### Opciones de Source

Controla qué campos se usan:

```rust
#[command(Create, source = "create")]     // Usa campos #[field(create)] (defecto)
#[command(Modify, source = "update")]     // Usa campos #[field(update)] (opcional)
#[command(Ping, source = "none")]         // Sin campos de payload
```

### Hints de Kind

Afectan la inferencia del tipo de resultado:

```rust
#[command(Create, kind = "create")]   // Devuelve entidad (defecto)
#[command(Update, kind = "update")]   // Devuelve entidad
#[command(Remove, kind = "delete")]   // Devuelve ()
#[command(Process, kind = "custom")]  // Inferido de source
```

## Ejemplo de Implementación

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
        // Validar
        if cmd.email.is_empty() {
            return Err(AppError::Validation("Email requerido".into()));
        }

        // Crear usuario
        let user = User {
            id: Uuid::now_v7(),
            email: cmd.email.to_lowercase(),
            name: cmd.name,
            active: true,
        };

        // Persistir
        sqlx::query(
            "INSERT INTO users (id, email, name, active) VALUES ($1, $2, $3, $4)"
        )
        .bind(user.id)
        .bind(&user.email)
        .bind(&user.name)
        .bind(user.active)
        .execute(&self.pool)
        .await?;

        // Efectos secundarios
        self.email_service.send_welcome(&user.email).await?;

        Ok(user)
    }

    async fn handle_update_email(&self, cmd: UpdateEmailUser, ctx: &Self::Context)
        -> Result<User, Self::Error>
    {
        // Verificar autorización
        if ctx.user_id != Some(cmd.id) {
            return Err(AppError::Forbidden("No puede actualizar email de otro usuario".into()));
        }

        // Actualizar
        let user: User = sqlx::query_as(
            "UPDATE users SET email = $1 WHERE id = $2 RETURNING *"
        )
        .bind(&cmd.email.to_lowercase())
        .bind(cmd.id)
        .fetch_one(&self.pool)
        .await?;

        // Enviar verificación
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

## Usando Comandos

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

// O llamar handler específico directamente
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

## Trait EntityCommand

Todos los enums de comando implementan el trait `EntityCommand`:

```rust
use entity_derive::{EntityCommand, CommandKind};

let cmd = UserCommand::Register(register_data);

// Obtener metadatos del comando
assert_eq!(cmd.name(), "Register");
assert!(matches!(cmd.kind(), CommandKind::Create));

// Pattern matching
match cmd.kind() {
    CommandKind::Create => println!("Creando entidad"),
    CommandKind::Update => println!("Actualizando entidad"),
    CommandKind::Delete => println!("Eliminando entidad"),
    CommandKind::Custom => println!("Operación personalizada"),
}
```

## Hooks de Comandos

Cuando `commands` y `hooks` están habilitados:

```rust
#[derive(Entity)]
#[entity(table = "orders", commands, hooks)]
#[command(Place)]
#[command(Cancel, requires_id)]
pub struct Order { /* ... */ }
```

**Hooks generados:**
```rust
#[async_trait]
pub trait OrderHooks: Send + Sync {
    type Error: std::error::Error + Send + Sync;

    // Hooks CRUD estándar...

    // Hooks específicos de comandos
    async fn before_command(&self, cmd: &OrderCommand) -> Result<(), Self::Error>;
    async fn after_command(&self, cmd: &OrderCommand, result: &OrderCommandResult) -> Result<(), Self::Error>;
}
```

## Mejores Prácticas

1. **Lenguaje de dominio** — Usa términos de negocio: `RegisterUser` no `CreateUser`
2. **Responsabilidad única** — Un comando = una operación de negocio
3. **Intención explícita** — Los nombres de comandos deben describir la acción
4. **Validación en handlers** — Mantén la lógica de validación en los handlers
5. **Idempotente cuando sea posible** — Diseña comandos para reintentos seguros
6. **Usa contexto** — Pasa metadatos de request (usuario, ID de correlación) vía contexto

## Patrón CQRS

Los comandos son una mitad de CQRS. Combina con proyecciones para el lado de consultas:

```rust
#[derive(Entity)]
#[entity(table = "orders", commands)]
#[projection(Summary: id, status, total_cents, created_at)]
#[projection(Details: id, status, items, shipping_address, total_cents)]
#[command(Place)]
#[command(Ship, requires_id)]
#[command(Cancel, requires_id)]
pub struct Order { /* ... */ }

// Comandos (lado de escritura)
let result = handler.handle(OrderCommand::Place(place_order), &ctx).await?;

// Consultas (lado de lectura)
let summary = repo.find_by_id_summary(order_id).await?;
let details = repo.find_by_id_details(order_id).await?;
```

## Ver También

- [[Hooks|Ganchos]] — Hooks del ciclo de vida incluyendo hooks de comandos
- [[Eventos|Eventos]] — Generación de eventos para auditoría
- [[Atributos|Atributos]] — Referencia completa de atributos
