use d1_rs::*;

use d1_rs::backends::QueryResult;
#[tokio::test]
async fn test_pragma_table_info() {
    // Create a test database
    let db = D1Client::new_in_memory().await.unwrap();
    
    // Create a test table
    let sql = "CREATE TABLE pragma_test (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, is_active INTEGER DEFAULT 1)";
    db.execute(sql, &[]).await.unwrap();
    
    // Test pragma_table_info function (better compatibility)
    let sql = "SELECT * FROM pragma_table_info('pragma_test')";
    let result = db.execute(sql, &[]).await;
    
    match result {
        Ok(query_result) => {
            println!("PRAGMA result: {:?}", query_result);
            for (i, row) in query_result.rows().iter().enumerate() {
                println!("Row {}: {:?}", i, row);
            }
        },
        Err(e) => {
            panic!("PRAGMA failed: {:?}", e);
        }
    }
}