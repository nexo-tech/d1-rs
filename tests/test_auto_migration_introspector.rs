use d1_rs::auto_migration::*;
use d1_rs::*;

#[tokio::test]
async fn test_schema_introspector_basic() {
    // Create a simple test database
    let db = D1Client::new_in_memory().await.unwrap();
    
    // Create a simple test table
    let sql = "CREATE TABLE test_table (id INTEGER PRIMARY KEY, name TEXT NOT NULL)";
    db.execute(sql, &[]).await.unwrap();
    
    // Test introspection
    let introspector = SchemaIntrospector::new(&db);
    let tables = introspector.introspect_tables().await.unwrap();
    
    assert!(tables.contains(&"test_table".to_string()));
}

#[tokio::test]
async fn test_schema_introspector_columns() {
    // Create a test database
    let db = D1Client::new_in_memory().await.unwrap();
    
    // Create a test table with various column types
    let sql = r#"
        CREATE TABLE test_users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            email TEXT UNIQUE,
            is_active INTEGER DEFAULT 1
        )
    "#;
    db.execute(sql, &[]).await.unwrap();
    
    // Test column introspection
    let introspector = SchemaIntrospector::new(&db);
    let columns = introspector.introspect_columns("test_users").await.unwrap();
    
    assert_eq!(columns.len(), 4);
    
    // Check ID column
    let id_col = columns.iter().find(|c| c.name == "id").unwrap();
    assert!(id_col.primary_key);
    assert!(id_col.auto_increment);
    
    // Check name column
    let name_col = columns.iter().find(|c| c.name == "name").unwrap();
    assert!(!name_col.nullable);
    
    // Check is_active column (should be detected as boolean)
    let active_col = columns.iter().find(|c| c.name == "is_active").unwrap();
    assert_eq!(active_col.column_type, "BOOLEAN");
}