use d1_rs::*;
use d1_rs::backends::QueryResult;

mod common;
use common::query_helpers::{
    build_create_table_query_simple, build_table_exists_query
};
#[tokio::test]
async fn test_basic_db_operation() {
    // Test basic database operation first
    let db = D1Client::new_in_memory().await.unwrap();
    
    // Create a simple table using database-agnostic helper
    let (sql, params) = build_create_table_query_simple(
        "debug_table",
        db.dialect()
    );
    let result = db.execute(&sql, &params).await;
    
    println!("Create table result: {:?}", result);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_table_existence_query() {
    // Test querying for table existence in a database-agnostic way
    let db = D1Client::new_in_memory().await.unwrap();
    
    // Create a test table using database-agnostic helper
    let (create_sql, create_params) = build_create_table_query_simple(
        "test_table",
        db.dialect()
    );
    db.execute(&create_sql, &create_params).await.unwrap();
    
    // Query for table existence using database-agnostic helper
    let (sql, params) = build_table_exists_query("test_table", db.dialect());
    let result = db.execute(&sql, &params).await;
    
    println!("Query result: {:?}", result);
    
    match result {
        Ok(query_result) => {
            println!("Number of rows: {}", query_result.rows().len());
            for row in query_result.rows() {
                println!("Row: {:?}", row);
            }
        },
        Err(e) => {
            println!("Error: {:?}", e);
            panic!("Query failed");
        }
    }
}