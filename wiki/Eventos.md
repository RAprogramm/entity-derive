Genera eventos de dominio para cambios en el ciclo de vida de entidades. Los eventos permiten registro de auditoría, event sourcing e integración con colas de mensajes.

## Inicio Rápido

```rust
#[derive(Entity)]
#[entity(table = "orders", events)]
pub struct Order {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub customer_id: Uuid,

    #[field(create, update, response)]
    pub status: String,

    #[field(create, response)]
    pub total_cents: i64,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,
}
```

## Código Generado

El atributo `events` genera un enum de eventos:

```rust
/// Generado por entity-derive
#[derive(Debug, Clone)]
pub enum OrderEvent {
    /// Entidad creada.
    Created(Order),

    /// Entidad actualizada.
    Updated {
        id: Uuid,
        changes: UpdateOrderRequest,
    },

    /// Entidad eliminada.
    Deleted(Uuid),
}
```

## Ejemplos de Uso

### Publicación Básica de Eventos

```rust
use async_trait::async_trait;

#[async_trait]
pub trait EventBus: Send + Sync {
    async fn publish<E: Send + Sync>(&self, event: E);
}

async fn create_order(
    repo: &impl OrderRepository,
    bus: &impl EventBus,
    dto: CreateOrderRequest,
) -> Result<Order, sqlx::Error> {
    let order = repo.create(dto).await?;

    // Publicar evento después de creación exitosa
    bus.publish(OrderEvent::Created(order.clone())).await;

    Ok(order)
}

async fn update_order(
    repo: &impl OrderRepository,
    bus: &impl EventBus,
    id: Uuid,
    dto: UpdateOrderRequest,
) -> Result<Order, sqlx::Error> {
    let order = repo.update(id, dto.clone()).await?;

    bus.publish(OrderEvent::Updated { id, changes: dto }).await;

    Ok(order)
}

async fn delete_order(
    repo: &impl OrderRepository,
    bus: &impl EventBus,
    id: Uuid,
) -> Result<bool, sqlx::Error> {
    let deleted = repo.delete(id).await?;

    if deleted {
        bus.publish(OrderEvent::Deleted(id)).await;
    }

    Ok(deleted)
}
```

### Registro de Auditoría

```rust
struct AuditLogger {
    pool: PgPool,
}

#[async_trait]
impl EventHandler<OrderEvent> for AuditLogger {
    async fn handle(&self, event: OrderEvent) {
        let (action, entity_id, details) = match &event {
            OrderEvent::Created(order) => (
                "created",
                order.id,
                serde_json::to_string(order).unwrap(),
            ),
            OrderEvent::Updated { id, changes } => (
                "updated",
                *id,
                serde_json::to_string(changes).unwrap(),
            ),
            OrderEvent::Deleted(id) => (
                "deleted",
                *id,
                String::new(),
            ),
        };

        sqlx::query(
            "INSERT INTO audit_log (entity_type, entity_id, action, details, created_at)
             VALUES ('order', $1, $2, $3, NOW())"
        )
        .bind(entity_id)
        .bind(action)
        .bind(details)
        .execute(&self.pool)
        .await
        .ok();
    }
}
```

### Integración con Cola de Mensajes

```rust
use rdkafka::producer::FutureProducer;

struct KafkaEventBus {
    producer: FutureProducer,
    topic: String,
}

#[async_trait]
impl EventBus for KafkaEventBus {
    async fn publish<E: Serialize + Send + Sync>(&self, event: E) {
        let payload = serde_json::to_vec(&event).unwrap();

        self.producer
            .send(
                FutureRecord::to(&self.topic)
                    .payload(&payload)
                    .key(&Uuid::new_v4().to_string()),
                Duration::from_secs(5),
            )
            .await
            .ok();
    }
}
```

### Patrón Event Sourcing

```rust
struct OrderAggregate {
    events: Vec<OrderEvent>,
    current_state: Option<Order>,
}

impl OrderAggregate {
    fn apply(&mut self, event: OrderEvent) {
        match &event {
            OrderEvent::Created(order) => {
                self.current_state = Some(order.clone());
            }
            OrderEvent::Updated { changes, .. } => {
                if let Some(ref mut order) = self.current_state {
                    if let Some(status) = &changes.status {
                        order.status = status.clone();
                    }
                }
            }
            OrderEvent::Deleted(_) => {
                self.current_state = None;
            }
        }
        self.events.push(event);
    }

    fn replay(events: Vec<OrderEvent>) -> Self {
        let mut aggregate = Self {
            events: Vec::new(),
            current_state: None,
        };
        for event in events {
            aggregate.apply(event);
        }
        aggregate
    }
}
```

## Con Borrado Lógico

Cuando `soft_delete` está habilitado, se generan eventos adicionales:

```rust
#[derive(Entity)]
#[entity(table = "documents", events, soft_delete)]
pub struct Document {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub title: String,

    #[field(skip)]
    pub deleted_at: Option<DateTime<Utc>>,
}
```

**Generado:**
```rust
pub enum DocumentEvent {
    Created(Document),
    Updated { id: Uuid, changes: UpdateDocumentRequest },
    Deleted(Uuid),       // Borrado lógico
    Restored(Uuid),      // Restaurado de borrado lógico
    HardDeleted(Uuid),   // Eliminación permanente
}
```

## Mejores Prácticas

1. **Publicar después del commit** — Solo publicar eventos después de que la transacción de BD tenga éxito
2. **Handlers idempotentes** — Los handlers de eventos deben ser idempotentes para entrega at-least-once
3. **Incluir contexto** — Considera agregar metadatos (user_id, timestamp, correlation_id)
4. **Procesamiento async** — Usa workers en segundo plano para procesamiento pesado de eventos
5. **Cola de errores** — Maneja eventos fallidos de forma elegante

## Combinando con Hooks

Eventos y hooks funcionan bien juntos:

```rust
#[derive(Entity)]
#[entity(table = "orders", events, hooks)]
pub struct Order { /* ... */ }

struct OrderService {
    repo: PgPool,
    bus: EventBus,
}

#[async_trait]
impl OrderHooks for OrderService {
    type Error = AppError;

    async fn after_create(&self, entity: &Order) -> Result<(), Self::Error> {
        // Publicar evento en hook
        self.bus.publish(OrderEvent::Created(entity.clone())).await;
        Ok(())
    }

    async fn after_update(&self, entity: &Order) -> Result<(), Self::Error> {
        // Los eventos también pueden publicarse aquí
        Ok(())
    }

    async fn after_delete(&self, id: &Uuid) -> Result<(), Self::Error> {
        self.bus.publish(OrderEvent::Deleted(*id)).await;
        Ok(())
    }
}
```

## Ver También

- [[Hooks|Ganchos]] — Ejecutar lógica personalizada en eventos del ciclo de vida
- [[Comandos|Comandos]] — Patrón CQRS con eventos de comandos
- [[Mejores Prácticas|Mejores-Prácticas]] — Consejos de producción
