# Your First Entity

In this guide, we'll create your first d1-rs entity and understand how the derive macro works its magic to generate type-safe database operations.

## What is an Entity?

An entity in d1-rs represents a database table as a Rust struct. The `#[derive(Entity)]` macro automatically generates:

- Query builders with type-safe methods
- Create and update builders  
- CRUD operations (Create, Read, Update, Delete)
- Boolean field handling
- Primary key management

## Creating a Simple Entity

Let's start with a basic `User` entity:

```rust
use d1_rs::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct User {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub email: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}
```

## Understanding the Attributes

### Required Derives

```rust
#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
```

- `Entity`: The d1-rs macro that generates database operations
- `Serialize, Deserialize`: For JSON conversion (serde)
- `Debug, Clone, PartialEq`: Standard Rust traits for debugging and comparisons

### Field Annotations

```rust
#[primary_key]
pub id: i64,
```

The `#[primary_key]` attribute marks the primary key field. If no attribute is specified, d1-rs looks for an `id` field by default.

### Custom Table Names

By default, d1-rs converts your struct name to snake_case and pluralizes it:
- `User` → `users`
- `BlogPost` → `blog_posts`

To specify a custom table name:

```rust
#[derive(Entity)]
#[table(name = "custom_users")]
pub struct User {
    // ...
}
```

## Generated Functionality

When you derive `Entity`, d1-rs automatically generates several builders and methods:

### Query Builder

```rust
// Generated automatically:
let users = User::query()
    .where_is_active_eq(true)           // Type-safe where clause
    .where_name_contains("Alice")       // String-specific methods
    .order_by_created_at_desc()         // Ordering
    .limit(10)                          // Pagination
    .all(&db)                           // Execute query
    .await?;
```

### Create Builder

```rust
// Generated automatically:
let user = User::create()
    .set_name("Alice Smith".to_string())
    .set_email("alice@example.com".to_string())
    .set_is_active(true)
    .set_created_at(Utc::now())
    .save(&db)                          // Save to database
    .await?;
```

### Update Builder

```rust
// Generated automatically:
let updated_user = User::update(user_id)
    .set_name("Alice Johnson".to_string())
    .set_is_active(false)
    .save(&db)
    .await?;
```

### CRUD Methods

```rust
// Find by primary key
let user = User::find(&db, 42).await?;

// Delete by primary key  
User::delete(&db, 42).await?;
```

## Field Types and Methods

d1-rs generates different query methods based on your field types:

### String Fields

For `String` fields, you get:

```rust
.where_name_eq("Alice")           // Exact match
.where_name_ne("Bob")             // Not equal
.where_name_like("Al%")           // SQL LIKE
.where_name_contains("lic")       // Contains substring
.where_name_starts_with("Al")     // Starts with
.where_name_ends_with("ice")      // Ends with
```

### Numeric Fields

For `i64`, `i32`, `f64` etc., you get:

```rust
.where_id_eq(42)                  // Exact match
.where_id_gt(10)                  // Greater than
.where_id_gte(10)                 // Greater than or equal
.where_id_lt(100)                 // Less than
.where_id_lte(100)                // Less than or equal
```

### Boolean Fields

For `bool` fields:

```rust
.where_is_active_eq(true)         // Boolean comparison
.where_is_active_ne(false)        // Boolean not equal
```

> **Note**: d1-rs automatically handles boolean conversion between Rust's `bool` and SQLite's `INTEGER` (0/1) storage.

## Creating the Database Table

Before using your entity, you need to create the corresponding database table:

```rust
use d1_rs::*;

async fn setup_database(db: &D1Client) -> Result<()> {
    let migration = SchemaMigration::new("create_users".to_string())
        .create_table("users")
            .integer("id").primary_key().auto_increment().build()
            .text("name").not_null().build()
            .text("email").not_null().unique().build()
            .boolean("is_active").not_null().default_value(DefaultValue::Boolean(true)).build()
            .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
        .build();

    migration.execute(db).await
}
```

## Complete Example

Here's a complete working example:

```rust
use d1_rs::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct User {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub email: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

async fn example_usage() -> Result<()> {
    // Setup database (in-memory for testing)
    let db = D1Client::new_in_memory().await?;
    
    // Create table
    let migration = SchemaMigration::new("create_users".to_string())
        .create_table("users")
            .integer("id").primary_key().auto_increment().build()
            .text("name").not_null().build()
            .text("email").not_null().unique().build()
            .boolean("is_active").not_null().default_value(DefaultValue::Boolean(true)).build()
            .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
        .build();
    
    migration.execute(&db).await?;
    
    // Create a user
    let user = User::create()
        .set_name("Alice Smith".to_string())
        .set_email("alice@example.com".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await?;
    
    println!("Created user: {:?}", user);
    
    // Query users
    let active_users = User::query()
        .where_is_active_eq(true)
        .order_by_name_asc()
        .all(&db)
        .await?;
    
    println!("Active users: {}", active_users.len());
    
    // Update user
    let updated_user = User::update(user.id)
        .set_name("Alice Johnson".to_string())
        .save(&db)
        .await?;
    
    println!("Updated user: {:?}", updated_user);
    
    // Find by ID
    let found_user = User::find(&db, user.id).await?;
    println!("Found user: {:?}", found_user);
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_user_entity() {
        example_usage().await.unwrap();
    }
}
```

## Common Patterns

### Optional Fields

Use `Option<T>` for nullable database columns:

```rust
#[derive(Entity)]
pub struct User {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub bio: Option<String>,  // Can be NULL in database
}
```

### Custom Primary Keys

You can use different primary key types:

```rust
#[derive(Entity)]
pub struct Product {
    #[primary_key]
    pub sku: String,  // String primary key
    pub name: String,
    pub price: f64,
}
```

### Timestamps

Common pattern for tracking creation and updates:

```rust
#[derive(Entity)]
pub struct Post {
    #[primary_key]
    pub id: i64,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}
```

## Next Steps

Now that you understand entities, explore these advanced topics:

- [Queries](./queries.md) - Advanced querying techniques
- [Relations](./relations/introduction.md) - Connecting entities together  
- [Migrations](./migrations.md) - Managing database schema changes
- [Boolean Handling](./advanced/boolean-handling.md) - Deep dive into boolean field conversion