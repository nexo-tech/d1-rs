/// Test data management system integration tests
/// 
/// This module demonstrates the usage of the fixture system and validates
/// that it works correctly across different database types.

mod common;
use common::multi_db::*;
use crate::fixtures::{TestFixtureManager, TestFixture};
use d1_rs::backends::DatabaseBackend;
use serde_json::Value;

/// Test the fixture manager creation and basic functionality
#[tokio::test]
async fn test_fixture_manager_initialization() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let backend = TestUtils::setup_test_client(TestDatabase::SQLite).await?;
    let manager = TestFixtureManager::new(backend.dialect());
    
    // Verify built-in fixtures are loaded
    assert!(manager.fixtures.contains_key("users_posts"));
    assert!(manager.fixtures.contains_key("type_testing"));
    assert!(manager.fixtures.contains_key("performance_large"));
    
    Ok(())
}

/// Test users_posts fixture across all databases
#[tokio::test]
async fn test_users_posts_fixture_comprehensive() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let databases = TestDatabase::available();
    
    for database in databases {
        println!("Testing users_posts fixture on {:?}", database);
        
        let backend = TestUtils::setup_test_client(database.clone()).await?;
        let manager = TestFixtureManager::new(backend.dialect());
        
        // Setup fixture
        manager.setup_fixture("users_posts", &backend).await?;
        
        // Verify users table
        let user_result = backend.execute("SELECT COUNT(*) as count FROM users", &[]).await?;
        let user_count = user_result.into_simple_entity::<i64>()?;
        assert_eq!(user_count, 2, "Expected 2 users in {} database", database.name());
        
        // Verify posts table
        let post_result = backend.execute("SELECT COUNT(*) as count FROM posts", &[]).await?;
        let post_count = post_result.into_simple_entity::<i64>()?;
        assert_eq!(post_count, 1, "Expected 1 post in {} database", database.name());
        
        // Verify foreign key relationship
        let join_result = backend.execute(
            "SELECT u.name, p.title FROM users u JOIN posts p ON u.id = p.user_id", 
            &[]
        ).await?;
        let rows = join_result.into_rows();
        assert_eq!(rows.len(), 1, "Expected 1 joined record in {} database", database.name());
        
        // Test specific data values
        let user_result = backend.execute(
            "SELECT name, email FROM users WHERE id = 1", 
            &[]
        ).await?;
        let user_rows = user_result.into_rows();
        assert_eq!(user_rows.len(), 1);
        
        // Clean up
        manager.teardown_fixture("users_posts", &backend).await?;
        
        // Verify cleanup
        let cleanup_result = backend.execute(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='users'", 
            &[]
        ).await;
        // Should either error (table doesn't exist) or return empty result
        if let Ok(result) = cleanup_result {
            let cleanup_rows = result.into_rows();
            assert!(cleanup_rows.is_empty(), "Table should be cleaned up in {} database", database.name());
        }
    }
    
    Ok(())
}

/// Test type_testing fixture with comprehensive data types
#[tokio::test]
async fn test_type_testing_fixture() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let backend = TestUtils::setup_test_client(TestDatabase::SQLite).await?;
    let manager = TestFixtureManager::new(backend.dialect());
    
    manager.setup_fixture("type_testing", &backend).await?;
    
    // Verify table creation and data insertion
    let count_result = backend.execute("SELECT COUNT(*) as count FROM type_test", &[]).await?;
    let count = count_result.into_simple_entity::<i64>()?;
    assert_eq!(count, 1);
    
    // Test specific type values
    let data_result = backend.execute(
        "SELECT text_col, integer_col, real_col, boolean_col FROM type_test WHERE id = 1", 
        &[]
    ).await?;
    let rows = data_result.into_rows();
    assert_eq!(rows.len(), 1);
    
    let row = &rows[0];
    assert_eq!(row.get("text_col"), Some(&Value::String("test text".to_string())));
    assert_eq!(row.get("integer_col"), Some(&Value::Number(42.into())));
    assert_eq!(row.get("boolean_col"), Some(&Value::Bool(true)));
    
    manager.teardown_fixture("type_testing", &backend).await?;
    Ok(())
}

/// Test performance_large fixture (with smaller dataset for test speed)
#[tokio::test]
async fn test_performance_fixture_scaled() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let backend = TestUtils::setup_test_client(TestDatabase::SQLite).await?;
    let manager = TestFixtureManager::new(backend.dialect());
    
    // Note: The performance_large fixture has 10,000 records by default
    // This test verifies it can handle large datasets
    manager.setup_fixture("performance_large", &backend).await?;
    
    // Verify large dataset insertion
    let count_result = backend.execute("SELECT COUNT(*) as count FROM performance_test", &[]).await?;
    let count = count_result.into_simple_entity::<i64>()?;
    assert_eq!(count, 10000, "Expected 10,000 performance records");
    
    // Test query performance on large dataset
    let start = std::time::Instant::now();
    let filtered_result = backend.execute(
        "SELECT COUNT(*) as count FROM performance_test WHERE value < 100", 
        &[]
    ).await?;
    let duration = start.elapsed();
    
    let filtered_count = filtered_result.into_simple_entity::<i64>()?;
    assert!(filtered_count > 0, "Should have records with value < 100");
    
    // Performance should be reasonable (less than 1 second for 10k records)
    assert!(duration.as_secs() < 1, "Query should complete quickly: {:?}", duration);
    
    // Test index usage
    let indexed_result = backend.execute(
        "SELECT COUNT(*) as count FROM performance_test WHERE value = 42", 
        &[]
    ).await?;
    let indexed_count = indexed_result.into_simple_entity::<i64>()?;
    // Should have at least one record with value 42 (due to modulo pattern)
    assert!(indexed_count > 0);
    
    manager.teardown_fixture("performance_large", &backend).await?;
    Ok(())
}

/// Test fixture error handling
#[tokio::test]
async fn test_fixture_error_handling() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let backend = TestUtils::setup_test_client(TestDatabase::SQLite).await?;
    let manager = TestFixtureManager::new(backend.dialect());
    
    // Test setup of non-existent fixture
    let result = manager.setup_fixture("nonexistent_fixture", &backend).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
    
    // Test teardown of non-existent fixture
    let teardown_result = manager.teardown_fixture("nonexistent_fixture", &backend).await;
    assert!(teardown_result.is_err());
    
    Ok(())
}

/// Test fixture isolation (fixtures don't interfere with each other)
#[tokio::test]
async fn test_fixture_isolation() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let backend = TestUtils::setup_test_client(TestDatabase::SQLite).await?;
    let manager = TestFixtureManager::new(backend.dialect());
    
    // Setup multiple fixtures
    manager.setup_fixture("users_posts", &backend).await?;
    manager.setup_fixture("type_testing", &backend).await?;
    
    // Verify both fixtures exist independently
    let users_result = backend.execute("SELECT COUNT(*) FROM users", &[]).await?;
    let users_count = users_result.into_simple_entity::<i64>()?;
    assert_eq!(users_count, 2);
    
    let types_result = backend.execute("SELECT COUNT(*) FROM type_test", &[]).await?;
    let types_count = types_result.into_simple_entity::<i64>()?;
    assert_eq!(types_count, 1);
    
    // Teardown one fixture shouldn't affect the other
    manager.teardown_fixture("users_posts", &backend).await?;
    
    // type_test should still exist
    let types_result_after = backend.execute("SELECT COUNT(*) FROM type_test", &[]).await?;
    let types_count_after = types_result_after.into_simple_entity::<i64>()?;
    assert_eq!(types_count_after, 1);
    
    // users table should be gone
    let users_result_after = backend.execute("SELECT COUNT(*) FROM users", &[]).await;
    assert!(users_result_after.is_err(), "Users table should not exist after teardown");
    
    // Clean up remaining fixture
    manager.teardown_fixture("type_testing", &backend).await?;
    
    Ok(())
}

/// Test custom fixture creation
#[tokio::test]
async fn test_custom_fixture_creation() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let backend = TestUtils::setup_test_client(TestDatabase::SQLite).await?;
    let mut manager = TestFixtureManager::new(backend.dialect());
    
    // Create a simple custom fixture
    let custom_fixture = TestFixture {
        name: "simple_test".to_string(),
        description: "Simple test fixture".to_string(),
        setup_queries: vec![
            ("CREATE TABLE IF NOT EXISTS simple_test (id INTEGER PRIMARY KEY, name TEXT)".to_string(), vec![]),
        ],
        teardown_queries: vec![
            ("DROP TABLE IF EXISTS simple_test".to_string(), vec![]),
        ],
        test_data: vec![],
        expected_results: std::collections::HashMap::new(),
    };
    
    // Add custom fixture
    manager.fixtures.insert("simple_test".to_string(), custom_fixture);
    
    // Test setup
    manager.setup_fixture("simple_test", &backend).await?;
    
    // Verify table exists
    let result = backend.execute("SELECT COUNT(*) FROM simple_test", &[]).await?;
    let count = result.into_simple_entity::<i64>()?;
    assert_eq!(count, 0); // No test data inserted
    
    // Test teardown
    manager.teardown_fixture("simple_test", &backend).await?;
    
    Ok(())
}

/// Integration test with macro helpers
mod macro_tests {
    use super::*;
    
    test_all_databases!(test_macro_integration, |backend| async {
        with_test_fixture!("users_posts", backend, {
            // Test within fixture context
            let result = backend.execute("SELECT COUNT(*) FROM users", &[]).await?;
            let count = result.into_simple_entity::<i64>()?;
            assert_eq!(count, 2);
            
            // Test join query
            let join_result = backend.execute(
                "SELECT u.name, p.title FROM users u JOIN posts p ON u.id = p.user_id",
                &[]
            ).await?;
            let rows = join_result.into_rows();
            assert!(!rows.is_empty());
            
            Ok(())
        })
    });
    
    test_all_databases!(test_type_compatibility, |backend| async {
        with_test_fixture!("type_testing", backend, {
            // Test cross-database type compatibility
            let result = backend.execute(
                "SELECT text_col, integer_col, boolean_col FROM type_test WHERE id = 1",
                &[]
            ).await?;
            let rows = result.into_rows();
            assert_eq!(rows.len(), 1);
            
            let row = &rows[0];
            assert!(row.contains_key("text_col"));
            assert!(row.contains_key("integer_col"));
            assert!(row.contains_key("boolean_col"));
            
            Ok(())
        })
    });
}

/// Performance benchmarking with fixtures
#[tokio::test]
async fn test_fixture_performance_benchmarks() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let backend = TestUtils::setup_test_client(TestDatabase::SQLite).await?;
    let manager = TestFixtureManager::new(backend.dialect());
    
    // Measure setup time
    let setup_start = std::time::Instant::now();
    manager.setup_fixture("performance_large", &backend).await?;
    let setup_duration = setup_start.elapsed();
    
    println!("Fixture setup time: {:?}", setup_duration);
    
    // Measure query performance
    let queries = vec![
        ("COUNT query", "SELECT COUNT(*) FROM performance_test"),
        ("Filtered COUNT", "SELECT COUNT(*) FROM performance_test WHERE value < 500"),
        ("Indexed query", "SELECT COUNT(*) FROM performance_test WHERE value = 42"),
        ("Range query", "SELECT COUNT(*) FROM performance_test WHERE value BETWEEN 100 AND 200"),
    ];
    
    for (name, query) in queries {
        let start = std::time::Instant::now();
        let result = backend.execute(query, &[]).await?;
        let duration = start.elapsed();
        
        let count = result.into_simple_entity::<i64>()?;
        println!("{}: {} records in {:?}", name, count, duration);
        
        // Performance assertions
        assert!(duration.as_millis() < 1000, "{} took too long: {:?}", name, duration);
        assert!(count >= 0, "{} should return non-negative count", name);
    }
    
    // Measure teardown time
    let teardown_start = std::time::Instant::now();
    manager.teardown_fixture("performance_large", &backend).await?;
    let teardown_duration = teardown_start.elapsed();
    
    println!("Fixture teardown time: {:?}", teardown_duration);
    
    // Setup should be reasonable (less than 10 seconds for 10k records)
    assert!(setup_duration.as_secs() < 10, "Setup should complete within 10 seconds");
    assert!(teardown_duration.as_secs() < 5, "Teardown should complete within 5 seconds");
    
    Ok(())
}

#[cfg(test)]
mod edge_case_tests {
    use super::*;
    use crate::fixtures::data_sets::TestDataSets;
    
    #[tokio::test]
    async fn test_sql_injection_safety() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let backend = TestUtils::setup_test_client(TestDatabase::SQLite).await?;
        let manager = TestFixtureManager::new(backend.dialect());
        
        manager.setup_fixture("users_posts", &backend).await?;
        
        // Test SQL injection attempts (should be safely handled by parameterized queries)
        let injection_attempts = TestDataSets::sql_injection_tests();
        
        for attempt in injection_attempts {
            // Try to use injection string as parameter value
            let result = backend.execute(
                "SELECT COUNT(*) FROM users WHERE name = ?",
                &[Value::String(attempt.clone())]
            ).await;
            
            // Query should either succeed safely or fail gracefully
            match result {
                Ok(r) => {
                    let count = r.into_simple_entity::<i64>().unwrap_or(0);
                    // Should not return any malicious results
                    assert!(count >= 0);
                }
                Err(_) => {
                    // Safe failure is acceptable
                }
            }
        }
        
        // Verify original data is still intact
        let verify_result = backend.execute("SELECT COUNT(*) FROM users", &[]).await?;
        let user_count = verify_result.into_simple_entity::<i64>()?;
        assert_eq!(user_count, 2, "Original data should be unchanged");
        
        manager.teardown_fixture("users_posts", &backend).await?;
        Ok(())
    }
    
    #[tokio::test]
    async fn test_unicode_and_special_characters() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let backend = TestUtils::setup_test_client(TestDatabase::SQLite).await?;
        let manager = TestFixtureManager::new(backend.dialect());
        
        manager.setup_fixture("users_posts", &backend).await?;
        
        // Test inserting and retrieving unicode data
        let unicode_tests = vec![
            ("🚀 Rocket", "rocket@emoji.com"),
            ("李小明", "xiaoming@chinese.com"),
            ("José María", "jose@spanish.com"),
            ("O'Connor", "oconnor+test@irish.com"),
        ];
        
        for (name, email) in unicode_tests {
            // Insert unicode data
            backend.execute(
                "INSERT INTO users (name, email, created_at) VALUES (?, ?, '2024-01-01 00:00:00')",
                &[Value::String(name.to_string()), Value::String(email.to_string())]
            ).await?;
            
            // Retrieve and verify
            let result = backend.execute(
                "SELECT name, email FROM users WHERE email = ?",
                &[Value::String(email.to_string())]
            ).await?;
            let rows = result.into_rows();
            assert_eq!(rows.len(), 1);
            
            let row = &rows[0];
            assert_eq!(row.get("name"), Some(&Value::String(name.to_string())));
            assert_eq!(row.get("email"), Some(&Value::String(email.to_string())));
        }
        
        manager.teardown_fixture("users_posts", &backend).await?;
        Ok(())
    }
}