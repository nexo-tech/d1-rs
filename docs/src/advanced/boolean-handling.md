# Boolean Handling

This guide explains how d1-rs handles boolean values, the automatic conversion between Rust's `bool` type and SQLite's integer storage, and best practices for working with boolean fields.

## SQLite Boolean Storage

SQLite doesn't have a native boolean type, so boolean values are stored as integers:
- `true` → `1`
- `false` → `0`

d1-rs handles this conversion automatically, providing a seamless experience where you work with Rust's `bool` type while the database stores integers.

## Automatic Conversion

### Database Schema

When you create a boolean field in your migration:

```rust
let migration = SchemaMigration::new("create_users".to_string())
    .create_table("users")
        .integer("id").primary_key().auto_increment().build()
        .text("name").not_null().build()
        .boolean("is_active").default_value(DefaultValue::Boolean(true)).build()
        .boolean("is_verified").default_value(DefaultValue::Boolean(false)).build()
        .boolean("receive_notifications").build() // No default (NULL allowed)
    .build();
```

This generates SQL like:

```sql
CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    is_active INTEGER DEFAULT 1,
    is_verified INTEGER DEFAULT 0,
    receive_notifications INTEGER
);
```

### Entity Definition

In your Rust entity, use standard `bool` types:

```rust
#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct User {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub is_active: bool,        // Will be stored as INTEGER in database
    pub is_verified: bool,
    pub receive_notifications: Option<bool>, // Optional boolean
}
```

### Conversion Process

d1-rs performs automatic conversion in both directions:

```rust
// When saving to database
let user = User::create()
    .set_name("Alice".to_string())
    .set_is_active(true)      // Becomes 1 in database
    .set_is_verified(false)   // Becomes 0 in database
    .save(&db)
    .await?;

// When loading from database
let loaded_user = User::find(&db, user.id).await?.unwrap();
assert_eq!(loaded_user.is_active, true);    // 1 becomes true
assert_eq!(loaded_user.is_verified, false); // 0 becomes false
```

## Query Operations

### Boolean Filtering

Use natural boolean values in queries:

```rust
// Query for active users
let active_users = User::query()
    .where_is_active_eq(true)    // Automatically converts to WHERE is_active = 1
    .all(&db)
    .await?;

// Query for unverified users
let unverified_users = User::query()
    .where_is_verified_eq(false) // Automatically converts to WHERE is_verified = 0
    .all(&db)
    .await?;

// Query for users who haven't set notification preference
let undecided_users = User::query()
    .where_receive_notifications_is_null()
    .all(&db)
    .await?;
```

### Boolean Updates

Update boolean fields naturally:

```rust
// Toggle user verification
let verified_user = User::update(user.id)
    .set_is_verified(true)       // Will store as 1
    .save(&db)
    .await?;

// Disable notifications
let updated_user = User::update(user.id)
    .set_receive_notifications(Some(false)) // Optional boolean
    .save(&db)
    .await?;
```

## Advanced Boolean Patterns

### Boolean Flags

Use multiple boolean fields for feature flags:

```rust
#[derive(Entity)]
pub struct UserSettings {
    #[primary_key]
    pub id: i64,
    pub user_id: i64,
    
    // Feature flags
    pub dark_mode: bool,
    pub email_notifications: bool,
    pub push_notifications: bool,
    pub marketing_emails: bool,
    pub beta_features: bool,
    
    // Privacy settings
    pub profile_public: bool,
    pub show_activity: bool,
    pub allow_friend_requests: bool,
}

impl UserSettings {
    // Helper methods for common operations
    pub async fn enable_all_notifications(&self, db: &D1Client) -> Result<UserSettings> {
        UserSettings::update(self.id)
            .set_email_notifications(true)
            .set_push_notifications(true)
            .save(db)
            .await
    }
    
    pub async fn privacy_mode(&self, db: &D1Client) -> Result<UserSettings> {
        UserSettings::update(self.id)
            .set_profile_public(false)
            .set_show_activity(false)
            .set_allow_friend_requests(false)
            .save(db)
            .await
    }
    
    // Query methods
    pub async fn users_with_notifications_enabled(db: &D1Client) -> Result<Vec<UserSettings>> {
        UserSettings::query()
            .where_email_notifications_eq(true)
            .where_push_notifications_eq(true)
            .all(db)
            .await
    }
}
```

### State Management

Use booleans for simple state tracking:

```rust
#[derive(Entity)]
pub struct Post {
    #[primary_key]
    pub id: i64,
    pub title: String,
    pub content: String,
    
    // State flags
    pub is_published: bool,
    pub is_featured: bool,
    pub is_archived: bool,
    pub comments_enabled: bool,
    pub is_deleted: bool, // Soft delete flag
}

impl Post {
    // State transition methods
    pub async fn publish(&self, db: &D1Client) -> Result<Post> {
        Post::update(self.id)
            .set_is_published(true)
            .save(db)
            .await
    }
    
    pub async fn archive(&self, db: &D1Client) -> Result<Post> {
        Post::update(self.id)
            .set_is_archived(true)
            .set_is_featured(false) // Remove from featured when archived
            .save(db)
            .await
    }
    
    pub async fn soft_delete(&self, db: &D1Client) -> Result<Post> {
        Post::update(self.id)
            .set_is_deleted(true)
            .set_is_published(false)
            .set_is_featured(false)
            .save(db)
            .await
    }
    
    // Query active posts (not deleted or archived)
    pub async fn active_posts(db: &D1Client) -> Result<Vec<Post>> {
        Post::query()
            .where_is_deleted_eq(false)
            .where_is_archived_eq(false)
            .all(db)
            .await
    }
    
    // Featured published posts
    pub async fn featured_posts(db: &D1Client) -> Result<Vec<Post>> {
        Post::query()
            .where_is_published_eq(true)
            .where_is_featured_eq(true)
            .where_is_deleted_eq(false)
            .all(db)
            .await
    }
}
```

## Working with Optional Booleans

### Nullable Boolean Fields

Sometimes you need three states: true, false, or unknown/unset:

```rust
#[derive(Entity)]
pub struct UserProfile {
    #[primary_key]
    pub id: i64,
    pub user_id: i64,
    
    // Three-state booleans (true/false/null)
    pub wants_newsletter: Option<bool>,    // NULL = hasn't decided
    pub is_looking_for_job: Option<bool>,  // NULL = not specified
    pub available_for_hire: Option<bool>,  // NULL = not set
}

impl UserProfile {
    // Handle three-state logic
    pub async fn users_open_to_newsletter(db: &D1Client) -> Result<Vec<UserProfile>> {
        // Only users who explicitly said yes
        UserProfile::query()
            .where_wants_newsletter_eq(true)
            .all(db)
            .await
    }
    
    pub async fn users_newsletter_undecided(db: &D1Client) -> Result<Vec<UserProfile>> {
        // Users who haven't made a choice
        UserProfile::query()
            .where_wants_newsletter_is_null()
            .all(db)
            .await
    }
    
    pub fn newsletter_preference_string(&self) -> &'static str {
        match self.wants_newsletter {
            Some(true) => "Yes",
            Some(false) => "No", 
            None => "Not specified",
        }
    }
}
```

### Default Values for Optional Booleans

Set sensible defaults in migrations:

```rust
let migration = SchemaMigration::new("user_preferences".to_string())
    .create_table("user_preferences")
        .integer("id").primary_key().auto_increment().build()
        .integer("user_id").not_null().build()
        
        // Required booleans with defaults
        .boolean("email_verified").default_value(DefaultValue::Boolean(false)).build()
        .boolean("profile_complete").default_value(DefaultValue::Boolean(false)).build()
        
        // Optional booleans (can be NULL)
        .boolean("marketing_consent").build()  // No default = NULL allowed
        .boolean("data_sharing_consent").build()
        
        // Boolean with explicit NULL default
        .boolean("newsletter_preference").default_value(DefaultValue::Null).build()
    .build();
```

## Boolean Conversion Edge Cases

### Handling Integer Values

d1-rs treats any non-zero integer as `true`:

```rust
// These are the standard conversions:
// 0 -> false
// 1 -> true
// Any other integer -> true (though this shouldn't happen in normal usage)

#[cfg(test)]
mod boolean_conversion_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_boolean_storage_and_retrieval() {
        let db = D1Client::new_in_memory().await.unwrap();
        setup_test_schema(&db).await.unwrap();
        
        // Test true value
        let user_true = User::create()
            .set_name("True User".to_string())
            .set_is_active(true)
            .save(&db)
            .await
            .unwrap();
        
        let loaded_true = User::find(&db, user_true.id).await.unwrap().unwrap();
        assert_eq!(loaded_true.is_active, true);
        
        // Test false value
        let user_false = User::create()
            .set_name("False User".to_string())
            .set_is_active(false)
            .save(&db)
            .await
            .unwrap();
        
        let loaded_false = User::find(&db, user_false.id).await.unwrap().unwrap();
        assert_eq!(loaded_false.is_active, false);
    }
    
    #[tokio::test]
    async fn test_optional_boolean_handling() {
        let db = D1Client::new_in_memory().await.unwrap();
        // Setup schema with optional boolean...
        
        // Test None/NULL value
        let profile_none = UserProfile::create()
            .set_user_id(1)
            .save(&db)  // wants_newsletter remains None/NULL
            .await
            .unwrap();
        
        assert_eq!(profile_none.wants_newsletter, None);
        
        // Test Some(true) value
        let profile_true = UserProfile::create()
            .set_user_id(2)
            .set_wants_newsletter(Some(true))
            .save(&db)
            .await
            .unwrap();
        
        assert_eq!(profile_true.wants_newsletter, Some(true));
        
        // Test Some(false) value
        let profile_false = UserProfile::create()
            .set_user_id(3)
            .set_wants_newsletter(Some(false))
            .save(&db)
            .await
            .unwrap();
        
        assert_eq!(profile_false.wants_newsletter, Some(false));
    }
}
```

## Performance Considerations

### Indexing Boolean Fields

Boolean fields can be effectively indexed:

```rust
let migration = SchemaMigration::new("boolean_indexes".to_string())
    .alter_table("users")
        // Single boolean indexes
        .add_index("idx_users_active", vec!["is_active"])
        .add_index("idx_users_verified", vec!["is_verified"])
        
        // Composite indexes with booleans
        .add_index("idx_users_active_verified", vec!["is_active", "is_verified"])
        .add_index("idx_users_status_created", vec!["is_active", "created_at"])
    .build();
```

### Efficient Boolean Queries

Structure queries to take advantage of indexes:

```rust
impl User {
    // Efficient: Uses index on is_active
    pub async fn active_users_count(db: &D1Client) -> Result<i64> {
        User::query()
            .where_is_active_eq(true)
            .count(db)
            .await
    }
    
    // Efficient: Uses compound index
    pub async fn active_verified_users(db: &D1Client) -> Result<Vec<User>> {
        User::query()
            .where_is_active_eq(true)
            .where_is_verified_eq(true)
            .order_by_created_at_desc()
            .all(db)
            .await
    }
    
    // Less efficient: Multiple separate conditions
    pub async fn complex_user_filter(db: &D1Client) -> Result<Vec<User>> {
        User::query()
            .where_is_active_eq(true)
            .where_is_verified_eq(true)
            .where_receive_notifications_eq(true)
            .all(db)
            .await
    }
}
```

## Migration Patterns

### Adding Boolean Fields

When adding boolean fields to existing tables:

```rust
let migration = SchemaMigration::new("add_user_preferences".to_string())
    .alter_table("users")
        // Add with sensible default for existing records
        .add_column("email_notifications", ColumnType::Boolean)
            .default_value(DefaultValue::Boolean(true))
            .build()
        
        // Add optional boolean (existing records will have NULL)
        .add_column("marketing_consent", ColumnType::Boolean)
            .build()
    .build();
```

### Boolean Field Evolution

Evolving boolean logic over time:

```rust
// Migration 1: Simple active flag
let migration_v1 = SchemaMigration::new("user_status_v1".to_string())
    .create_table("users")
        .boolean("is_active").default_value(DefaultValue::Boolean(true)).build()
    .build();

// Migration 2: More granular status
let migration_v2 = SchemaMigration::new("user_status_v2".to_string())
    .alter_table("users")
        .add_column("is_suspended", ColumnType::Boolean)
            .default_value(DefaultValue::Boolean(false))
            .build()
        .add_column("is_banned", ColumnType::Boolean)
            .default_value(DefaultValue::Boolean(false))
            .build()
    .build();

// Migration 3: Consolidate into enum (future enhancement)
// This would involve string fields instead of multiple booleans
```

## Best Practices

### Boolean Field Design

1. **Use descriptive names**: `is_active` instead of `active`
2. **Default to safe values**: Usually `false` for permissions, `true` for enabled features
3. **Consider three-state logic**: Use `Option<bool>` when "unknown" is meaningful
4. **Group related booleans**: Consider separate settings tables for complex boolean groups

### Query Optimization

1. **Index frequently queried boolean fields**
2. **Use compound indexes for common boolean combinations**
3. **Structure WHERE clauses to use indexes effectively**
4. **Consider denormalization for complex boolean logic**

### Testing

Always test boolean conversion:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_all_boolean_combinations() {
        let db = D1Client::new_in_memory().await.unwrap();
        
        // Test all combinations of boolean values
        let combinations = [
            (true, true, Some(true)),
            (true, false, Some(false)),
            (false, true, None),
            (false, false, Some(true)),
        ];
        
        for (is_active, is_verified, notifications) in combinations {
            let user = User::create()
                .set_name("Test".to_string())
                .set_is_active(is_active)
                .set_is_verified(is_verified)
                .set_receive_notifications(notifications)
                .save(&db)
                .await
                .unwrap();
            
            // Verify values are preserved
            assert_eq!(user.is_active, is_active);
            assert_eq!(user.is_verified, is_verified);
            assert_eq!(user.receive_notifications, notifications);
        }
    }
}
```

## Next Steps

- Learn about [Performance Optimization](./performance.md) for boolean field indexing strategies
- Explore [Testing Strategies](./testing.md) for comprehensive boolean testing
- Check out [Migrations](../migrations.md) for boolean field evolution patterns