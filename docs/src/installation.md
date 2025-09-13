# Installation

Setting up d1-rs in your project is straightforward. This guide covers installation for both Cloudflare Workers and local development.

## Requirements

- **Rust**: 1.70.0 or later
- **Cargo**: Latest stable version
- **wrangler** (for Cloudflare Workers deployment)

## Adding d1-rs to Your Project

Add d1-rs to your `Cargo.toml`:

```toml
[dependencies]
d1-rs = "0.1.0"
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }

# For Cloudflare Workers
[target.'cfg(target_arch = "wasm32")'.dependencies]
worker = "0.0.18"

# For local development and testing  
[dev-dependencies]
rusqlite = "0.30"
```

## Feature Flags

d1-rs uses conditional compilation to optimize for your target environment:

```toml
[dependencies]
d1-rs = { version = "0.1.0", features = ["sqlite"] }  # For testing
```

### Available Features

| Feature | Description | When to Use |
|---------|-------------|-------------|
| `sqlite` | Enable SQLite backend | Local development, testing |
| `d1` | Enable Cloudflare D1 backend | Production deployment |

> **Note**: Features are automatically enabled based on your target architecture. You typically don't need to specify them manually.

## Project Structure

Here's a recommended project structure for d1-rs applications:

```
my-d1-project/
├── Cargo.toml
├── wrangler.toml              # Cloudflare Workers config
├── src/
│   ├── lib.rs                 # Main library
│   ├── models/                # Entity definitions
│   │   ├── mod.rs
│   │   ├── user.rs
│   │   └── post.rs
│   ├── migrations/            # Database migrations
│   │   ├── mod.rs
│   │   └── 001_create_users.rs
│   └── main.rs               # Worker entry point
├── tests/                    # Integration tests
│   ├── common/
│   │   └── mod.rs           # Test utilities  
│   ├── test_users.rs
│   └── test_posts.rs
└── README.md
```

## Cloudflare Workers Setup

### 1. Install Wrangler

```bash
npm install -g wrangler
# or
pnpm install -g wrangler
```

### 2. Configure wrangler.toml

```toml
name = "my-d1-app"
main = "src/main.rs"
compatibility_date = "2024-01-01"

[build]
command = "cargo build --release --target wasm32-unknown-unknown"

[[d1_databases]]
binding = "DB"
database_name = "my-database"
database_id = "your-database-id"
```

### 3. Create D1 Database

```bash
# Create database
wrangler d1 create my-database

# Apply migrations (once you've created them)
wrangler d1 migrations apply my-database
```

## Local Development Setup

For local development and testing, d1-rs automatically uses SQLite:

### 1. Add Test Dependencies

```toml
[dev-dependencies]
rusqlite = "0.30"
tokio-test = "0.4"
```

### 2. Create Test Configuration

```rust
// tests/common/mod.rs
use d1_rs::*;

pub async fn setup_test_db() -> D1Client {
    D1Client::new_in_memory()
        .await
        .expect("Failed to create test database")
}
```

### 3. Run Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_user_creation
```

## Verification

Verify your installation with this simple test:

```rust
// src/lib.rs
use d1_rs::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Entity)]
pub struct User {
    #[primary_key]
    pub id: i64,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_functionality() {
        let db = D1Client::new_in_memory().await.unwrap();
        
        let user = User::create()
            .set_name("Test User".to_string())
            .save(&db)
            .await
            .unwrap();

        assert_eq!(user.name, "Test User");
        println!("✅ d1-rs installation verified!");
    }
}
```

Run the test:

```bash
cargo test test_basic_functionality
```

If you see `✅ d1-rs installation verified!`, you're all set!

## Next Steps

Now that you have d1-rs installed, check out the [Quick Start](./quick-start.md) guide to build your first application!