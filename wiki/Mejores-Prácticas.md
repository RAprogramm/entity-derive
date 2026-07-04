Guías para usar `entity-derive` efectivamente en producción.

## Diseño de Entidades

### Mantén las Entidades Enfocadas

Una entidad por tabla de base de datos. No intentes modelar relaciones complejas en una sola entidad.

```rust
// Bueno: Entidades separadas
#[derive(Entity)]
#[entity(table = "users")]
pub struct User {
    #[id]
    pub id: Uuid,
    #[field(create, update, response)]
    pub name: String,
}

#[derive(Entity)]
#[entity(table = "posts")]
pub struct Post {
    #[id]
    pub id: Uuid,
    #[field(create, response)]
    pub author_id: Uuid,  // Referencia, no embeber
    #[field(create, update, response)]
    pub title: String,
}

// Malo: Intentar embeber relaciones
pub struct User {
    pub id: Uuid,
    pub posts: Vec<Post>,  // No hagas esto
}
```

### Usa Atributos de Campo Significativos

Sé explícito sobre el propósito de cada campo:

```rust
// Bueno: Intención clara
#[field(create, response)]      // Se establece una vez, siempre visible
pub email: String,

#[field(update, response)]      // Puede cambiar, siempre visible
pub display_name: Option<String>,

#[field(response)]              // Solo lectura, calculado/gestionado externamente
pub post_count: i64,

#[field(skip)]                  // Nunca expuesto
pub password_hash: String,

// Malo: Todo en todas partes
#[field(create, update, response)]  // ¿Es realmente necesario para todo?
pub internal_id: String,
```

### Prefiere Option para Campos Nulables

Haz coincidir con tu esquema de base de datos:

```rust
// Base de datos: email VARCHAR NOT NULL
#[field(create, update, response)]
pub email: String,

// Base de datos: bio TEXT NULL
#[field(update, response)]
pub bio: Option<String>,
```

## Seguridad

### Siempre Usa `#[field(skip)]` para Datos Sensibles

```rust
// Contraseñas
#[field(skip)]
pub password_hash: String,

// Claves API
#[field(skip)]
pub api_key: String,

// Tokens internos
#[field(skip)]
pub refresh_token: Option<String>,

// PII que no debería estar en respuestas
#[field(skip)]
pub ssn: String,

// Datos de auditoría interna
#[field(skip)]
pub created_by_ip: String,
```

### Separa Entidades Internas y Externas

Para datos solo de administrador, considera entidades separadas:

```rust
// Entidad pública
#[derive(Entity)]
#[entity(table = "users")]
pub struct User {
    #[id]
    pub id: Uuid,
    #[field(create, update, response)]
    pub name: String,
    #[field(skip)]
    pub admin_notes: Option<String>,
}

// Entidad solo admin (misma tabla, vista diferente)
#[derive(Entity)]
#[entity(table = "users", sql = "trait")]
pub struct AdminUser {
    #[id]
    pub id: Uuid,
    #[field(response)]
    pub name: String,
    #[field(update, response)]  // Ahora visible y editable
    pub admin_notes: Option<String>,
    #[field(response)]
    pub last_login_ip: Option<String>,
}
```

## Rendimiento

### Usa `sql = "trait"` para Consultas Complejas

No luches contra el SQL generado. Si necesitas joins o lógica compleja, impleméntalo tú mismo:

```rust
// CRUD simple - usa generación completa
#[entity(table = "categories", sql = "full")]

// Consultas complejas necesarias - implementa tú mismo
#[entity(table = "posts", sql = "trait")]
```

### Operaciones por Lotes

Para inserciones masivas, implementa métodos personalizados:

```rust
#[entity(table = "events", sql = "trait")]
pub struct Event { /* ... */ }

pub trait EventBatchRepository {
    async fn create_batch(&self, events: Vec<CreateEventRequest>) -> Result<(), sqlx::Error>;
}

#[async_trait]
impl EventBatchRepository for PgPool {
    async fn create_batch(&self, events: Vec<CreateEventRequest>) -> Result<(), sqlx::Error> {
        let mut tx = self.begin().await?;

        for event in events {
            let entity = Event::from(event);
            let insertable = InsertableEvent::from(&entity);
            // Insertar dentro de transacción
        }

        tx.commit().await?;
        Ok(())
    }
}
```

### Evita Consultas N+1

Usa joins en lugar de cargar entidades relacionadas una por una:

```rust
// Malo: Consultas N+1
let posts = pool.list(100, 0).await?;
for post in &posts {
    let author = pool.find_user_by_id(post.author_id).await?;  // ¡N consultas!
}

// Bueno: Una sola consulta con join
let posts_with_authors = pool.list_with_authors(100, 0).await?;  // 1 consulta
```

## Testing

### Usa Base de Datos de Prueba Separada

```rust
#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    async fn setup_test_db() -> PgPool {
        let url = std::env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL must be set");

        let pool = PgPool::connect(&url).await.unwrap();

        // Ejecutar migraciones
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_create_user() {
        let pool = setup_test_db().await;

        let request = CreateUserRequest {
            username: "test_user".into(),
            email: "test@example.com".into(),
        };

        let user = pool.create(request).await.unwrap();
        assert_eq!(user.username, "test_user");
    }
}
```

### Prueba DTOs Separadamente

```rust
#[test]
fn test_user_response_excludes_password() {
    let user = User {
        id: Uuid::new_v4(),
        username: "test".into(),
        email: "test@example.com".into(),
        password_hash: "secret_hash".into(),
        created_at: Utc::now(),
    };

    let response = UserResponse::from(&user);

    // password_hash no está en UserResponse
    assert_eq!(response.username, "test");
    // No hay forma de acceder a password_hash a través de response
}

#[test]
fn test_update_request_is_partial() {
    let update = UpdateUserRequest {
        username: Some("new_name".into()),
        email: None,  // No actualizando email
    };

    assert!(update.username.is_some());
    assert!(update.email.is_none());
}
```

## Organización del Proyecto

### Estructura Recomendada

```
src/
├── entities/           # Definiciones de entidades
│   ├── mod.rs
│   ├── user.rs
│   ├── post.rs
│   └── comment.rs
├── repositories/       # Extensiones de repository personalizadas
│   ├── mod.rs
│   └── post_search.rs
├── handlers/           # Handlers HTTP
│   ├── mod.rs
│   ├── users.rs
│   └── posts.rs
├── services/           # Lógica de negocio
│   ├── mod.rs
│   └── auth.rs
└── main.rs
```

### Re-exporta Tipos Generados

```rust
// src/entities/mod.rs
mod user;
mod post;

pub use user::*;
pub use post::*;
```

### Agrupa Entidades Relacionadas

```rust
// src/entities/auth/mod.rs
mod user;
mod session;
mod api_key;

pub use user::*;
pub use session::*;
pub use api_key::*;
```

## Errores Comunes

### 1. Olvidar `#[field(skip)]` en Campos Sensibles

```rust
// Incorrecto: password_hash estará en Response!
pub struct User {
    pub password_hash: String,
}

// Correcto
#[field(skip)]
pub password_hash: String,
```

### 2. Usar `sql = "full"` Cuando Necesitas Joins

Si necesitas datos relacionados, usa `sql = "trait"` e implementa tú mismo.

### 3. No Manejar Actualizaciones Opcionales

Recuerda: los campos de `UpdateRequest` son `Option<T>`. Verifica antes de aplicar:

```rust
// UpdateUserRequest generado tiene Option<String> para name
// Tu lógica de actualización debería manejar None (sin cambio) vs Some (cambio)
```

### 4. Duplicar Lógica de Negocio

Pon validación y reglas de negocio en una capa de servicio, no en handlers:

```rust
// Bueno: Capa de servicio
impl UserService {
    pub async fn create_user(&self, request: CreateUserRequest) -> Result<User, AppError> {
        self.validate_email(&request.email)?;
        self.check_username_available(&request.username).await?;
        self.pool.create(request).await.map_err(Into::into)
    }
}

// Malo: Lógica dispersa en handlers
pub async fn create_user(pool: State<PgPool>, request: Json<CreateUserRequest>) -> ... {
    // Validación aquí
    // Reglas de negocio aquí
    // Llamada al repository aquí
    // Todo mezclado
}
```

## Lista de Verificación

Antes de desplegar:

- [ ] Todos los campos sensibles tienen `#[field(skip)]`
- [ ] Los DTOs coinciden con las expectativas del contrato API
- [ ] Las consultas complejas usan `sql = "trait"`
- [ ] Los tests de integración cubren métodos del repository
- [ ] El manejo de errores es consistente
- [ ] La paginación está implementada para endpoints de lista
- [ ] Existen índices de base de datos para patrones de consulta

## Ver También

- [[Atributos|Atributos]] — Referencia completa de atributos
- [[Ejemplos|Ejemplos]] — Ejemplos del mundo real
- [[Frameworks Web|Frameworks-Web]] — Integración con frameworks
