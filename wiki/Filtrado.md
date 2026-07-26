Genera estructuras de consulta type-safe para filtrar entidades. El filtrado permite paginación, búsqueda y consultas de rango con seguridad en tiempo de compilación.

## Inicio Rápido

```rust
#[derive(Entity)]
#[entity(table = "products")]
pub struct Product {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    #[filter]
    pub name: String,

    #[field(create, update, response)]
    #[filter(like)]
    pub description: String,

    #[field(create, update, response)]
    #[filter(range)]
    pub price: i64,

    #[field(create, response)]
    #[filter]
    pub category_id: Uuid,

    #[field(response)]
    #[auto]
    #[filter(range)]
    pub created_at: DateTime<Utc>,
}
```

## Código Generado

### Estructura de Query

```rust
/// Parámetros de consulta para filtrar entidades Product.
#[derive(Debug, Clone, Default)]
pub struct ProductQuery {
    /// Filtrar por coincidencia exacta de name.
    pub name: Option<String>,

    /// Filtrar por patrón de description (ILIKE).
    pub description: Option<String>,

    /// Filtrar por precio mínimo.
    pub price_from: Option<i64>,

    /// Filtrar por precio máximo.
    pub price_to: Option<i64>,

    /// Filtrar por coincidencia exacta de category_id.
    pub category_id: Option<Uuid>,

    /// Filtrar por created_at mínimo.
    pub created_at_from: Option<DateTime<Utc>>,

    /// Filtrar por created_at máximo.
    pub created_at_to: Option<DateTime<Utc>>,

    /// Número máximo de resultados.
    pub limit: Option<i64>,

    /// Número de resultados a omitir.
    pub offset: Option<i64>,
}
```

### Método de Repository

```rust
#[async_trait]
pub trait ProductRepository: Send + Sync {
    // ... métodos CRUD estándar

    /// Consultar productos con filtros.
    async fn query(&self, query: ProductQuery) -> Result<Vec<Product>, Self::Error>;
}
```

### SQL Generado

```sql
SELECT id, name, description, price, category_id, created_at
FROM products
WHERE ($1 IS NULL OR name = $1)
  AND ($2 IS NULL OR description ILIKE $2)
  AND ($3 IS NULL OR price >= $3)
  AND ($4 IS NULL OR price <= $4)
  AND ($5 IS NULL OR category_id = $5)
  AND ($6 IS NULL OR created_at >= $6)
  AND ($7 IS NULL OR created_at <= $7)
ORDER BY created_at DESC
LIMIT $8 OFFSET $9
```

## Tipos de Filtro

### Coincidencia Exacta (`#[filter]` o `#[filter(eq)]`)

Filtra donde el campo es igual al valor proporcionado.

```rust
#[filter]
pub status: String,

#[filter(eq)]  // Igual que arriba
pub category_id: Uuid,
```

**Generado:**
```rust
pub status: Option<String>,
pub category_id: Option<Uuid>,
```

**SQL:**
```sql
WHERE status = $1
  AND category_id = $2
```

### Coincidencia de Patrón (`#[filter(like)]`)

Filtra usando coincidencia de patrones case-insensitive (ILIKE).

```rust
#[filter(like)]
pub name: String,

#[filter(like)]
pub description: String,
```

**Generado:**
```rust
pub name: Option<String>,
pub description: Option<String>,
```

**SQL:**
```sql
WHERE name ILIKE $1
  AND description ILIKE $2
```

**Uso:**

Pase la subcadena sin comodines. El código generado envuelve el valor en
`%...%` y escapa los `%`, `_` y `\` que contenga, de modo que un comodín
escrito por el usuario final coincide literalmente en lugar de ampliar
la búsqueda:

```rust
let query = ProductQuery {
    name: Some("widget".into()),          // Contiene "widget"
    description: Some("premium".into()),  // Contiene "premium"
    ..Default::default()
};
```

Todo filtro `like` es una coincidencia de subcadena: buscar solo por
prefijo o solo por sufijo no se puede expresar con él.

### Filtro de Rango (`#[filter(range)]`)

Filtra dentro de un rango (inclusivo).

```rust
#[filter(range)]
pub price: i64,

#[filter(range)]
pub created_at: DateTime<Utc>,
```

**Generado:**
```rust
pub price_from: Option<i64>,
pub price_to: Option<i64>,
pub created_at_from: Option<DateTime<Utc>>,
pub created_at_to: Option<DateTime<Utc>>,
```

**SQL:**
```sql
WHERE price >= $1 AND price <= $2
  AND created_at >= $3 AND created_at <= $4
```

## Ejemplos de Uso

### Filtrado Básico

```rust
// Encontrar productos por categoría
let query = ProductQuery {
    category_id: Some(electronics_category_id),
    ..Default::default()
};
let products = repo.query(query).await?;
```

### Paginación

```rust
// Obtener página 2 (20 items por página)
let query = ProductQuery {
    limit: Some(20),
    offset: Some(20),
    ..Default::default()
};
let products = repo.query(query).await?;
```

### Filtros Combinados

```rust
// Buscar electrónicos asequibles
let query = ProductQuery {
    category_id: Some(electronics_category_id),
    price_from: Some(0),
    price_to: Some(10000),  // $100.00
    name: Some("phone".into()),
    limit: Some(50),
    ..Default::default()
};
let products = repo.query(query).await?;
```

### Rango de Fechas

```rust
// Obtener productos creados este mes
let now = Utc::now();
let month_start = now.with_day(1).unwrap().date_naive().and_hms_opt(0, 0, 0).unwrap();

let query = ProductQuery {
    created_at_from: Some(month_start.and_utc()),
    created_at_to: Some(now),
    ..Default::default()
};
let products = repo.query(query).await?;
```

### Integración con Endpoint API

```rust
use axum::{extract::Query, Json};

#[derive(Deserialize)]
pub struct ProductQueryParams {
    pub name: Option<String>,
    pub category_id: Option<Uuid>,
    pub min_price: Option<i64>,
    pub max_price: Option<i64>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

async fn list_products(
    Query(params): Query<ProductQueryParams>,
    pool: Extension<PgPool>,
) -> Result<Json<Vec<ProductResponse>>, AppError> {
    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(20).min(100);

    let query = ProductQuery {
        name: params.name,
        category_id: params.category_id,
        price_from: params.min_price,
        price_to: params.max_price,
        limit: Some(per_page),
        offset: Some((page - 1) * per_page),
        ..Default::default()
    };

    let products = pool.query(query).await?;
    let responses: Vec<_> = products.into_iter().map(ProductResponse::from).collect();

    Ok(Json(responses))
}
```

## Con Borrado Lógico

Cuando `soft_delete` está habilitado, la consulta excluye automáticamente registros eliminados:

```rust
#[derive(Entity)]
#[entity(table = "documents", soft_delete)]
pub struct Document {
    #[id]
    pub id: Uuid,

    #[field(create, response)]
    #[filter(like)]
    pub title: String,

    #[field(skip)]
    pub deleted_at: Option<DateTime<Utc>>,
}
```

**SQL Generado:**
```sql
SELECT * FROM documents
WHERE deleted_at IS NULL
  AND ($1 IS NULL OR title ILIKE $1)
LIMIT $2 OFFSET $3
```

Método adicional para incluir eliminados:
```rust
async fn query_with_deleted(&self, query: DocumentQuery) -> Result<Vec<Document>, Self::Error>;
```

## Extensiones de Query Personalizadas

Para consultas complejas, usa `sql = "trait"` e implementa filtrado personalizado:

```rust
#[derive(Entity)]
#[entity(table = "products", sql = "trait")]
pub struct Product { /* ... */ }

pub trait ProductQueryExt {
    async fn search_fulltext(&self, term: &str, limit: i64) -> Result<Vec<Product>, sqlx::Error>;
    async fn find_by_tags(&self, tags: &[String]) -> Result<Vec<Product>, sqlx::Error>;
}

#[async_trait]
impl ProductQueryExt for PgPool {
    async fn search_fulltext(&self, term: &str, limit: i64) -> Result<Vec<Product>, sqlx::Error> {
        let rows: Vec<ProductRow> = sqlx::query_as(
            r#"
            SELECT * FROM products
            WHERE to_tsvector('english', name || ' ' || description)
                  @@ plainto_tsquery('english', $1)
            ORDER BY ts_rank(to_tsvector('english', name || ' ' || description),
                            plainto_tsquery('english', $1)) DESC
            LIMIT $2
            "#
        )
        .bind(term)
        .bind(limit)
        .fetch_all(self)
        .await?;

        Ok(rows.into_iter().map(Product::from).collect())
    }

    async fn find_by_tags(&self, tags: &[String]) -> Result<Vec<Product>, sqlx::Error> {
        let rows: Vec<ProductRow> = sqlx::query_as(
            "SELECT * FROM products WHERE tags && $1"
        )
        .bind(tags)
        .fetch_all(self)
        .await?;

        Ok(rows.into_iter().map(Product::from).collect())
    }
}
```

## Mejores Prácticas

1. **Paginación por defecto** — Siempre aplica límites sensatos para prevenir conjuntos de resultados grandes
2. **Validar patrones** — Sanitiza patrones LIKE para prevenir problemas SQL
3. **Indexar columnas filtradas** — Crea índices de BD para campos filtrados frecuentemente
4. **Usar filtros específicos** — Prefiere coincidencia exacta sobre coincidencia de patrón cuando sea posible
5. **Combinar con ordenación** — Considera agregar campos de ordenación a tu estructura de query

## Ver También

- [[Atributos|Atributos]] — Referencia completa de atributos
- [[SQL Personalizado|SQL-Personalizado]] — Consultas personalizadas complejas
- [[Relaciones|Relaciones]] — Filtrado con relaciones

## Ordenación y paginación keyset

Marca las columnas ordenables con `#[sort]`: la estructura Query obtiene un selector con lista blanca `{Entity}SortField` (una variante `Asc`/`Desc` por columna, JSON en `snake_case`), de modo que la entrada del usuario nunca puede inyectar SQL. Cada repositorio también obtiene paginación keyset `list_after`: con ids UUIDv7 el recorrido por id es cronológicamente estable y no se degrada en páginas profundas, a diferencia de OFFSET.

```rust
#[derive(Entity)]
#[entity(table = "posts")]
pub struct Post {
    #[id]
    pub id: Uuid,

    #[field(create, update, response)]
    #[sort]
    #[filter(like)]
    pub title: String,

    #[field(create, response)]
    #[sort]
    pub views: i64,
}

let query = PostQuery {
    sort: Some(PostSortField::ViewsDesc),
    limit: Some(20),
    ..Default::default()
};
let top: Vec<Post> = pool.query(query).await?;

let page: Vec<Post> = pool.list_after(None, 20).await?;
let next: Vec<Post> = pool.list_after(page.last().map(|p| p.id), 20).await?;
```

## Búsqueda por trigramas

`#[filter(search)]` en una columna de texto añade un filtro difuso de subcadena (`col ILIKE '%' || $n || '%'`; el término va ligado tal cual, por lo que un `%` dentro de él actúa como comodín en lugar de coincidir literalmente, a diferencia de `#[filter(like)]`). Con `migrations`, el índice `gin_trgm_ops` correspondiente aterriza en `MIGRATION_UP` y `pg_trgm` se añade automáticamente a `MIGRATION_EXTENSIONS`. Verificación en compilación: el campo debe ser `String`.

```rust
#[field(create, update, response)]
#[filter(search)]
pub title: String,

let hits = pool.query(ArticleQuery { title: Some("rust".into()), ..Default::default() }).await?;
```
