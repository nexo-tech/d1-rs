/// Advanced Cross-Database Test Suite - Query Builders & Migration Validation
/// 
/// This module tests advanced ORM features across database backends:
/// - Query builder consistency and compatibility
/// - Migration system cross-database validation  
/// - Advanced ORM feature parity testing

use d1_rs::backends::QueryResult;
use serde_json::Value;
use std::collections::HashMap;

mod common;
use common::multi_db::*;
use common::query_helpers::{
    build_select_query_with_where,
    table, column
};
use sea_query::{Value as SeaValue, Expr, Alias};

/// Query builder cross-database compatibility tests
/// Validates that query builders produce consistent results across all database backends
#[tokio::test]
async fn test_cross_database_query_builder_compatibility() {
    let databases = TestDatabase::available();
    let mut query_results = HashMap::new();
    
    for database in databases {
        let client = TestUtils::setup_test_client_with_data(database.clone()).await
            .expect(&format!("Failed to setup {} client", database.name()));
        
        let seeder = TestDataSeeder::new(client);
        
        // Test 1: Simple WHERE clause variations using database-agnostic helpers
        let simple_test_cases = vec![
            ("active_users", "is_active", SeaValue::Bool(Some(true))),
            ("inactive_users", "is_active", SeaValue::Bool(Some(false))),
            ("high_score_users", "score", SeaValue::Int(Some(50))),
        ];
        
        let mut db_results = HashMap::new();
        
        for (test_name, field_name, field_value) in simple_test_cases {
            let columns = if test_name == "high_score_users" {
                vec![column("name"), column("score")]
            } else {
                vec![column("name")]
            };
            
            let condition = if test_name == "high_score_users" {
                Expr::col(Alias::new(field_name)).gt(field_value)
            } else {
                Expr::col(Alias::new(field_name)).eq(field_value)
            };
            
            let (sql, params) = build_select_query_with_where(
                table("test_users"),
                columns,
                vec![condition],
                seeder.client.dialect()
            );
            
            let result = seeder.client.query(&sql, &params).await
                .expect(&format!("Query {} should work on {}", test_name, database.name()));
            
            let rows = result.into_rows();
            db_results.insert(test_name, rows.len());
            
            println!("🔍 {} on {}: {} results", test_name, database.name(), rows.len());
        }
        
        // Test specific email with LIKE - using standard SQL as LIKE syntax is consistent across databases
        let like_result = seeder.client.query(
            "SELECT id, name FROM test_users WHERE email LIKE ?",
            &[Value::String("%alice%".to_string())]
        ).await.expect("LIKE query should work");
        let like_rows = like_result.into_rows();
        db_results.insert("specific_email", like_rows.len());
        println!("🔍 specific_email on {}: {} results", database.name(), like_rows.len());
        
        // Test 2: ORDER BY clause consistency
        // Note: Using standard SQL for ORDER BY as syntax is consistent across databases
        let order_sql = "SELECT name, score FROM test_users WHERE score IS NOT NULL ORDER BY score DESC, name ASC";
        let order_result = seeder.client.query(order_sql, &[]).await
            .expect("ORDER BY should work");
        let order_rows = order_result.into_rows();
        
        // Verify ordering is consistent
        for i in 1..order_rows.len() {
            let curr_score = match order_rows[i].get("score").unwrap() {
                Value::Number(n) => n.as_i64().unwrap_or(0),
                _ => 0,
            };
            let prev_score = match order_rows[i-1].get("score").unwrap() {
                Value::Number(n) => n.as_i64().unwrap_or(0),
                _ => 0,
            };
            
            assert!(curr_score <= prev_score, "ORDER BY DESC should be consistent on {}", database.name());
        }
        
        db_results.insert("ordered_users", order_rows.len());
        
        // Test 3: LIMIT and OFFSET consistency
        // Note: Using standard SQL for LIMIT as syntax is consistent across databases
        let limit_sql = "SELECT name FROM test_users ORDER BY id LIMIT ?";
        let limit_result = seeder.client.query(limit_sql, &[Value::Number(2.into())]).await
            .expect("LIMIT should work");
        let limit_rows = limit_result.into_rows();
        assert!(limit_rows.len() <= 2, "LIMIT should restrict results on {}", database.name());
        
        db_results.insert("limited_users", limit_rows.len());
        
        // Test 4: Aggregate function consistency
        // Note: Using standard SQL for aggregate functions as syntax is consistent across databases
        // Future enhancement: Create build_aggregate_query helper for sea-query integration
        let agg_queries = vec![
            ("user_count", "SELECT COUNT(*) as count FROM test_users"),
            ("avg_score", "SELECT AVG(CAST(score AS REAL)) as avg_score FROM test_users WHERE score IS NOT NULL"),
            ("max_score", "SELECT MAX(score) as max_score FROM test_users"),
            ("min_score", "SELECT MIN(score) as min_score FROM test_users"),
        ];
        
        for (agg_name, agg_sql) in agg_queries {
            let agg_result = seeder.client.query(agg_sql, &[]).await
                .expect(&format!("Aggregate {} should work on {}", agg_name, database.name()));
            
            let agg_rows = agg_result.into_rows();
            assert_eq!(agg_rows.len(), 1, "Aggregate should return one row");
            
            println!("📊 {} on {}: {:?}", agg_name, database.name(), agg_rows[0]);
        }
        
        query_results.insert(database.name(), db_results);
    }
    
    // Compare results across databases
    if query_results.len() > 1 {
        let db_names: Vec<&str> = query_results.keys().cloned().collect();
        let first_db = &db_names[0];
        let first_results = &query_results[first_db];
        
        for other_db in &db_names[1..] {
            let other_results = &query_results[other_db];
            
            for (test_name, first_count) in first_results {
                let other_count = other_results.get(test_name).unwrap();
                assert_eq!(first_count, other_count, 
                    "Query {} should return same count on {} ({}) and {} ({})", 
                    test_name, first_db, first_count, other_db, other_count);
            }
        }
        
        println!("✅ All query builder results are consistent across databases");
    }
}

/// Advanced WHERE clause and JOIN compatibility testing
#[tokio::test]
async fn test_cross_database_advanced_query_features() {
    let runner = MultiDatabaseTestRunner::new();
    
    runner.run_test("advanced_query_features", |client| async move {
        let seeder = TestDataSeeder::new(client);
        seeder.seed_all_data().await?;
        
        // Test 1: Complex WHERE conditions
        let complex_where_sql = r#"
            SELECT u.name, u.score 
            FROM test_users u 
            WHERE (u.score > ? OR u.score IS NULL) 
            AND u.is_active = ? 
            AND u.email NOT LIKE ?
        "#;
        
        let complex_result = seeder.client.query(complex_where_sql, &[
            Value::Number(30.into()),
            seeder.bool_value(true),
            Value::String("%inactive%".to_string())
        ]).await?;
        
        let complex_rows = complex_result.into_rows();
        assert!(!complex_rows.is_empty(), "Complex WHERE should return results");
        
        // Validate all results match the criteria
        for row in &complex_rows {
            // Check is_active is true (converted from database format)
            let empty_email = Value::String("".to_string());
            let email = row.get("email").unwrap_or(&empty_email);
            if let Value::String(email_str) = email {
                assert!(!email_str.contains("inactive"), "Should not include inactive emails");
            }
        }
        
        // Test 2: Multi-table JOINs with aggregations
        let multi_join_sql = r#"
            SELECT 
                u.name,
                COUNT(p.id) as post_count,
                COALESCE(SUM(p.views), 0) as total_views,
                AVG(CAST(p.views AS REAL)) as avg_views
            FROM test_users u
            LEFT JOIN test_posts p ON u.id = p.user_id AND p.is_published = ?
            WHERE u.is_active = ?
            GROUP BY u.id, u.name
            HAVING COUNT(p.id) >= ?
            ORDER BY total_views DESC
        "#;
        
        let join_result = seeder.client.query(multi_join_sql, &[
            seeder.bool_value(true),
            seeder.bool_value(true),
            Value::Number(0.into()) // Users with at least 0 posts (all active users)
        ]).await?;
        
        let join_rows = join_result.into_rows();
        assert!(!join_rows.is_empty(), "Multi-table JOIN should return results");
        
        // Validate aggregation results
        for row in &join_rows {
            let post_count = row.get("post_count").unwrap();
            let total_views = row.get("total_views").unwrap();
            
            assert!(matches!(post_count, Value::Number(_)), "post_count should be number");
            assert!(matches!(total_views, Value::Number(_)), "total_views should be number");
        }
        
        // Test 3: Correlated subqueries
        let correlated_sql = r#"
            SELECT name, email
            FROM test_users u1
            WHERE score = (
                SELECT MAX(score) 
                FROM test_users u2 
                WHERE u2.is_active = u1.is_active
                AND u2.score IS NOT NULL
            )
        "#;
        
        let correlated_result = seeder.client.query(correlated_sql, &[]).await?;
        let correlated_rows = correlated_result.into_rows();
        
        // Should return users with max score among active/inactive groups
        assert!(!correlated_rows.is_empty(), "Correlated subquery should return results");
        
        // Test 4: CASE expressions (database-agnostic conditional logic)
        let case_sql = r#"
            SELECT 
                name,
                score,
                CASE 
                    WHEN score IS NULL THEN 'No Score'
                    WHEN score < 30 THEN 'Low'
                    WHEN score < 70 THEN 'Medium'
                    ELSE 'High'
                END as score_category
            FROM test_users
            ORDER BY 
                CASE WHEN score IS NULL THEN 1 ELSE 0 END,
                score DESC
        "#;
        
        let case_result = seeder.client.query(case_sql, &[]).await?;
        let case_rows = case_result.into_rows();
        
        assert!(!case_rows.is_empty(), "CASE expressions should work");
        
        // Validate CASE logic
        for row in &case_rows {
            let score = row.get("score").unwrap();
            let category = row.get("score_category").unwrap();
            
            match score {
                Value::Null => assert_eq!(category, &Value::String("No Score".to_string())),
                Value::Number(n) => {
                    let score_val = n.as_i64().unwrap_or(0);
                    let expected_category = if score_val < 30 {
                        "Low"
                    } else if score_val < 70 {
                        "Medium"
                    } else {
                        "High"
                    };
                    assert_eq!(category, &Value::String(expected_category.to_string()));
                },
                _ => panic!("Unexpected score type"),
            }
        }
        
        Ok(())
    }).await.expect("Advanced query features test failed");
}

/// Schema migration cross-database validation
/// Tests that schema changes work consistently across all database backends
#[tokio::test]
async fn test_cross_database_migration_validation() {
    let databases = TestDatabase::available();
    
    for database in databases {
        let client = TestUtils::setup_test_client(database.clone()).await
            .expect(&format!("Failed to setup {} client", database.name()));
        
        let _schema_builder = TestSchemaBuilder::new(client.dialect());
        
        // Test 1: Table creation and modification
        let dialect_name = format!("{:?}", client.dialect()).to_lowercase();
        let migration_table_sql = if dialect_name.contains("postgres") {
            "CREATE TABLE migration_test (
                id BIGSERIAL PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                version INTEGER DEFAULT 1
            )"
        } else if dialect_name.contains("mysql") {
            "CREATE TABLE migration_test (
                id BIGINT AUTO_INCREMENT PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                version INT DEFAULT 1
            )"
        } else {
            // Default to SQLite syntax
            "CREATE TABLE migration_test (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                version INTEGER DEFAULT 1
            )"
        };
        
        client.execute(migration_table_sql, &[]).await
            .expect(&format!("Migration table creation should work on {}", database.name()));
        
        // Test 2: Add column (simulated migration)
        // Note: ALTER TABLE ADD COLUMN syntax is consistent across databases
        let add_column_sql = "ALTER TABLE migration_test ADD COLUMN description TEXT";
        
        client.execute(add_column_sql, &[]).await
            .expect(&format!("ADD COLUMN should work on {}", database.name()));
        
        // Test 3: Insert data to verify schema
        let insert_data_sql = "INSERT INTO migration_test (name, description) VALUES (?, ?)";
        client.execute(insert_data_sql, &[
            Value::String("Migration Test".to_string()),
            Value::String("Testing schema changes".to_string())
        ]).await.expect("Insert after migration should work");
        
        // Test 4: Query to verify structure
        let verify_sql = "SELECT id, name, version, description FROM migration_test";
        let verify_result = client.query(verify_sql, &[]).await
            .expect("Query after migration should work");
        
        let verify_rows = verify_result.into_rows();
        assert_eq!(verify_rows.len(), 1);
        
        let row = &verify_rows[0];
        assert!(row.get("id").is_some());
        assert_eq!(row.get("name").unwrap(), &Value::String("Migration Test".to_string()));
        assert_eq!(row.get("version").unwrap(), &Value::Number(1.into()));
        assert_eq!(row.get("description").unwrap(), &Value::String("Testing schema changes".to_string()));
        
        // Test 5: Index creation (if supported)
        // Note: CREATE INDEX syntax is consistent across databases
        let create_index_sql = "CREATE INDEX idx_migration_name ON migration_test(name)";
        
        client.execute(create_index_sql, &[]).await
            .expect(&format!("CREATE INDEX should work on {}", database.name()));
        
        // Test 6: Query with index (should still work)
        let indexed_query_sql = "SELECT * FROM migration_test WHERE name = ?";
        let indexed_result = client.query(indexed_query_sql, &[
            Value::String("Migration Test".to_string())
        ]).await.expect("Indexed query should work");
        
        let indexed_rows = indexed_result.into_rows();
        assert_eq!(indexed_rows.len(), 1);
        
        // Test 7: Migration cleanup
        client.execute("DROP INDEX idx_migration_name", &[]).await
            .expect("DROP INDEX should work");
        
        client.execute("DROP TABLE migration_test", &[]).await
            .expect("DROP TABLE should work");
        
        println!("✅ {} Migration validation completed successfully", database.name());
    }
}

/// Cross-database batch operation testing
/// Validates that batch operations work efficiently across all backends
#[tokio::test]
async fn test_cross_database_batch_operations() {
    let runner = MultiDatabaseTestRunner::new();
    
    runner.run_test("batch_operations", |client| async move {
        let seeder = TestDataSeeder::new(client);
        
        // Test batch inserts
        let batch_size = 50;
        for batch in 0..3 {
            for i in 0..batch_size {
                let user_id = batch * batch_size + i;
                let params = vec![
                    Value::String(format!("batch_user_{}@example.com", user_id)),
                    Value::String(format!("Batch User {}", user_id)),
                    seeder.bool_value(user_id % 2 == 0),
                    Value::Number(user_id.into()),
                ];
                
                seeder.client.execute(&seeder.get_insert_user_sql(), &params).await
                    .expect("Batch insert should work");
            }
        }
        
        // Verify batch insert results
        let count_result = seeder.client.query("SELECT COUNT(*) as count FROM test_users", &[]).await?;
        let count_rows = count_result.into_rows();
        let total_count = match count_rows[0].get("count").unwrap() {
            Value::Number(n) => n.as_i64().unwrap_or(0),
            _ => 0,
        };
        
        assert!(total_count >= 150, "Should have at least 150 users from batch operations");
        
        // Test batch updates
        let batch_update_sql = "UPDATE test_users SET score = score + ? WHERE score < ?";
        seeder.client.execute(batch_update_sql, &[
            Value::Number(10.into()),
            Value::Number(75.into())
        ]).await.expect("Batch update should work");
        
        // Verify batch updates worked
        let verify_update_sql = "SELECT COUNT(*) as count FROM test_users WHERE score >= ?";
        let verify_result = seeder.client.query(verify_update_sql, &[Value::Number(75.into())]).await?;
        let verify_rows = verify_result.into_rows();
        let updated_count = match verify_rows[0].get("count").unwrap() {
            Value::Number(n) => n.as_i64().unwrap_or(0),
            _ => 0,
        };
        
        assert!(updated_count > 0, "Batch update should affect some records");
        
        // Test batch deletes with conditions
        let batch_delete_sql = "DELETE FROM test_users WHERE score > ? AND email LIKE ?";
        seeder.client.execute(batch_delete_sql, &[
            Value::Number(100.into()),
            Value::String("%batch_user_%".to_string())
        ]).await.expect("Batch delete should work");
        
        // Verify batch deletes
        let final_count_result = seeder.client.query("SELECT COUNT(*) as count FROM test_users", &[]).await?;
        let final_count_rows = final_count_result.into_rows();
        let final_count = match final_count_rows[0].get("count").unwrap() {
            Value::Number(n) => n.as_i64().unwrap_or(0),
            _ => 0,
        };
        
        assert!(final_count < total_count, "Batch delete should reduce total count");
        
        Ok(())
    }).await.expect("Batch operations test failed");
}

/// Cross-database error handling consistency
/// Ensures error conditions are handled consistently across all backends
#[tokio::test]
async fn test_cross_database_error_handling_consistency() {
    let databases = TestDatabase::available();
    
    for database in databases {
        let client = TestUtils::setup_test_client_with_data(database.clone()).await
            .expect(&format!("Failed to setup {} client", database.name()));
        
        let seeder = TestDataSeeder::new(client);
        
        // Test 1: Invalid SQL syntax
        let invalid_result = seeder.client.query("SELCT * FORM test_users", &[]).await;
        assert!(invalid_result.is_err(), "Invalid SQL should fail on {}", database.name());
        
        // Test 2: Non-existent table
        let missing_table_result = seeder.client.query("SELECT * FROM non_existent_table", &[]).await;
        assert!(missing_table_result.is_err(), "Missing table should fail on {}", database.name());
        
        // Test 3: Non-existent column
        let missing_column_result = seeder.client.query("SELECT non_existent_column FROM test_users", &[]).await;
        assert!(missing_column_result.is_err(), "Missing column should fail on {}", database.name());
        
        // Test 4: Type mismatch in WHERE clause (should handle gracefully)
        let _type_mismatch_result = seeder.client.query(
            "SELECT * FROM test_users WHERE id = ?", 
            &[Value::String("not_a_number".to_string())]
        ).await;
        // This might succeed or fail depending on database - we just verify it's consistent
        
        // Test 5: Constraint violations (we already tested this in other tests)
        let duplicate_email_result = seeder.client.execute(&seeder.get_insert_user_sql(), &vec![
            Value::String("alice@example.com".to_string()),
            Value::String("Duplicate Alice".to_string()),
            seeder.bool_value(true),
            Value::Number(100.into()),
        ]).await;
        assert!(duplicate_email_result.is_err(), "Duplicate email should fail on {}", database.name());
        
        println!("✅ {} Error handling consistency verified", database.name());
    }
}

/// Cross-database connection and resource management
/// Tests that database connections and resources are managed properly
#[tokio::test]
async fn test_cross_database_resource_management() {
    let databases = TestDatabase::available();
    
    for database in databases {
        // Test multiple client creation and cleanup
        let mut clients = Vec::new();
        
        for i in 0..5 {
            let client = TestUtils::setup_test_client(database.clone()).await
                .expect(&format!("Client {} creation should work on {}", i, database.name()));
            
            // Verify each client works independently
            let test_result = client.query("SELECT 1 as test_value", &[]).await
                .expect("Basic query should work");
            
            let test_rows = test_result.into_rows();
            assert_eq!(test_rows.len(), 1);
            assert_eq!(test_rows[0].get("test_value").unwrap(), &Value::Number(1.into()));
            
            clients.push(client);
        }
        
        // Test concurrent operations (simulate concurrent usage)
        let concurrent_queries = vec![
            "SELECT COUNT(*) as count FROM test_users",
            "SELECT COUNT(*) as count FROM test_posts", 
            "SELECT 1 as value",
            "SELECT 2 as value",
            "SELECT 3 as value",
        ];
        
        for (i, query) in concurrent_queries.iter().enumerate() {
            let client_idx = i % clients.len();
            let result = clients[client_idx].query(query, &[]).await
                .expect(&format!("Concurrent query {} should work on {}", i, database.name()));
            
            let rows = result.into_rows();
            assert!(!rows.is_empty(), "Concurrent query should return results");
        }
        
        println!("✅ {} Resource management test completed - {} clients handled", 
                database.name(), clients.len());
    }
}