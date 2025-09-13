# Entities

Entities are the heart of d1-rs - they represent your database tables as Rust structs with automatically generated type-safe operations.

## Entity Basics

An entity is defined using the `#[derive(Entity)]` macro:

```rust
use d1_rs::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct User {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub email: String,
    pub is_active: bool,
}
```

## Generated Builders

When you derive `Entity`, d1-rs automatically generates three builder types:

### QueryBuilder
For reading data with type-safe where clauses:

```rust
let active_users = User::query()
    .where_is_active_eq(true)
    .where_name_starts_with("A")
    .order_by_created_at_desc()
    .limit(10)
    .all(&db)
    .await?;
```

### CreateBuilder  
For inserting new records:

```rust
let user = User::create()
    .set_name("Alice Smith".to_string())
    .set_email("alice@example.com".to_string())
    .set_is_active(true)
    .save(&db)
    .await?;
```

### UpdateBuilder
For modifying existing records:

```rust
let updated_user = User::update(user_id)
    .set_name("Alice Johnson".to_string())
    .set_is_active(false)
    .save(&db)
    .await?;
```

## Field Types & Query Methods

d1-rs generates different query methods based on your field types:

### String Fields

```rust
pub name: String,
```

Generates methods:
- `.where_name_eq("exact")` - Exact match
- `.where_name_ne("not_this")` - Not equal  
- `.where_name_like("pattern%")` - SQL LIKE
- `.where_name_contains("substring")` - Contains text
- `.where_name_starts_with("prefix")` - Starts with
- `.where_name_ends_with("suffix")` - Ends with

### Numeric Fields

```rust
pub age: i32,
pub score: f64,
```

Generates methods:
- `.where_age_eq(25)` - Exact match
- `.where_age_gt(18)` - Greater than
- `.where_age_gte(21)` - Greater than or equal
- `.where_age_lt(65)` - Less than
- `.where_age_lte(30)` - Less than or equal

### Boolean Fields

```rust
pub is_active: bool,
```

Generates methods:
- `.where_is_active_eq(true)` - Boolean comparison
- `.where_is_active_ne(false)` - Boolean not equal

> **Important**: Boolean fields are automatically converted between Rust's `bool` and SQLite's `INTEGER` (0/1) storage.

### Optional Fields

```rust
pub middle_name: Option<String>,
```

Generates additional methods:
- `.where_middle_name_is_null()` - Field is NULL
- `.where_middle_name_is_not_null()` - Field is not NULL

## Ordering and Pagination

Every field generates ordering methods:

```rust
User::query()
    .order_by_name_asc()        // Ascending order
    .order_by_created_at_desc() // Descending order
    .limit(50)                  // Limit results
    .offset(100)                // Skip results
    .all(&db)
    .await?;
```

## Custom Table Names

By default, d1-rs converts struct names to snake_case and pluralizes them:

- `User` → `users`
- `BlogPost` → `blog_posts`  
- `Category` → `categories` (correct pluralization)

Override with the `table` attribute:

```rust
#[derive(Entity)]
#[table(name = "people")]
pub struct User {
    // Uses "people" table instead of "users"
}
```

## Primary Keys

### Default Primary Key
If no `#[primary_key]` attribute is found, d1-rs looks for an `id` field:

```rust
#[derive(Entity)]
pub struct User {
    pub id: i64,  // Automatically treated as primary key
    pub name: String,
}
```

### Custom Primary Key
Use `#[primary_key]` to specify a different field:

```rust
#[derive(Entity)]
pub struct Product {
    #[primary_key]
    pub sku: String,  // String primary key
    pub name: String,
    pub price: f64,
}
```

### Composite Primary Keys
Currently not supported. Use a single primary key field.

## CRUD Operations

Every entity gets these methods for free:

### Find by Primary Key

```rust
// Returns Option<User>
let user = User::find(&db, 42).await?;

match user {
    Some(u) => println!("Found user: {}", u.name),
    None => println!("User not found"),
}
```

### Delete by Primary Key

```rust
// Deletes the user with id = 42
User::delete(&db, 42).await?;
```

### Exists Check

```rust
// Check if a user exists
let exists = User::query()
    .where_id_eq(42)
    .count(&db)
    .await? > 0;
```

## Advanced Patterns

### Timestamps

Common pattern for tracking creation and modification:

```rust
use chrono::{DateTime, Utc};

#[derive(Entity)]
pub struct Post {
    #[primary_key]
    pub id: i64,
    pub title: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

// Usage
let post = Post::create()
    .set_title("My Post".to_string())
    .set_content("Content here".to_string())
    .set_created_at(Utc::now())
    .save(&db)
    .await?;
```

### Soft Deletes

Implement soft deletes with a boolean field:

```rust
#[derive(Entity)]
pub struct User {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub is_deleted: bool,
}

// Instead of User::delete(), update the flag
let soft_deleted = User::update(user_id)
    .set_is_deleted(true)
    .save(&db)
    .await?;

// Query only non-deleted users
let active_users = User::query()
    .where_is_deleted_eq(false)
    .all(&db)
    .await?;
```

### Enums as Strings

Store enums as their string representation:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum UserRole {
    Admin,
    User,
    Guest,
}

#[derive(Entity)]
pub struct User {
    #[primary_key]
    pub id: i64,
    pub name: String,
    // This will be stored as "Admin", "User", or "Guest" in the database
    pub role: UserRole,
}
```

## Performance Tips

### Indexing
Create database indexes for fields you query frequently:

```rust
// In your migration
let migration = SchemaMigration::new("add_indexes".to_string())
    .alter_table("users")
        .add_index("idx_users_email", vec!["email"])
        .add_unique_index("idx_users_username", vec!["username"])
    .build();
```

### Batch Operations
For bulk inserts, use multiple `create()` calls in a transaction (when transactions are implemented):

```rust
// Current approach - individual inserts
for user_data in bulk_data {
    User::create()
        .set_name(user_data.name)
        .set_email(user_data.email)
        .save(&db)
        .await?;
}
```

### Query Optimization
Use specific queries instead of loading all data:

```rust
// Good - only load what you need
let count = User::query()
    .where_is_active_eq(true)
    .count(&db)
    .await?;

// Less efficient - loads all data
let users = User::query().all(&db).await?;
let active_count = users.iter().filter(|u| u.is_active).count();
```

## Error Handling

Entity operations can fail in several ways:

```rust
use d1_rs::{D1RsError, Result};

async fn handle_user_creation(db: &D1Client) -> Result<()> {
    let result = User::create()
        .set_email("invalid-email")  // This might cause validation errors
        .save(db)
        .await;
    
    match result {
        Ok(user) => {
            println!("User created: {}", user.id);
            Ok(())
        }
        Err(D1RsError::Database(msg)) => {
            eprintln!("Database error: {}", msg);
            Err(D1RsError::Database(msg))
        }
        Err(D1RsError::SerializationError(msg)) => {
            eprintln!("Serialization error: {}", msg);
            Err(D1RsError::SerializationError(msg))
        }
        Err(other) => Err(other),
    }
}
```

## Next Steps

- Learn about [Queries](./queries.md) for advanced querying techniques
- Explore [Relations](./relations/introduction.md) to connect entities together
- Check out [Boolean Handling](./advanced/boolean-handling.md) for deep understanding of boolean conversion