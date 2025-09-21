# Auto Migration System

The auto migration system in d1-rs provides automatic schema generation and evolution based on your entity definitions. This powerful feature analyzes your Rust entities and automatically creates or updates your database schema to match.

## AutoSchemaClient

The `AutoSchemaClient` is the core component that provides automatic migration capabilities:

```rust
use d1_rs::*;

// Create an auto schema client
let auto_client = AutoSchemaClient::new(db);

// Register your entities
auto_client.register_entity::<User>()?;
auto_client.register_entity::<Post>()?;
auto_client.register_entity::<Category>()?;

// Automatically migrate schema
let result = auto_client.auto_migrate().await?;
```

## Basic Usage

### Entity Registration

Before running migrations, register all your entity types:

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

#[derive(Debug, Serialize, Deserialize, Entity)]
pub struct Post {
    #[primary_key]
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub content: String,
    pub is_published: bool,
}

// Register entities for migration analysis
let auto_client = AutoSchemaClient::new(db);
auto_client.register_entity::<User>()?;
auto_client.register_entity::<Post>()?;
```

### Multiple Entity Registration

For convenience, you can register multiple entities at once:

```rust
// Register 2 entities
auto_client.register_entities::<User, Post>()?;

// Register 3 entities  
auto_client.register_entities_3::<User, Post, Category>()?;
```

## Migration Operations

### Automatic Migration

The `auto_migrate()` method performs a complete schema evolution:

```rust
let result = auto_client.auto_migrate().await?;

println!("Migrations applied: {:?}", result.migrations_applied);
println!("Execution time: {:?}", result.execution_time);
```

### Dry Run Preview

Preview what changes will be made without applying them:

```rust
let plan = auto_client.dry_run().await?;

for operation in &plan.operations {
    println!("Will execute: {:?}", operation);
}

for warning in &plan.safety_warnings {
    println!("Warning: {}", warning.message);
}
```

### Schema Verification

Verify that your current database schema matches your entities:

```rust
let validation = auto_client.verify_schema().await?;

if validation.is_valid {
    println!("Schema is up to date!");
} else {
    println!("Schema differences found:");
    for diff in &validation.differences.table_changes {
        println!("Table {}: {:?}", diff.table_name, diff.change_type);
    }
}
```

## Advanced Features

### Environment-Specific Migration

Use different migration strategies for development vs production:

```rust
use d1_rs::MigrationEnvironment;

// Development: allows aggressive changes
let result = auto_client
    .auto_migrate_for_environment(MigrationEnvironment::Development)
    .await?;

// Production: conservative, safer changes
let result = auto_client
    .auto_migrate_for_environment(MigrationEnvironment::Production)
    .await?;
```

### Zero-Downtime Migration

For production systems, use zero-downtime migrations:

```rust
let result = auto_client.zero_downtime_migrate().await?;

println!("Migration completed with zero downtime");
println!("Steps executed: {}", result.steps_completed);
```

### Zero-Downtime with Custom Configuration

Configure zero-downtime migration parameters:

```rust
use std::time::Duration;
use d1_rs::ConcurrentSafetyMode;

let result = auto_client.zero_downtime_migrate_with_config(
    Duration::from_secs(30),  // Max 30 seconds per step
    true,                     // Use shadow tables
    ConcurrentSafetyMode::Balanced,
).await?;
```

## Schema Versioning

### Initialize Versioning

Set up version tracking for your database:

```rust
let version = auto_client.initialize_versioning().await?;
println!("Initialized schema versioning at version: {}", version.version);
```

### Create Version After Migration

Track schema changes with versions:

```rust
let version = auto_client.create_schema_version(
    "v1.2.0",
    "Added user profile features",
    "production",
    Some("developer@example.com".to_string()),
).await?;
```

### Version History

Get version history and comparisons:

```rust
// Get current version
let current = auto_client.get_current_schema_version().await?;

// Get all versions
let all_versions = auto_client.get_all_schema_versions().await?;

// Compare versions
let comparison = auto_client.compare_schema_versions("v1.1.0", "v1.2.0").await?;
```

## Migration Snapshots

### Create Snapshots

Capture database state for rollback purposes:

```rust
let snapshot = auto_client.create_migration_snapshot(
    "pre_user_migration".to_string(),
    "Before adding user profile fields".to_string(),
    vec!["user_features".to_string()],
).await?;
```

### Automatic Snapshots

Create automatic snapshots before migrations:

```rust
let pre_snapshot = auto_client
    .create_pre_migration_snapshot("migration_v1_2")
    .await?;

// Run migration
let result = auto_client.auto_migrate().await?;

// Create post-migration snapshot
let post_snapshot = auto_client
    .create_post_migration_snapshot("migration_v1_2", result.changes_made)
    .await?;
```

### Rollback to Snapshot

Safely rollback to a previous state:

```rust
// Validate rollback safety first
let validation = auto_client
    .validate_rollback_to_snapshot("pre_user_migration")
    .await?;

if validation.is_safe {
    let rollback_result = auto_client
        .rollback_to_snapshot("pre_user_migration", true)  // Create backup
        .await?;
}
```

## Data Seeding

### Register Seed Data

Register seed data for automatic population:

```rust
use d1_rs::SeedEnvironment;

let seed_users = vec![
    User {
        id: 1,
        name: "Admin User".to_string(),
        email: "admin@example.com".to_string(),
        is_active: true,
    },
];

auto_client.register_seed_data(
    "admin_users",
    "Initial admin users",
    seed_users,
    SeedEnvironment::All,
    true,  // Required seed
)?;
```

### Raw Seed Data

Register raw data without entity types:

```rust
use std::collections::HashMap;

let raw_data = vec![
    HashMap::from([
        ("name".to_string(), serde_json::Value::String("Category 1".to_string())),
        ("slug".to_string(), serde_json::Value::String("category-1".to_string())),
    ]),
];

auto_client.register_raw_seed_data(
    "categories",
    "Initial categories",
    "categories",
    raw_data,
    vec!["slug".to_string()],
    SeedEnvironment::Development,
    false,
);
```

### Seed Dependencies

Define dependencies between seeds:

```rust
auto_client.add_seed_dependency("user_posts", "admin_users")?;
```

### Execute Seeding

Run data seeding for an environment:

```rust
let seed_result = auto_client.seed_data("development").await?;

println!("Seeds completed: {}", seed_result.completed_seeds.len());
println!("Records inserted: {}", seed_result.records_inserted);
```

## Parallel Execution

### Parallel Migration

Execute migrations concurrently for better performance:

```rust
let result = auto_client.parallel_migrate().await?;

println!("Total execution time: {:?}", result.total_execution_time);
println!("Parallel operations: {}", result.parallel_operations_count);
```

### Custom Parallel Configuration

Configure parallel execution settings:

```rust
use d1_rs::ParallelExecutionConfig;

let config = ParallelExecutionConfig {
    max_concurrent_operations: 4,
    batch_size: 100,
    timeout_per_operation: Duration::from_secs(60),
};

let result = auto_client.parallel_migrate_with_config(config).await?;
```

## Ultimate Migration

Combine all features for comprehensive migration:

```rust
let result = auto_client.ultimate_migrate(
    "v2.0.0",
    "Major feature update",
    "production",
    Some("team@example.com".to_string()),
    true,  // Create snapshots
    Some(ParallelExecutionConfig::default()),
).await?;

println!("Migration completed with all features:");
println!("- Schema version: {}", result.schema_version.version);
println!("- Pre-snapshot: {:?}", result.pre_snapshot.map(|s| s.id));
println!("- Post-snapshot: {:?}", result.post_snapshot.map(|s| s.id));
```

## Best Practices

### Development Workflow

```rust
async fn development_migration(auto_client: &AutoSchemaClient) -> Result<()> {
    // 1. Register all entities
    auto_client.register_entities::<User, Post>()?;
    
    // 2. Preview changes
    let plan = auto_client.dry_run().await?;
    println!("Migration plan: {:?}", plan);
    
    // 3. Run migration with seeding
    let (migration_result, seed_result) = auto_client
        .auto_migrate_with_seeding("development")
        .await?;
    
    Ok(())
}
```

### Production Workflow

```rust
async fn production_migration(auto_client: &AutoSchemaClient) -> Result<()> {
    // 1. Initialize versioning if not done
    if auto_client.get_current_schema_version().await?.is_none() {
        auto_client.initialize_versioning().await?;
    }
    
    // 2. Create pre-migration snapshot
    let snapshot = auto_client
        .create_pre_migration_snapshot("prod_v1_3")
        .await?;
    
    // 3. Run zero-downtime migration
    let result = auto_client.zero_downtime_migrate().await?;
    
    // 4. Create version record
    let version = auto_client.create_schema_version(
        "v1.3.0",
        "Production update",
        "production",
        None,
    ).await?;
    
    Ok(())
}
```

## Migration Types

The auto migration system supports various operation types:

- **CreateTable**: Create new tables from entity definitions
- **DropTable**: Remove tables that no longer have corresponding entities
- **AddColumn**: Add new fields to existing tables
- **DropColumn**: Remove fields that no longer exist in entities
- **ModifyColumn**: Change column types, constraints, or defaults
- **CreateIndex**: Add indexes for performance
- **DropIndex**: Remove unused indexes
- **AddForeignKey**: Create foreign key relationships
- **DropForeignKey**: Remove foreign key constraints
- **RenameTable**: Rename tables (detected automatically)
- **RenameColumn**: Rename columns (detected automatically)

## Error Handling

```rust
use d1_rs::{AutoMigrationError, Result};

async fn handle_migration_errors(auto_client: &AutoSchemaClient) -> Result<()> {
    match auto_client.auto_migrate().await {
        Ok(result) => {
            println!("Migration successful: {:?}", result);
        }
        Err(AutoMigrationError::ValidationFailed(errors)) => {
            eprintln!("Validation errors:");
            for error in errors {
                eprintln!("  {}", error);
            }
        }
        Err(AutoMigrationError::UnsafeOperation(op)) => {
            eprintln!("Unsafe operation detected: {:?}", op);
            eprintln!("Use MigrationEnvironment::Development to allow aggressive changes");
        }
        Err(e) => {
            eprintln!("Migration failed: {}", e);
        }
    }
    Ok(())
}
```

## Next Steps

- Learn about [Type-Safe Migrations](./type-safe-migrations.md) for manual migration control
- Explore [Relations](./relations/introduction.md) for entity relationships
- Check out [Testing](./advanced/testing.md) for migration testing strategies