use d1_rs::*;

#[tokio::test]
async fn test_basic_db_operation() {
    // Test basic database operation first
    let db = D1Client::new_in_memory().await.unwrap();
    
    // Create a simple table
    let sql = "CREATE TABLE debug_table (id INTEGER PRIMARY KEY, name TEXT)";
    let result = db.execute(sql, &[]).await;
    
    println!("Create table result: {:?}", result);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_sqlite_master_query() {
    // Test querying sqlite_master directly
    let db = D1Client::new_in_memory().await.unwrap();
    
    // Create a test table
    let create_sql = "CREATE TABLE test_table (id INTEGER PRIMARY KEY, name TEXT)";
    db.execute(create_sql, &[]).await.unwrap();
    
    // Query sqlite_master
    let sql = "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'";
    let result = db.execute(sql, &[]).await;
    
    println!("Query result: {:?}", result);
    
    match result {
        Ok(query_result) => {
            println!("Number of rows: {}", query_result.rows.len());
            for row in &query_result.rows {
                println!("Row: {:?}", row);
            }
        },
        Err(e) => {
            println!("Error: {:?}", e);
            panic!("Query failed");
        }
    }
}