# 🚀 d1-rs: The World's Most Advanced Type-Safe ORM

[![Build Status](https://github.com/d1-rs/d1-rs/workflows/CI/badge.svg)](https://github.com/d1-rs/d1-rs/actions)
[![Crates.io](https://img.shields.io/crates/v/d1-rs)](https://crates.io/crates/d1-rs)
[![Documentation](https://docs.rs/d1-rs/badge.svg)](https://docs.rs/d1-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

**d1-rs** is a revolutionary type-safe ORM for Cloudflare D1 with SQLite testing support. It is the **first ORM in any language** to provide compile-time safe relationships, nested eager loading, recursive relationships, and rich M2M entities.

## 🎯 **Perfect for**
- **Cloudflare Workers** with D1 databases
- **Type-safe** database operations
- **Local development** with SQLite
- **Graph-based relations** inspired by ent-go

## ⚡ **Quick Example**

```rust
use d1_rs::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Entity)]
pub struct User {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub email: String,
    pub is_active: bool,
}

// Create
let user = User::create()
    .set_name("Alice".to_string())
    .set_email("alice@example.com".to_string())
    .set_is_active(true)
    .save(&db)
    .await?;

// Query with type-safe methods
let users = User::query()
    .where_is_active_eq(true)
    .where_name_contains("Alice")
    .order_by_name_asc()
    .limit(10)
    .all(&db)
    .await?;

// Relations (type-safe, no string literals!)
let posts = user.posts().all(&db).await?;
```

## 🚀 **Key Features**

### **Dual Backend System**
- **Production**: Native Cloudflare D1 integration
- **Development**: Local SQLite with in-memory testing
- **Zero configuration** switching between environments

### **Type-Safe ORM Experience**  
- **Derive macros** for zero boilerplate
- **Compile-time query validation**
- **Automatic CRUD operations**
- **Rich query builder** with method chaining

### **Type-Safe Relations**
- **One-to-one**, **one-to-many**, **many-to-many** relations
- **Zero string literals** - fully type-safe at compile time
- **Association methods** generated automatically (`user.posts().all()`)
- **Junction table** handling and migration auto-generation

### **Schema Evolution**
- **Fluent migration API** with full ALTER TABLE support
- **Type-safe schema definitions**
- **Automatic boolean handling** (SQLite integers ↔ Rust booleans)
- **Foreign key constraints**

## 📦 **Installation**

```toml
[dependencies]
d1-rs = "0.1.0"
serde = { version = "1.0", features = ["derive"] }

# For Cloudflare Workers
[target.'cfg(target_arch = "wasm32")'.dependencies]
worker = { version = "0.4", features = ["d1"] }

# For testing  
[dev-dependencies]
rusqlite = { version = "0.32", features = ["chrono", "bundled"] }
tokio = { version = "1.32", features = ["full"] }
```

## 🏗️ **Schema & Migrations**

```rust
// Define your schema with migrations
let migration = SchemaMigration::new("create_blog".to_string())
    .create_table("users")
        .integer("id").primary_key().auto_increment().build()
        .text("name").not_null().build()
        .text("email").not_null().unique().build()
        .boolean("is_active").default_value(DefaultValue::Boolean(true)).build()
    .build()
    
    .create_table("posts")  
        .integer("id").primary_key().auto_increment().build()
        .integer("user_id").not_null().build()
        .text("title").not_null().build()
        .text("content").not_null().build()
    .build()
    
    // Relations defined separately with type-safe macro
    .build();

migration.execute(&db).await?;
```

## 🔗 **Type-Safe Relations**

```rust
use d1_rs::*;

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct User {
    #[primary_key] pub id: i64,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]  
pub struct Post {
    #[primary_key] pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub content: String,
}

// Define relations with type-safe macro - NO STRING LITERALS!
relations! {
    User {
        has_many posts: Post via user_id,
    }
    
    Post {
        belongs_to user: User via user_id,
    }
}

// Use type-safe association methods
let user_posts = user.posts().all(&db).await?;
let post_count = user.posts().count(&db).await?;
let first_post = user.posts().first(&db).await?;

// Belongs-to relations
let post_author = post.user().first(&db).await?;
```

## 🧪 **Testing**

d1-rs makes testing effortless with automatic SQLite fallback:

```rust
#[tokio::test]
async fn test_user_operations() {
    let db = D1Client::new_in_memory().await.unwrap();
    
    // Your entities work the same in tests!
    let user = User::create()
        .set_name("Test User".to_string())
        .save(&db)
        .await
        .unwrap();
    
    assert_eq!(user.name, "Test User");
}
```

## 🌐 **Cloudflare Workers Setup**

```toml
# wrangler.toml
[[d1_databases]]
binding = "DB" 
database_name = "my-database"

[build]
command = "cargo build --release --target wasm32-unknown-unknown"
```

```rust
// src/main.rs
use worker::*;
use d1_rs::*;

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    let db = D1Client::new(env.d1("DB")?);
    
    let users = User::query().all(&db).await.unwrap();
    Response::from_json(&users)
}
```

## 🎓 **Examples**

### CRUD Operations
```rust
// Create
let user = User::create()
    .set_name("Alice".to_string())
    .set_email("alice@example.com".to_string())
    .save(&db).await?;

// Read
let user = User::find(&db, 1).await?;
let users = User::query()
    .where_is_active_eq(true)
    .all(&db).await?;

// Update  
let updated = User::update(1)
    .set_name("Alice Smith".to_string())
    .save(&db).await?;

// Delete
User::delete(&db, 1).await?;
```

### Advanced Queries
```rust
let results = User::query()
    .where_name_like("John%")
    .where_created_at_gte(yesterday)
    .where_is_active_eq(true)
    .order_by_created_at_desc()
    .limit(50)
    .all(&db)
    .await?;
```

## 📚 **Documentation**

- **[Getting Started Guide](https://your-username.github.io/d1-rs/installation.html)** - Installation and setup
- **[Quick Start Tutorial](https://your-username.github.io/d1-rs/quick-start.html)** - Build your first app
- **[Relations Guide](https://your-username.github.io/d1-rs/relations/introduction.html)** - Graph-based relationships
- **[API Reference](https://docs.rs/d1-rs)** - Complete API documentation

## 🏃‍♂️ **Quick Start**

1. **Install d1-rs**:
   ```bash
   cargo add d1-rs serde tokio
   ```

2. **Create your first entity**:
   ```rust
   #[derive(Entity)]
   pub struct User {
       #[primary_key] pub id: i64,
       pub name: String,
   }
   ```

3. **Run migrations**:
   ```rust
   SchemaMigration::new("users".to_string())
       .create_table("users")
           .integer("id").primary_key().auto_increment().build()
           .text("name").not_null().build()
       .build()
       .execute(&db).await?;
   ```

4. **Use your entity**:
   ```rust
   let user = User::create()
       .set_name("Alice".to_string())
       .save(&db).await?;
   ```

## 🤝 **Contributing**

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## 📜 **License**

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## ⭐ **Star History**

If you find d1-rs useful, please give it a star! ⭐

---

**Built with ❤️ for the Cloudflare Workers ecosystem**