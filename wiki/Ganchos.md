Ejecuta lógica personalizada antes y después de operaciones de entidad. Los hooks permiten validación, normalización, efectos secundarios y autorización.

## Inicio Rápido

```rust
#[derive(Entity)]
#[entity(table = "users", hooks)]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub email: String,

    #[field(create, response)]
    pub name: String,

    #[field(skip)]
    pub password_hash: String,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,
}
```

## Código Generado

El atributo `hooks` genera un trait async:

```rust
/// Generado por entity-derive
#[async_trait]
pub trait UserHooks: Send + Sync {
    type Error: std::error::Error + Send + Sync;

    /// Llamado antes de crear una nueva entidad.
    /// Modifica el DTO o devuelve error para abortar.
    async fn before_create(&self, dto: &mut CreateUserRequest) -> Result<(), Self::Error>;

    /// Llamado después de la creación de entidad.
    async fn after_create(&self, entity: &User) -> Result<(), Self::Error>;

    /// Llamado antes de actualizar una entidad.
    /// Modifica el DTO o devuelve error para abortar.
    async fn before_update(&self, id: &Uuid, dto: &mut UpdateUserRequest) -> Result<(), Self::Error>;

    /// Llamado después de actualizar entidad.
    async fn after_update(&self, entity: &User) -> Result<(), Self::Error>;

    /// Llamado antes de eliminar una entidad.
    /// Devuelve error para abortar.
    async fn before_delete(&self, id: &Uuid) -> Result<(), Self::Error>;

    /// Llamado después de eliminar entidad.
    async fn after_delete(&self, id: &Uuid) -> Result<(), Self::Error>;
}
```

## Ejemplo de Implementación

```rust
use async_trait::async_trait;

struct UserService {
    pool: PgPool,
    cache: RedisPool,
    email_sender: EmailService,
}

#[async_trait]
impl UserHooks for UserService {
    type Error = AppError;

    async fn before_create(&self, dto: &mut CreateUserRequest) -> Result<(), Self::Error> {
        // Normalizar email
        dto.email = dto.email.trim().to_lowercase();

        // Validar formato de email
        if !dto.email.contains('@') {
            return Err(AppError::Validation("Formato de email inválido".into()));
        }

        // Verificar email duplicado
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)"
        )
        .bind(&dto.email)
        .fetch_one(&self.pool)
        .await?;

        if exists {
            return Err(AppError::Conflict("Email ya registrado".into()));
        }

        Ok(())
    }

    async fn after_create(&self, entity: &User) -> Result<(), Self::Error> {
        // Enviar email de bienvenida
        self.email_sender
            .send_welcome(&entity.email, &entity.name)
            .await?;

        // Cachear el nuevo usuario
        self.cache.set(&format!("user:{}", entity.id), entity).await?;

        Ok(())
    }

    async fn before_update(&self, id: &Uuid, dto: &mut UpdateUserRequest) -> Result<(), Self::Error> {
        // Normalizar email si se proporciona
        if let Some(ref mut email) = dto.email {
            *email = email.trim().to_lowercase();

            // Verificar duplicado (excluyendo usuario actual)
            let exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM users WHERE email = $1 AND id != $2)"
            )
            .bind(&*email)
            .bind(id)
            .fetch_one(&self.pool)
            .await?;

            if exists {
                return Err(AppError::Conflict("Email ya en uso".into()));
            }
        }

        Ok(())
    }

    async fn after_update(&self, entity: &User) -> Result<(), Self::Error> {
        // Invalidar caché
        self.cache.del(&format!("user:{}", entity.id)).await?;

        Ok(())
    }

    async fn before_delete(&self, id: &Uuid) -> Result<(), Self::Error> {
        // Verificar si el usuario puede ser eliminado
        let has_orders = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM orders WHERE user_id = $1 AND status = 'pending')"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        if has_orders {
            return Err(AppError::Forbidden("No se puede eliminar usuario con pedidos pendientes".into()));
        }

        Ok(())
    }

    async fn after_delete(&self, id: &Uuid) -> Result<(), Self::Error> {
        // Invalidar caché
        self.cache.del(&format!("user:{}", id)).await?;

        // Limpiar datos relacionados
        sqlx::query("DELETE FROM user_sessions WHERE user_id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
```

## Casos de Uso

### Validación

```rust
async fn before_create(&self, dto: &mut CreateProductRequest) -> Result<(), Self::Error> {
    // Validación de precio
    if dto.price_cents <= 0 {
        return Err(AppError::Validation("El precio debe ser positivo".into()));
    }

    // Validación de formato SKU
    if !dto.sku.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err(AppError::Validation("Formato de SKU inválido".into()));
    }

    Ok(())
}
```

### Normalización

```rust
async fn before_create(&self, dto: &mut CreateUserRequest) -> Result<(), Self::Error> {
    // Normalizar email
    dto.email = dto.email.trim().to_lowercase();

    // Normalizar nombre
    dto.name = dto.name.trim().to_string();

    // Capitalizar primera letra de cada palabra
    dto.name = dto.name
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    Ok(())
}
```

### Autorización

```rust
async fn before_update(&self, id: &Uuid, _dto: &mut UpdatePostRequest) -> Result<(), Self::Error> {
    // Obtener usuario actual del contexto
    let current_user = self.current_user()?;

    // Verificar propiedad
    let post = sqlx::query_as::<_, Post>(
        "SELECT * FROM posts WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&self.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    if post.author_id != current_user.id && !current_user.is_admin {
        return Err(AppError::Forbidden("No puede editar posts de otros usuarios".into()));
    }

    Ok(())
}
```

### Efectos Secundarios

```rust
async fn after_create(&self, entity: &Order) -> Result<(), Self::Error> {
    // Actualizar inventario
    for item in &entity.items {
        sqlx::query(
            "UPDATE products SET stock = stock - $1 WHERE id = $2"
        )
        .bind(item.quantity)
        .bind(item.product_id)
        .execute(&self.pool)
        .await?;
    }

    // Enviar notificación
    self.notifications.send_order_confirmation(entity).await?;

    // Programar trabajo de fulfillment
    self.job_queue.enqueue(FulfillOrderJob { order_id: entity.id }).await?;

    Ok(())
}
```

## Con Borrado Lógico

Cuando `soft_delete` está habilitado, se generan hooks adicionales:

```rust
#[derive(Entity)]
#[entity(table = "documents", hooks, soft_delete)]
pub struct Document { /* ... */ }
```

**Hooks generados:**
```rust
#[async_trait]
pub trait DocumentHooks: Send + Sync {
    type Error: std::error::Error + Send + Sync;

    // Hooks CRUD estándar...
    async fn before_create(&self, dto: &mut CreateDocumentRequest) -> Result<(), Self::Error>;
    async fn after_create(&self, entity: &Document) -> Result<(), Self::Error>;
    async fn before_update(&self, id: &Uuid, dto: &mut UpdateDocumentRequest) -> Result<(), Self::Error>;
    async fn after_update(&self, entity: &Document) -> Result<(), Self::Error>;
    async fn before_delete(&self, id: &Uuid) -> Result<(), Self::Error>;  // Borrado lógico
    async fn after_delete(&self, id: &Uuid) -> Result<(), Self::Error>;

    // Hooks específicos de borrado lógico
    async fn before_restore(&self, id: &Uuid) -> Result<(), Self::Error>;
    async fn after_restore(&self, entity: &Document) -> Result<(), Self::Error>;
    async fn before_hard_delete(&self, id: &Uuid) -> Result<(), Self::Error>;
    async fn after_hard_delete(&self, id: &Uuid) -> Result<(), Self::Error>;
}
```

## Con Comandos

Cuando `commands` y `hooks` están habilitados, se generan hooks de comandos:

```rust
#[derive(Entity)]
#[entity(table = "orders", hooks, commands)]
#[command(Place)]
#[command(Cancel, requires_id)]
pub struct Order { /* ... */ }
```

**Hooks adicionales:**
```rust
#[async_trait]
pub trait OrderHooks: Send + Sync {
    type Error: std::error::Error + Send + Sync;

    // Hooks CRUD estándar...

    // Hooks de comandos
    async fn before_command(&self, cmd: &OrderCommand) -> Result<(), Self::Error>;
    async fn after_command(&self, cmd: &OrderCommand, result: &OrderCommandResult) -> Result<(), Self::Error>;
}
```

## Mejores Prácticas

1. **Mantén los hooks rápidos** — Las operaciones largas deben ser trabajos async
2. **Usa transacciones** — Envuelve hook + llamada a repository en una transacción
3. **Maneja errores elegantemente** — Devuelve tipos de error significativos
4. **No dupliques lógica** — Usa hooks para concerns transversales
5. **Prueba hooks independientemente** — Pruebas unitarias para implementaciones de hooks

## Patrón de Manejo de Errores

```rust
#[derive(Debug)]
pub enum HookError {
    Validation(String),
    Authorization(String),
    Conflict(String),
    Database(sqlx::Error),
}

impl std::error::Error for HookError {}
impl std::fmt::Display for HookError { /* ... */ }

impl From<sqlx::Error> for HookError {
    fn from(err: sqlx::Error) -> Self {
        HookError::Database(err)
    }
}

#[async_trait]
impl UserHooks for UserService {
    type Error = HookError;

    async fn before_create(&self, dto: &mut CreateUserRequest) -> Result<(), Self::Error> {
        if dto.email.is_empty() {
            return Err(HookError::Validation("Email requerido".into()));
        }
        Ok(())
    }
}
```

## Ver También

- [[Eventos|Eventos]] — Eventos del ciclo de vida para auditoría
- [[Comandos|Comandos]] — Patrón CQRS con hooks de comandos
- [[Mejores Prácticas|Mejores-Prácticas]] — Consejos de producción
