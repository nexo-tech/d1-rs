use d1_rs::auto_migration::SchemaIntrospector;
use d1_rs::*;

mod common;
use common::query_helpers::{
    build_create_table_query_with_columns
};

#[tokio::test]
async fn test_introspector_direct() {
    // Create a test database
    let db = D1Client::new_in_memory().await.unwrap();
    
    // Create a simple table using database-agnostic helper
    let (sql, params) = build_create_table_query_with_columns(
        "simple_test",
        vec![
            ("id", "INTEGER", true, None, false),
            ("name", "TEXT", false, None, false)
        ],
        db.dialect()
    );
    db.execute(&sql, &params).await.unwrap();
    
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
    
    // Create a test table using database-agnostic helper
    let (sql, params) = build_create_table_query_with_columns(
        "column_test",
        vec![
            ("id", "INTEGER", true, Some("AUTOINCREMENT"), false),
            ("name", "TEXT", false, None, false),
            ("is_active", "INTEGER", false, Some("1"), true)
        ],
        db.dialect()
    );
    db.execute(&sql, &params).await.unwrap();
    
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
            
            // REVOLUTIONARY: Generic introspection NO LONGER detects booleans via heuristics!
            let active_col = columns.iter().find(|c| c.name == "is_active").unwrap();
            assert_eq!(active_col.column_type, "INTEGER", "Generic introspection: no heuristics, INTEGER stays INTEGER");
        },
        Err(e) => {
            panic!("Failed to introspect columns: {:?}", e);
        }
    }
}