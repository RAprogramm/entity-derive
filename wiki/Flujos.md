Suscríbete a cambios de entidades en tiempo real usando Postgres LISTEN/NOTIFY. Los streams permiten dashboards en vivo, notificaciones instantáneas, invalidación de caché y arquitecturas orientadas a eventos.
## Inicio Rápido

```rust
#[derive(Entity, Serialize, Deserialize)]
#[entity(table = "orders", events, streams)]
pub struct Order {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub status: String,

    #[field(create, response)]
    pub customer_id: Uuid,
}
```

**Requisitos:**
- La entidad debe derivar `Serialize` y `Deserialize` (para payloads JSON)
- Se requieren ambos atributos `events` y `streams`
- Habilita la feature `streams` en Cargo.toml

```toml
[dependencies]
entity-derive = { version = "0.3", features = ["postgres", "streams"] }
serde = { version = "1", features = ["derive"] }
```

## Código Generado

El atributo `streams` genera:

### Constante de Canal

```rust
impl Order {
    /// Nombre del canal Postgres NOTIFY.
    pub const CHANNEL: &'static str = "entity_orders";
}
```

### Estructura Subscriber

```rust
/// Suscriptor para cambios de Order en tiempo real.
pub struct OrderSubscriber {
    listener: PgListener,
}

impl OrderSubscriber {
    /// Conectar y suscribirse al canal.
    pub async fn new(pool: &PgPool) -> Result<Self, sqlx::Error>;

    /// Esperar el próximo evento (bloqueante).
    pub async fn recv(&mut self) -> Result<OrderEvent, StreamError<sqlx::Error>>;

    /// Verificar evento sin bloquear.
    pub async fn try_recv(&mut self) -> Result<Option<OrderEvent>, StreamError<sqlx::Error>>;
}
```

### Notificaciones Automáticas

Las operaciones CRUD emiten eventos automáticamente:

```rust
// En el método create() generado:
async fn create(&self, dto: CreateOrderRequest) -> Result<Order, Self::Error> {
    let order = /* insert */;

    // Notificación auto-generada
    let event = OrderEvent::created(order.clone());
    let payload = serde_json::to_string(&event)?;
    sqlx::query("SELECT pg_notify($1, $2)")
        .bind(Order::CHANNEL)
        .bind(&payload)
        .execute(self)
        .await?;

    Ok(order)
}
```

## Ejemplos de Uso

### Suscripción Básica

```rust
use entity_derive::StreamError;

async fn watch_orders(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let mut subscriber = OrderSubscriber::new(pool).await?;

    loop {
        match subscriber.recv().await {
            Ok(event) => {
                match event {
                    OrderEvent::Created(order) => {
                        println!("Nueva orden: {}", order.id);
                    }
                    OrderEvent::Updated { old, new } => {
                        println!("Orden {} actualizada: {} -> {}", new.id, old.status, new.status);
                    }
                    OrderEvent::HardDeleted { id } => {
                        println!("Orden {} eliminada", id);
                    }
                    _ => {}
                }
            }
            Err(StreamError::Database(e)) => {
                eprintln!("Error de base de datos: {}", e);
                break;
            }
            Err(StreamError::Deserialize(e)) => {
                eprintln!("Payload de evento inválido: {}", e);
            }
        }
    }

    Ok(())
}
```

### Dashboard en Tiempo Real (Axum WebSocket)

```rust
use axum::{
    extract::{State, WebSocketUpgrade, ws::{Message, WebSocket}},
    response::IntoResponse,
};

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(pool): State<PgPool>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, pool))
}

async fn handle_socket(mut socket: WebSocket, pool: PgPool) {
    let mut subscriber = match OrderSubscriber::new(&pool).await {
        Ok(s) => s,
        Err(_) => return,
    };

    loop {
        match subscriber.recv().await {
            Ok(event) => {
                let json = serde_json::to_string(&event).unwrap();
                if socket.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}
```

### Invalidación de Caché

```rust
struct CacheInvalidator {
    cache: Redis,
    pool: PgPool,
}

impl CacheInvalidator {
    async fn run(&self) -> Result<(), StreamError<sqlx::Error>> {
        let mut subscriber = OrderSubscriber::new(&self.pool).await
            .map_err(StreamError::Database)?;

        loop {
            let event = subscriber.recv().await?;
            let key = format!("order:{}", event.entity_id());

            match event {
                OrderEvent::Created(_) | OrderEvent::Updated { .. } => {
                    self.cache.del(&key).await.ok();
                }
                OrderEvent::HardDeleted { id } | OrderEvent::SoftDeleted { id } => {
                    self.cache.del(&format!("order:{}", id)).await.ok();
                }
                _ => {}
            }
        }
    }
}
```

### Worker en Segundo Plano con Graceful Shutdown

```rust
use tokio::sync::watch;

async fn notification_worker(
    pool: PgPool,
    mut shutdown: watch::Receiver<bool>,
) {
    let mut subscriber = OrderSubscriber::new(&pool).await.unwrap();

    loop {
        tokio::select! {
            result = subscriber.recv() => {
                match result {
                    Ok(event) => process_event(event).await,
                    Err(e) => {
                        eprintln!("Error de stream: {:?}", e);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
            _ = shutdown.changed() => {
                println!("Apagando worker de notificaciones");
                break;
            }
        }
    }
}
```

## Manejo de Errores

```rust
use entity_derive::StreamError;

match subscriber.recv().await {
    Ok(event) => { /* procesar */ }
    Err(StreamError::Database(sqlx_error)) => {
        // Conexión perdida, query fallido, etc.
        // El subscriber se reconectará automáticamente en el próximo recv()
    }
    Err(StreamError::Deserialize(message)) => {
        // Payload JSON inválido
        // Loguear y continuar - no romper el loop
    }
}
```

## Arquitectura

```text
Operación CRUD (create/update/delete)
         │
         ▼
    pg_notify(channel, event_json)
         │
         ▼
    Postgres NOTIFY
         │
    ┌────┴────┐
    ▼         ▼
Subscriber  Subscriber  (múltiples listeners)
    │         │
    ▼         ▼
 WebSocket   Invalidador
 Dashboard   de Caché
```

## Mejores Prácticas

1. **Reconexión** — PgListener se reconecta automáticamente; diseña tu loop para manejar fallos temporales
2. **Idempotencia** — Los eventos pueden entregarse múltiples veces; los handlers deben ser idempotentes
3. **Tamaño del payload** — Mantén las entidades pequeñas; payloads grandes pueden exceder límites de Postgres
4. **Pools separados** — Usa un pool de conexiones dedicado para listeners
5. **Monitoreo** — Loguea errores de stream y rastrea la latencia de procesamiento de eventos
6. **Graceful shutdown** — Usa select! con señal de shutdown para limpiar recursos

## Con Soft Delete

Cuando `soft_delete` está habilitado, hay eventos adicionales disponibles:

```rust
#[derive(Entity, Serialize, Deserialize)]
#[entity(table = "documents", events, streams, soft_delete)]
pub struct Document {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub title: String,

    #[field(skip)]
    pub deleted_at: Option<DateTime<Utc>>,
}

// Los eventos incluyen:
// - DocumentEvent::SoftDeleted { id }
// - DocumentEvent::Restored { id }
// - DocumentEvent::HardDeleted { id }
```

## Ver También

- [[Eventos|Eventos]] — Enum de eventos sin streaming en tiempo real
- [[Hooks]] — Ejecutar lógica personalizada en eventos del ciclo de vida
- [[Mejores Prácticas|Mejores-Prácticas]] — Tips para producción
