/// Integration test demonstrating multi-database test utilities
/// 
/// This test shows how to use the new multi-database testing infrastructure
/// to run the same test across SQLite, PostgreSQL, and MySQL backends.

use d1_rs::*;
use d1_rs::backends::QueryResult;
use serde_json::Value;

mod common;
use common::multi_db::*;
use common::query_helpers::{
    build_count_query_with_where, build_select_query_with_where, build_select_query,
    table, column
};
use sea_query::{Value as SeaValue, Expr, Alias};

#[tokio::test]
async fn test_multi_database_basic_operations() {
    let runner = MultiDatabaseTestRunner::new();
    
    runner.run_test("basic_operations", |client| async move {
        // Test basic database operations across all databases
        let seeder = TestDataSeeder::new(client);
        
        // Test user insertion
        seeder.seed_test_users().await?;
        let user_count = TestUtils::count_records(&seeder.client, "test_users").await?;
        assert_eq!(user_count, 3, "Should have 3 test users");
        
        // Test post insertion
        seeder.seed_test_posts().await?;
        let post_count = TestUtils::count_records(&seeder.client, "test_posts").await?;
        assert_eq!(post_count, 3, "Should have 3 test posts");
        
        // Test query functionality using database-agnostic helper
        let (count_sql, count_params) = build_count_query_with_where(
            table("test_users"),
            vec![Expr::col(Alias::new("is_active")).eq(SeaValue::Bool(Some(true)))],
            seeder.client.dialect()
        );
        let result = seeder.client.query(&count_sql, &count_params).await?;
        
        let rows = result.into_rows();
        assert!(!rows.is_empty(), "Query should return results");
        
        Ok(())
    }).await.expect("Multi-database basic operations test failed");
}

#[tokio::test]
async fn test_database_agnostic_schema_creation() {
    // Test that schema creation works across different database dialects
    for database in TestDatabase::available() {
        let client = TestClient::new(database.clone()).await
            .expect(&format!("Failed to create {} client", database.name()));
            
        let schema_builder = TestSchemaBuilder::new(client.dialect());
        
        // Create users table
        client.execute(&schema_builder.create_users_table(), &[]).await
            .expect(&format!("Failed to create users table on {}", database.name()));
            
        // Create posts table  
        client.execute(&schema_builder.create_posts_table(), &[]).await
            .expect(&format!("Failed to create posts table on {}", database.name()));
            
        // Verify tables exist
        assert!(TestUtils::verify_table_exists(&client, "test_users").await
            .expect(&format!("Failed to verify users table on {}", database.name())));
        assert!(TestUtils::verify_table_exists(&client, "test_posts").await
            .expect(&format!("Failed to verify posts table on {}", database.name())));
            
        println!("✅ Schema creation successful on {}", database.name());
    }
}

#[tokio::test]
async fn test_cross_database_data_consistency() {
    // Test that the same data operations produce consistent results across databases
    let mut results = Vec::new();
    
    for database in TestDatabase::available() {
        let client = TestUtils::setup_test_client_with_data(database.clone()).await
            .expect(&format!("Failed to setup {} test client", database.name()));
            
        // Query active users using database-agnostic helper
        let (active_users_sql, active_users_params) = build_count_query_with_where(
            table("test_users"),
            vec![Expr::col(Alias::new("is_active")).eq(SeaValue::Bool(Some(true)))],
            client.dialect()
        );
        
        let result = client.query(&active_users_sql, &active_users_params).await
            .expect(&format!("Failed to query active users on {}", database.name()));
            
        let rows = result.into_rows();
        assert!(!rows.is_empty(), "Should get count result");
        
        if let Some(count_value) = rows[0].get("count") {
            if let Value::Number(count) = count_value {
                results.push((database.name(), count.as_i64().unwrap_or(0)));
            }
        }
        
        println!("✅ Data consistency check passed on {}", database.name());
    }
    
    // Verify all databases return the same count
    if results.len() > 1 {
        let first_count = results[0].1;
        for (db_name, count) in &results {
            assert_eq!(*count, first_count, 
                "Active user count should be consistent across databases. {} returned {}, expected {}", 
                db_name, count, first_count);
        }
        println!("✅ All databases returned consistent results: {} active users", first_count);
    }
}

#[tokio::test]
async fn test_database_specific_boolean_handling() {
    // Test that boolean values are handled correctly across different databases
    let runner = MultiDatabaseTestRunner::new();
    
    runner.run_test("boolean_handling", |client| async move {
        let seeder = TestDataSeeder::new(client);
        seeder.seed_test_users().await?;
        
        // Test boolean true query using database-agnostic helper
        let (true_sql, true_params) = build_select_query_with_where(
            table("test_users"),
            vec![column("name")],
            vec![Expr::col(Alias::new("is_active")).eq(SeaValue::Bool(Some(true)))],
            seeder.client.dialect()
        );
        
        let result = seeder.client.query(&true_sql, &true_params).await?;
        let active_users = result.into_rows();
        assert_eq!(active_users.len(), 2, "Should have 2 active users (Alice and Charlie)");
        
        // Test boolean false query using database-agnostic helper
        let (false_sql, false_params) = build_select_query_with_where(
            table("test_users"),
            vec![column("name")],
            vec![Expr::col(Alias::new("is_active")).eq(SeaValue::Bool(Some(false)))],
            seeder.client.dialect()
        );
        
        let result = seeder.client.query(&false_sql, &false_params).await?;
        let inactive_users = result.into_rows();
        assert_eq!(inactive_users.len(), 1, "Should have 1 inactive user (Bob)");
        
        Ok(())
    }).await.expect("Boolean handling test failed");
}

#[tokio::test] 
async fn test_test_utils_functionality() {
    // Test the utility functions work correctly
    for database in TestDatabase::available() {
        // Test setup without data
        let client = TestUtils::setup_test_client(database.clone()).await
            .expect(&format!("Failed to setup clean {} client", database.name()));
            
        assert_eq!(TestUtils::count_records(&client, "test_users").await
            .expect("Failed to count users"), 0);
        assert_eq!(TestUtils::count_records(&client, "test_posts").await
            .expect("Failed to count posts"), 0);
            
        // Test setup with data
        let client_with_data = TestUtils::setup_test_client_with_data(database.clone()).await
            .expect(&format!("Failed to setup {} client with data", database.name()));
            
        assert_eq!(TestUtils::count_records(&client_with_data, "test_users").await
            .expect("Failed to count users"), 3);
        assert_eq!(TestUtils::count_records(&client_with_data, "test_posts").await
            .expect("Failed to count posts"), 3);
            
        // Test cleanup
        TestUtils::cleanup_test_client(&client_with_data).await
            .expect(&format!("Failed to cleanup {} client", database.name()));
            
        println!("✅ Test utilities work correctly on {}", database.name());
    }
}

#[tokio::test]
async fn test_foreign_key_relationships() {
    // Test that foreign key relationships work across databases
    let runner = MultiDatabaseTestRunner::new();
    
    runner.run_test("foreign_key_relationships", |client| async move {
        let seeder = TestDataSeeder::new(client);
        seeder.seed_all_data().await?;
        
        // Test join query to verify relationships work
        let join_sql = r#"
            SELECT u.name as user_name, p.title as post_title 
            FROM test_users u 
            INNER JOIN test_posts p ON u.id = p.user_id 
            WHERE u.name = 'Alice'
        "#;
        
        let result = seeder.client.query(join_sql, &[]).await?;
        let rows = result.into_rows();
        
        assert_eq!(rows.len(), 2, "Alice should have 2 posts");
        
        // Verify post titles
        let titles: Vec<String> = rows.iter()
            .filter_map(|row| {
                if let Some(Value::String(title)) = row.get("post_title") {
                    Some(title.clone())
                } else {
                    None
                }
            })
            .collect();
            
        assert!(titles.contains(&"First Post".to_string()));
        assert!(titles.contains(&"Second Post".to_string()));
        
        Ok(())
    }).await.expect("Foreign key relationships test failed");
}

#[tokio::test]
async fn test_error_handling_across_databases() {
    // Test that error handling works consistently across databases
    for database in TestDatabase::available() {
        let client = TestClient::new(database.clone()).await
            .expect(&format!("Failed to create {} client", database.name()));
            
        // Try to query non-existent table - should fail gracefully using database-agnostic helper
        let (select_sql, select_params) = build_select_query(
            table("non_existent_table"),
            vec![column("*")],
            client.dialect()
        );
        let result = client.query(&select_sql, &select_params).await;
        assert!(result.is_err(), "Query to non-existent table should fail on {}", database.name());
        
        // Try to insert into non-existent table - should fail gracefully
        // Note: This intentionally uses raw SQL as it's testing error handling with invalid syntax
        let result = client.execute("INSERT INTO non_existent_table VALUES (1)", &[]).await;
        assert!(result.is_err(), "Insert to non-existent table should fail on {}", database.name());
        
        println!("✅ Error handling works correctly on {}", database.name());
    }
}

#[tokio::test]
async fn test_macro_usage_examples() {
    // Demonstrate the convenience macros
    
    // Example 1: Setup database with data using macro
    let client = setup_test_db_with_data!(TestDatabase::SQLite);
    assert_eq!(TestUtils::count_records(&client, "test_users").await.unwrap(), 3);
    
    // Example 2: Setup database without data using macro  
    let empty_client = setup_test_db!(TestDatabase::SQLite);
    assert_eq!(TestUtils::count_records(&empty_client, "test_users").await.unwrap(), 0);
    
    println!("✅ Convenience macros work correctly");
}

// Example of how to use the multi_db_test! macro
multi_db_test!(test_using_convenience_macro, |client| async move {
    // This test will automatically run on all available databases
    let seeder = TestDataSeeder::new(client);
    seeder.seed_test_users().await?;
    
    let count = TestUtils::count_records(&seeder.client, "test_users").await?;
    assert_eq!(count, 3);
    
    Ok(())
});

#[tokio::test]
async fn test_database_availability_detection() {
    // Test that database availability detection works
    let available = TestDatabase::available();
    assert!(!available.is_empty(), "At least SQLite should be available");
    assert!(available.contains(&TestDatabase::SQLite), "SQLite should always be available");
    
    println!("Available databases: {:?}", available.iter().map(|db| db.name()).collect::<Vec<_>>());
}

#[tokio::test]
async fn test_dialect_specific_sql_generation() {
    // Test that SQL generation varies appropriately by dialect
    let sqlite_builder = TestSchemaBuilder::new(DatabaseDialect::SQLite);
    let sqlite_ddl = sqlite_builder.create_users_table();
    assert!(sqlite_ddl.contains("INTEGER PRIMARY KEY AUTOINCREMENT"));
    assert!(sqlite_ddl.contains("DATETIME DEFAULT CURRENT_TIMESTAMP"));
    
    #[cfg(feature = "postgres")]
    {
        let postgres_builder = TestSchemaBuilder::new(DatabaseDialect::PostgreSQL);
        let postgres_ddl = postgres_builder.create_users_table();
        assert!(postgres_ddl.contains("BIGSERIAL PRIMARY KEY"));
        assert!(postgres_ddl.contains("TIMESTAMP WITH TIME ZONE DEFAULT NOW()"));
    }
    
    #[cfg(feature = "mysql")]
    {
        let mysql_builder = TestSchemaBuilder::new(DatabaseDialect::MySQL);
        let mysql_ddl = mysql_builder.create_users_table();
        assert!(mysql_ddl.contains("BIGINT AUTO_INCREMENT PRIMARY KEY"));
        assert!(mysql_ddl.contains("TIMESTAMP DEFAULT CURRENT_TIMESTAMP"));
    }
    
    println!("✅ Dialect-specific SQL generation works correctly");
}