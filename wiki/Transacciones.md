Transacciones multi-entidad con tipado seguro y commit/rollback automatico.

## Que son las transacciones?

Una **transaccion de base de datos** es una forma de agrupar multiples operaciones de base de datos en una sola unidad atomica. Esto significa:

- **Todo o nada**: O TODAS las operaciones tienen exito, o NINGUNA se aplica
- **Rollback automatico**: Si alguna operacion falla, todos los cambios anteriores se deshacen automaticamente
- **Consistencia de datos**: Tu base de datos nunca termina en un estado inconsistente

### Por que necesitas transacciones?

Imagina que estas construyendo una app bancaria y necesitas transferir dinero entre cuentas:

```
1. Restar $100 de la Cuenta A
2. Sumar $100 a la Cuenta B
```

**Sin transacciones**, si el paso 1 tiene exito pero el paso 2 falla (error de red, caida de BD, etc.), acabas de perder $100! El dinero se resto de A pero nunca se agrego a B.

**Con transacciones**, si el paso 2 falla, el paso 1 se revierte automaticamente. El dinero permanece en la Cuenta A como si nada hubiera pasado.

## Habilitando Transacciones

Agrega el atributo `transactions` a tu entidad:

```rust
use entity_derive::Entity;
use uuid::Uuid;

#[derive(Entity)]
#[entity(table = "accounts", transactions)]  // <- Agrega esto
pub struct Account {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub user_id: Uuid,

    #[field(create, update, response)]
    pub balance: i64,
}
```

## Que se genera?

Para una entidad `Account` con `#[entity(transactions)]`, el macro genera:

### 1. Adaptador de Repositorio para Transacciones

```rust
pub struct AccountTransactionRepo<'t> {
    tx: &'t mut sqlx::Transaction<'static, sqlx::Postgres>,
}
```

Es como tu repositorio regular, pero todas las operaciones ocurren dentro de la transaccion.

### 2. Trait de Extension del Builder

```rust
pub trait TransactionWithAccount<'p> {
    fn with_accounts(self) -> Transaction<'p, PgPool, AccountTransactionRepo<'static>>;
}
```

Esto agrega el metodo `with_accounts()` al builder de transacciones.

## Metodos Disponibles

Dentro de una transaccion, tienes acceso a estos metodos:

| Metodo | Firma | Descripcion |
|--------|-------|-------------|
| `create` | `create(dto) -> Result<Entity, Error>` | Insertar nuevo registro |
| `find_by_id` | `find_by_id(id) -> Result<Option<Entity>, Error>` | Buscar por clave primaria |
| `update` | `update(id, dto) -> Result<Entity, Error>` | Actualizar registro existente |
| `delete` | `delete(id) -> Result<bool, Error>` | Eliminar registro (o soft-delete) |
| `list` | `list(limit, offset) -> Result<Vec<Entity>, Error>` | Lista paginada |

## Ejemplo Basico

```rust
use entity_core::prelude::*;

async fn create_account(pool: &PgPool, user_id: Uuid) -> Result<Account, AppError> {
    Transaction::new(pool)           // 1. Comenzar a construir transaccion
        .with_accounts()              // 2. Agregar repositorio Account
        .run(|mut ctx| async move {   // 3. Ejecutar operaciones
            let account = ctx.accounts().create(CreateAccountRequest {
                user_id,
                balance: 0,
            }).await?;

            Ok(account)               // 4. Retornar resultado (auto-commit)
        })
        .await
}
```

### Paso a Paso:

1. **`Transaction::new(pool)`** - Crea un nuevo builder de transaccion con tu pool de base de datos
2. **`.with_accounts()`** - Agrega el repositorio Account al contexto de transaccion
3. **`.run(|mut ctx| async move { ... })`** - Ejecuta tus operaciones dentro de la transaccion
4. **`Ok(account)`** - Retornar `Ok` hace commit de la transaccion. Retornar `Err` hace rollback.

## Ejemplo Completo: Transferencia de Dinero

Este ejemplo muestra todo el poder de las transacciones:

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

/// Transferir dinero entre dos cuentas atomicamente.
///
/// Si CUALQUIER paso falla, todos los cambios se revierten automaticamente.
pub async fn transfer(
    pool: &PgPool,
    from_id: Uuid,
    to_id: Uuid,
    amount: i64,
) -> Result<(), TransferError> {
    Transaction::new(pool)
        .with_accounts()
        .run(|mut ctx| async move {
            // Paso 1: Obtener cuenta origen
            let from = ctx.accounts()
                .find_by_id(from_id)
                .await?
                .ok_or(TransferError::AccountNotFound(from_id))?;

            // Paso 2: Verificar si tiene suficiente dinero
            if from.balance < amount {
                return Err(TransferError::InsufficientFunds {
                    available: from.balance,
                    requested: amount,
                });
            }

            // Paso 3: Obtener cuenta destino
            let to = ctx.accounts()
                .find_by_id(to_id)
                .await?
                .ok_or(TransferError::AccountNotFound(to_id))?;

            // Paso 4: Restar del origen
            // Si esto tiene exito pero el paso 5 falla, esto se REVERTIRA
            ctx.accounts().update(from_id, UpdateAccountRequest {
                balance: Some(from.balance - amount),
                user_id: None,
            }).await?;

            // Paso 5: Sumar al destino
            ctx.accounts().update(to_id, UpdateAccountRequest {
                balance: Some(to.balance + amount),
                user_id: None,
            }).await?;

            // Todas las operaciones exitosas - la transaccion hara COMMIT
            Ok(())
        })
        .await
}
```

### Que pasa en diferentes escenarios:

| Escenario | Resultado |
|-----------|-----------|
| Ambas actualizaciones exitosas | Transaccion hace commit, dinero transferido |
| Cuenta origen no encontrada | Transaccion hace rollback (sin cambios) |
| Fondos insuficientes | Transaccion hace rollback (sin cambios) |
| Primera actualizacion exitosa, segunda falla | Transaccion hace rollback (primera actualizacion deshecha!) |
| Error de red a mitad de transaccion | Transaccion hace rollback (sin cambios parciales) |

## Multiples Entidades en Una Transaccion

Puedes operar en multiples entidades atomicamente:

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
        .with_accounts()      // Agregar repo Account
        .with_transfer_logs() // Agregar repo TransferLog
        .run(|mut ctx| async move {
            // Actualizar saldos
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

            // Crear entrada de log - todo en la misma transaccion!
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

Si la creacion del log falla, ambas actualizaciones de cuentas se revierten!

## Manejo de Errores

### Rollback Automatico

Cualquier error retornado desde el closure dispara un rollback:

```rust
Transaction::new(pool)
    .with_accounts()
    .run(|mut ctx| async move {
        ctx.accounts().update(id, dto).await?;  // Exito

        // Alguna validacion falla
        if amount < 0 {
            return Err(AppError::InvalidAmount);  // <- Dispara rollback!
        }

        // Esto nunca se ejecuta, y la actualizacion de arriba se deshace
        ctx.accounts().update(other_id, other_dto).await?;

        Ok(())
    })
    .await
```

### Tipos de Error de Transaccion

El enum `TransactionError` te dice que salio mal:

```rust
use entity_core::transaction::TransactionError;

let result = Transaction::new(pool)
    .with_accounts()
    .run(|mut ctx| async move { /* ... */ })
    .await;

match result {
    Ok(value) => {
        println!("Exito: {:?}", value);
    }
    Err(e) => {
        if e.is_begin() {
            println!("Fallo al iniciar transaccion");
        } else if e.is_operation() {
            println!("Operacion fallo: {}", e);
        } else if e.is_commit() {
            println!("Fallo al hacer commit");
        } else if e.is_rollback() {
            println!("Fallo al hacer rollback");
        }

        // Obtener error interno de base de datos
        let db_error: sqlx::Error = e.into_inner();
    }
}
```

## Con Soft Delete

Las transacciones respetan el atributo `soft_delete`:

```rust
#[derive(Entity)]
#[entity(table = "documents", transactions, soft_delete)]
pub struct Document {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub title: String,

    #[field(skip)]
    pub deleted_at: Option<DateTime<Utc>>,  // Requerido para soft_delete
}

async fn archive_document(pool: &PgPool, id: Uuid) -> Result<bool, AppError> {
    Transaction::new(pool)
        .with_documents()
        .run(|mut ctx| async move {
            // Esto establece deleted_at = NOW() en lugar de DELETE
            let deleted = ctx.documents().delete(id).await?;
            Ok(deleted)
        })
        .await
}
```

## Mejores Practicas

### 1. Manten las Transacciones Cortas

Malo: Transacciones largas

```rust
Transaction::new(pool)
    .with_accounts()
    .run(|mut ctx| async move {
        let account = ctx.accounts().find_by_id(id).await?;

        // NO: Llamar APIs externas dentro de transacciones
        let rate = external_api.get_exchange_rate().await?;  // <- LENTO!

        ctx.accounts().update(id, dto).await?;
        Ok(())
    })
    .await
```

Bueno: Operaciones lentas afuera

```rust
// Obtener datos externos ANTES de iniciar transaccion
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

### 2. No Uses Transacciones para Operaciones Simples

Innecesario:
```rust
Transaction::new(pool)
    .with_users()
    .run(|mut ctx| async move {
        ctx.users().find_by_id(id).await  // Solo una operacion!
    })
    .await
```

Mejor: Usa repositorio regular
```rust
pool.find_by_id(id).await  // No se necesita transaccion
```

### 3. Maneja Todos los Errores Correctamente

Siempre propaga errores con `?`:

```rust
Transaction::new(pool)
    .with_accounts()
    .run(|mut ctx| async move {
        let result = ctx.accounts().update(id, dto).await;

        // NO: Tragarse errores
        if let Err(e) = result {
            log::error!("Update fallo: {}", e);
            // La transaccion no hara rollback correctamente!
        }

        // SI: Propagar errores
        ctx.accounts().update(id, dto).await?;  // <- Usa ?

        Ok(())
    })
    .await
```

## Patrones Comunes

### Verificar-Luego-Actualizar

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

### Crear Multiples Registros Relacionados

```rust
Transaction::new(pool)
    .with_orders()
    .with_order_items()
    .run(|mut ctx| async move {
        // Crear padre
        let order = ctx.orders().create(CreateOrderRequest {
            customer_id,
            status: "pending".to_string(),
        }).await?;

        // Crear hijos
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

## Ver Tambien

- [[Atributos|Referencia de Atributos]] - Documentacion completa de atributos
- [[Ganchos|Hooks de Ciclo de Vida]] - Ejecutar codigo antes/despues de operaciones
- [[Comandos|Comandos]] - Patron de comandos CQRS
- [[Eventos|Eventos]] - Rastrear cambios de entidades
