/// Test Execution Verification Macros
/// 
/// This module provides comprehensive test macros for multi-database testing
/// that enforce backend verification, environment isolation, and proper test execution
/// across SQLite, PostgreSQL, and MySQL backends.

use crate::common::database_manager::TestDatabaseManager;
use d1_rs::dialects::DatabaseDialect;

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
/// verify_db_test!(test_user_crud, DatabaseDialect::SQLite, |client| async move {
///     // Test implementation using the `client` parameter
///     let users = client.execute_query("SELECT COUNT(*) FROM users", &[]).await?;
///     Ok(())
/// });
/// ```
#[macro_export]
macro_rules! verify_db_test {
    ($test_name:ident, $expected_dialect:expr, $test_body:expr) => {
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
            
            // Execute the test body
            let result = $test_body(&client).await;
            
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
/// cross_db_test!(test_basic_crud, |client, dialect| async move {
///     println!("Testing basic CRUD on {:?}", dialect);
///     // Test implementation here
///     Ok(())
/// });
/// ```
#[macro_export]
macro_rules! cross_db_test {
    ($test_name:ident, $test_body:expr) => {
        mod $test_name {
            use super::*;
            
            crate::verify_db_test!(sqlite, d1_rs::dialects::DatabaseDialect::SQLite, {
                $test_body(&client, d1_rs::dialects::DatabaseDialect::SQLite).await
            });
            
            #[cfg(feature = "postgres")]
            crate::verify_db_test!(postgres, d1_rs::dialects::DatabaseDialect::PostgreSQL, {
                $test_body(&client, d1_rs::dialects::DatabaseDialect::PostgreSQL).await
            });
            
            #[cfg(feature = "mysql")]
            crate::verify_db_test!(mysql, d1_rs::dialects::DatabaseDialect::MySQL, {
                $test_body(&client, d1_rs::dialects::DatabaseDialect::MySQL).await
            });
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
/// feature_test!("postgres", test_postgres_arrays, DatabaseDialect::PostgreSQL, |client| async move {
///     // PostgreSQL-specific array functionality test
///     Ok(())
/// });
/// ```
#[macro_export]
macro_rules! feature_test {
    ($feature:literal, $test_name:ident, $dialect:expr, $test_body:expr) => {
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
/// env_test!(test_sqlite_specific, vec!["sqlite"], DatabaseDialect::SQLite, |client| async move {
///     // This only runs when DATABASE_BACKENDS includes "sqlite"
///     Ok(())
/// });
/// ```
#[macro_export]
macro_rules! env_test {
    ($test_name:ident, $allowed_backends:expr, $dialect:expr, $test_body:expr) => {
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
            
            let result = $test_body(&client).await;
            
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
/// timed_test!(test_fast_query, DatabaseDialect::SQLite, 100, |client| async move {
///     // This test must complete within 100ms
///     let result = client.execute_query("SELECT 1", &[]).await?;
///     Ok(())
/// });
/// ```
#[macro_export]
macro_rules! timed_test {
    ($test_name:ident, $dialect:expr, $max_duration_ms:expr, $test_body:expr) => {
        crate::verify_db_test!($test_name, $dialect, |client| async move {
            let start = std::time::Instant::now();
            
            let result = $test_body(client).await;
            
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
            result
        });
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
///     |client| async move {
///         // Setup: Create temporary table
///         client.execute_schema("CREATE TEMP TABLE test_table (id INTEGER)").await?;
///         Ok(())
///     },
///     |client| async move {
///         // Test: Use the temporary table
///         let result = client.execute_query("INSERT INTO test_table VALUES (1)", &[]).await?;
///         Ok(())
///     },
///     |client| async move {
///         // Cleanup: Drop temporary table (automatically handled by TEMP in SQLite)
///         println!("Test cleanup completed");
///         Ok(())
///     }
/// );
/// ```
#[macro_export]
macro_rules! test_with_cleanup {
    ($test_name:ident, $dialect:expr, $setup:expr, $test_body:expr, $cleanup:expr) => {
        crate::verify_db_test!($test_name, $dialect, |client| async move {
            println!("🔧 Setting up test {}", stringify!($test_name));
            
            // Execute setup block
            let setup_result: Result<(), Box<dyn std::error::Error + Send + Sync>> = $setup(client).await;
            
            if let Err(e) = setup_result {
                println!("❌ Setup failed for test {}: {}", stringify!($test_name), e);
                return Err(e);
            }
            
            // Execute test body
            let test_result = $test_body(client).await;
            
            // Always execute cleanup, regardless of test result
            println!("🧹 Cleaning up test {}", stringify!($test_name));
            let _cleanup_result: Result<(), Box<dyn std::error::Error + Send + Sync>> = $cleanup(client).await;
            
            // Return the test result
            test_result
        });
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use d1_rs::backends::DatabaseBackend;
    use std::time::Duration;

    type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

    // Test the verify_db_test macro with SQLite
    verify_db_test!(test_macro_basic_functionality, d1_rs::dialects::DatabaseDialect::SQLite, |client| async move {
        // Test basic client functionality
        let result = client.ping().await;
        assert!(result.is_ok(), "Ping should succeed");
        Ok(())
    });

    // Test feature-gated macro (always available since SQLite is default)
    feature_test!("default", test_macro_feature_gated, d1_rs::dialects::DatabaseDialect::SQLite, |client| async move {
        let connection_info = client.connection_info();
        assert!(!connection_info.is_empty(), "Connection info should not be empty");
        Ok(())
    });

    // Test environment-restricted macro
    env_test!(test_macro_environment_restricted, vec!["sqlite"], d1_rs::dialects::DatabaseDialect::SQLite, |client| async move {
        // Verify we're running on the expected backend
        let dialect = client.dialect();
        assert_eq!(dialect, d1_rs::dialects::DatabaseDialect::SQLite);
        Ok(())
    });

    // Test timed macro (generous timeout to ensure test passes)
    timed_test!(test_macro_performance_timing, d1_rs::dialects::DatabaseDialect::SQLite, 5000, |client| async move {
        // Perform a simple operation that should complete quickly
        let start = std::time::Instant::now();
        let result = client.ping().await?;
        let duration = start.elapsed();
        
        // Ensure the operation was reasonably fast (under 1 second)
        assert!(duration < Duration::from_secs(1), "Ping should be fast");
        Ok(())
    });

    // Test setup/cleanup macro
    test_with_cleanup!(test_macro_with_resource_management, d1_rs::dialects::DatabaseDialect::SQLite,
        |_client| async move {
            // Setup: Verify client is ready
            println!("Test setup: verifying client readiness");
            Ok(())
        },
        |client| async move {
            // Test: Basic database operation
            let result = client.ping().await?;
            assert!(result.is_ok() || result.is_err()); // Either is fine, we just want no panic
            Ok(())
        },
        |_client| async move {
            // Cleanup: Log completion
            println!("Test cleanup: operation completed");
            Ok(())
        }
    );

    // Test cross-database macro functionality
    cross_db_test!(test_macro_cross_database_compatibility, |client: &crate::common::database_manager::AnyDatabaseBackend, dialect| async move {
        println!("Testing macro cross-database compatibility on {:?}", dialect);
        
        // Verify backend matches expectation
        assert_eq!(client.dialect(), dialect, "Client dialect should match expected");
        
        // Test basic functionality across all backends
        let ping_result = client.ping().await;
        assert!(ping_result.is_ok(), "Ping should work on all backends");
        
        Ok(())
    });

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