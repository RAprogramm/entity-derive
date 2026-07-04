<div align="center">

# entity-derive

> Una macro para gobernarlos a todos

[![English](https://img.shields.io/badge/🇬🇧_English-gray?style=for-the-badge)](Home-en)
[![Русский](https://img.shields.io/badge/🇷🇺_Русский-gray?style=for-the-badge)](Home-ru)
[![한국어](https://img.shields.io/badge/🇰🇷_한국어-gray?style=for-the-badge)](Home-ko)
[![Español](https://img.shields.io/badge/🇪🇸_Español-blue?style=for-the-badge)](#)
[![中文](https://img.shields.io/badge/🇨🇳_中文-gray?style=for-the-badge)](Home-zh)

</div>

---

## ¿Qué es?

**entity-derive** es una macro procedural de Rust que genera una capa de dominio completa a partir de una única definición de entidad. No es solo CRUD — es un framework arquitectónico con eventos, hooks, comandos y filtrado type-safe.

---

## El Problema

Un backend Rust típico con ~10 entidades significa:

| Componente | Líneas de Código | Problemas |
|------------|-----------------|-----------|
| DTOs (Create, Update, Response) | ~60 por entidad | Sincronización manual, campos olvidados |
| Repository trait + impl | ~150 por entidad | Errores SQL en runtime, copy-paste |
| Mapeo Entity ↔ DTO | ~40 por entidad | Fugas de datos (`password_hash` en Response) |
| Validación y hooks | Dispersos en servicios | Duplicación, sin fuente única |
| Eventos/auditoría | Ausentes o ad-hoc | Sin historial de cambios |

**Total: ~2500 líneas de boilerplate** para 10 entidades. Y cada cambio de esquema requiere ediciones manuales en 5+ lugares.

---

## La Solución

```rust
#[derive(Entity)]
#[entity(table = "users", events, hooks, commands)]
#[command(Register)]
#[command(Deactivate, requires_id)]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    #[filter(like)]
    pub email: String,

    #[field(skip)]  // Nunca se filtra a la API
    pub password_hash: String,

    #[auto]
    #[field(response)]
    pub created_at: DateTime<Utc>,
}
```

**15 líneas → capa de dominio completa:**
- `CreateUserRequest`, `UpdateUserRequest`, `UserResponse`
- `UserRepository` con SQL type-safe
- `UserEvent::Created`, `Updated`, `Deleted`
- `UserHooks` para lógica de negocio
- Comandos `RegisterUser`, `DeactivateUser`
- `UserQuery` para filtrado

---

## Simple para Principiantes

**Ejemplo mínimo — 10 líneas:**

```rust
#[derive(Entity)]
#[entity(table = "posts")]
pub struct Post {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub title: String,

    #[field(create, update, response)]
    pub content: String,
}
```

**Listo.** Tienes:
- `CreatePostRequest`, `UpdatePostRequest`, `PostResponse`
- `PostRepository` con `create()`, `find_by_id()`, `update()`, `delete()`, `list()`
- SQL type-safe para PostgreSQL
- Todo funciona de inmediato

**Sin magia.** Ejecuta `cargo expand` — verás exactamente el código que escribirías tú mismo. Solo que sin errores y en segundos.

---

## ¿Por qué Eventos?

**Problema:** Las aplicaciones CRUD no tienen historial. ¿Quién cambió el registro? ¿Cuándo? ¿Qué había antes? La auditoría requiere infraestructura separada y disciplina.

**Solución:** `#[entity(events)]` genera eventos tipados:

```rust
pub enum UserEvent {
    Created(User),
    Updated { id: Uuid, changes: UpdateUserRequest },
    Deleted(Uuid),
}
```

**Beneficios:**
- **Auditoría de serie** — suscríbete a eventos, guarda en log
- **Event Sourcing** — puedes restaurar estado desde historial de eventos
- **Integraciones** — Kafka, notificaciones WebSocket, invalidación de caché
- **Depuración** — historial completo de cambios para cada entidad

---

## ¿Por qué Hooks?

**Problema:** La lógica de negocio está dispersa. Validación de email — en controlador. Hashing de contraseña — en servicio. Envío de email — en worker separado. ¿Dónde buscar la lógica de creación de usuario?

**Solución:** `#[entity(hooks)]` centraliza el ciclo de vida:

```rust
impl UserHooks for MyHooks {
    async fn before_create(&self, dto: &mut CreateUserRequest) -> Result<(), Error> {
        dto.email = dto.email.to_lowercase();  // Normalización
        validate_email(&dto.email)?;            // Validación
        Ok(())
    }

    async fn after_create(&self, user: &User) -> Result<(), Error> {
        self.mailer.send_welcome(user).await?;  // Acción de negocio
        Ok(())
    }
}
```

**Beneficios:**
- **Lugar único** — toda la lógica de entidad junto a la definición
- **Predecibilidad** — claro cuándo se ejecuta qué
- **Testabilidad** — los hooks se pueden mockear y probar aisladamente
- **Composición** — diferentes implementaciones para diferentes contextos

---

## ¿Por qué Comandos?

**Problema:** La API REST oculta la intención. `POST /users` — ¿es registro? ¿Creación por admin? ¿Importación CSV? `PATCH /users/123` — ¿desactivación? ¿Cambio de email? ¿Ban?

**Solución:** `#[command(...)]` expresa el dominio de negocio:

```rust
#[command(Register)]           // Auto-registro
#[command(Invite)]             // Invitación por admin
#[command(Deactivate, requires_id)]  // Desactivación de cuenta
#[command(Ban, requires_id)]   // Ban por violaciones
```

**Compara:**
```rust
// CRUD (¿qué está pasando?)
pool.update(user_id, UpdateUserRequest { active: Some(false), ..default() }).await?;

// Comandos (intención clara)
handler.handle(DeactivateUser { id: user_id }).await?;
```

**Beneficios:**
- **API auto-documentada** — nombres de comandos = vocabulario de negocio
- **Lógica diferente** — `Deactivate` y `Ban` pueden tener diferentes side-effects
- **CQRS listo** — comandos fáciles de enrutar, loguear, reintentar
- **Type safety** — el compilador verifica que el comando existe

---

## ¿Por qué Filtrado Type-Safe?

**Problema:** Los query params de string son fuente de errores en runtime:
```
GET /users?stauts=active  // Typo — ignorado silenciosamente
GET /users?created_at=tomorrow  // Fecha inválida — panic en runtime
```

**Solución:** `#[filter]` genera struct tipado:

```rust
let query = UserQuery {
    email: Some("@company.com".into()),  // ILIKE '%@company.com%'
    created_at_min: Some(week_ago),       // >= week_ago
    created_at_max: Some(now),            // <= now
    ..Default::default()
};

let users = pool.list_filtered(&query, 100, 0).await?;
```

**Beneficios:**
- **Verificación en tiempo de compilación** — typo en nombre de campo = error de compilación
- **Type safety** — no puedes comparar `DateTime` con `String`
- **Autocompletado** — el IDE sugiere filtros disponibles
- **Protección contra SQL injection** — parámetros se bindean, no se concatenan

---

## Transparencia

La macro no oculta lógica. Todo lo generado es código Rust normal que puedes:

- **Leer** — `cargo expand` muestra todo el código generado
- **Entender** — sin reflection en runtime, solo structs y traits
- **Sobrescribir** — `sql = "trait"` y escribe tu propio SQL
- **Depurar** — errores del compilador apuntan a tu código, no a internos de la macro

```rust
// ¿Quieres entender qué se genera?
cargo expand --lib | grep -A 50 "impl UserRepository"
```

**Zero magic.** Si la macro falla — siempre puedes escribir código a mano. Sin lock-in.

---

## Todo el Poder de Rust

### Garantías en Tiempo de Compilación

```rust
#[field(skip)]
pub password_hash: String,
```

Esto no es una verificación en runtime "no serialices este campo". Es la **ausencia física** del campo en la struct `UserResponse`. No puedes devolverlo accidentalmente — el campo simplemente no existe.

### Abstracciones Zero-cost

El código generado es:
- `struct` regular sin Box/dyn
- Llamadas directas a sqlx sin capas intermedias
- `#[inline]` en hot paths
- Sin allocaciones más allá de lo necesario

**Benchmark:** el repositorio generado corre a la misma velocidad que el escrito a mano. Porque es el mismo código.

### Async de Serie

```rust
// Todo async, todo Send + Sync
let user = pool.find_by_id(id).await?;
let users = pool.list(100, 0).await?;
```

Compatibilidad total con tokio, async-std, cualquier runtime async.

### Tipado Estricto

```rust
// Error de compilación: no existe ese campo
let query = UserQuery { naem: "test".into(), ..default() };
                        ^^^^ unknown field

// Error de compilación: tipo incorrecto
let query = UserQuery { created_at_min: "yesterday".into(), ..default() };
                                        ^^^^^^^^^^^^ expected DateTime<Utc>
```

Si el código compila — funciona correctamente.

---

## Arquitectura Profesional

### Clean Architecture Listo

```
Domain Layer (entity-derive)
├── Entities — #[derive(Entity)]
├── DTOs — CreateRequest, UpdateRequest, Response
├── Repository Trait — abstracción de almacenamiento
├── Events — eventos de dominio
├── Commands — operaciones de negocio
└── Hooks — lógica de ciclo de vida

Infrastructure Layer (tu código)
├── Repository Impl — PgPool automático o custom
├── Event Handlers — suscripciones a eventos
├── Command Handlers — implementación de lógica de negocio
└── External Services — integraciones
```

Separación limpia. Domain no sabe nada de HTTP, base de datos, Kafka. Esos son detalles de implementación.

### CQRS/Event Sourcing Listo

```rust
// Command side
handler.handle(RegisterUser { email, name }).await?;

// Query side
let users = pool.list_filtered(&query, 100, 0).await?;

// Event side
match event {
    UserEvent::Created(user) => kafka.send("user.created", &user).await?,
    UserEvent::Updated { id, changes } => audit_log.record(id, changes).await?,
    _ => {}
}
```

¿Quieres CRUD simple? Lo tienes. ¿Quieres CQRS completo? Activa `commands` y `events`. La arquitectura crece con el proyecto.

### Extensibilidad

**Nivel 1: CRUD Básico**
```rust
#[entity(table = "users")]
```

**Nivel 2: + Filtrado**
```rust
#[entity(table = "users")]
// + #[filter] en campos
```

**Nivel 3: + Eventos y Hooks**
```rust
#[entity(table = "users", events, hooks)]
```

**Nivel 4: + Comandos CQRS**
```rust
#[entity(table = "users", events, hooks, commands)]
#[command(Register)]
#[command(Deactivate, requires_id)]
```

**Nivel 5: Control Total**
```rust
#[entity(table = "users", sql = "trait", events, hooks, commands)]
// Tu SQL, tu lógica, pero manteniendo todos los DTOs y tipos
```

Empieza simple. Añade features mientras creces. No reescribas — extiende.

---

## Seguridad

```rust
#[field(skip)]
pub password_hash: String,
```

`skip` significa: este campo **nunca** aparecerá en:
- `CreateUserRequest` (no se puede pasar desde fuera)
- `UpdateUserRequest` (no se puede modificar vía API)
- `UserResponse` (no se puede devolver accidentalmente al cliente)

La única forma de trabajar con `password_hash` — directamente a través de la entidad en código. Fuga imposible por diseño.

---

## Por Qué Esto es Genial

| Aspecto | Lo Que Obtienes |
|---------|-----------------|
| **Velocidad de Desarrollo** | 10 entidades en una hora en lugar de un día |
| **Fiabilidad** | Verificación en tiempo de compilación de todo |
| **Seguridad** | Imposible filtrar datos accidentalmente |
| **Rendimiento** | Zero-cost, como código escrito a mano |
| **Claridad** | Generación transparente, sin magia |
| **Flexibilidad** | De CRUD simple a CQRS con un atributo |
| **Escalabilidad** | La arquitectura crece con el proyecto |
| **Mantenibilidad** | Una sola fuente de verdad, menos bugs |

---

## Documentación

| Tema | Descripción |
|------|-------------|
| [[Atributos\|Atributos]] | Referencia completa de atributos |
| [[Filtrado\|Filtrado]] | Filtrado de consultas type-safe |
| [[Relaciones\|Relaciones]] | `belongs_to` y `has_many` |
| [[Eventos\|Eventos]] | Eventos de ciclo de vida |
| [[Hooks\|Ganchos]] | Hooks before/after |
| [[Comandos\|Comandos]] | Patrón CQRS |
| [[SQL Personalizado\|SQL-Personalizado]] | Consultas complejas |
| [[Ejemplos\|Ejemplos]] | Casos de uso reales |
| [[Web Frameworks\|Frameworks-Web]] | Integración con Axum, Actix |
| [[Mejores Prácticas\|Mejores-Prácticas]] | Guías para producción |

---

<div align="center">

**Esto no es un framework que dicta cómo vivir.**
**Es una herramienta que elimina la rutina y te permite construir correctamente.**

---

</div>
