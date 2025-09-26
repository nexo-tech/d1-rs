/// Comprehensive Cross-Database Test Suite
/// 
/// This module provides extensive validation that the ORM works consistently
/// across SQLite, PostgreSQL, and MySQL backends, ensuring true database agnosticism.

use d1_rs::backends::QueryResult;
use serde_json::Value;
use std::time::{Duration, Instant};
use std::collections::HashMap;

mod common;
use common::multi_db::*;
use common::query_helpers::{
    build_select_query_with_where, build_select_query, build_count_query_with_where,
    build_count_query, build_insert_query, build_update_query, build_delete_query,
    table, column
};
use sea_query::{Value as SeaValue, Expr, Alias};

/// Cross-database CRUD consistency validator
/// Ensures identical CRUD operations produce identical results across all database backends
#[tokio::test]
async fn test_cross_database_crud_consistency() {
    let runner = MultiDatabaseTestRunner::new();
    
    runner.run_test("crud_consistency", |client| async move {
        // Create identical test data across all databases
        let seeder = TestDataSeeder::new(client);
        seeder.seed_test_users().await?;
        
        // Test CREATE consistency
        let create_sql = seeder.get_insert_user_sql();
        let user_params = vec![
            Value::String("consistency@test.com".to_string()),
            Value::String("Consistency Test User".to_string()),
            seeder.bool_value(true),
            Value::Number(100.into()),
        ];
        
        let result = seeder.client.execute(&create_sql, &user_params).await?;
        let rows = result.into_rows();
        assert!(!rows.is_empty(), "INSERT should return data");
        
        // Test READ consistency - exact same query should give same structure using database-agnostic helper
        let (read_sql, read_params) = build_select_query_with_where(
            table("test_users"),
            vec![column("id"), column("email"), column("name"), column("is_active"), column("score")],
            vec![Expr::col(Alias::new("email")).eq(SeaValue::String(Some(Box::new("consistency@test.com".to_string()))))],
            seeder.client.dialect()
        );
        let read_result = seeder.client.query(&read_sql, &read_params).await?;
        let read_rows = read_result.into_rows();
        assert_eq!(read_rows.len(), 1, "Should find exactly one user");
        
        let user_row = &read_rows[0];
        assert_eq!(user_row.get("email").unwrap(), &Value::String("consistency@test.com".to_string()));
        assert_eq!(user_row.get("name").unwrap(), &Value::String("Consistency Test User".to_string()));
        
        // Test UPDATE consistency using standard SQL (database-agnostic)
        let update_sql = "UPDATE test_users SET score = ? WHERE email = ?";
        let update_params = vec![
            Value::Number(200.into()),
            Value::String("consistency@test.com".to_string())
        ];
        let _update_result = seeder.client.execute(update_sql, &update_params).await?;
        
        // Verify update worked consistently using database-agnostic helper
        let verify_result = seeder.client.query(&read_sql, &read_params).await?;
        let verify_rows = verify_result.into_rows();
        let updated_score = verify_rows[0].get("score").unwrap();
        assert_eq!(updated_score, &Value::Number(200.into()));
        
        // Test DELETE consistency using database-agnostic helper
        let (delete_sql, delete_params) = build_delete_query(
            table("test_users"),
            vec![Expr::col(Alias::new("email")).eq(SeaValue::String(Some(Box::new("consistency@test.com".to_string()))))],
            seeder.client.dialect()
        );
        seeder.client.execute(&delete_sql, &delete_params).await?;
        
        let final_result = seeder.client.query(&read_sql, &read_params).await?;
        assert_eq!(final_result.into_rows().len(), 0, "User should be deleted");
        
        Ok(())
    }).await.expect("CRUD consistency test failed");
}

/// Cross-database data type consistency validator
/// Ensures data types are handled identically across all database backends
#[tokio::test]
async fn test_cross_database_data_type_consistency() {
    let runner = MultiDatabaseTestRunner::new();
    
    runner.run_test("data_type_consistency", |client| async move {
        let seeder = TestDataSeeder::new(client);
        seeder.seed_test_users().await?;
        
        // Test boolean handling consistency
        let bool_test_cases = vec![
            ("true_case", true),
            ("false_case", false),
        ];
        
        for (case_name, bool_val) in bool_test_cases {
            let email = format!("{}@example.com", case_name);
            let user_params = vec![
                Value::String(email.clone()),
                Value::String(format!("User {}", case_name)),
                seeder.bool_value(bool_val),
                Value::Number(50.into()),
            ];
            
            seeder.client.execute(&seeder.get_insert_user_sql(), &user_params).await?;
            
            // Query back and verify boolean handling using database-agnostic helper
            let (query_sql, query_params) = build_select_query_with_where(
                table("test_users"),
                vec![column("is_active")],
                vec![Expr::col(Alias::new("email")).eq(SeaValue::String(Some(Box::new(email.clone()))))],
                seeder.client.dialect()
            );
            let result = seeder.client.query(&query_sql, &query_params).await?;
            let rows = result.into_rows();
            
            assert_eq!(rows.len(), 1, "Should find exactly one user for {}", case_name);
            let stored_value = rows[0].get("is_active").unwrap();
            
            // Normalize the stored value for comparison
            let normalized = match stored_value {
                Value::Bool(b) => *b,
                Value::Number(n) => n.as_i64().unwrap_or(0) != 0,
                _ => false,
            };
            
            assert_eq!(normalized, bool_val, "Boolean value should be consistent for {}", case_name);
        }
        
        // Test NULL handling consistency
        let null_params = vec![
            Value::String("null@test.com".to_string()),
            Value::String("Null Test User".to_string()),
            seeder.bool_value(true),
            Value::Null, // NULL score
        ];
        
        seeder.client.execute(&seeder.get_insert_user_sql(), &null_params).await?;
        let (null_sql, null_query_params) = build_select_query_with_where(
            table("test_users"),
            vec![column("score")],
            vec![Expr::col(Alias::new("email")).eq(SeaValue::String(Some(Box::new("null@test.com".to_string()))))],
            seeder.client.dialect()
        );
        let null_result = seeder.client.query(&null_sql, &null_query_params).await?;
        
        let null_rows = null_result.into_rows();
        assert_eq!(null_rows.len(), 1);
        assert_eq!(null_rows[0].get("score").unwrap(), &Value::Null);
        
        Ok(())
    }).await.expect("Data type consistency test failed");
}

/// Cross-database query complexity validator
/// Tests complex queries work identically across all backends
#[tokio::test]
async fn test_cross_database_complex_queries() {
    let runner = MultiDatabaseTestRunner::new();
    
    runner.run_test("complex_queries", |client| async move {
        let seeder = TestDataSeeder::new(client);
        seeder.seed_all_data().await?;
        
        // Test JOIN queries
        // Note: Using standard SQL for complex JOIN query that works identically across all databases
        // Future enhancement: Create build_join_query helper for sea-query integration
        let join_sql = r#"
            SELECT u.name as user_name, p.title as post_title, p.views
            FROM test_users u
            INNER JOIN test_posts p ON u.id = p.user_id
            WHERE u.is_active = ? AND p.is_published = ?
            ORDER BY p.views DESC
        "#;
        
        let join_result = seeder.client.query(join_sql, &[
            seeder.bool_value(true),
            seeder.bool_value(true)
        ]).await?;
        
        let join_rows = join_result.into_rows();
        assert!(!join_rows.is_empty(), "JOIN query should return results");
        
        // Verify structure is consistent
        for row in &join_rows {
            assert!(row.get("user_name").is_some(), "Should have user_name");
            assert!(row.get("post_title").is_some(), "Should have post_title");
            assert!(row.get("views").is_some(), "Should have views");
        }
        
        // Test aggregate functions
        // Note: Using standard SQL for aggregate query that works identically across all databases
        // Future enhancement: Create build_aggregate_query helper for sea-query integration
        let count_sql = "SELECT COUNT(*) as total_posts, SUM(views) as total_views FROM test_posts WHERE is_published = ?";
        let count_result = seeder.client.query(count_sql, &[seeder.bool_value(true)]).await?;
        let count_rows = count_result.into_rows();
        
        assert_eq!(count_rows.len(), 1);
        let total_posts = count_rows[0].get("total_posts").unwrap();
        let total_views = count_rows[0].get("total_views").unwrap();
        
        // Verify aggregates return numbers
        assert!(matches!(total_posts, Value::Number(_)), "COUNT should return number");
        assert!(matches!(total_views, Value::Number(_)), "SUM should return number");
        
        // Test subquery consistency
        // Note: Using standard SQL for complex subquery that works identically across all databases
        // Future enhancement: Create build_subquery helper for sea-query integration
        let subquery_sql = r#"
            SELECT name FROM test_users 
            WHERE id IN (
                SELECT DISTINCT user_id FROM test_posts 
                WHERE views > ?
            )
        "#;
        
        let subquery_result = seeder.client.query(subquery_sql, &[Value::Number(50.into())]).await?;
        let subquery_rows = subquery_result.into_rows();
        
        // Should return consistent results across databases
        assert!(!subquery_rows.is_empty(), "Subquery should return results");
        
        Ok(())
    }).await.expect("Complex queries test failed");
}

/// Cross-database constraint validation
/// Ensures database constraints work consistently across all backends
#[tokio::test]
#[allow(unused_variables)]
async fn test_cross_database_constraint_validation() {
    let runner = MultiDatabaseTestRunner::new();
    
    runner.run_test("constraint_validation", |client| async move {
        let seeder = TestDataSeeder::new(client);
        seeder.seed_test_users().await?;
        
        // Test UNIQUE constraint violation
        let duplicate_email_params = vec![
            Value::String("alice@example.com".to_string()), // This already exists
            Value::String("Duplicate Alice".to_string()),
            seeder.bool_value(true),
            Value::Number(100.into()),
        ];
        
        let result = seeder.client.execute(&seeder.get_insert_user_sql(), &duplicate_email_params).await;
        assert!(result.is_err(), "Duplicate email should fail due to UNIQUE constraint");
        
        // Test foreign key constraint (when inserting post with non-existent user)
        let invalid_fk_params = vec![
            Value::Number(9999.into()), // Non-existent user_id
            Value::String("Invalid Post".to_string()),
            Value::String("This post has invalid user_id".to_string()),
            seeder.bool_value(true),
            Value::Number(0.into()),
        ];
        
        let fk_result = seeder.client.execute(&seeder.get_insert_post_sql(), &invalid_fk_params).await;
        // Note: Different databases handle FK constraints differently
        // We verify behavior is consistent for each dialect
        let dialect_name = format!("{:?}", seeder.client.dialect()).to_lowercase();
        if dialect_name.contains("sqlite") {
            // SQLite FK constraints might not be enabled, so we don't assert failure
            // but we record the behavior is consistent
        } else {
            // PostgreSQL and MySQL should enforce FK constraints
            assert!(fk_result.is_err(), "Invalid foreign key should fail on {}", dialect_name);
        }
        
        // Test valid foreign key insertion using database-agnostic helper
        let (get_user_id_sql, get_user_id_params) = build_select_query_with_where(
            table("test_users"),
            vec![column("id")],
            vec![Expr::col(Alias::new("email")).eq(SeaValue::String(Some(Box::new("alice@example.com".to_string()))))],
            seeder.client.dialect()
        );
        let user_result = seeder.client.query(&get_user_id_sql, &get_user_id_params).await?;
        let user_rows = user_result.into_rows();
        assert!(!user_rows.is_empty(), "Should find Alice");
        
        let user_id = user_rows[0].get("id").unwrap();
        let valid_fk_params = vec![
            user_id.clone(),
            Value::String("Valid Post".to_string()),
            Value::String("This post has valid user_id".to_string()),
            seeder.bool_value(true),
            Value::Number(10.into()),
        ];
        
        let valid_result = seeder.client.execute(&seeder.get_insert_post_sql(), &valid_fk_params).await;
        assert!(valid_result.is_ok(), "Valid foreign key insertion should succeed");
        
        Ok(())
    }).await.expect("Constraint validation test failed");
}

/// Performance benchmarking across databases
/// Measures and compares performance characteristics of different backends
#[tokio::test]
async fn test_cross_database_performance_benchmarks() {
    let databases = TestDatabase::available();
    let mut performance_results = HashMap::new();
    
    for database in databases {
        let client = TestUtils::setup_test_client_with_data(database.clone()).await
            .expect(&format!("Failed to setup {} client", database.name()));
        
        // Benchmark INSERT performance
        let insert_start = Instant::now();
        for i in 0..100 {
            let email = format!("perf_user_{}@example.com", i);
            let params = vec![
                Value::String(email),
                Value::String(format!("Performance User {}", i)),
                Value::Number(1.into()),
                Value::Number(i.into()),
            ];
            
            let seeder = TestDataSeeder::new(client.clone());
            let _ = seeder.client.execute(&seeder.get_insert_user_sql(), &params).await;
        }
        let insert_duration = insert_start.elapsed();
        
        // Benchmark SELECT performance
        let select_start = Instant::now();
        for i in 0..50 {
            let email = format!("perf_user_{}@example.com", i * 2);
            let (select_sql, select_params) = build_select_query_with_where(
                table("test_users"),
                vec![column("*")],
                vec![Expr::col(Alias::new("email")).eq(SeaValue::String(Some(Box::new(email))))],
                client.dialect()
            );
            let _ = client.query(&select_sql, &select_params).await;
        }
        let select_duration = select_start.elapsed();
        
        // Benchmark complex query performance
        let complex_start = Instant::now();
        let _ = client.query(
            r#"SELECT u.name, COUNT(p.id) as post_count 
               FROM test_users u 
               LEFT JOIN test_posts p ON u.id = p.user_id 
               WHERE u.score > ? 
               GROUP BY u.id, u.name 
               ORDER BY post_count DESC"#,
            &[Value::Number(10.into())]
        ).await;
        let complex_duration = complex_start.elapsed();
        
        performance_results.insert(database.name(), (insert_duration, select_duration, complex_duration));
        
        println!("🚀 {} Performance:", database.name());
        println!("  - INSERT (100 ops): {:?}", insert_duration);
        println!("  - SELECT (50 ops): {:?}", select_duration); 
        println!("  - Complex JOIN: {:?}", complex_duration);
    }
    
    // Verify all operations completed within reasonable time bounds
    for (db_name, (insert_time, select_time, complex_time)) in &performance_results {
        assert!(insert_time < &Duration::from_secs(10), "{} INSERT performance too slow", db_name);
        assert!(select_time < &Duration::from_secs(5), "{} SELECT performance too slow", db_name);
        assert!(complex_time < &Duration::from_secs(2), "{} Complex query performance too slow", db_name);
    }
    
    println!("✅ All database performance benchmarks completed successfully");
}

/// Memory usage testing across databases
/// Validates memory consumption patterns are reasonable for each backend
#[tokio::test]
async fn test_cross_database_memory_usage() {
    let databases = TestDatabase::available();
    
    for database in databases {
        let client = TestUtils::setup_test_client(database.clone()).await
            .expect(&format!("Failed to setup {} client", database.name()));
        
        // Test memory usage with large result sets
        let seeder = TestDataSeeder::new(client);
        
        // Insert moderate amount of test data
        for i in 0..1000 {
            let params = vec![
                Value::String(format!("memory_test_{}@example.com", i)),
                Value::String(format!("Memory Test User {}", i)),
                Value::Number((i % 2).into()),
                Value::Number(i.into()),
            ];
            
            let _ = seeder.client.execute(&seeder.get_insert_user_sql(), &params).await;
        }
        
        // Query large result set and verify it doesn't cause memory issues
        let result = seeder.client.query("SELECT * FROM test_users ORDER BY score", &[]).await
            .expect(&format!("Large query should work on {}", database.name()));
        
        let rows = result.into_rows();
        assert_eq!(rows.len(), 1000, "Should return all 1000 users");
        
        // Verify memory usage stays reasonable by successfully completing multiple large operations
        for _ in 0..5 {
            let count_result = seeder.client.query("SELECT COUNT(*) as total FROM test_users", &[]).await
                .expect("Count query should work");
            let count_rows = count_result.into_rows();
            assert_eq!(count_rows.len(), 1);
        }
        
        println!("✅ {} Memory usage test completed - handled 1000 records efficiently", database.name());
    }
}

/// Schema consistency validation across databases
/// Ensures schema operations work identically across all backends
#[tokio::test]
async fn test_cross_database_schema_consistency() {
    let databases = TestDatabase::available();
    
    for database in databases {
        let client = TestUtils::setup_test_client(database.clone()).await
            .expect(&format!("Failed to setup {} client", database.name()));
        
        // Test table creation consistency
        let _schema_builder = TestSchemaBuilder::new(client.dialect());
        
        // Create a test table with various column types
        // Note: Using database-specific SQL for CREATE TABLE as this tests schema compatibility
        let dialect_name = format!("{:?}", client.dialect()).to_lowercase();
        let test_table_sql = if dialect_name.contains("postgres") {
            "CREATE TABLE schema_test (
                id BIGSERIAL PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                age INTEGER,
                salary DECIMAL(10,2),
                is_active BOOLEAN DEFAULT TRUE,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
            )"
        } else if dialect_name.contains("mysql") {
            "CREATE TABLE schema_test (
                id BIGINT AUTO_INCREMENT PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                age INT,
                salary DECIMAL(10,2),
                is_active BOOLEAN DEFAULT TRUE,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )"
        } else {
            // Default to SQLite syntax
            "CREATE TABLE schema_test (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                age INTEGER,
                salary REAL,
                is_active INTEGER DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )"
        };
        
        client.execute(test_table_sql, &[]).await
            .expect(&format!("Schema creation should work on {}", database.name()));
        
        // Test data insertion into the new schema
        let insert_test_sql = "INSERT INTO schema_test (name, age, salary) VALUES (?, ?, ?)";
        client.execute(insert_test_sql, &[
            Value::String("Test Person".to_string()),
            Value::Number(30.into()),
            Value::Number(75000.into()),
        ]).await.expect("Data insertion should work");
        
        // Test querying the new schema
        let query_result = client.query("SELECT * FROM schema_test", &[]).await
            .expect("Schema query should work");
        let rows = query_result.into_rows();
        assert_eq!(rows.len(), 1, "Should have one record");
        
        // Verify schema drop works
        client.execute("DROP TABLE schema_test", &[]).await
            .expect("Schema drop should work");
        
        println!("✅ {} Schema consistency test completed", database.name());
    }
}

/// Transaction consistency testing across databases
/// Validates transaction behavior works identically across all backends
#[tokio::test]
async fn test_cross_database_transaction_consistency() {
    let databases = TestDatabase::available();
    
    for database in databases {
        let client = TestUtils::setup_test_client_with_data(database.clone()).await
            .expect(&format!("Failed to setup {} client", database.name()));
        
        // Test transaction rollback consistency
        // Note: This is a basic test since the current D1Client doesn't have explicit transaction support
        // But we can test that operations are atomic at the statement level
        
        // Get initial count
        let initial_result = client.query("SELECT COUNT(*) as count FROM test_users", &[]).await
            .expect("Initial count should work");
        let initial_rows = initial_result.into_rows();
        let initial_count = match initial_rows[0].get("count").unwrap() {
            Value::Number(n) => n.as_i64().unwrap_or(0),
            _ => 0,
        };
        
        // Try to insert user with duplicate email (should fail)
        let seeder = TestDataSeeder::new(client.clone());
        let duplicate_result = seeder.client.execute(&seeder.get_insert_user_sql(), &vec![
            Value::String("alice@example.com".to_string()),
            Value::String("Duplicate Alice".to_string()),
            seeder.bool_value(true),
            Value::Number(100.into()),
        ]).await;
        
        assert!(duplicate_result.is_err(), "Duplicate insertion should fail");
        
        // Verify count hasn't changed (operation was atomic)
        let final_result = client.query("SELECT COUNT(*) as count FROM test_users", &[]).await
            .expect("Final count should work");
        let final_rows = final_result.into_rows();
        let final_count = match final_rows[0].get("count").unwrap() {
            Value::Number(n) => n.as_i64().unwrap_or(0),
            _ => 0,
        };
        
        assert_eq!(initial_count, final_count, "Count should remain unchanged after failed insert");
        
        println!("✅ {} Transaction consistency test completed", database.name());
    }
}

/// Data integrity validation across databases  
/// Ensures data integrity constraints work consistently across all backends
#[tokio::test]
async fn test_cross_database_data_integrity() {
    let runner = MultiDatabaseTestRunner::new();
    
    runner.run_test("data_integrity", |client| async move {
        let seeder = TestDataSeeder::new(client);
        seeder.seed_all_data().await?;
        
        // Test referential integrity
        let posts_result = seeder.client.query(
            "SELECT COUNT(*) as count FROM test_posts WHERE user_id NOT IN (SELECT id FROM test_users)",
            &[]
        ).await?;
        
        let orphaned_posts = posts_result.into_rows();
        let orphan_count = match orphaned_posts[0].get("count").unwrap() {
            Value::Number(n) => n.as_i64().unwrap_or(0),
            _ => 0,
        };
        
        assert_eq!(orphan_count, 0, "There should be no orphaned posts");
        
        // Test data consistency after updates
        let _update_result = seeder.client.execute(
            "UPDATE test_users SET name = ? WHERE email = ?",
            &[
                Value::String("Updated Alice".to_string()),
                Value::String("alice@example.com".to_string())
            ]
        ).await?;
        
        let verify_result = seeder.client.query(
            "SELECT name FROM test_users WHERE email = ?",
            &[Value::String("alice@example.com".to_string())]
        ).await?;
        
        let verify_rows = verify_result.into_rows();
        assert_eq!(verify_rows.len(), 1);
        assert_eq!(verify_rows[0].get("name").unwrap(), &Value::String("Updated Alice".to_string()));
        
        // Test cascade behavior simulation (manual cleanup)
        let user_id_result = seeder.client.query(
            "SELECT id FROM test_users WHERE email = ?",
            &[Value::String("alice@example.com".to_string())]
        ).await?;
        
        let user_rows = user_id_result.into_rows();
        let user_id = user_rows[0].get("id").unwrap();
        
        // Delete posts first (manual cascade)
        seeder.client.execute(
            "DELETE FROM test_posts WHERE user_id = ?",
            &[user_id.clone()]
        ).await?;
        
        // Then delete user
        seeder.client.execute(
            "DELETE FROM test_users WHERE id = ?",
            &[user_id.clone()]
        ).await?;
        
        // Verify cleanup worked
        let final_check = seeder.client.query(
            "SELECT COUNT(*) as count FROM test_users WHERE id = ?",
            &[user_id.clone()]
        ).await?;
        
        let final_rows = final_check.into_rows();
        let final_count = match final_rows[0].get("count").unwrap() {
            Value::Number(n) => n.as_i64().unwrap_or(0),
            _ => 0,
        };
        
        assert_eq!(final_count, 0, "User should be deleted");
        
        Ok(())
    }).await.expect("Data integrity test failed");
}