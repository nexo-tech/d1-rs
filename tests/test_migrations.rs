mod common;

use d1orm::*;

#[tokio::test]
async fn test_migration_runner() {
    let db = D1Client::new_in_memory().await.expect("Failed to create database");
    
    let mut runner = MigrationRunner::new();
    
    // Add a simple migration
    let migration = CreateTableMigration::new("create_test_table", 1, "test_table".to_string())
        .column("id", "INTEGER").primary_key()
        .column("name", "TEXT").not_null()
        .column("is_active", "INTEGER").default("1")
        .column("created_at", "DATETIME").default("CURRENT_TIMESTAMP");
    
    runner.add_migration(Box::new(migration));
    
    // Run migrations
    runner.run_pending_migrations(&db)
        .await
        .expect("Failed to run migrations");
    
    // Verify table was created
    let result = db.execute("SELECT name FROM sqlite_master WHERE type='table' AND name='test_table'", &[])
        .await
        .expect("Failed to query tables");
    
    assert_eq!(result.rows.len(), 1);
    
    // Verify migration was recorded
    let migrations = db.execute("SELECT * FROM _migrations WHERE name='create_test_table'", &[])
        .await
        .expect("Failed to query migrations");
    
    assert_eq!(migrations.rows.len(), 1);
}

#[tokio::test]
async fn test_multiple_migrations_in_order() {
    let db = D1Client::new_in_memory().await.expect("Failed to create database");
    
    let mut runner = MigrationRunner::new();
    
    // Add multiple migrations
    let migration1 = CreateTableMigration::new("create_users", 1, "users".to_string())
        .column("id", "INTEGER").primary_key()
        .column("email", "TEXT").not_null().unique();
    
    let migration2 = CreateTableMigration::new("create_posts", 2, "posts".to_string())
        .column("id", "INTEGER").primary_key()
        .column("user_id", "INTEGER").not_null()
        .column("title", "TEXT").not_null();
    
    let migration3 = CreateTableMigration::new("create_comments", 3, "comments".to_string())
        .column("id", "INTEGER").primary_key()
        .column("post_id", "INTEGER").not_null()
        .column("content", "TEXT").not_null();
    
    runner.add_migration(Box::new(migration1));
    runner.add_migration(Box::new(migration2));
    runner.add_migration(Box::new(migration3));
    
    // Run migrations
    runner.run_pending_migrations(&db)
        .await
        .expect("Failed to run migrations");
    
    // Verify all tables were created
    let tables = db.execute(
        "SELECT name FROM sqlite_master WHERE type='table' AND name IN ('users', 'posts', 'comments') ORDER BY name",
        &[]
    ).await.expect("Failed to query tables");
    
    assert_eq!(tables.rows.len(), 3);
    
    // Verify all migrations were recorded
    let migrations = db.execute(
        "SELECT name, version FROM _migrations ORDER BY version",
        &[]
    ).await.expect("Failed to query migrations");
    
    assert_eq!(migrations.rows.len(), 3);
}

#[tokio::test]
async fn test_migration_idempotency() {
    let db = D1Client::new_in_memory().await.expect("Failed to create database");
    
    let mut runner = MigrationRunner::new();
    
    let migration = CreateTableMigration::new("idempotent_test", 1, "idempotent_table".to_string())
        .column("id", "INTEGER").primary_key()
        .column("value", "TEXT");
    
    runner.add_migration(Box::new(migration));
    
    // Run migrations first time
    runner.run_pending_migrations(&db)
        .await
        .expect("Failed to run migrations first time");
    
    // Run migrations second time - should not fail
    runner.run_pending_migrations(&db)
        .await
        .expect("Failed to run migrations second time");
    
    // Verify migration was only recorded once
    let migrations = db.execute(
        "SELECT * FROM _migrations WHERE name='idempotent_test'",
        &[]
    ).await.expect("Failed to query migrations");
    
    assert_eq!(migrations.rows.len(), 1);
}

#[tokio::test]
async fn test_migration_with_complex_schema() {
    let db = D1Client::new_in_memory().await.expect("Failed to create database");
    
    let mut runner = MigrationRunner::new();
    
    // Create a complex table with various column types and constraints
    let migration = CreateTableMigration::new("complex_table", 1, "complex_table".to_string())
        .column("id", "INTEGER").primary_key()
        .column("email", "TEXT").not_null().unique()
        .column("name", "TEXT").not_null()
        .column("age", "INTEGER").not_null()
        .column("score", "REAL")
        .column("is_active", "INTEGER").not_null().default("1")
        .column("metadata", "TEXT")
        .column("created_at", "DATETIME").default("CURRENT_TIMESTAMP")
        .column("updated_at", "DATETIME").default("CURRENT_TIMESTAMP");
    
    runner.add_migration(Box::new(migration));
    
    // Run migration
    runner.run_pending_migrations(&db)
        .await
        .expect("Failed to run migration");
    
    // Test inserting data into the complex table
    use serde_json::Value;
    
    let insert_sql = r#"
        INSERT INTO complex_table (email, name, age, score, is_active, metadata)
        VALUES (?, ?, ?, ?, ?, ?)
    "#;
    
    db.execute(insert_sql, &[
        Value::String("test@example.com".to_string()),
        Value::String("Test User".to_string()),
        Value::Number(25.into()),
        Value::Number(serde_json::Number::from_f64(98.5).unwrap()),
        Value::Bool(true),
        Value::String(r#"{"key": "value"}"#.to_string()),
    ]).await.expect("Failed to insert into complex table");
    
    // Verify the data was inserted correctly
    let result = db.execute("SELECT * FROM complex_table", &[])
        .await
        .expect("Failed to query complex table");
    
    assert_eq!(result.rows.len(), 1);
}

#[tokio::test]
async fn test_migration_lock() {
    let db = D1Client::new_in_memory().await.expect("Failed to create database");
    
    // Create migrations table manually to test lock mechanism
    let create_migrations = r#"
        CREATE TABLE IF NOT EXISTS _migrations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            version INTEGER NOT NULL,
            applied_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )
    "#;
    
    db.execute(create_migrations, &[]).await.expect("Failed to create migrations table");
    
    // Create lock table
    let create_lock = r#"
        CREATE TABLE IF NOT EXISTS _migration_lock (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            locked INTEGER NOT NULL DEFAULT 0,
            locked_at DATETIME
        )
    "#;
    
    db.execute(create_lock, &[]).await.expect("Failed to create lock table");
    
    // Insert lock record
    db.execute("INSERT INTO _migration_lock (id, locked) VALUES (1, 0)", &[])
        .await
        .expect("Failed to insert lock record");
    
    // Now run a migration
    let mut runner = MigrationRunner::new();
    
    let migration = CreateTableMigration::new("lock_test", 1, "lock_test_table".to_string())
        .column("id", "INTEGER").primary_key();
    
    runner.add_migration(Box::new(migration));
    
    runner.run_pending_migrations(&db)
        .await
        .expect("Failed to run migration with lock");
    
    // Verify lock was released
    let lock_status = db.execute("SELECT locked FROM _migration_lock WHERE id = 1", &[])
        .await
        .expect("Failed to query lock");
    
    if let Some(serde_json::Value::Object(row)) = lock_status.rows.first() {
        if let Some(serde_json::Value::Number(locked)) = row.get("locked") {
            assert_eq!(locked.as_i64(), Some(0), "Lock should be released");
        }
    }
}