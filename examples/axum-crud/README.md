<!--
SPDX-FileCopyrightText: 2025-2026 RAprogramm <andrey.rozanov.vl@gmail.com>
SPDX-License-Identifier: MIT
-->

# Axum CRUD Example

Complete CRUD API example using `entity-derive` with Axum and PostgreSQL.

## Features

- Full CRUD operations (Create, Read, Update, Delete, List)
- Auto-generated DTOs and repository
- OpenAPI documentation with Swagger UI
- Docker Compose for PostgreSQL

## Quick Start

1. Start PostgreSQL:

```bash
docker compose up -d
```

2. Run the application:

```bash
cargo run
```

3. Open Swagger UI: <http://localhost:3000/swagger-ui>

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/users` | Create user |
| GET | `/users` | List users |
| GET | `/users/{id}` | Get user by ID |
| PATCH | `/users/{id}` | Update user |
| DELETE | `/users/{id}` | Delete user |

## Example Requests

### Create User

```bash
curl -X POST http://localhost:3000/users \
  -H "Content-Type: application/json" \
  -d '{"name": "John", "email": "john@example.com", "password_hash": "hashed"}'
```

### List Users

```bash
curl http://localhost:3000/users?limit=10&offset=0
```

### Get User

```bash
curl http://localhost:3000/users/{id}
```

### Update User

```bash
curl -X PATCH http://localhost:3000/users/{id} \
  -H "Content-Type: application/json" \
  -d '{"name": "John Updated"}'
```

### Delete User

```bash
curl -X DELETE http://localhost:3000/users/{id}
```

## Project Structure

```
axum-crud/
├── Cargo.toml
├── docker-compose.yml
├── migrations/
│   └── 001_create_users.sql
└── src/
    └── main.rs
```

## Entity Definition

```rust
#[derive(Entity)]
#[entity(table = "users")]
pub struct User {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    pub name: String,

    #[field(create, update, response)]
    pub email: String,

    #[field(create, skip)]  // Never in response
    pub password_hash: String,

    #[field(response)]
    #[auto]
    pub created_at: DateTime<Utc>,
}
```

## Generated Code

The `Entity` derive generates:

- `CreateUserRequest` — DTO for POST requests
- `UpdateUserRequest` — DTO for PATCH requests (all fields optional)
- `UserResponse` — DTO for API responses
- `UserRepository` — Async trait with CRUD methods
- `impl UserRepository for PgPool` — PostgreSQL implementation
