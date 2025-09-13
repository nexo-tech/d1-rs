# API Reference Overview

This section provides detailed API documentation for all d1-rs types, traits, and functions. The API is organized by functionality to help you find what you need quickly.

## Core Components

### Client & Database

- **[D1Client](./client.md)** - Database client abstraction for both Cloudflare D1 and SQLite
- **[Connection Management](./connections.md)** - Establishing and managing database connections

### Entity System

- **[Entity Trait](./entity.md)** - Core trait that all database entities implement
- **[Entity Derive Macro](./derive.md)** - Procedural macro for automatic entity implementation
- **[RelationalEntity Trait](./relational-entity.md)** - Trait for entities that participate in relations

### Query System

- **[Query Builders](./query-builders.md)** - Type-safe query construction
- **[Where Clauses](./where-clauses.md)** - Field-specific filtering methods
- **[Ordering & Pagination](./ordering.md)** - Result ordering and pagination

### Schema & Migrations

- **[Schema Migration](./schema-migration.md)** - Database schema evolution
- **[Column Types](./column-types.md)** - Available column types and constraints
- **[Relation Types](./relation-types.md)** - Relationship definitions

### Relations System

- **[TraversalContext](./traversal.md)** - Graph traversal and cycle prevention
- **[Relation Types](./relations.md)** - One-to-one, one-to-many, many-to-many
- **[Eager Loading](./eager-loading.md)** - Optimized data loading

### Type System

- **[Type Conversions](./type-conversions.md)** - Rust ↔ SQLite type mapping
- **[Boolean Handling](./boolean-conversion.md)** - Automatic boolean conversion
- **[Default Values](./default-values.md)** - Column default value types

### Error Handling

- **[Error Types](./errors.md)** - d1-rs error types and handling
- **[Result Types](./results.md)** - Return types and error propagation

## Quick Reference

### Common Types

```rust
// Core client type
pub struct D1Client { /* ... */ }

// Entity trait (implemented by derive macro)
pub trait Entity {
    type CreateBuilder;
    type UpdateBuilder;
    type QueryBuilder;
    
    fn table_name() -> &'static str;
    fn primary_key_field() -> &'static str;
    fn boolean_fields() -> Vec<&'static str>;
    // ... other methods
}

// Relational entity trait
pub trait RelationalEntity: Entity {
    fn traverse<T: RelationalEntity>(&self, db: &D1Client, relation_name: &str) -> impl Future<Output = Result<Vec<T>>>;
}

// Main error type
pub enum D1RsError {
    Database(String),
    SerializationError(String),
    NotFound,
    InvalidQuery(String),
    // ... other variants
}

pub type Result<T> = std::result::Result<T, D1RsError>;
```

### Builder Patterns

All entities generate three builder types:

```rust
// For entity named "User"
pub struct UserCreateBuilder { /* ... */ }
pub struct UserUpdateBuilder { /* ... */ }
pub struct UserQueryBuilder { /* ... */ }

// Usage patterns
let user = User::create()     // Returns UserCreateBuilder
    .set_name(name)
    .set_email(email)
    .save(&db).await?;

let updated = User::update(id) // Returns UserUpdateBuilder
    .set_name(new_name)
    .save(&db).await?;

let users = User::query()      // Returns UserQueryBuilder
    .where_is_active_eq(true)
    .order_by_name_asc()
    .all(&db).await?;
```

### Schema Definition

```rust
use d1_rs::schema::*;

let migration = SchemaMigration::new("migration_name".to_string())
    .create_table("table_name")
        .integer("id").primary_key().auto_increment().build()
        .text("name").not_null().build()
        .boolean("is_active").default_value(DefaultValue::Boolean(true)).build()
        .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
    .build()
    
    .create_relation("relation_name", "parent_table", "child_table")
        .one_to_many("foreign_key", "references")
    .build();

migration.execute(&db).await?;
```

## API Stability

d1-rs follows semantic versioning:
- **Major versions** (1.x → 2.x): Breaking API changes
- **Minor versions** (1.1 → 1.2): New features, backwards compatible
- **Patch versions** (1.1.1 → 1.1.2): Bug fixes, no API changes

### Current Stability

- ✅ **Stable**: Core entity system, basic CRUD operations
- ⚠️ **Beta**: Relations system, advanced query features
- 🚧 **Experimental**: Performance optimizations, raw SQL support

## Platform Differences

### Cloudflare Workers (WASM)

```rust
// Available when targeting wasm32-unknown-unknown
#[cfg(target_arch = "wasm32")]
impl D1Client {
    pub fn new(database: worker::d1::D1Database) -> Self;
}
```

### Native (SQLite)

```rust
// Available on native targets (for testing)
#[cfg(not(target_arch = "wasm32"))]
impl D1Client {
    pub async fn new_in_memory() -> Result<Self>;
    pub async fn new_file(path: &str) -> Result<Self>;
}
```

## Examples Index

Each API section includes practical examples. For comprehensive examples, see:

- **[Quick Start](../quick-start.md)** - Getting started examples
- **[Entities Guide](../entities.md)** - Entity usage examples  
- **[Queries Guide](../queries.md)** - Query examples
- **[Relations Guide](../relations/introduction.md)** - Relationship examples
- **[Migrations Guide](../migrations.md)** - Schema evolution examples

## Next Steps

Choose the section most relevant to your current needs:

- **New to d1-rs?** Start with [Entity Trait](./entity.md)
- **Working with relationships?** See [Relations API](./relations.md)
- **Building complex queries?** Check [Query Builders](./query-builders.md)
- **Managing schema changes?** Visit [Schema Migration](./schema-migration.md)
- **Handling errors?** Review [Error Types](./errors.md)