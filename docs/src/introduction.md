# Introduction

Welcome to **d1-rs**, a **type-safe ORM** for Cloudflare D1 that provides compile-time safety and performance optimizations for Rust applications.

## What is d1-rs?

d1-rs is an Object-Relational Mapping (ORM) library designed specifically for Cloudflare D1, with first-class support for local SQLite testing. It provides type-safe database operations, automatic query generation, and compile-time validation of entity relationships.

## 🚀 **Key Features**

### 🛡️ **Type Safety**
- **Compile-time validation**: Field and relation names validated at compile-time
- **No string literals**: Type-safe query methods for all entity fields
- **Automatic CRUD generation**: Create, read, update, delete operations generated automatically
- **Type-safe query builders**: Fluent API with method chaining

### 🔗 **Relationships**
- **Type-safe relations**: Define relationships using the `relations!` macro
- **Automatic query generation**: Association methods generated for related entities
- **Foreign key validation**: Compile-time validation of foreign key relationships
- **Recursive relationships**: Support for self-referential entity relationships

### ⚡ **Performance**
- **Optimal SQL generation**: Efficient database queries using proper SQL operations
- **Memory efficient**: COUNT(*) and LIMIT 1 queries instead of loading unnecessary data
- **Zero-cost abstractions**: Compile-time optimizations with no runtime overhead

### ⚡ **Dual Backend System**
- **Production**: Seamless Cloudflare D1 integration
- **Development**: Local SQLite with in-memory testing
- Zero configuration switching between environments

### 🎯 **Superior Performance**
- **MEMORY EFFICIENT**: SQL COUNT(*) and LIMIT 1 queries, never load unnecessary data
- **OPTIMAL SQL GENERATION**: Database operations, not in-memory processing
- **ZERO WASTE**: Every query optimized for the specific operation

## Why Choose d1-rs?

### **Perfect for Cloudflare Workers**
Built from the ground up for Cloudflare's edge computing platform with D1 database integration.

### **Testing Made Easy**
Write comprehensive tests using local SQLite without needing cloud database access.

### **Performance Focused**
Conditional compilation ensures only necessary code is included for your target platform.

### **Type-Safe Relations**
Advanced graph traversal system with eager loading and cycle prevention.

## 🔥 Revolutionary Quick Example

Experience the world's most advanced ORM capabilities:

```rust
use d1_rs::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Entity)]
pub struct User {
    #[primary_key] pub id: i64,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize, Entity)]
pub struct Post {
    #[primary_key] pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub is_published: bool,
}

#[derive(Debug, Serialize, Deserialize, Entity)]
pub struct Category {
    #[primary_key] pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>, // ✅ Recursive relationships!
}

// ✅ REVOLUTIONARY: Zero string literals, compile-time safe relationships
relations! {
    User { has_many posts: Post via user_id }
    Post { 
        belongs_to user: User via user_id,
        has_many post_categories: PostCategory via post_id,
    }
    Category {
        belongs_to parent: Category via parent_id,    // ✅ Recursive!
        has_many children: Category via parent_id,    // ✅ Self-referential!
        has_many post_categories: PostCategory via category_id,
    }
    PostCategory {  // ✅ Rich M2M junction entity!
        belongs_to post: Post via post_id,
        belongs_to category: Category via category_id,
    }
}

// Type-safe query operations
let users = User::query()
    .where_is_active_eq(true)
    .all(&db).await?;

// Get related data using association methods
for user in &users {
    let user_posts = user.posts().all(&db).await?;
}

// 🚀 REVOLUTIONARY: Type-safe recursive relationships
let category_hierarchy = root_category.children().all(&db).await?;
let parent = child_category.parent().first(&db).await?;

// 🔥 UNPRECEDENTED: Rich M2M with junction entities
let junction_records = PostCategory::query()
    .where_assigned_by_eq(admin_user.name)  // ✅ Type-safe field access!
    .where_is_primary_eq(true)              // ✅ Boolean type, not string!
    .all(&db).await?;

// Navigate through junction entities - IMPOSSIBLE in other ORMs!
let category_from_junction = junction.category().first(&db).await?;
```

## 🏗️ Revolutionary Architecture

d1-rs delivers world-first compile-time safe relationships through advanced architecture:

```
┌──────────────────────────────────────┐
│        🏆 WORLD'S MOST ADVANCED      │
│         COMPILE-TIME VALIDATION      │
│                                      │
│ ✅ Nested Eager Loading Validation   │
│ ✅ Recursive Relationship Analysis   │  
│ ✅ Junction Entity Type Checking     │
│ ✅ Zero String Literal Enforcement   │
└──────────────────┬───────────────────┘
                   │
┌─────────────────┐│   ┌──────────────────┐
│   Your Code     ││   │  🚀 d1-rs ORM    │
│                 ││───│  REVOLUTIONARY   │
│ Type-Safe Rust  ││   │ ● Entity System  │
│ Zero Strings!   ││   │ ● Query Builders │
└─────────────────┘│   │ ● Relations API  │
                   │   └──────────────────┘
                   │            │
                   │   ┌────────┴────────┐
                   │   │                 │
                   │┌──▼─────────────┐ ┌─▼──────┐
                   ││ Cloudflare D1  │ │ SQLite │
                   ││ (Production)   │ │(Testing)│
                   │└────────────────┘ └────────┘
                   │
        ┌──────────▼────────────┐
        │ 🧠 COMPILE-TIME MAGIC │
        │                       │
        │ ● All errors caught   │
        │   before runtime!     │
        │ ● Perfect IDE support │
        │ ● Zero runtime cost   │
        └───────────────────────┘
```

## Why Choose d1-rs?

### **Perfect for Cloudflare Workers**
d1-rs is specifically designed for Cloudflare's edge computing platform with D1 database integration, providing seamless deployment and optimal performance.

### **Type-Safe Development**
Leverage Rust's type system to catch database-related errors at compile-time rather than runtime, improving reliability and developer experience.

### **Testing Made Easy**
Write comprehensive tests using local SQLite without needing cloud database access, enabling fast development cycles and CI/CD integration.

## Getting Started

Jump into the [Installation](./installation.md) guide to set up d1-rs in your project, or check out the [Quick Start](./quick-start.md) for a hands-on tutorial!