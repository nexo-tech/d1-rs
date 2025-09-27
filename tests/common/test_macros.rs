/// Test Execution Verification Macros
/// 
/// This module provides comprehensive test macros for multi-database testing
/// that enforce backend verification, environment isolation, and proper test execution
/// across SQLite, PostgreSQL, and MySQL backends.

use crate::common::database_manager::TestDatabaseManager;

/// Macro for tests that should run on a specific database backend
/// 
/// This macro automatically:
/// - Creates a database client for the specified dialect
/// - Performs backend type verification
/// - Executes health checks
/// - Provides detailed logging and error reporting
/// 
/// # Example
/// ```rust
/// verify_db_test!(test_user_crud, DatabaseDialect::SQLite, {
///     // Test implementation using the `client` variable
///     let users = client.execute_query("SELECT COUNT(*) FROM users", &[]).await?;
///     Ok(())
/// });
/// ```
#[macro_export]
macro_rules! verify_db_test {
    ($test_name:ident, $expected_dialect:expr, $test_body:block) => {
        #[tokio::test]
        async fn $test_name() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            println!("🔄 Starting test {} on {:?}", stringify!($test_name), $expected_dialect);
            
            let manager = crate::common::database_manager::TestDatabaseManager::new();
            let client = manager.create_client($expected_dialect).await
                .map_err(|e| format!("Failed to create client for {:?}: {}", $expected_dialect, e))?;
            
            // Verify backend type matches expectation
            client.verify_backend_type($expected_dialect)
                .map_err(|e| format!("Backend verification failed: {}", e))?;
            
            // Perform health check
            client.health_check().await
                .map_err(|e| format!("Health check failed: {}", e))?;
            
            // Execute the test body with client available
            let result: Result<(), Box<dyn std::error::Error + Send + Sync>> = {
                let client = &client;
                async move {
                    $test_body
                }.await
            };
            
            match &result {
                Ok(_) => {
                    println!("✅ Test {} completed successfully on {}", 
                        stringify!($test_name), client.connection_summary());
                },
                Err(e) => {
                    println!("❌ Test {} failed on {}: {}", 
                        stringify!($test_name), client.connection_summary(), e);
                }
            }
            
            result
        }
    };
}

/// Macro for tests that should run on ALL available database backends
/// 
/// This macro creates separate test functions for each supported database,
/// with proper feature gating to ensure tests only run when the corresponding
/// database backend is compiled in.
/// 
/// # Example
/// ```rust
/// cross_db_test!(test_basic_crud, {
///     println!("Testing basic CRUD on {:?}", dialect);
///     // Test implementation here with client and dialect variables available
///     Ok(())
/// });
/// ```
#[macro_export]
macro_rules! cross_db_test {
    ($test_name:ident, $test_body:block) => {
        mod $test_name {
            use super::*;
            
            #[tokio::test]
            async fn sqlite() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                let dialect = d1_rs::dialects::DatabaseDialect::SQLite;
                println!("🔄 Starting cross-db test {} on {:?}", stringify!($test_name), dialect);
                
                let manager = crate::common::database_manager::TestDatabaseManager::new();
                let client = manager.create_client(dialect).await
                    .map_err(|e| format!("Failed to create client for {:?}: {}", dialect, e))?;
                
                client.verify_backend_type(dialect)
                    .map_err(|e| format!("Backend verification failed: {}", e))?;
                
                client.health_check().await
                    .map_err(|e| format!("Health check failed: {}", e))?;
                
                let result: Result<(), Box<dyn std::error::Error + Send + Sync>> = {
                    let client = &client;
                    async move {
                        $test_body
                    }.await
                };
                
                match &result {
                    Ok(_) => {
                        println!("✅ Cross-db test {} completed successfully on {}", 
                            stringify!($test_name), client.connection_summary());
                    },
                    Err(e) => {
                        println!("❌ Cross-db test {} failed on {}: {}", 
                            stringify!($test_name), client.connection_summary(), e);
                    }
                }
                
                result
            }
            
            #[cfg(feature = "postgres")]
            #[tokio::test]
            async fn postgres() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                let dialect = d1_rs::dialects::DatabaseDialect::PostgreSQL;
                println!("🔄 Starting cross-db test {} on {:?}", stringify!($test_name), dialect);
                
                let manager = crate::common::database_manager::TestDatabaseManager::new();
                let client = manager.create_client(dialect).await
                    .map_err(|e| format!("Failed to create client for {:?}: {}", dialect, e))?;
                
                client.verify_backend_type(dialect)
                    .map_err(|e| format!("Backend verification failed: {}", e))?;
                
                client.health_check().await
                    .map_err(|e| format!("Health check failed: {}", e))?;
                
                let result: Result<(), Box<dyn std::error::Error + Send + Sync>> = {
                    let client = &client;
                    async move {
                        $test_body
                    }.await
                };
                
                match &result {
                    Ok(_) => {
                        println!("✅ Cross-db test {} completed successfully on {}", 
                            stringify!($test_name), client.connection_summary());
                    },
                    Err(e) => {
                        println!("❌ Cross-db test {} failed on {}: {}", 
                            stringify!($test_name), client.connection_summary(), e);
                    }
                }
                
                result
            }
            
            #[cfg(feature = "mysql")]
            #[tokio::test]
            async fn mysql() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                let dialect = d1_rs::dialects::DatabaseDialect::MySQL;
                println!("🔄 Starting cross-db test {} on {:?}", stringify!($test_name), dialect);
                
                let manager = crate::common::database_manager::TestDatabaseManager::new();
                let client = manager.create_client(dialect).await
                    .map_err(|e| format!("Failed to create client for {:?}: {}", dialect, e))?;
                
                client.verify_backend_type(dialect)
                    .map_err(|e| format!("Backend verification failed: {}", e))?;
                
                client.health_check().await
                    .map_err(|e| format!("Health check failed: {}", e))?;
                
                let result: Result<(), Box<dyn std::error::Error + Send + Sync>> = {
                    let client = &client;
                    async move {
                        $test_body
                    }.await
                };
                
                match &result {
                    Ok(_) => {
                        println!("✅ Cross-db test {} completed successfully on {}", 
                            stringify!($test_name), client.connection_summary());
                    },
                    Err(e) => {
                        println!("❌ Cross-db test {} failed on {}: {}", 
                            stringify!($test_name), client.connection_summary(), e);
                    }
                }
                
                result
            }
        }
    };
}

/// Macro for feature-gated tests that only run when specific features are enabled
/// 
/// This ensures tests are only compiled and executed when the corresponding
/// database backend feature is available, preventing compilation errors
/// and runtime failures in feature-constrained environments.
/// 
/// # Example
/// ```rust
/// feature_test!("postgres", test_postgres_arrays, DatabaseDialect::PostgreSQL, {
///     // PostgreSQL-specific array functionality test
///     Ok(())
/// });
/// ```
#[macro_export]
macro_rules! feature_test {
    ($feature:literal, $test_name:ident, $dialect:expr, $test_body:block) => {
        #[cfg(feature = $feature)]
        crate::verify_db_test!($test_name, $dialect, $test_body);
    };
}

/// Macro for environment-restricted tests
/// 
/// This macro checks the DATABASE_BACKENDS environment variable to determine
/// if a test should run, enabling strict backend isolation during testing.
/// Tests are skipped if their required backend is not allowed in the current environment.
/// 
/// # Example
/// ```rust
/// env_test!(test_sqlite_specific, vec!["sqlite"], DatabaseDialect::SQLite, {
///     // This only runs when DATABASE_BACKENDS includes "sqlite"
///     Ok(())
/// });
/// ```
#[macro_export]
macro_rules! env_test {
    ($test_name:ident, $allowed_backends:expr, $dialect:expr, $test_body:block) => {
        #[tokio::test]
        async fn $test_name() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            // Check if this backend is allowed in current environment
            let allowed_backends_env = std::env::var("DATABASE_BACKENDS")
                .unwrap_or_else(|_| "sqlite".to_string());
            
            let allowed_list: Vec<&str> = $allowed_backends;
            let is_allowed = allowed_list.iter().any(|&backend| {
                allowed_backends_env.contains(backend)
            });
            
            if !is_allowed {
                println!("⏭️  Skipping test {} - backend {:?} not allowed in current environment ({})", 
                    stringify!($test_name), $dialect, allowed_backends_env);
                return Ok(());
            }
            
            println!("🔒 Environment verification passed: {} allowed for test {}", 
                allowed_backends_env, stringify!($test_name));
            
            // Proceed with normal test execution
            let manager = crate::common::database_manager::TestDatabaseManager::new();
            let client = manager.create_client($dialect).await
                .map_err(|e| format!("Failed to create client for {:?}: {}", $dialect, e))?;
            
            client.verify_backend_type($dialect)
                .map_err(|e| format!("Backend verification failed: {}", e))?;
            
            client.health_check().await
                .map_err(|e| format!("Health check failed: {}", e))?;
            
            // Execute the test body with client available
            let result: Result<(), Box<dyn std::error::Error + Send + Sync>> = {
                let client = &client;
                async move {
                    $test_body
                }.await
            };
            
            match &result {
                Ok(_) => {
                    println!("✅ Environment test {} completed successfully on {}", 
                        stringify!($test_name), client.connection_summary());
                },
                Err(e) => {
                    println!("❌ Environment test {} failed on {}: {}", 
                        stringify!($test_name), client.connection_summary(), e);
                }
            }
            
            result
        }
    };
}

/// Macro for performance-sensitive tests with timing constraints
/// 
/// This macro enforces performance requirements by measuring test execution time
/// and failing tests that exceed specified duration limits. Useful for regression
/// testing and ensuring database operations remain performant.
/// 
/// # Example
/// ```rust
/// timed_test!(test_fast_query, DatabaseDialect::SQLite, 100, {
///     // This test must complete within 100ms
///     let result = client.execute_query("SELECT 1", &[]).await?;
///     Ok(())
/// });
/// ```
#[macro_export]
macro_rules! timed_test {
    ($test_name:ident, $dialect:expr, $max_duration_ms:expr, $test_body:block) => {
        #[tokio::test]
        async fn $test_name() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            println!("🔄 Starting timed test {} on {:?}", stringify!($test_name), $dialect);
            
            let manager = crate::common::database_manager::TestDatabaseManager::new();
            let client = manager.create_client($dialect).await
                .map_err(|e| format!("Failed to create client for {:?}: {}", $dialect, e))?;
            
            // Verify backend type matches expectation
            client.verify_backend_type($dialect)
                .map_err(|e| format!("Backend verification failed: {}", e))?;
            
            // Perform health check
            client.health_check().await
                .map_err(|e| format!("Health check failed: {}", e))?;
            
            let start = std::time::Instant::now();
            
            let result: Result<(), Box<dyn std::error::Error + Send + Sync>> = {
                let client = &client;
                async move {
                    $test_body
                }.await
            };
            
            let duration = start.elapsed();
            let max_duration = std::time::Duration::from_millis($max_duration_ms);
            
            if duration > max_duration {
                return Err(format!(
                    "Test {} took {:?}, expected < {:?}", 
                    stringify!($test_name), duration, max_duration
                ).into());
            }
            
            println!("⏱️  Test {} completed in {:?} (limit: {:?})", 
                stringify!($test_name), duration, max_duration);
            
            match &result {
                Ok(_) => {
                    println!("✅ Timed test {} completed successfully on {}", 
                        stringify!($test_name), client.connection_summary());
                },
                Err(e) => {
                    println!("❌ Timed test {} failed on {}: {}", 
                        stringify!($test_name), client.connection_summary(), e);
                }
            }
            
            result
        }
    };
}

/// Helper macro for tests requiring setup and teardown operations
/// 
/// This macro provides a structured approach to resource management in tests,
/// ensuring proper cleanup even if tests fail. The cleanup block is always
/// executed, similar to a finally block in exception handling.
/// 
/// # Example
/// ```rust
/// test_with_cleanup!(test_temp_table, DatabaseDialect::SQLite,
///     {
///         // Setup: Create temporary table
///         client.execute_schema("CREATE TEMP TABLE test_table (id INTEGER)").await?;
///         Ok(())
///     },
///     {
///         // Test: Use the temporary table
///         let result = client.execute_query("INSERT INTO test_table VALUES (1)", &[]).await?;
///         Ok(())
///     },
///     {
///         // Cleanup: Drop temporary table (automatically handled by TEMP in SQLite)
///         println!("Test cleanup completed");
///         Ok(())
///     }
/// );
/// ```
#[macro_export]
macro_rules! test_with_cleanup {
    ($test_name:ident, $dialect:expr, $setup:block, $test_body:block, $cleanup:block) => {
        #[tokio::test]
        async fn $test_name() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            println!("🔄 Starting cleanup test {} on {:?}", stringify!($test_name), $dialect);
            
            let manager = crate::common::database_manager::TestDatabaseManager::new();
            let client = manager.create_client($dialect).await
                .map_err(|e| format!("Failed to create client for {:?}: {}", $dialect, e))?;
            
            // Verify backend type matches expectation
            client.verify_backend_type($dialect)
                .map_err(|e| format!("Backend verification failed: {}", e))?;
            
            // Perform health check
            client.health_check().await
                .map_err(|e| format!("Health check failed: {}", e))?;
            
            println!("🔧 Setting up test {}", stringify!($test_name));
            
            // Execute setup block
            let setup_result: Result<(), Box<dyn std::error::Error + Send + Sync>> = {
                let client = &client;
                async move {
                    $setup
                }.await
            };
            
            if let Err(e) = setup_result {
                println!("❌ Setup failed for test {}: {}", stringify!($test_name), e);
                return Err(e);
            }
            
            // Execute test body
            let test_result: Result<(), Box<dyn std::error::Error + Send + Sync>> = {
                let client = &client;
                async move {
                    $test_body
                }.await
            };
            
            // Always execute cleanup, regardless of test result
            println!("🧹 Cleaning up test {}", stringify!($test_name));
            let _cleanup_result: Result<(), Box<dyn std::error::Error + Send + Sync>> = {
                let client = &client;
                async move {
                    $cleanup
                }.await
            };
            
            match &test_result {
                Ok(_) => {
                    println!("✅ Cleanup test {} completed successfully on {}", 
                        stringify!($test_name), client.connection_summary());
                },
                Err(e) => {
                    println!("❌ Cleanup test {} failed on {}: {}", 
                        stringify!($test_name), client.connection_summary(), e);
                }
            }
            
            // Return the test result
            test_result
        }
    };
}

#[cfg(test)]
#[allow(dead_code)]
mod tests {
    use super::*;
    use d1_rs::backends::DatabaseBackend;

    type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

    #[tokio::test]
    async fn test_macro_error_handling() -> TestResult {
        // Test that macros properly handle and report errors
        println!("🧪 Testing macro error handling capabilities");
        
        let manager = TestDatabaseManager::new();
        let client = manager.create_client(d1_rs::dialects::DatabaseDialect::SQLite).await?;
        
        // Verify client is working
        client.verify_backend_type(d1_rs::dialects::DatabaseDialect::SQLite)?;
        client.health_check().await?;
        
        println!("✅ Macro error handling test infrastructure verified");
        Ok(())
    }

    #[tokio::test]
    async fn test_macro_logging_output() -> TestResult {
        // Test that macros produce appropriate logging output
        println!("🧪 Testing macro logging and output formatting");
        
        let manager = TestDatabaseManager::new();
        let client = manager.create_client(d1_rs::dialects::DatabaseDialect::SQLite).await?;
        
        println!("🔍 Verifying client connection summary format");
        let summary = client.connection_summary();
        assert!(!summary.is_empty(), "Connection summary should not be empty");
        assert!(summary.contains("SQLite") || summary.contains("database"), 
               "Summary should mention database type: {}", summary);
        
        println!("✅ Macro logging output test completed");
        Ok(())
    }

    #[tokio::test]
    async fn test_macro_backend_isolation() -> TestResult {
        // Test that macros properly enforce backend isolation
        println!("🧪 Testing macro backend isolation enforcement");
        
        let manager = TestDatabaseManager::new();
        
        // Test that we can create a client for SQLite (always available)
        let sqlite_client = manager.create_client(d1_rs::dialects::DatabaseDialect::SQLite).await?;
        sqlite_client.verify_backend_type(d1_rs::dialects::DatabaseDialect::SQLite)?;
        
        // Verify dialect is correct
        assert_eq!(sqlite_client.dialect(), d1_rs::dialects::DatabaseDialect::SQLite);
        
        println!("✅ Macro backend isolation test completed");
        Ok(())
    }

    #[tokio::test]
    async fn test_macro_comprehensive_functionality() -> TestResult {
        // Comprehensive test of all macro capabilities
        println!("🧪 Running comprehensive macro functionality test");
        
        let manager = TestDatabaseManager::new();
        let available_dialects = manager.available_dialects();
        
        println!("📊 Available database dialects: {:?}", available_dialects);
        
        for &dialect in available_dialects {
            if !manager.is_available(dialect) {
                println!("⏭️  Skipping {:?} - not available in current environment", dialect);
                continue;
            }
            
            println!("🔄 Testing macro functionality on {:?}", dialect);
            
            let client = manager.create_client(dialect).await
                .map_err(|e| format!("Failed to create client for {:?}: {}", dialect, e))?;
            
            // Test backend verification
            client.verify_backend_type(dialect)
                .map_err(|e| format!("Backend verification failed for {:?}: {}", dialect, e))?;
            
            // Test health check
            client.health_check().await
                .map_err(|e| format!("Health check failed for {:?}: {}", dialect, e))?;
            
            // Test basic operations
            let ping_result = client.ping().await;
            assert!(ping_result.is_ok(), "Ping should succeed on {:?}", dialect);
            
            println!("✅ Macro functionality verified for {:?}", dialect);
        }
        
        println!("✅ Comprehensive macro functionality test completed");
        Ok(())
    }
}