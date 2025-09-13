# Introduction

Welcome to **d1-rs**, the modern, type-safe ORM for Cloudflare D1 that makes database interactions in Rust a joy!

## What is d1-rs?

d1-rs is a next-generation Object-Relational Mapping (ORM) library designed specifically for Cloudflare D1, with first-class support for local SQLite testing. It brings the power of Rust's type system to database operations while maintaining excellent performance and developer experience.

## ✨ Key Features

### 🛡️ **Type Safety First**
- Compile-time query validation
- Automatic type conversion
- Zero runtime type errors
- Full support for Rust's ownership model

### ⚡ **Dual Backend System**
- **Production**: Seamless Cloudflare D1 integration
- **Development**: Local SQLite with in-memory testing
- Zero configuration switching between environments

### 🎯 **Modern ORM Experience**
- Fluent query builder API
- Automatic CRUD operations  
- Graph-based relations inspired by ent-go
- Schema evolution with migrations
- Boolean field handling (SQLite integers ↔ Rust booleans)

### 🚀 **Developer Experience**
- Derive macros for zero boilerplate
- Rich query methods generated automatically
- Comprehensive error handling
- Extensive test coverage

## Why Choose d1-rs?

### **Perfect for Cloudflare Workers**
Built from the ground up for Cloudflare's edge computing platform with D1 database integration.

### **Testing Made Easy**
Write comprehensive tests using local SQLite without needing cloud database access.

### **Performance Focused**
Conditional compilation ensures only necessary code is included for your target platform.

### **Type-Safe Relations**
Advanced graph traversal system with eager loading and cycle prevention.

## Quick Example

Here's a taste of what working with d1-rs looks like:

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

// Usage
let user = User::create()
    .set_name("Alice".to_string())
    .set_email("alice@example.com".to_string())  
    .set_is_active(true)
    .save(&db)
    .await?;

let users = User::query()
    .where_is_active_eq(true)
    .where_name_contains("Alice")
    .order_by_name_asc()
    .limit(10)
    .all(&db)
    .await?;
```

## Architecture Overview

d1-rs uses a dual-backend architecture:

```
┌─────────────────┐    ┌──────────────────┐
│   Your Code     │    │    d1-rs ORM     │
│                 │────│                  │
│ Rust + Macros   │    │ Entity + Query   │
└─────────────────┘    │    Builders      │
                       └──────────────────┘
                                │
                       ┌────────┴────────┐
                       │                 │
            ┌──────────▼─────────┐ ┌─────▼──────┐
            │ Cloudflare D1      │ │   SQLite   │
            │ (Production)       │ │ (Testing)  │
            └────────────────────┘ └────────────┘
```

## Ready to Get Started?

Jump into the [Installation](./installation.md) guide to set up d1-rs in your project, or check out the [Quick Start](./quick-start.md) for a hands-on tutorial!