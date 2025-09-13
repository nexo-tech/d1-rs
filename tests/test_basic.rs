use d1_rs::*;
use serde_json::Value;

#[tokio::test]
async fn test_basic_database_operations() {
    // Test that we can create an in-memory database
    let db = D1Client::new_in_memory()
        .await
        .expect("Failed to create in-memory database");
    
    // Create a simple table
    let create_table = r#"
        CREATE TABLE test_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            score INTEGER
        )
    "#;
    
    db.execute(create_table, &[]).await.expect("Failed to create table");
    
    // Insert some test data
    let insert_sql = "INSERT INTO test_items (name, is_active, score) VALUES (?, ?, ?)";
    
    db.execute(insert_sql, &[
        Value::String("Test Item".to_string()),
        Value::Bool(true),
        Value::Number(100.into()),
    ]).await.expect("Failed to insert");
    
    // Query the data back
    let select_sql = "SELECT * FROM test_items WHERE name = ?";
    let result = db.execute(select_sql, &[
        Value::String("Test Item".to_string()),
    ]).await.expect("Failed to query");
    
    // Verify we got data
    assert_eq!(result.rows.len(), 1);
    
    let row = &result.rows[0];
    if let Value::Object(obj) = row {
        assert_eq!(obj.get("name"), Some(&Value::String("Test Item".to_string())));
        // SQLite stores booleans as integers
        assert_eq!(obj.get("is_active"), Some(&Value::Number(1.into())));
        assert_eq!(obj.get("score"), Some(&Value::Number(100.into())));
    }
}

#[tokio::test]
async fn test_boolean_conversion() {
    let db = D1Client::new_in_memory().await.expect("Failed to create database");
    
    // Create table with boolean-like field
    db.execute(
        "CREATE TABLE test_bools (id INTEGER PRIMARY KEY, is_active INTEGER)",
        &[]
    ).await.expect("Failed to create table");
    
    // Insert with boolean value
    db.execute(
        "INSERT INTO test_bools (is_active) VALUES (?)",
        &[Value::Bool(true)]
    ).await.expect("Failed to insert");
    
    // Query back
    let result = db.execute("SELECT * FROM test_bools", &[])
        .await.expect("Failed to query");
    
    assert_eq!(result.rows.len(), 1);
    
    // Test the boolean conversion feature
    #[derive(serde::Deserialize, serde::Serialize, Debug, d1_rs::Entity)]
    #[table(name = "test_bools")]
    struct TestBool {
        #[primary_key]
        id: i64,
        is_active: bool,
    }
    
    // First let's debug what we got from the query
    println!("Raw result rows: {:?}", result.rows);
    
    let converted: Vec<TestBool> = result.into_entities()
        .expect("Failed to convert with boolean handling");
    
    println!("Converted entities: {:?}", converted);
    
    assert_eq!(converted.len(), 1);
    assert_eq!(converted[0].is_active, true);
}

#[tokio::test] 
async fn test_migration_runner() {
    let db = D1Client::new_in_memory().await.expect("Failed to create database");
    
    let mut runner = MigrationRunner::new();
    
    // Add a test migration
    let migration = CreateTableMigration::new("test_migration", 1, "test_table".to_string())
        .column("id", "INTEGER").primary_key()
        .column("name", "TEXT").not_null();
    
    runner.add_migration(Box::new(migration));
    
    // Run migrations
    runner.run_pending_migrations(&db).await.expect("Migrations failed");
    
    // Verify table was created
    let result = db.execute(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='test_table'",
        &[]
    ).await.expect("Failed to query schema");
    
    assert_eq!(result.rows.len(), 1);
}