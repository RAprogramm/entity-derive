# Real-Time Streams
Subscribe to entity changes in real-time using Postgres LISTEN/NOTIFY. Streams enable live dashboards, instant notifications, cache invalidation, and event-driven architectures.
## Quick Start

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

**Requirements:**
- Entity must derive `Serialize` and `Deserialize` (for JSON payloads)
- Both `events` and `streams` attributes are required
- Enable the `streams` feature in Cargo.toml

```toml
[dependencies]
entity-derive = { version = "0.3", features = ["postgres", "streams"] }
serde = { version = "1", features = ["derive"] }
```

## Generated Code

The `streams` attribute generates:

### Channel Constant

```rust
impl Order {
    /// Postgres NOTIFY channel name.
    pub const CHANNEL: &'static str = "entity_orders";
}
```

### Subscriber Struct

```rust
/// Subscriber for real-time Order changes.
pub struct OrderSubscriber {
    listener: PgListener,
}

impl OrderSubscriber {
    /// Connect and subscribe to the channel.
    pub async fn new(pool: &PgPool) -> Result<Self, sqlx::Error>;

    /// Wait for next event (blocking).
    pub async fn recv(&mut self) -> Result<OrderEvent, StreamError<sqlx::Error>>;

    /// Check for event without blocking.
    pub async fn try_recv(&mut self) -> Result<Option<OrderEvent>, StreamError<sqlx::Error>>;
}
```

### Automatic Notifications

CRUD operations automatically emit events:

```rust
// In generated create() method:
async fn create(&self, dto: CreateOrderRequest) -> Result<Order, Self::Error> {
    let order = /* insert */;

    // Auto-generated notification
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

## Usage Examples

### Basic Subscription

```rust
use entity_derive::StreamError;

async fn watch_orders(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let mut subscriber = OrderSubscriber::new(pool).await?;

    loop {
        match subscriber.recv().await {
            Ok(event) => {
                match event {
                    OrderEvent::Created(order) => {
                        println!("New order: {}", order.id);
                    }
                    OrderEvent::Updated { old, new } => {
                        println!("Order {} updated: {} -> {}", new.id, old.status, new.status);
                    }
                    OrderEvent::HardDeleted { id } => {
                        println!("Order {} deleted", id);
                    }
                    _ => {}
                }
            }
            Err(StreamError::Database(e)) => {
                eprintln!("Database error: {}", e);
                break;
            }
            Err(StreamError::Deserialize(e)) => {
                eprintln!("Invalid event payload: {}", e);
            }
        }
    }

    Ok(())
}
```

### Real-Time Dashboard (Axum WebSocket)

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

### Cache Invalidation

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

### Background Worker with Graceful Shutdown

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
                        eprintln!("Stream error: {:?}", e);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
            _ = shutdown.changed() => {
                println!("Shutting down notification worker");
                break;
            }
        }
    }
}
```

## Error Handling

```rust
use entity_derive::StreamError;

match subscriber.recv().await {
    Ok(event) => { /* process */ }
    Err(StreamError::Database(sqlx_error)) => {
        // Connection lost, query failed, etc.
        // Subscriber will auto-reconnect on next recv()
    }
    Err(StreamError::Deserialize(message)) => {
        // Invalid JSON payload
        // Log and continue - don't crash the loop
    }
}
```

## Architecture

```text
CRUD Operation (create/update/delete)
         │
         ▼
    pg_notify(channel, event_json)
         │
         ▼
    Postgres NOTIFY
         │
    ┌────┴────┐
    ▼         ▼
Subscriber  Subscriber  (multiple listeners)
    │         │
    ▼         ▼
 WebSocket   Cache
 Dashboard   Invalidator
```

## Best Practices

1. **Reconnection** — PgListener auto-reconnects; design your loop to handle temporary failures
2. **Idempotency** — Events may be delivered multiple times; handlers should be idempotent
3. **Payload size** — Keep entities small; large payloads may hit Postgres limits
4. **Separate pools** — Use dedicated connection pool for listeners to avoid blocking queries
5. **Monitoring** — Log stream errors and track event processing latency
6. **Graceful shutdown** — Use select! with shutdown signal to clean up resources

## With Soft Delete

When `soft_delete` is enabled, additional events are available:

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

// Events include:
// - DocumentEvent::SoftDeleted { id }
// - DocumentEvent::Restored { id }
// - DocumentEvent::HardDeleted { id }
```

## See Also

- [[Events]] — Event enum without real-time streaming
- [[Hooks]] — Execute custom logic on lifecycle events
- [[Best Practices]] — Production tips
