# Migrations

d1-rs provides a powerful schema migration system that lets you evolve your database schema over time with type-safe operations and full ALTER TABLE support.

## Schema Migrations

### Creating Your First Migration

```rust
use d1_rs::*;

async fn create_users_table(db: &D1Client) -> Result<()> {
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

### Migration Structure

Migrations use a fluent builder API:

```rust
SchemaMigration::new("migration_name".to_string())
    .create_table("table_name")
        // Column definitions
    .build()
    .alter_table("existing_table") 
        // Modifications
    .build()
    .raw_sql("CREATE INDEX ...")
    .execute(&db).await
```

## Column Types

d1-rs supports all standard SQLite column types:

### Basic Types

```rust
.create_table("products")
    .integer("id").primary_key().auto_increment().build()
    .text("name").not_null().build()
    .real("price").not_null().build()
    .boolean("in_stock").default_value(DefaultValue::Boolean(true)).build()
    .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
    .json("metadata").build()
.build()
```

### Column Types Reference

| d1-rs Type | SQLite Type | Rust Type | Description |
|------------|-------------|-----------|-------------|
| `integer()` | `INTEGER` | `i64` | 64-bit signed integer |
| `text()` | `TEXT` | `String` | UTF-8 string |  
| `real()` | `REAL` | `f64` | Floating point number |
| `boolean()` | `INTEGER` | `bool` | Boolean (stored as 0/1) |
| `datetime()` | `DATETIME` | `DateTime<Utc>` | ISO 8601 datetime |
| `json()` | `TEXT` | `serde_json::Value` | JSON data |

## Column Constraints

### Primary Keys

```rust
.integer("id").primary_key().auto_increment().build()
.text("sku").primary_key().build()  // String primary key
```

### Constraints

```rust
.text("email")
    .not_null()                                    // NOT NULL
    .unique()                                      // UNIQUE
    .default_value(DefaultValue::Text("".to_string()))  // DEFAULT
    .build()

.integer("age")
    .check("age >= 0 AND age <= 150")             // CHECK constraint
    .build()
```

### Default Values

```rust
// Type-safe default values
.boolean("is_active").default_value(DefaultValue::Boolean(true)).build()
.integer("count").default_value(DefaultValue::Integer(0)).build()
.text("status").default_value(DefaultValue::Text("pending".to_string())).build()
.datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
.real("score").default_value(DefaultValue::Real(0.0)).build()

// Raw SQL expressions
.datetime("updated_at").default_value(DefaultValue::Expression("CURRENT_TIMESTAMP".to_string())).build()
```

## Foreign Keys

### Basic Foreign Keys

```rust
.create_table("posts")
    .integer("id").primary_key().auto_increment().build()
    .integer("user_id")
        .not_null()
        .references("users", "id")  // Foreign key reference
        .build()
    .text("title").not_null().build()
.build()
```

### Foreign Key Actions

```rust
use d1_rs::schema::ForeignKeyAction;

.integer("user_id")
    .references("users", "id")
    .on_delete(ForeignKeyAction::Cascade)     // ON DELETE CASCADE
    .on_update(ForeignKeyAction::SetNull)     // ON UPDATE SET NULL
    .build()
```

Available actions:
- `Cascade` - Delete/update cascades to child records
- `SetNull` - Set foreign key to NULL
- `SetDefault` - Set foreign key to default value  
- `Restrict` - Prevent the action if child records exist
- `NoAction` - No action (database default)

## Schema Evolution

### Adding Columns

```rust
let migration = SchemaMigration::new("add_user_phone".to_string())
    .alter_table("users")
        .add_column("phone", ColumnType::Text)
            .build()
    .build();

migration.execute(&db).await?;
```

### Adding Indexes

```rust
let migration = SchemaMigration::new("add_indexes".to_string())
    .alter_table("users")
        .add_index("idx_users_email", vec!["email"])
        .add_unique_index("idx_users_username", vec!["username"]) 
        .add_index("idx_users_multi", vec!["is_active", "created_at"])
    .build();
```

### Renaming Columns

```rust
let migration = SchemaMigration::new("rename_column".to_string())
    .alter_table("users")
        .rename_column("old_name", "new_name")
    .build();
```

## Relations in Migrations

Define relationships between tables:

```rust
let migration = SchemaMigration::new("create_blog_schema".to_string())
    // Create tables first
    .create_table("users")
        .integer("id").primary_key().auto_increment().build()
        .text("name").not_null().build()
    .build()
    
    .create_table("posts")
        .integer("id").primary_key().auto_increment().build()
        .integer("user_id").not_null().build()
        .text("title").not_null().build()
    .build()
    
    // Then define relations
    .create_relation("user_posts", "users", "posts")
        .one_to_many("user_id", "id")
    .build();

migration.execute(&db).await?;
```

### Relation Types

```rust
// One-to-One: User has one Profile
.create_relation("user_profile", "users", "profiles")
    .one_to_one("user_id", "id")
.build()

// One-to-Many: User has many Posts  
.create_relation("user_posts", "users", "posts")
    .one_to_many("user_id", "id")
.build()

// Many-to-Many: Posts have many Categories
.create_relation("post_categories", "posts", "categories")
    .many_to_many("post_categories", "post_id", "id", "category_id", "id")
.build()
```

## Advanced Migrations

### Raw SQL

For complex operations not supported by the builder:

```rust
let migration = SchemaMigration::new("complex_operation".to_string())
    .raw_sql("CREATE INDEX IF NOT EXISTS idx_users_fulltext ON users USING fts5(name, email)")
    .raw_sql("CREATE TRIGGER update_modified_time AFTER UPDATE ON users BEGIN UPDATE users SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id; END")
    .execute(&db).await?;
```

### Multiple Operations

Chain multiple operations in a single migration:

```rust
let migration = SchemaMigration::new("blog_setup".to_string())
    .create_table("categories")
        .integer("id").primary_key().auto_increment().build()
        .text("name").not_null().unique().build()
    .build()
    
    .create_table("posts")
        .integer("id").primary_key().auto_increment().build()
        .text("title").not_null().build()
        .text("content").not_null().build()
        .integer("category_id").references("categories", "id").build()
    .build()
    
    .alter_table("posts")
        .add_index("idx_posts_category", vec!["category_id"])
        .add_index("idx_posts_title", vec!["title"])
    .build();

migration.execute(&db).await?;
```

## Migration Management

### Organizing Migrations

Create a migration runner for your application:

```rust
pub struct MigrationRunner;

impl MigrationRunner {
    pub async fn run_all(db: &D1Client) -> Result<()> {
        // Run migrations in order
        Self::create_users_table(db).await?;
        Self::create_posts_table(db).await?;
        Self::add_user_indexes(db).await?;
        Self::create_relations(db).await?;
        Ok(())
    }
    
    async fn create_users_table(db: &D1Client) -> Result<()> {
        let migration = SchemaMigration::new("001_create_users".to_string())
            .create_table("users")
                .integer("id").primary_key().auto_increment().build()
                .text("name").not_null().build()
                .text("email").not_null().unique().build()
                .boolean("is_active").default_value(DefaultValue::Boolean(true)).build()
                .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
            .build();
        
        migration.execute(db).await
    }
    
    async fn create_posts_table(db: &D1Client) -> Result<()> {
        let migration = SchemaMigration::new("002_create_posts".to_string())
            .create_table("posts")
                .integer("id").primary_key().auto_increment().build()
                .integer("user_id").not_null().references("users", "id").build()
                .text("title").not_null().build()
                .text("content").not_null().build()
                .boolean("is_published").default_value(DefaultValue::Boolean(false)).build()
                .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
            .build();
        
        migration.execute(db).await
    }
    
    // ... more migrations
}

// Usage
#[tokio::main]
async fn main() -> Result<()> {
    let db = D1Client::new_in_memory().await?;
    MigrationRunner::run_all(&db).await?;
    Ok(())
}
```

### Environment-Specific Migrations

```rust
pub async fn setup_database(db: &D1Client, env: Environment) -> Result<()> {
    // Always run core migrations
    MigrationRunner::run_core_migrations(db).await?;
    
    match env {
        Environment::Test => {
            MigrationRunner::run_test_data(db).await?;
        }
        Environment::Development => {
            MigrationRunner::run_dev_data(db).await?;
        }
        Environment::Production => {
            // Production-only migrations
        }
    }
    
    Ok(())
}
```

## Testing Migrations

### Migration Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_user_migration() {
        let db = D1Client::new_in_memory().await.unwrap();
        
        // Run migration
        MigrationRunner::create_users_table(&db).await.unwrap();
        
        // Test that table was created correctly
        let user = User::create()
            .set_name("Test User".to_string())
            .set_email("test@example.com".to_string())
            .save(&db)
            .await
            .unwrap();
        
        assert_eq!(user.name, "Test User");
        assert!(user.is_active); // Should default to true
    }
    
    #[tokio::test]
    async fn test_full_schema() {
        let db = D1Client::new_in_memory().await.unwrap();
        
        // Run all migrations
        MigrationRunner::run_all(&db).await.unwrap();
        
        // Test that all entities work
        let user = User::create()
            .set_name("Author".to_string())
            .set_email("author@example.com".to_string())
            .save(&db)
            .await
            .unwrap();
        
        let post = Post::create()
            .set_user_id(user.id)
            .set_title("Test Post".to_string())
            .set_content("Content here".to_string())
            .save(&db)
            .await
            .unwrap();
        
        assert_eq!(post.user_id, user.id);
    }
}
```

## Best Practices

### Migration Naming

Use descriptive, sequential names:

```rust
"001_create_users_table"
"002_create_posts_table"
"003_add_user_phone_column"
"004_create_categories_table"
"005_add_post_category_relation"
```

### Safe Schema Changes

Always use safe operations in production:

```rust
// Safe - adds optional column
.alter_table("users")
    .add_column("phone", ColumnType::Text)  // NULL allowed
    .build()

// Potentially unsafe - adds required column to existing table
.alter_table("users") 
    .add_column("required_field", ColumnType::Text)
        .not_null()  // This could fail if table has existing rows
        .build()
```

### Rollback Strategy

Plan for rollbacks by keeping operations reversible when possible:

```rust
// Instead of dropping columns, consider marking as unused
.alter_table("users")
    .add_column("old_column_unused", ColumnType::Boolean)
        .default_value(DefaultValue::Boolean(false))
        .build()

// Rather than renaming, add new column and migrate data gradually
```

## Type-Safe Migrations

d1-rs also provides a revolutionary type-safe migration system that eliminates string literals and provides compile-time validation.

### TypeSafeMigration Trait

The `TypeSafeMigration` trait allows you to define migrations using your entity types directly:

```rust
use d1_rs::*;

struct CreateUsersTable;

impl TypeSafeMigration for CreateUsersTable {
    async fn up(&self, db: &D1Client) -> Result<()> {
        // Use entity-based type-safe migrations
        let migration = TypeSafeMigrationBuilder::new()
            .create_table_for_entity::<User>()
            .execute(db).await?;
        Ok(())
    }
    
    async fn down(&self, db: &D1Client) -> Result<()> {
        // Type-safe rollback
        let migration = TypeSafeMigrationBuilder::new()
            .drop_table_for_entity::<User>()
            .execute(db).await?;
        Ok(())
    }
}
```

### Type-Safe Column Definitions

Define columns using compile-time safe types:

```rust
use d1_rs::*;

// Define type-safe columns based on entity fields
struct UserNameColumn;
impl TypeSafeColumn for UserNameColumn {
    type RustType = String;
    fn column_name() -> &'static str { "name" }
}

// Use in migrations
let migration = TypeSafeMigrationBuilder::new()
    .create_table("users")
        .add_typed_column::<UserNameColumn>()
            .not_null()
            .build()
    .build();
```

### Entity-Based Migration Generation

Generate migrations automatically from entity definitions:

```rust
#[derive(Entity)]
pub struct User {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub email: String,
    pub is_active: bool,
}

// Automatically generate CREATE TABLE migration
let migration = TypeSafeMigrationBuilder::from_entity::<User>()
    .build();
```

### Benefits of Type-Safe Migrations

1. **Compile-Time Validation**: All column names and types validated at compile time
2. **No String Literals**: Eliminate typos and runtime errors in column names
3. **IDE Support**: Full auto-completion for column names and operations
4. **Type Safety**: Column types automatically inferred from Rust types
5. **Refactoring Safe**: Renaming entity fields automatically updates migrations

### Using Type-Safe Migrations

```rust
use d1_rs::*;

pub struct MigrationRunner;

impl MigrationRunner {
    pub async fn run_type_safe_migrations(db: &D1Client) -> Result<()> {
        // Create migration instances
        let create_users = CreateUsersTable;
        let create_posts = CreatePostsTable;
        
        // Execute migrations
        create_users.up(db).await?;
        create_posts.up(db).await?;
        
        Ok(())
    }
}
```

## Next Steps

- Learn about [Auto Migration](./auto-migration.md) for automatic schema evolution
- Explore [Schema Evolution](./schema-evolution.md) for advanced schema management
- Check out [Relations](./relations/introduction.md) for connecting tables with foreign keys
- Review [Testing](./advanced/testing.md) for comprehensive migration testing strategies