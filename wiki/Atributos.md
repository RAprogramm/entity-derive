Guía completa de todos los atributos soportados por `entity-derive`.

## Atributos a Nivel de Entidad

Se aplican a la estructura con `#[entity(...)]`:

```rust
#[derive(Entity)]
#[entity(
    table = "users",
    schema = "core",
    sql = "full",
    dialect = "postgres",
    uuid = "v7",
    soft_delete,
    returning = "full",
    error = "AppError",
    events,
    hooks,
    commands
)]
pub struct User { /* ... */ }
```

### Referencia Rápida

| Atributo | Requerido | Por Defecto | Descripción |
|----------|-----------|-------------|-------------|
| `table` | Sí | — | Nombre de la tabla en BD |
| `schema` | No | `"public"` | Esquema de BD |
| `sql` | No | `"full"` | Nivel de generación SQL |
| `dialect` | No | `"postgres"` | Dialecto de BD |
| `uuid` | No | `"v7"` | Versión UUID para generación de ID |
| `soft_delete` | No | `false` | Habilitar borrado lógico |
| `returning` | No | `"full"` | Modo de cláusula RETURNING |
| `upsert(...)` | No | — | Genera un método upsert con `INSERT ... ON CONFLICT` |
| `api(guard = "...")` | No | — | Guard `FromRequestParts` aplicado en los handlers generados |
| `error` | No | `sqlx::Error` | Tipo de error personalizado |
| `events` | No | `false` | Generar eventos de ciclo de vida |
| `hooks` | No | `false` | Generar trait de hooks |
| `commands` | No | `false` | Habilitar patrón CQRS |

### `table` (requerido)

Nombre de la tabla en base de datos.

```rust
#[entity(table = "users")]           // → FROM users
#[entity(table = "user_profiles")]   // → FROM user_profiles
```

### `schema` (opcional)

Esquema de base de datos. Por defecto: `"public"`.

```rust
#[entity(table = "users")]                    // → FROM public.users
#[entity(table = "users", schema = "core")]   // → FROM core.users
#[entity(table = "users", schema = "auth")]   // → FROM auth.users
```

### `sql` (opcional)

Nivel de generación SQL. Por defecto: `"full"`.

| Valor | Trait Repository | Impl PgPool | Caso de Uso |
|-------|-----------------|-------------|-------------|
| `"full"` | Sí | Sí | Entidades CRUD estándar |
| `"trait"` | Sí | No | Consultas personalizadas (joins, CTEs) |
| `"none"` | No | No | Solo DTOs, sin base de datos |

```rust
#[entity(table = "users", sql = "full")]   // Automatización completa (defecto)
#[entity(table = "users", sql = "trait")]  // Solo trait, implementar SQL manualmente
#[entity(table = "users", sql = "none")]   // Sin capa de base de datos
```

### `dialect` (opcional)

Dialecto de BD para generación SQL. Por defecto: `"postgres"`.

| Dialecto | Alias | Tipo Cliente | Estado |
|----------|-------|--------------|--------|
| `"postgres"` | `"pg"`, `"postgresql"` | `sqlx::PgPool` | Estable |
| `"clickhouse"` | `"ch"` | `clickhouse::Client` | No implementado — error de compilación |
| `"mongodb"` | `"mongo"` | `mongodb::Client` | No implementado — error de compilación |

### `uuid` (opcional)

Versión UUID para claves primarias auto-generadas. Por defecto: `"v7"`.

| Versión | Método | Propiedades |
|---------|--------|-------------|
| `"v7"` | `Uuid::now_v7()` | Ordenado por tiempo, ordenable (recomendado) |
| `"v4"` | `Uuid::new_v4()` | Aleatorio, ampliamente compatible |

```rust
#[entity(table = "users", uuid = "v7")]     // Ordenado por tiempo (defecto)
#[entity(table = "sessions", uuid = "v4")]  // UUID aleatorio
```

**¿Por qué UUID v7?**
- Ordenado por tiempo: ordenamiento natural por fecha de creación
- Mejor rendimiento de índices en BD
- No requiere coordinación (a diferencia de secuencias)
- Globalmente único en sistemas distribuidos

### `soft_delete` (opcional)

Habilita borrado lógico para marcar registros como eliminados en lugar de borrarlos.

```rust
#[derive(Entity)]
#[entity(table = "documents", soft_delete)]
pub struct Document {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    pub title: String,

    #[field(skip)]
    pub deleted_at: Option<DateTime<Utc>>,  // Campo requerido
}
```

**Métodos generados:**
- `delete()` — Establece `deleted_at = NOW()` en lugar de DELETE
- `hard_delete()` — Elimina permanentemente el registro
- `restore()` — Establece `deleted_at = NULL`
- `find_by_id()` / `list()` — Filtran automáticamente registros eliminados
- `find_by_id_with_deleted()` / `list_with_deleted()` — Incluyen registros eliminados

### `returning` (opcional)

Controla qué datos se obtienen después de INSERT/UPDATE. Por defecto: `"full"`.

| Modo | Cláusula SQL | Caso de Uso |
|------|--------------|-------------|
| `"full"` | `RETURNING *` | Obtener todos los campos incluyendo los generados por BD |
| `"id"` | `RETURNING id` | Confirmar inserción, devolver entidad pre-construida |
| `"none"` | (sin RETURNING) | Fire-and-forget, opción más rápida |
| `"col1, col2"` | `RETURNING col1, col2` | Devolver columnas específicas |

```rust
#[entity(table = "logs", returning = "none")]              // Más rápido
#[entity(table = "users", returning = "full")]             // Obtener valores generados
#[entity(table = "events", returning = "id, created_at")]  // Columnas personalizadas
```

### `events(outbox)` (opcional)

Entrega duradera de eventos mediante un outbox transaccional. `events` solo genera el enum; con `streams`, NOTIFY es fire-and-forget. `events(outbox)` hace que cada escritura generada inserte el evento serializado en la tabla `entity_outbox` dentro de la misma transacción, y el runtime `OutboxDrainer` (entity-core, feature `outbox`) entrega las filas con `FOR UPDATE SKIP LOCKED`, backoff exponencial y aparcamiento tras `max_attempts`. At-least-once — los handlers deben ser idempotentes. Se combina con `streams`.

```rust
#[derive(Entity, Serialize, Deserialize)]
#[entity(table = "orders", events(outbox), migrations)]
pub struct Order { /* ... */ }

sqlx::query(Order::MIGRATION_OUTBOX).execute(&pool).await?;

struct Notifier;

#[async_trait::async_trait]
impl entity_core::outbox::OutboxHandler for Notifier {
    type Error = anyhow::Error;
    async fn handle(&self, row: &OutboxRow) -> Result<(), Self::Error> {
        deliver(&row.entity, &row.payload).await
    }
}

entity_core::outbox::OutboxDrainer::new(pool, Notifier).run().await;
```

### `upsert(...)` (opcional)

Genera un método `upsert` del repositorio basado en `INSERT ... ON CONFLICT`.

```rust
#[derive(Entity)]
#[entity(table = "users", upsert(conflict = "email"))]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    #[column(unique)]
    pub email: String,

    #[field(create, update, response)]
    pub name: String,
}
```

| Opción | Requerida | Por defecto | Descripción |
|--------|-----------|-------------|-------------|
| `conflict` | Sí | — | Columnas de conflicto separadas por comas |
| `action` | No | `"update"` | `"update"` (DO UPDATE) o `"nothing"` (DO NOTHING) |

**Generado:**
- `action = "update"` → `async fn upsert(&self, dto: CreateUserRequest) -> Result<User, Error>` — sobrescribe todas las columnas no conflictivas (`DO UPDATE SET col = EXCLUDED.col`) y devuelve la fila persistida
- `action = "nothing"` → `async fn upsert(&self, dto: CreateUserRequest) -> Result<Option<User>, Error>` — mantiene la fila existente; `None` significa que ya existía una fila en conflicto

**Validación en tiempo de compilación:**
- las columnas de conflicto deben existir y tener garantía de unicidad (`#[id]`, `#[column(unique)]` o un `unique_index(...)` coincidente)
- requiere `returning = "full"` (el valor por defecto)
- `action = "update"` necesita al menos una columna actualizable fuera del conflicto

Con `streams` habilitado, upsert publica una notificación `Created` por cada fila devuelta.

### `api(guard = "...")` (opcional)

Aplica autenticación en los handlers generados. `security = "..."` solo documenta la autenticación en OpenAPI; `guard` inyecta un extractor axum (`FromRequestParts`) como primer argumento de cada handler CRUD y de comandos generado — una extracción fallida rechaza la petición antes de ejecutar el cuerpo del handler.

```rust
pub struct RequireAuth;

impl<S: Send + Sync> FromRequestParts<S> for RequireAuth {
    type Rejection = StatusCode;
    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        parts.headers.contains_key("authorization")
            .then_some(Self)
            .ok_or(StatusCode::UNAUTHORIZED)
    }
}

#[derive(Entity)]
#[entity(table = "users", api(tag = "Users", handlers, guard = "RequireAuth", guard(list = "none")))]
pub struct User { /* ... */ }
```

Overrides por operación: `guard(create = "Admin", list = "none", ...)` con operaciones `create`, `get`, `update`, `delete`, `list`, `commands`; el literal `"none"` desactiva el guard. Los comandos en `public = [...]` nunca reciben guard.

### `error` (opcional)

Tipo de error personalizado para repositorio. Por defecto: `sqlx::Error`.

```rust
#[derive(Debug)]
pub enum AppError {
    Database(sqlx::Error),
    NotFound,
    Validation(String),
}

impl std::error::Error for AppError {}
impl std::fmt::Display for AppError { /* ... */ }

// Requerido: convertir desde sqlx::Error
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err)
    }
}

#[derive(Entity)]
#[entity(table = "users", error = "AppError")]
pub struct User { /* ... */ }

// El repositorio generado usa AppError:
// impl UserRepository for PgPool {
//     type Error = AppError;
//     ...
// }
```

### `events` (opcional)

Genera enum de eventos del ciclo de vida. Ver [[Eventos|Eventos]] para detalles.

```rust
#[entity(table = "orders", events)]
```

**Generado:**
```rust
pub enum OrderEvent {
    Created(Order),
    Updated { id: Uuid, changes: UpdateOrderRequest },
    Deleted(Uuid),
}
```

### `hooks` (opcional)

Genera trait de hooks del ciclo de vida. Ver [[Ganchos|Hooks]] para detalles.

```rust
#[entity(table = "users", hooks)]
```

**Generado:**
```rust
#[async_trait]
pub trait UserHooks: Send + Sync {
    type Error: std::error::Error + Send + Sync;

    async fn before_create(&self, dto: &mut CreateUserRequest) -> Result<(), Self::Error>;
    async fn after_create(&self, entity: &User) -> Result<(), Self::Error>;
    async fn before_update(&self, id: &Uuid, dto: &mut UpdateUserRequest) -> Result<(), Self::Error>;
    async fn after_update(&self, entity: &User) -> Result<(), Self::Error>;
    async fn before_delete(&self, id: &Uuid) -> Result<(), Self::Error>;
    async fn after_delete(&self, id: &Uuid) -> Result<(), Self::Error>;
}
```

### `commands` (opcional)

Habilita patrón de comandos CQRS. Ver [[Comandos|Comandos]] para detalles.

```rust
#[entity(table = "users", commands)]
#[command(Register)]
#[command(Deactivate, requires_id)]
```

## Atributos a Nivel de Campo

Se aplican a campos individuales.

### `#[id]`

Marca el campo de clave primaria.

**Comportamiento:**
- Auto-genera UUID (v7 por defecto, configurable con atributo `uuid`)
- Siempre incluido en DTO `Response`
- Excluido de `CreateRequest` y `UpdateRequest`

```rust
#[id]
pub id: Uuid,
```

### `#[auto]`

Marca campos auto-generados (timestamps, secuencias).

**Comportamiento:**
- Obtiene `Default::default()` en `From<CreateRequest>`
- Excluido de `CreateRequest` y `UpdateRequest`
- Puede incluirse en `Response` con `#[field(response)]`
- Excluido del `INSERT` generado, por lo que con `migrations` una
  columna temporal no nulable recibe el valor por defecto correspondiente
  en la base de datos: `NOW()` para `TIMESTAMPTZ` y `TIMESTAMP`,
  `CURRENT_DATE` para `DATE`, `CURRENT_TIME` para `TIME`. Un
  `#[column(default = "...")]` explícito tiene prioridad

```rust
#[auto]
#[field(response)]
pub created_at: DateTime<Utc>,
```

### `#[field(...)]`

Controla inclusión en DTOs. Combina múltiples opciones:

```rust
#[field(create)]                    // Solo en CreateRequest
#[field(update)]                    // Solo en UpdateRequest
#[field(response)]                  // Solo en Response
#[field(create, response)]          // En Create y Response
#[field(create, update, response)]  // En los tres
#[field(skip)]                      // Excluido de todos los DTOs
```

#### `create`

Incluye campo en DTO `CreateRequest`.

```rust
#[field(create)]
pub email: String,

// Generado:
pub struct CreateUserRequest {
    pub email: String,
}
```

#### `update`

Incluye campo en DTO `UpdateRequest`.

**Importante:** Los campos no opcionales se envuelven automáticamente en `Option<T>` para actualizaciones parciales.

```rust
#[field(update)]
pub name: String,  // No Option

// Generado:
pub struct UpdateUserRequest {
    pub name: Option<String>,  // Envuelto automáticamente
}
```

#### `response`

Incluye campo en DTO `Response`.

```rust
#[field(response)]
pub email: String,

// Generado:
pub struct UserResponse {
    pub id: Uuid,        // Siempre incluido (tiene #[id])
    pub email: String,   // Incluido
}
```

#### `skip`

Excluye campo de **todos** los DTOs. Usar para datos sensibles.

```rust
#[field(skip)]
pub password_hash: String,
```

**Importante:** `skip` anula todas las demás opciones de campo. El campo solo existirá en:
- Estructura de entidad original
- Estructura `Row` (para lecturas de BD)
- Estructura `Insertable` (para escrituras en BD)

### `#[column(pg_enum = "...")]`

Conecta un enum Postgres (`ValueObject`) a la generación de DDL.

```rust
#[derive(ValueObject, Debug, Clone, Serialize, Deserialize)]
#[value_object(pg_type = "order_status", sqlx)]
pub enum OrderStatus { Pending, Shipped, Delivered }

#[derive(Entity)]
#[entity(table = "orders", migrations)]
pub struct Order {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    #[column(pg_enum = "order_status")]
    pub status: OrderStatus,
}

for ddl in Order::MIGRATION_TYPES {
    sqlx::query(ddl).execute(&pool).await?;
}
sqlx::query(Order::MIGRATION_UP).execute(&pool).await?;
```

- Define el tipo de columna en el DDL (de lo contrario los campos enum caen a TEXT)
- Registra el DDL idempotente `PG_CREATE_TYPE` del enum en `{Entity}::MIGRATION_TYPES` — ejecútalos antes de `MIGRATION_UP`
- El nombre declarado se verifica contra la constante `PG_TYPE` del enum en tiempo de compilación; una discrepancia rompe la compilación
- El flag opcional `sqlx` de `ValueObject` genera impls `sqlx::Type` / `Encode` / `Decode`; omítelo si ya derivas `sqlx::Type`

### `#[owner]`

Alcance por propietario a nivel de fila. Marca la columna con el id del propietario; el repositorio obtiene métodos con alcance que nunca revelan si una fila existe para otro propietario y respetan `soft_delete`.

```rust
#[derive(Entity)]
#[entity(table = "orders")]
pub struct Order {
    #[id]
    pub id: Uuid,

    #[owner]
    pub user_id: Uuid,

    #[field(create, update, response)]
    pub note: String,
}

let mine = pool.list_by_owner(user_id, 20, 0).await?;
let order = pool.find_by_id_scoped(id, user_id).await?;
let updated = pool.update_scoped(id, user_id, patch).await?;
let removed = pool.delete_scoped(id, user_id).await?;
```

Generado: `find_by_id_scoped`, `list_by_owner`, `update_scoped` (cuando hay campos update; `None` si la fila no es suya), `delete_scoped`. Como máximo un campo `#[owner]`; combinarlo con `#[id]` se rechaza en compilación.

### `#[filter]` / `#[filter(...)]`

Genera campos de filtro de consulta. Ver [[Filtrado|Filtrado]] para detalles.

```rust
#[filter]              // Coincidencia exacta: WHERE field = $n
#[filter(eq)]          // Igual que arriba
#[filter(like)]        // Patrón: WHERE field ILIKE $n
#[filter(range)]       // Rango: WHERE field >= $n AND field <= $m
```

### `#[belongs_to(Entity)]`

Relación de clave foránea. Ver [[Relaciones|Relaciones]] para detalles.

```rust
#[belongs_to(User)]
pub user_id: Uuid,
```

**Generado:** Método `find_user()` en repositorio.

### `#[has_many(Entity)]`

Relación uno-a-muchos (nivel de entidad). Ver [[Relaciones|Relaciones]] para detalles.

```rust
#[has_many(Post)]
pub struct User { /* ... */ }
```

**Generado:** Método `find_posts()` en repositorio.

### `#[projection(Name: fields)]`

Genera estructura de vista parcial (nivel de entidad).

```rust
#[projection(Public: id, name, avatar)]
#[projection(Admin: id, name, email, role)]
pub struct User { /* ... */ }
```

**Generado:**
- `UserPublic { id, name, avatar }`
- `UserAdmin { id, name, email, role }`
- Implementaciones `From<User>`
- Métodos `find_by_id_public()`, `find_by_id_admin()`

## Atributos de Comando

Se aplican a nivel de entidad con `#[command(...)]`.

### Referencia Rápida

| Sintaxis | Efecto |
|----------|--------|
| `#[command(Name)]` | Usa todos los campos `#[field(create)]` |
| `#[command(Name: field1, field2)]` | Usa solo campos especificados (añade `requires_id`) |
| `#[command(Name, requires_id)]` | Añade campo ID, sin otros campos |
| `#[command(Name, source = "create")]` | Usa explícitamente campos create (defecto) |
| `#[command(Name, source = "update")]` | Usa campos update (opcionales, añade `requires_id`) |
| `#[command(Name, source = "none")]` | Sin campos de payload |
| `#[command(Name, payload = "Type")]` | Usa estructura payload personalizada |
| `#[command(Name, result = "Type")]` | Usa tipo de resultado personalizado |
| `#[command(Name, kind = "create")]` | Sugerencia: crea entidad (defecto) |
| `#[command(Name, kind = "update")]` | Sugerencia: modifica entidad |
| `#[command(Name, kind = "delete")]` | Sugerencia: elimina entidad (devuelve `()`) |
| `#[command(Name, kind = "custom")]` | Sugerencia: operación personalizada |

Ver [[Comandos|Comandos]] para documentación detallada.

## Ejemplo Completo

```rust
#[derive(Entity)]
#[entity(
    table = "posts",
    schema = "blog",
    sql = "full",
    dialect = "postgres",
    uuid = "v7",
    soft_delete,
    returning = "full",
    events,
    hooks,
    commands
)]
#[has_many(Comment)]
#[projection(Summary: id, title, author_id, created_at)]
#[command(Publish)]
#[command(Archive, requires_id)]
pub struct Post {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    #[filter(like)]
    pub title: String,

    #[field(create, update, response)]
    pub content: String,

    #[field(create, response)]
    #[belongs_to(User)]
    #[filter]
    pub author_id: Uuid,

    #[field(update, response)]
    pub published: bool,

    #[field(response)]
    #[filter(range)]
    pub view_count: i64,

    #[field(skip)]
    pub moderation_notes: String,

    #[field(skip)]
    pub deleted_at: Option<DateTime<Utc>>,

    #[auto]
    #[field(response)]
    #[filter(range)]
    pub created_at: DateTime<Utc>,

    #[auto]
    #[field(response)]
    pub updated_at: DateTime<Utc>,
}
```

## Matriz de Decisiones

| Quiero... | Atributos |
|-----------|-----------|
| Auto-generar clave primaria | `#[id]` |
| Usar UUID aleatorio | `uuid = "v4"` en entidad |
| Usar UUID ordenado por tiempo | `uuid = "v7"` (defecto) |
| Aceptar en cuerpo POST | `#[field(create)]` |
| Aceptar en cuerpo PATCH | `#[field(update)]` |
| Devolver en respuesta API | `#[field(response)]` |
| Aceptar y devolver | `#[field(create, update, response)]` |
| Ocultar de todas las APIs | `#[field(skip)]` |
| Auto-generar timestamp | `#[auto]` + `#[field(response)]` |
| Solo lectura (gestionado por BD) | Solo `#[field(response)]` |
| Solo escritura (sin retorno) | Solo `#[field(create)]` |
| Consultas SQL personalizadas | `sql = "trait"` |
| Solo DTOs, sin BD | `sql = "none"` |
| Borrado lógico de registros | `soft_delete` en entidad |
| Tipo de error personalizado | `error = "MyError"` en entidad |
| Filtrar por valor exacto | `#[filter]` en campo |
| Filtrar por patrón | `#[filter(like)]` en campo |
| Filtrar por rango | `#[filter(range)]` en campo |
| Rastrear cambios de entidad | `events` en entidad |
| Ejecutar código en ciclo de vida | `hooks` en entidad |
| Usar comandos de dominio | `commands` en entidad + `#[command(...)]` |
| Definir relación | `#[belongs_to(Entity)]` o `#[has_many(Entity)]` |
| Vista parcial de entidad | `#[projection(Name: fields)]` |

### DTO de actualización: semántica PATCH

Las actualizaciones generadas son parches parciales reales: la cláusula SET se construye en tiempo de ejecución con los campos realmente presentes; los omitidos no se tocan. Las columnas anulables usan doble `Option` (`None` = dejar, `Some(None)` = poner NULL, `Some(Some(v))` = poner v) mediante `entity_core::serde_helpers::double_option`.

```rust
// {}                   → nothing changes
// {"nickname": null}   → nickname = NULL
// {"nickname": "neo"}  → nickname = 'neo'
let patch: UpdateProfileRequest = serde_json::from_str(body)?;
let profile = pool.update(id, patch).await?;
```

### Opciones de `migrations(...)`

Además del flag simple, `migrations` acepta opciones DDL: `touch_updated_at` (función plpgsql compartida + trigger BEFORE UPDATE por tabla que mantiene `updated_at` fresco; requiere un campo `updated_at`, verificado en compilación), `audit` (tabla `entity_audit_log` + trigger con diffs `to_jsonb(OLD/NEW)`) y `extensions = "pg_trgm, pgcrypto"` (`CREATE EXTENSION` idempotente). Nuevas constantes: `MIGRATION_TRIGGERS` (ejecutar después de `MIGRATION_UP`) y `MIGRATION_EXTENSIONS` (antes).

```rust
#[entity(table = "articles", migrations(touch_updated_at, audit, extensions = "pg_trgm"))]
pub struct Article { /* ... */ }

for ddl in Article::MIGRATION_EXTENSIONS { sqlx::query(ddl).execute(&pool).await?; }
sqlx::query(Article::MIGRATION_UP).execute(&pool).await?;
for ddl in Article::MIGRATION_TRIGGERS { sqlx::query(ddl).execute(&pool).await?; }
```

### `#[version]`

Bloqueo optimista. Marca una columna entera (`i16`/`i32`/`i64`); el DTO de actualización gana un `expected_version` obligatorio, el UPDATE generado incrementa la columna y solo se aplica mientras la versión almacenada coincide — una escritura obsoleta falla con un error de conflicto en lugar de sobrescribir datos más recientes. El DDL usa `INTEGER NOT NULL DEFAULT 0`. Aplica a updates normales, con alcance y transaccionales.

```rust
#[derive(Entity)]
#[entity(table = "orders", migrations)]
pub struct Order {
    #[id] pub id: Uuid,
    #[field(create, update, response)] pub note: String,
    #[version] #[field(response)] #[auto] pub version: i32,
}

let patch = UpdateOrderRequest { note: Some("v2".into()), expected_version: order.version };
let updated = pool.update(order.id, patch).await?;
```

### `typed_constraints` (opcional)

La macro conoce cada constraint que crea. Con este flag, los métodos de escritura generados resuelven los nombres de constraints violados (columnas únicas, claves foráneas `belongs_to`, checks de columna, nombres de `unique_index`) y devuelven `entity_core::ConstraintError { kind, constraint, field }` en lugar de un error crudo del driver. Requiere un tipo `error` propio con `From<ConstraintError>`; sin el flag nada cambia.

```rust
#[entity(table = "users", typed_constraints, error = "AppError")]
pub struct User {
    #[id] pub id: Uuid,
    #[field(create, response)] #[column(unique)] pub email: String,
}

match pool.create(dto).await {
    Err(AppError::Constraint(v)) if v.field == Some("email") => conflict_409(),
    other => other?,
}
```

### `#[embed(prefix = "...", fields(...))]`

Aplana un value object en columnas escalares con prefijo. El DDL, la estructura Row, el SQL CRUD y los PATCH dinámicos operan sobre `price_amount_cents` / `price_currency`, mientras los DTO y la entidad llevan la estructura misma. La forma declarada se desestructura contra la estructura real en compilación — un campo renombrado, retipado, ausente o extra rompe la compilación. Padres `Option<T>` aún no soportados.

```rust
pub struct Money { pub amount_cents: i64, pub currency: String }

#[derive(Entity)]
#[entity(table = "products", migrations)]
pub struct Product {
    #[id] pub id: Uuid,
    #[field(create, update, response)]
    #[embed(prefix = "price_", fields(amount_cents: i64, currency: String))]
    pub price: Money,
}
```

### Feature `garde` (backend de validación)

Habilita `garde::Validate` en los DTO generados como alternativa mantenida a `validator`. Las reglas `#[validate(...)]` (`length`, `range`, `email`, `url`, `pattern`) se traducen a sintaxis garde; los campos sin restricciones reciben `garde(skip)`; los campos Option del DTO de actualización validan el valor interno con `inner(...)`. Con ambas features activas, `validate` tiene prioridad.

```rust
#[field(create, update, response)]
#[validate(length(min = 3, max = 8))]
pub name: String,

let dto: CreateUserRequest = serde_json::from_str(body)?;
garde::Validate::validate(&dto)?;
```

### `constraint(...)` (opcional, junto con `typed_constraints`)

Declara constraints que la macro no puede inferir — claves foráneas sobre claves naturales, CHECKs con nombres personalizados, índices de migraciones manuales — para que las violaciones se resuelvan a `ConstraintError` con el campo declarado. Tipos: `unique`, `foreign_key`, `check`. Las entradas personalizadas tienen prioridad sobre las derivadas con el mismo nombre. Requiere `typed_constraints`.

```rust
#[entity(
    table = "orders",
    typed_constraints,
    constraint(name = "orders_currency_fkey", kind = "foreign_key", field = "currency"),
    constraint(name = "orders_window_check", kind = "check"),
)]
```

### Upsert transaccional

Con `transactions` y `upsert(...)` habilitados, el adaptador `{Entity}TransactionRepo` expone `upsert` con la misma semántica SQL que el método del pool, ejecutado sobre el handle de la transacción — para flujos donde el upsert debe compartir atomicidad con sentencias adyacentes.

```rust
let mut tx = pool.begin().await?;
sqlx::query("UPDATE users SET username = NULL WHERE ...").execute(&mut *tx).await?;
let user = UserTransactionRepo::new(&mut tx).upsert(dto).await?;
tx.commit().await?;
```
