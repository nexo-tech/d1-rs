use d1_rs::auto_migration::SchemaIntrospector;
use d1_rs::*;

#[tokio::test]
async fn test_introspector_direct() {
    // Create a test database
    let db = D1Client::new_in_memory().await.unwrap();
    
    // Create a simple table
    let sql = "CREATE TABLE simple_test (id INTEGER PRIMARY KEY, name TEXT NOT NULL)";
    db.execute(sql, &[]).await.unwrap();
    
    // Create the introspector
    let introspector = SchemaIntrospector::new(&db);
    
    // Test the introspect_tables method directly
    match introspector.introspect_tables().await {
        Ok(tables) => {
            println!("Tables found: {:?}", tables);
            assert!(tables.contains(&"simple_test".to_string()));
        },
        Err(e) => {
            panic!("Failed to introspect tables: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_introspector_columns() {
    // Create a test database
    let db = D1Client::new_in_memory().await.unwrap();
    
    // Create a test table
    let sql = "CREATE TABLE column_test (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, is_active INTEGER DEFAULT 1)";
    db.execute(sql, &[]).await.unwrap();
    
    // Create the introspector
    let introspector = SchemaIntrospector::new(&db);
    
    // Test the introspect_columns method
    match introspector.introspect_columns("column_test").await {
        Ok(columns) => {
            println!("Columns found: {:?}", columns);
            assert_eq!(columns.len(), 3);
            
            let id_col = columns.iter().find(|c| c.name == "id").unwrap();
            assert!(id_col.primary_key);
            
            let name_col = columns.iter().find(|c| c.name == "name").unwrap();
            assert!(!name_col.nullable);
            
            let active_col = columns.iter().find(|c| c.name == "is_active").unwrap();
            assert_eq!(active_col.column_type, "BOOLEAN");
        },
        Err(e) => {
            panic!("Failed to introspect columns: {:?}", e);
        }
    }
}