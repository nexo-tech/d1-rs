# D1Client API Reference

The `D1Client` is the central database client that provides a unified interface for both Cloudflare D1 (WASM) and SQLite (native) backends.

## Type Definition

```rust
pub struct D1Client {
    // Internal implementation varies by target
}
```

## Constructors

### Cloudflare Workers (WASM)

```rust
impl D1Client {
    /// Create a new client from a Cloudflare D1 database binding
    pub fn new(database: worker::d1::D1Database) -> Self
}
```

**Usage in Workers:**

```rust
use worker::*;
use d1_rs::*;

#[event(fetch)]
async fn main(req: Request, env: Env, ctx: worker::Context) -> Result<Response> {
    let db = D1Client::new(env.d1("DB")?);
    
    // Use the client...
    let users = User::query().all(&db).await?;
    Response::from_json(&users)
}
```

### Native (SQLite)

```rust
impl D1Client {
    /// Create a new in-memory SQLite database (for testing)
    pub async fn new_in_memory() -> Result<Self>
    
    /// Create a new file-based SQLite database
    pub async fn new_file(path: &str) -> Result<Self>
}
```

**Usage in tests:**

```rust
#[tokio::test]
async fn test_user_operations() {
    let db = D1Client::new_in_memory().await.unwrap();
    
    // Run migrations
    setup_test_schema(&db).await.unwrap();
    
    // Test operations...
    let user = User::create()
        .set_name("Test User".to_string())
        .save(&db)
        .await
        .unwrap();
    
    assert_eq!(user.name, "Test User");
}
```

**Usage with file database:**

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let db = D1Client::new_file("./app.db").await?;
    
    // Run migrations
    run_migrations(&db).await?;
    
    // Application logic...
    Ok(())
}
```

## Methods

The `D1Client` implements internal methods for query execution. These are typically not called directly - instead, use the entity methods that take `&D1Client` as a parameter.

### Internal Query Methods

```rust
impl D1Client {
    // These methods are used internally by the ORM
    // and are not part of the public API
    
    async fn execute_query(&self, sql: &str, params: &[serde_json::Value]) -> Result<Vec<QueryResult>>;
    async fn execute_update(&self, sql: &str, params: &[serde_json::Value]) -> Result<u64>;
    // ... other internal methods
}
```

## Error Handling

All `D1Client` methods return `Result<T>` types:

```rust
use d1_rs::{D1Client, D1RsError, Result};

async fn handle_database_errors() -> Result<()> {
    let db = match D1Client::new_in_memory().await {
        Ok(client) => client,
        Err(D1RsError::Database(msg)) => {
            eprintln!("Failed to create database: {}", msg);
            return Err(D1RsError::Database(msg));
        }
        Err(other) => return Err(other),
    };
    
    // Use database...
    Ok(())
}
```

## Connection Management

### Connection Pooling

On native targets, `D1Client` manages SQLite connections internally. Each client instance maintains its own connection.

### Cloudflare Workers

In Workers, the D1 binding handles connection management automatically. The client is a lightweight wrapper around the D1 binding.

### Cloning

`D1Client` can be cloned safely:

```rust
let db = D1Client::new_in_memory().await?;
let db_clone = db.clone(); // Shares the same underlying connection

// Both can be used concurrently
let users_future = User::query().all(&db);
let posts_future = Post::query().all(&db_clone);

let (users, posts) = tokio::join!(users_future, posts_future);
```

## Thread Safety

`D1Client` is thread-safe and can be shared across async tasks:

```rust
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let db = Arc::new(D1Client::new_in_memory().await?);
    
    // Spawn multiple tasks
    let handles: Vec<_> = (0..10).map(|i| {
        let db = db.clone();
        tokio::spawn(async move {
            User::create()
                .set_name(format!("User {}", i))
                .set_email(format!("user{}@example.com", i))
                .save(&*db)
                .await
        })
    }).collect();
    
    // Wait for all tasks to complete
    for handle in handles {
        let user = handle.await??;
        println!("Created user: {}", user.name);
    }
    
    Ok(())
}
```

## Configuration

### SQLite-Specific Options

When using file-based SQLite, you can configure various options:

```rust
// Currently, configuration is limited
// Future versions may support:
// - Connection pool size
// - SQLite pragmas
// - Timeout settings
// - Journal modes

let db = D1Client::new_file("./app.db").await?;
```

### Cloudflare D1 Options

D1 configuration is handled through your `wrangler.toml`:

```toml
[[d1_databases]]
binding = "DB"
database_name = "my-app-database"
database_id = "your-database-id"
```

## Best Practices

### Client Lifecycle

```rust
// ✅ Good: Create client once, reuse throughout application
async fn good_pattern() -> Result<()> {
    let db = D1Client::new_in_memory().await?;
    
    // Run migrations once at startup
    run_migrations(&db).await?;
    
    // Reuse client for all operations
    let users = User::query().all(&db).await?;
    let posts = Post::query().all(&db).await?;
    
    Ok(())
}

// ❌ Bad: Creating multiple clients unnecessarily
async fn bad_pattern() -> Result<()> {
    for i in 0..10 {
        let db = D1Client::new_in_memory().await?; // Inefficient!
        User::create()
            .set_name(format!("User {}", i))
            .save(&db)
            .await?;
    }
    Ok(())
}
```

### Error Handling

```rust
async fn robust_client_usage() -> Result<Vec<User>> {
    let db = D1Client::new_in_memory().await
        .map_err(|e| {
            eprintln!("Failed to initialize database: {}", e);
            e
        })?;
    
    // Always handle potential database errors
    match User::query().all(&db).await {
        Ok(users) => {
            println!("Loaded {} users", users.len());
            Ok(users)
        }
        Err(D1RsError::Database(msg)) => {
            eprintln!("Database query failed: {}", msg);
            // Could retry, use cache, or return empty vec
            Ok(Vec::new())
        }
        Err(other) => {
            eprintln!("Unexpected error: {}", other);
            Err(other)
        }
    }
}
```

### Testing Patterns

```rust
// Test utility for consistent database setup
async fn create_test_db() -> D1Client {
    let db = D1Client::new_in_memory().await.unwrap();
    
    // Set up schema
    let migration = SchemaMigration::new("test_schema".to_string())
        .create_table("users")
            .integer("id").primary_key().auto_increment().build()
            .text("name").not_null().build()
            .text("email").not_null().unique().build()
            .boolean("is_active").default_value(DefaultValue::Boolean(true)).build()
        .build();
    
    migration.execute(&db).await.unwrap();
    
    db
}

#[tokio::test]
async fn test_with_fresh_database() {
    let db = create_test_db().await;
    
    // Each test gets a clean database
    let user = User::create()
        .set_name("Test User".to_string())
        .set_email("test@example.com".to_string())
        .save(&db)
        .await
        .unwrap();
    
    assert_eq!(user.name, "Test User");
}
```

## Platform-Specific Notes

### Cloudflare Workers

- Client creation is synchronous (wraps existing D1 binding)
- All database operations are async and may have latency
- Query results are limited by Workers execution time
- Connection management is handled by Cloudflare

### SQLite (Native)

- Client creation is async (opens database connection)
- In-memory databases are isolated per client instance
- File databases can be shared across process restarts
- Connection management is handled by rusqlite

## Related Types

- **[Entity](./entity.md)** - Entities that work with D1Client
- **[SchemaMigration](./schema-migration.md)** - Schema operations using the client
- **[Error Types](./errors.md)** - Errors that can be returned
- **[Query Builders](./query-builders.md)** - Query construction with the client

## Examples

See the [Quick Start Guide](../quick-start.md) for complete examples of D1Client usage in different environments.