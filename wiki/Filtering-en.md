# Query Filtering
Generate type-safe query structs for filtering entities. Filtering enables pagination, search, and range queries with compile-time safety.
## Quick Start

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

## Generated Code

### Query Struct

```rust
/// Query parameters for filtering Product entities.
#[derive(Debug, Clone, Default)]
pub struct ProductQuery {
    /// Filter by exact name match.
    pub name: Option<String>,

    /// Filter by description pattern (ILIKE).
    pub description: Option<String>,

    /// Filter by minimum price.
    pub price_from: Option<i64>,

    /// Filter by maximum price.
    pub price_to: Option<i64>,

    /// Filter by exact category_id match.
    pub category_id: Option<Uuid>,

    /// Filter by minimum created_at.
    pub created_at_from: Option<DateTime<Utc>>,

    /// Filter by maximum created_at.
    pub created_at_to: Option<DateTime<Utc>>,

    /// Maximum number of results.
    pub limit: Option<i64>,

    /// Number of results to skip.
    pub offset: Option<i64>,
}
```

### Repository Method

```rust
#[async_trait]
pub trait ProductRepository: Send + Sync {
    // ... standard CRUD methods

    /// Query products with filters.
    async fn query(&self, query: ProductQuery) -> Result<Vec<Product>, Self::Error>;
}
```

### Generated SQL

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

## Filter Types

### Exact Match (`#[filter]` or `#[filter(eq)]`)

Filters where field equals the provided value.

```rust
#[filter]
pub status: String,

#[filter(eq)]  // Same as above
pub category_id: Uuid,
```

**Generated:**
```rust
pub status: Option<String>,
pub category_id: Option<Uuid>,
```

**SQL:**
```sql
WHERE status = $1
  AND category_id = $2
```

### Pattern Match (`#[filter(like)]`)

Filters using case-insensitive pattern matching (ILIKE).

```rust
#[filter(like)]
pub name: String,

#[filter(like)]
pub description: String,
```

**Generated:**
```rust
pub name: Option<String>,
pub description: Option<String>,
```

**SQL:**
```sql
WHERE name ILIKE $1
  AND description ILIKE $2
```

**Usage:**
```rust
let query = ProductQuery {
    name: Some("%widget%".into()),  // Contains "widget"
    description: Some("premium%".into()),  // Starts with "premium"
    ..Default::default()
};
```

### Range Filter (`#[filter(range)]`)

Filters within a range (inclusive).

```rust
#[filter(range)]
pub price: i64,

#[filter(range)]
pub created_at: DateTime<Utc>,
```

**Generated:**
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

## Usage Examples

### Basic Filtering

```rust
// Find products by category
let query = ProductQuery {
    category_id: Some(electronics_category_id),
    ..Default::default()
};
let products = repo.query(query).await?;
```

### Pagination

```rust
// Get page 2 (20 items per page)
let query = ProductQuery {
    limit: Some(20),
    offset: Some(20),
    ..Default::default()
};
let products = repo.query(query).await?;
```

### Combined Filters

```rust
// Search for affordable electronics
let query = ProductQuery {
    category_id: Some(electronics_category_id),
    price_from: Some(0),
    price_to: Some(10000),  // $100.00
    name: Some("%phone%".into()),
    limit: Some(50),
    ..Default::default()
};
let products = repo.query(query).await?;
```

### Date Range

```rust
// Get products created this month
let now = Utc::now();
let month_start = now.with_day(1).unwrap().date_naive().and_hms_opt(0, 0, 0).unwrap();

let query = ProductQuery {
    created_at_from: Some(month_start.and_utc()),
    created_at_to: Some(now),
    ..Default::default()
};
let products = repo.query(query).await?;
```

### API Endpoint Integration

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
        name: params.name.map(|n| format!("%{}%", n)),
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

## With Soft Delete

When `soft_delete` is enabled, the query automatically excludes deleted records:

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

**Generated SQL:**
```sql
SELECT * FROM documents
WHERE deleted_at IS NULL
  AND ($1 IS NULL OR title ILIKE $1)
LIMIT $2 OFFSET $3
```

Additional method for including deleted:
```rust
async fn query_with_deleted(&self, query: DocumentQuery) -> Result<Vec<Document>, Self::Error>;
```

## Custom Query Extensions

For complex queries, use `sql = "trait"` and implement custom filtering:

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

## Best Practices

1. **Default pagination** — Always apply sensible limits to prevent large result sets
2. **Validate patterns** — Sanitize LIKE patterns to prevent SQL issues
3. **Index filtered columns** — Create database indexes for frequently filtered fields
4. **Use specific filters** — Prefer exact match over pattern match when possible
5. **Combine with sorting** — Consider adding sort fields to your query struct

## See Also

- [[Attributes]] — Complete attribute reference
- [[Custom SQL]] — Complex custom queries
- [[Relations]] — Filtering with relationships

## Sorting & Keyset Pagination

Mark sortable columns with `#[sort]`: the Query struct gains a whitelisted `{Entity}SortField` selector (one `Asc`/`Desc` variant per column, `snake_case` JSON), so user input can never inject SQL. Every repository also gets `list_after` keyset pagination — with default UUIDv7 ids the id-ordered walk is chronologically stable and stays fast on deep pages, unlike OFFSET.

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

## Trigram Search

`#[filter(search)]` on a text column adds a fuzzy substring filter (`col ILIKE '%' || $n || '%'`, the term is bound). With `migrations`, the matching `gin_trgm_ops` index lands in `MIGRATION_UP` and `pg_trgm` is added to `MIGRATION_EXTENSIONS` automatically. Compile-time check: the field must be a `String`.

```rust
#[field(create, update, response)]
#[filter(search)]
pub title: String,

let hits = pool.query(ArticleQuery { title: Some("rust".into()), ..Default::default() }).await?;
```
