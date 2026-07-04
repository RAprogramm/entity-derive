Ejemplos prácticos para casos de uso comunes.

## Gestión de Usuarios

Entidad de usuario clásica con campos de autenticación:

```rust
use entity_derive::Entity;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Entity)]
#[entity(table = "users", schema = "auth")]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub username: String,

    #[field(create, update, response)]
    pub email: String,

    #[field(create, update)]  // Acepta pero nunca retorna
    pub password_hash: String,

    #[field(response)]
    pub email_verified: bool,

    #[field(update, response)]
    pub avatar_url: Option<String>,

    #[field(response)]
    pub role: String,

    #[field(skip)]  // Campo de auditoría interno
    pub last_login_ip: Option<String>,

    #[auto]
    #[field(response)]
    pub created_at: DateTime<Utc>,

    #[auto]
    #[field(response)]
    pub updated_at: DateTime<Utc>,
}
```

**Uso:**

```rust
// Crear usuario
let request = CreateUserRequest {
    username: "john_doe".into(),
    email: "john@example.com".into(),
    password_hash: hash_password("secret123"),
};
let user = pool.create(request).await?;

// Actualizar usuario
let update = UpdateUserRequest {
    avatar_url: Some(Some("https://cdn.example.com/avatar.jpg".into())),
    ..Default::default()
};
let user = pool.update(user.id, update).await?;

// La respuesta es segura - sin password_hash, sin last_login_ip
let response = UserResponse::from(&user);
```

## Sistema de Blog

Posts con relación de autor:

```rust
#[derive(Entity)]
#[entity(table = "posts", schema = "blog")]
pub struct Post {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub title: String,

    #[field(create, update, response)]
    pub slug: String,

    #[field(create, update, response)]
    pub content: String,

    #[field(create, update, response)]
    pub excerpt: Option<String>,

    #[field(create, response)]  // Se establece una vez, no se puede cambiar autor
    pub author_id: Uuid,

    #[field(update, response)]
    pub published: bool,

    #[field(update, response)]
    pub published_at: Option<DateTime<Utc>>,

    #[field(response)]  // Solo lectura, gestionado por triggers
    pub view_count: i64,

    #[field(skip)]  // Moderación interna
    pub moderation_status: String,

    #[auto]
    #[field(response)]
    pub created_at: DateTime<Utc>,

    #[auto]
    #[field(response)]
    pub updated_at: DateTime<Utc>,
}
```

**Categorías con muchos-a-muchos:**

```rust
#[derive(Entity)]
#[entity(table = "categories", schema = "blog")]
pub struct Category {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,

    #[field(create, update, response)]
    pub slug: String,

    #[field(create, update, response)]
    pub description: Option<String>,

    #[field(response)]
    pub post_count: i64,  // Campo calculado

    #[auto]
    #[field(response)]
    pub created_at: DateTime<Utc>,
}
```

## E-Commerce

Catálogo de productos:

```rust
#[derive(Entity)]
#[entity(table = "products", schema = "catalog")]
pub struct Product {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,

    #[field(create, update, response)]
    pub sku: String,

    #[field(create, update, response)]
    pub description: Option<String>,

    #[field(create, update, response)]
    pub price_cents: i64,

    #[field(create, update, response)]
    pub currency: String,

    #[field(update, response)]
    pub stock_quantity: i32,

    #[field(update, response)]
    pub is_active: bool,

    #[field(create, response)]
    pub category_id: Uuid,

    #[field(skip)]  // Seguimiento interno de costos
    pub cost_cents: i64,

    #[field(skip)]  // Info del proveedor
    pub supplier_id: Option<Uuid>,

    #[auto]
    #[field(response)]
    pub created_at: DateTime<Utc>,

    #[auto]
    #[field(response)]
    pub updated_at: DateTime<Utc>,
}
```

Pedidos con seguimiento de estado:

```rust
#[derive(Entity)]
#[entity(table = "orders", schema = "sales")]
pub struct Order {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub customer_id: Uuid,

    #[field(response)]
    pub order_number: String,  // Generado por secuencia de BD

    #[field(update, response)]
    pub status: String,

    #[field(create, response)]
    pub total_cents: i64,

    #[field(create, response)]
    pub currency: String,

    #[field(create, update, response)]
    pub shipping_address: String,

    #[field(update, response)]
    pub tracking_number: Option<String>,

    #[field(skip)]  // Datos del procesador de pago
    pub payment_intent_id: Option<String>,

    #[field(skip)]  // Notas internas
    pub admin_notes: Option<String>,

    #[auto]
    #[field(response)]
    pub created_at: DateTime<Utc>,

    #[auto]
    #[field(response)]
    pub updated_at: DateTime<Utc>,
}
```

## SaaS Multi-Tenant

Entidades con ámbito de organización:

```rust
#[derive(Entity)]
#[entity(table = "organizations", schema = "tenants")]
pub struct Organization {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,

    #[field(create, response)]
    pub slug: String,  // Inmutable después de creación

    #[field(update, response)]
    pub plan: String,

    #[field(response)]
    pub member_count: i32,

    #[field(skip)]  // Info de facturación
    pub stripe_customer_id: Option<String>,

    #[auto]
    #[field(response)]
    pub created_at: DateTime<Utc>,
}

#[derive(Entity)]
#[entity(table = "projects", schema = "tenants")]
pub struct Project {
    #[id]
    pub id: Uuid,

    #[field(create, response)]  // Se establece una vez
    pub organization_id: Uuid,

    #[field(create, update, response)]
    pub name: String,

    #[field(create, update, response)]
    pub description: Option<String>,

    #[field(update, response)]
    pub archived: bool,

    #[auto]
    #[field(response)]
    pub created_at: DateTime<Utc>,

    #[auto]
    #[field(response)]
    pub updated_at: DateTime<Utc>,
}
```

## Solo DTOs (Sin Base de Datos)

Para contratos de API sin persistencia:

```rust
#[derive(Entity)]
#[entity(table = "webhooks", sql = "none")]
pub struct WebhookPayload {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub event_type: String,

    #[field(create, response)]
    pub payload: String,

    #[field(create, response)]
    pub timestamp: DateTime<Utc>,

    #[field(response)]
    pub signature: String,
}
```

Esto genera solo:
- `CreateWebhookPayloadRequest`
- `WebhookPayloadResponse`
- Implementaciones `From`

Sin repositorio, sin SQL, sin estructuras Row/Insertable.

## Solo Consultas Personalizadas

Cuando el CRUD estándar no es suficiente:

```rust
#[derive(Entity)]
#[entity(table = "analytics_events", schema = "analytics", sql = "trait")]
pub struct AnalyticsEvent {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub event_name: String,

    #[field(create, response)]
    pub user_id: Option<Uuid>,

    #[field(create, response)]
    pub properties: serde_json::Value,

    #[auto]
    #[field(response)]
    pub created_at: DateTime<Utc>,
}

// Implementa consultas personalizadas tú mismo:
impl AnalyticsEventRepository for PgPool {
    type Error = sqlx::Error;

    async fn create(&self, dto: CreateAnalyticsEventRequest) -> Result<AnalyticsEvent, Self::Error> {
        // Inserción por lotes, tablas particionadas, etc.
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<AnalyticsEvent>, Self::Error> {
        // Consulta con particionamiento basado en tiempo
    }

    // Métodos personalizados más allá del CRUD:
    async fn aggregate_by_event(&self, start: DateTime<Utc>, end: DateTime<Utc>)
        -> Result<Vec<EventAggregate>, Self::Error> {
        // Consulta de agregación compleja
    }
}
```

## Ver También

- [[Atributos|Atributos]] — Referencia completa de atributos
- [[SQL Personalizado|SQL-Personalizado]] — Implementación de repositorio personalizado
- [[Frameworks Web|Frameworks-Web]] — Integración con Axum/Actix

## Operaciones masivas

Cada repositorio incluye primitivas por lotes: `find_by_ids` (`WHERE id = ANY($1)`, un solo viaje), `create_many` atómico (una transacción; una fila fallida revierte todo el lote) y `delete_many` consciente del soft delete que devuelve el número de filas realmente afectadas. Con entrega de eventos (streams u outbox) se emiten eventos por fila dentro de la misma transacción.

```rust
let posts: Vec<Post> = pool.find_by_ids(ids).await?;
let created: Vec<Post> = pool.create_many(dtos).await?;
let removed: u64 = pool.delete_many(stale_ids).await?;
```
