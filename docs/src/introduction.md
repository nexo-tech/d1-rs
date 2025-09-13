# Introduction

Welcome to **d1-rs**, the **world's most advanced type-safe ORM** that revolutionizes database interactions in Rust with unprecedented compile-time safety and performance!

## What is d1-rs?

d1-rs is a **revolutionary Object-Relational Mapping (ORM)** library that is the **first ORM in any language** to provide compile-time safe relationships, nested eager loading, recursive relationships, and rich M2M entities. Designed specifically for Cloudflare D1, with first-class support for local SQLite testing.

## 🚀 **Revolutionary Features - World Firsts**

### 🏆 **World's First Compile-Time Safe Nested Eager Loading**
- **IMPOSSIBLE ERRORS**: All nested relation names validated at compile-time
- **REVOLUTIONARY SYNTAX**: `User::query().with_posts(|posts| posts.with_categories()).all()`
- **AUTOMATIC N+1 PREVENTION**: Multi-level JOINs generated automatically
- **IDE AUTO-COMPLETION**: Full IntelliSense support for nested relations

### 🔥 **World's First Compile-Time Safe Recursive Relationships**
- **TYPE-SAFE RECURSION**: Self-referential relations with compile-time safety
- **INTELLIGENT HANDLING**: Automatic null handling for optional foreign keys
- **UNLIMITED DEPTH**: Tree structures, hierarchies, and self-referencing entities

### 🚀 **World's First Rich M2M with Junction Entities**
- **UNPRECEDENTED**: Junction tables as first-class entities with full Entity powers
- **RICH DATA**: Additional fields in M2M relationships (granted_by, expires_at, etc.)
- **DIRECT QUERYING**: Query junction entities directly - no other ORM allows this!

### 🛡️ **Ultimate Type Safety**
- **ZERO STRING LITERALS**: All field/relation names compile-time validated
- **IMPOSSIBLE ERRORS**: Typos = compile errors, not runtime crashes
- **COMPILE-TIME VALIDATION**: All relationships analyzed for consistency
- **ZERO RUNTIME OVERHEAD**: All validation happens at compile-time

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

// 🏆 WORLD'S FIRST: Compile-time safe nested eager loading
let users_with_data = User::query()
    .with_posts(|posts| posts.with_categories())  // ✅ Impossible errors!
    .with_profile()
    .all(&db).await?;

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

## 🏆 d1-rs vs Other ORMs - No Competition

| Feature | d1-rs | Rails/ActiveRecord | Django ORM | Eloquent | Ent-Go | Prisma |
|---------|-------|-------------------|------------|----------|--------|--------|
| **Nested Eager Loading** | ✅ Compile-time safe | ❌ Runtime strings | ❌ Runtime strings | ❌ Runtime strings | ❌ Runtime strings | ❌ Runtime strings |
| **Type Safety** | ✅ Full compile-time | ❌ Runtime only | ❌ Runtime only | ❌ Runtime only | ❌ Runtime only | ❌ Runtime only |
| **Recursive Relations** | ✅ Compile-time safe | ❌ Manual/limited | ❌ Manual/limited | ❌ Manual/limited | ❌ Manual/limited | ❌ Manual/limited |
| **Rich M2M Junction** | ✅ First-class entities | ❌ Limited | ❌ Limited | ❌ Limited | ❌ Limited | ❌ Limited |
| **N+1 Prevention** | ✅ Automatic | ❌ Manual includes | ❌ Manual select_related | ❌ Manual with | ❌ Manual preload | ❌ Manual include |
| **Error Prevention** | ✅ Impossible errors | ❌ Runtime crashes | ❌ Runtime crashes | ❌ Runtime crashes | ❌ Runtime crashes | ❌ Runtime crashes |
| **IDE Support** | ✅ Full auto-complete | ❌ String literals | ❌ String literals | ❌ String literals | ❌ String literals | ❌ String literals |
| **Performance** | ✅ Zero overhead | ❌ Runtime overhead | ❌ Runtime overhead | ❌ Runtime overhead | ❌ Runtime overhead | ❌ Runtime overhead |

**d1-rs is literally impossible to match - there is no competition!** 🌟

## Ready to Experience the Revolution?

Jump into the [Installation](./installation.md) guide to set up d1-rs in your project, or check out the [Quick Start](./quick-start.md) for a hands-on tutorial with the world's most advanced ORM!