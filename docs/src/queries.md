# Queries

d1-rs provides a powerful, type-safe query builder that generates SQL at compile time. Every entity automatically gets rich querying capabilities based on its field types.

## Basic Querying

### Simple Queries

```rust
// Get all users
let users = User::query().all(&db).await?;

// Get first user
let user = User::query().first(&db).await?;

// Count users
let count = User::query().count(&db).await?;
```

### Where Clauses

Every field generates type-appropriate where methods:

```rust
let results = User::query()
    .where_name_eq("Alice")           // Exact match
    .where_age_gte(18)                // Greater than or equal
    .where_is_active_eq(true)         // Boolean comparison
    .where_email_contains("@gmail")   // String contains
    .all(&db)
    .await?;
```

## Field-Specific Methods

### String Fields

```rust
User::query()
    .where_name_eq("Alice")              // name = 'Alice'
    .where_name_ne("Bob")                // name != 'Bob'  
    .where_name_like("Al%")              // name LIKE 'Al%'
    .where_name_contains("lic")          // name LIKE '%lic%'
    .where_name_starts_with("Al")        // name LIKE 'Al%'
    .where_name_ends_with("ice")         // name LIKE '%ice'
    .all(&db).await?;
```

### Numeric Fields

```rust
User::query()
    .where_age_eq(25)                    // age = 25
    .where_age_ne(30)                    // age != 30
    .where_age_gt(18)                    // age > 18
    .where_age_gte(21)                   // age >= 21
    .where_age_lt(65)                    // age < 65  
    .where_age_lte(64)                   // age <= 64
    .all(&db).await?;
```

### Boolean Fields

```rust
User::query()
    .where_is_active_eq(true)            // is_active = 1
    .where_is_active_ne(false)           // is_active != 0
    .all(&db).await?;
```

### Optional/Nullable Fields

```rust
User::query()
    .where_middle_name_is_null()         // middle_name IS NULL
    .where_middle_name_is_not_null()     // middle_name IS NOT NULL
    .all(&db).await?;
```

## Combining Conditions

### Multiple Where Clauses

All where clauses are combined with `AND`:

```rust
let results = User::query()
    .where_is_active_eq(true)           // AND is_active = 1
    .where_age_gte(18)                  // AND age >= 18
    .where_name_contains("john")        // AND name LIKE '%john%'
    .all(&db)
    .await?;

// Generates: WHERE is_active = 1 AND age >= 18 AND name LIKE '%john%'
```

### IN Queries (Limited)

Basic IN support (currently uses first value):

```rust
let user_ids = vec![1, 2, 3, 4, 5];
let users = User::query()
    .where_id_in(user_ids)              // Limited implementation
    .all(&db)
    .await?;
```

> **Note**: Full IN query support is planned for future releases.

## Ordering

Every field generates ordering methods:

```rust
User::query()
    .order_by_name_asc()                // ORDER BY name ASC
    .order_by_created_at_desc()         // ORDER BY created_at DESC
    .all(&db).await?;

// Multiple ordering
User::query()
    .order_by_is_active_desc()          // ORDER BY is_active DESC,
    .order_by_name_asc()                //         name ASC
    .all(&db).await?;
```

### Generic Ordering

Use the generic `order_by` method for dynamic ordering:

```rust
let order_column = "name";
let ascending = true;

User::query()
    .order_by(order_column, if ascending { "ASC" } else { "DESC" })
    .all(&db).await?;
```

## Pagination

### Limit and Offset

```rust
// Get first 10 users
let first_page = User::query()
    .limit(10)
    .all(&db).await?;

// Get next 10 users (skip first 10)
let second_page = User::query()
    .limit(10)
    .offset(10)
    .all(&db).await?;

// Pagination helper function
async fn get_users_page(db: &D1Client, page: i64, per_page: i64) -> Result<Vec<User>> {
    User::query()
        .order_by_id_asc()              // Consistent ordering for pagination
        .limit(per_page)
        .offset(page * per_page)
        .all(db)
        .await
}
```

## Query Results

### Different Result Types

```rust
// All matching records
let users: Vec<User> = User::query()
    .where_is_active_eq(true)
    .all(&db).await?;

// First matching record
let user: Option<User> = User::query()
    .where_email_eq("alice@example.com")
    .first(&db).await?;

// Count of matching records
let count: i64 = User::query()
    .where_is_active_eq(true)
    .count(&db).await?;
```

### Handling Results

```rust
// Handle optional results
match User::query().where_id_eq(42).first(&db).await? {
    Some(user) => println!("Found user: {}", user.name),
    None => println!("User not found"),
}

// Check if any records exist
let has_active_users = User::query()
    .where_is_active_eq(true)
    .count(&db).await? > 0;

// Get first or fail
let user = User::query()
    .where_email_eq("admin@example.com")
    .first(&db).await?
    .ok_or(D1RsError::NotFound)?;
```

## Performance Considerations

### Query Optimization

```rust
// Good - specific query
let active_count = User::query()
    .where_is_active_eq(true)
    .count(&db).await?;

// Less efficient - loads all data
let all_users = User::query().all(&db).await?;
let active_count = all_users.iter().filter(|u| u.is_active).count();
```

### Indexing Strategy

Create indexes for frequently queried fields:

```rust
// In your migration
let migration = SchemaMigration::new("add_indexes".to_string())
    .alter_table("users")
        .add_index("idx_users_email", vec!["email"])
        .add_index("idx_users_active", vec!["is_active"])
        .add_index("idx_users_created", vec!["created_at"])
    .build();
```

### Limit Large Queries

Always use limits for potentially large result sets:

```rust
// Good - limited results
let recent_posts = Post::query()
    .order_by_created_at_desc()
    .limit(100)
    .all(&db).await?;

// Potentially dangerous - could return millions of records
let all_posts = Post::query().all(&db).await?;
```

## Complex Query Patterns

### Search Functionality

```rust
async fn search_users(db: &D1Client, search_term: &str) -> Result<Vec<User>> {
    let term = format!("%{}%", search_term.to_lowercase());
    
    User::query()
        .where_name_like(&term)
        .order_by_name_asc()
        .limit(50)
        .all(db)
        .await
}
```

### Date Range Queries

```rust
use chrono::{DateTime, Utc, Duration};

async fn get_recent_posts(db: &D1Client, days: i64) -> Result<Vec<Post>> {
    let cutoff = Utc::now() - Duration::days(days);
    
    Post::query()
        .where_created_at_gte(cutoff)
        .where_is_published_eq(true)
        .order_by_created_at_desc()
        .all(db)
        .await
}
```

### Active Record Pattern

```rust
impl User {
    // Custom query methods on your entity
    pub async fn find_by_email(db: &D1Client, email: &str) -> Result<Option<User>> {
        User::query()
            .where_email_eq(email)
            .first(db)
            .await
    }
    
    pub async fn active_users(db: &D1Client) -> Result<Vec<User>> {
        User::query()
            .where_is_active_eq(true)
            .order_by_name_asc()
            .all(db)
            .await
    }
    
    pub async fn users_by_domain(db: &D1Client, domain: &str) -> Result<Vec<User>> {
        let pattern = format!("%@{}", domain);
        User::query()
            .where_email_like(&pattern)
            .all(db)
            .await
    }
}

// Usage
let user = User::find_by_email(&db, "alice@example.com").await?;
let active_users = User::active_users(&db).await?;
let gmail_users = User::users_by_domain(&db, "gmail.com").await?;
```

## Error Handling

```rust
use d1_rs::{D1RsError, Result};

async fn safe_user_query(db: &D1Client, user_id: i64) -> Result<User> {
    match User::find(db, user_id).await {
        Ok(Some(user)) => Ok(user),
        Ok(None) => Err(D1RsError::NotFound),
        Err(D1RsError::Database(msg)) => {
            eprintln!("Database error: {}", msg);
            Err(D1RsError::Database(msg))
        }
        Err(other) => Err(other),
    }
}
```

## Raw SQL (Future)

For complex queries not supported by the query builder:

```rust
// This will be available in future versions
let users = db.query_raw::<User>(
    "SELECT * FROM users WHERE LOWER(name) = LOWER(?) AND created_at > ?",
    &[
        serde_json::Value::String("Alice".to_string()),
        serde_json::Value::String(yesterday.to_rfc3339()),
    ]
).await?;
```

## Next Steps

- Learn about [Relations](./relations/introduction.md) for joining data across tables
- Explore [Boolean Handling](./advanced/boolean-handling.md) for understanding boolean conversion
- Check out [Performance Optimization](./advanced/performance.md) for query tuning tips