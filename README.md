# D1 ORM

Type-safe ORM for Cloudflare D1 with SQLite testing support.

## Features

- **Dual Backend**: Works with both Cloudflare D1 (WASM) and native SQLite (testing)
- **Boolean Conversion**: Automatically handles SQLite's integer↔boolean storage
- **Type Safety**: Compile-time query validation through derive macros
- **Cross-Platform**: Compiles for both WASM and native targets

## Running Tests

```bash
# From the root directory
just test-orm

# Or directly with cargo
cargo test -p d1orm --features test-utils
```

## Usage

```rust
use d1orm::*;

#[derive(Entity)]
#[table(name = "users")]
pub struct User {
    #[primary_key]
    pub id: i64,
    pub email: String,
    pub is_active: bool,
}

// Create
let user = User::create()
    .set_email("test@example.com".to_string())
    .set_is_active(true)
    .save(&db).await?;

// Query
let active_users = User::query()
    .where_is_active_eq(true)
    .all(&db).await?;
```

## Architecture

- **WASM Target**: Uses `worker::d1::D1Database` for Cloudflare Workers
- **Native Target**: Uses `rusqlite` with in-memory databases for testing
- **Boolean Handling**: Automatically converts SQLite integers (0/1) to proper booleans
- **Conditional Compilation**: Zero runtime overhead - only includes necessary backends