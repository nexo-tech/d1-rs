mod common;

use d1_rs::*;
use serde_json::Value;
use d1_rs::backends::QueryResult;
use d1_rs::dialects::DatabaseDialect;
use common::query_helpers::{
    build_select_query, build_insert_query, build_create_table_query_simple,
    build_create_table_query_with_columns, table, column
};
use sea_query::{Value as SeaValue, ColumnType};

#[tokio::test]
async fn test_basic_database_operations() {
    // Test that we can create an in-memory database
    let db = D1Client::new_in_memory()
        .await
        .expect("Failed to create in-memory database");
    
    // Create a simple table using database-agnostic helpers
    let (create_sql, _create_params) = build_create_table_query_simple("test_items", DatabaseDialect::SQLite);
    db.execute_schema(&create_sql).await.expect("Failed to create table");
    
    // Insert some test data using sea-query helpers
    let (insert_sql, insert_params) = build_insert_query(
        table("test_items"),
        vec![column("name")],
        vec![
            SeaValue::String(Some(Box::new("Test Item".to_string()))),
        ],
        DatabaseDialect::SQLite
    );
    
    db.execute(&insert_sql, &insert_params).await.expect("Failed to insert");
    
    // Query the data back using sea-query helpers
    let (select_sql, select_params) = build_select_query(
        table("test_items"),
        vec![], // Empty means SELECT *
        DatabaseDialect::SQLite
    );
    let result = db.execute(&select_sql, &select_params).await.expect("Failed to query");
    
    // Verify we got data
    assert_eq!(result.rows().len(), 1);
    
    let row = &result.rows()[0];
    if let Value::Object(obj) = row {
        assert_eq!(obj.get("name"), Some(&Value::String("Test Item".to_string())));
        // Basic table only has id and name columns
        assert!(obj.get("id").is_some());
    }
}

#[tokio::test]
async fn test_boolean_conversion() {
    let db = D1Client::new_in_memory().await.expect("Failed to create database");
    
    // Create table with boolean field using sea-query helpers
    let (create_sql, _create_params) = build_create_table_query_with_columns(
        "test_bools",
        vec![
            ("name", ColumnType::Text, false),
            ("is_active", ColumnType::Boolean, true)
        ],
        DatabaseDialect::SQLite
    );
    db.execute_schema(&create_sql).await.expect("Failed to create table");
    
    // Insert with boolean value using sea-query helpers
    let (insert_sql, insert_params) = build_insert_query(
        table("test_bools"),
        vec![column("name"), column("is_active")],
        vec![
            SeaValue::String(Some(Box::new("Test Bool".to_string()))),
            SeaValue::Bool(Some(true))
        ],
        DatabaseDialect::SQLite
    );
    db.execute(&insert_sql, &insert_params).await.expect("Failed to insert");
    
    // Query back using sea-query helpers
    let (select_sql, select_params) = build_select_query(
        table("test_bools"),
        vec![], // Empty means SELECT *
        DatabaseDialect::SQLite
    );
    let result = db.execute(&select_sql, &select_params)
        .await.expect("Failed to query");
    
    assert_eq!(result.rows().len(), 1);
    
    // Test the boolean conversion feature
    #[derive(serde::Deserialize, serde::Serialize, Debug, d1_rs::Entity)]
    #[table(name = "test_bools")]
    struct TestBool {
        #[primary_key]
        id: i64,
        is_active: bool,
    }
    
    // First let's debug what we got from the query
    println!("Raw result rows: {:?}", result.rows());
    
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
    
    // Insert test data to verify the migration worked
    let (insert_sql, insert_params) = build_insert_query(
        table("test_table"),
        vec![column("name")],
        vec![SeaValue::String(Some(Box::new("Migration Test".to_string())))],
        DatabaseDialect::SQLite
    );
    db.execute(&insert_sql, &insert_params).await.expect("Failed to insert test data");
    
    // Verify table was created and data was inserted using database-agnostic schema query
    let (select_sql, select_params) = build_select_query(
        table("test_table"),
        vec![], // Just check if table exists and has data
        DatabaseDialect::SQLite
    );
    let result = db.execute(&select_sql, &select_params).await.expect("Failed to query schema");
    
    assert_eq!(result.rows().len(), 1);
}