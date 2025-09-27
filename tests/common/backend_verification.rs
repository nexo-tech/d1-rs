/// Backend Verification System
/// 
/// This module provides comprehensive verification capabilities for database backends,
/// ensuring type safety, connection health, and proper isolation across all supported
/// database types (SQLite, PostgreSQL, MySQL).

use d1_rs::dialects::DatabaseDialect;
use std::fmt;

/// Backend verification errors
#[derive(Debug)]
pub enum BackendVerificationError {
    TypeMismatch { expected: DatabaseDialect, actual: DatabaseDialect },
    ConnectionFailure(String),
    HealthCheckFailure(String),
}

impl fmt::Display for BackendVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendVerificationError::TypeMismatch { expected, actual } => {
                write!(f, "❌ Backend verification FAILED! Expected: {:?}, Got: {:?}", expected, actual)
            },
            BackendVerificationError::ConnectionFailure(msg) => {
                write!(f, "❌ Connection verification FAILED: {}", msg)
            },
            BackendVerificationError::HealthCheckFailure(msg) => {
                write!(f, "❌ Health check FAILED: {}", msg)
            },
        }
    }
}

impl std::error::Error for BackendVerificationError {}

/// Global verification functions for test environment validation
pub async fn verify_test_environment() -> Result<(), BackendVerificationError> {
    println!("🔍 Verifying test environment...");
    
    // Check environment variables
    let expected_backends = std::env::var("DATABASE_BACKENDS")
        .unwrap_or_else(|_| "sqlite".to_string());
    
    println!("📊 Expected backends: {}", expected_backends);
    
    // Verify each expected backend
    for backend_name in expected_backends.split(',') {
        match backend_name.trim().to_lowercase().as_str() {
            "sqlite" => {
                let manager = crate::common::database_manager::TestDatabaseManager::new();
                let client = manager.create_client(DatabaseDialect::SQLite).await
                    .map_err(|e| BackendVerificationError::ConnectionFailure(e.to_string()))?;
                client.verify_isolation(DatabaseDialect::SQLite).await?;
                client.health_check().await?;
            },
            #[cfg(feature = "postgres")]
            "postgres" | "postgresql" => {
                let manager = crate::common::database_manager::TestDatabaseManager::new();
                let client = manager.create_client(DatabaseDialect::PostgreSQL).await
                    .map_err(|e| BackendVerificationError::ConnectionFailure(e.to_string()))?;
                client.verify_isolation(DatabaseDialect::PostgreSQL).await?;
                client.health_check().await?;
            },
            #[cfg(feature = "mysql")]
            "mysql" => {
                let manager = crate::common::database_manager::TestDatabaseManager::new();
                let client = manager.create_client(DatabaseDialect::MySQL).await
                    .map_err(|e| BackendVerificationError::ConnectionFailure(e.to_string()))?;
                client.verify_isolation(DatabaseDialect::MySQL).await?;
                client.health_check().await?;
            },
            _ => {
                return Err(BackendVerificationError::ConnectionFailure(
                    format!("Unknown backend: {}", backend_name)
                ));
            }
        }
    }
    
    println!("✅ Test environment verification complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::database_manager::TestDatabaseManager;
    use d1_rs::backends::{DatabaseBackend, QueryResult};

    type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

    #[tokio::test]
    async fn test_backend_verification_error_display() -> TestResult {
        let type_mismatch = BackendVerificationError::TypeMismatch {
            expected: DatabaseDialect::SQLite,
            actual: DatabaseDialect::SQLite,
        };
        assert!(type_mismatch.to_string().contains("Backend verification FAILED"));
        
        let connection_failure = BackendVerificationError::ConnectionFailure("test error".to_string());
        assert!(connection_failure.to_string().contains("Connection verification FAILED"));
        
        let health_check_failure = BackendVerificationError::HealthCheckFailure("test error".to_string());
        assert!(health_check_failure.to_string().contains("Health check FAILED"));
        
        Ok(())
    }

    #[tokio::test]
    async fn test_environment_verification_sqlite() -> TestResult {
        // Set environment to SQLite only
        std::env::set_var("DATABASE_BACKENDS", "sqlite");
        
        // Should pass verification
        verify_test_environment().await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        
        // Clean up
        std::env::remove_var("DATABASE_BACKENDS");
        Ok(())
    }

    #[tokio::test]
    async fn test_environment_verification_invalid_backend() -> TestResult {
        // Set environment to invalid backend
        std::env::set_var("DATABASE_BACKENDS", "invalid_db");
        
        // Should fail verification
        let result = verify_test_environment().await;
        assert!(result.is_err());
        
        if let Err(BackendVerificationError::ConnectionFailure(msg)) = result {
            assert!(msg.contains("Unknown backend: invalid_db"));
        } else {
            panic!("Expected ConnectionFailure error");
        }
        
        // Clean up
        std::env::remove_var("DATABASE_BACKENDS");
        Ok(())
    }

    #[tokio::test]
    async fn validate_sqlite_isolation() -> TestResult {
        // Ensure we're testing SQLite only
        std::env::set_var("DATABASE_BACKENDS", "sqlite");
        
        let manager = TestDatabaseManager::new();
        let client = manager.create_client(DatabaseDialect::SQLite).await?;
        
        client.verify_isolation(DatabaseDialect::SQLite).await?;
        
        // Verify connection info doesn't mention other databases
        let conn_info = client.connection_info().to_lowercase();
        assert!(!conn_info.contains("postgres"), "SQLite connection info mentions postgres");
        assert!(!conn_info.contains("mysql"), "SQLite connection info mentions mysql");
        
        println!("✅ Backend isolation validated");
        
        // Clean up
        std::env::remove_var("DATABASE_BACKENDS");
        Ok(())
    }

    #[tokio::test]
    #[cfg(feature = "postgres")]
    async fn validate_postgres_isolation() -> TestResult {
        // Only run if PostgreSQL is configured
        if std::env::var("POSTGRES_TEST_URL").is_err() {
            println!("⏭️  Skipping PostgreSQL isolation test - POSTGRES_TEST_URL not set");
            return Ok(());
        }

        std::env::set_var("DATABASE_BACKENDS", "postgres");
        std::env::set_var("POSTGRES_TEST_URL", "postgresql://test:test@localhost:5432/test");
        
        let manager = TestDatabaseManager::new();
        let client = manager.create_client(DatabaseDialect::PostgreSQL).await?;
        
        client.verify_isolation(DatabaseDialect::PostgreSQL).await?;
        
        let conn_info = client.connection_info().to_lowercase();
        assert!(conn_info.contains("postgres"), "PostgreSQL connection info should mention postgres");
        
        println!("✅ PostgreSQL isolation validated");
        
        // Clean up
        std::env::remove_var("DATABASE_BACKENDS");
        Ok(())
    }

    #[tokio::test]
    #[cfg(feature = "mysql")]
    async fn validate_mysql_isolation() -> TestResult {
        // Only run if MySQL is configured
        if std::env::var("MYSQL_TEST_URL").is_err() {
            println!("⏭️  Skipping MySQL isolation test - MYSQL_TEST_URL not set");
            return Ok(());
        }

        std::env::set_var("DATABASE_BACKENDS", "mysql");
        std::env::set_var("MYSQL_TEST_URL", "mysql://test:test@localhost:3306/test");
        
        let manager = TestDatabaseManager::new();
        let client = manager.create_client(DatabaseDialect::MySQL).await?;
        
        client.verify_isolation(DatabaseDialect::MySQL).await?;
        
        let conn_info = client.connection_info().to_lowercase();
        assert!(conn_info.contains("mysql"), "MySQL connection info should mention mysql");
        
        println!("✅ MySQL isolation validated");
        
        // Clean up
        std::env::remove_var("DATABASE_BACKENDS");
        Ok(())
    }

    #[tokio::test]
    async fn validate_feature_gates() -> TestResult {
        println!("🔍 Validating feature gate compilation...");
        
        // This test validates that conditional compilation works correctly
        #[cfg(feature = "postgres")]
        {
            use d1_rs::backends::PostgreSQLBackend;
            let _: Option<PostgreSQLBackend> = None;
            println!("✅ PostgreSQL feature gate active");
        }
        
        #[cfg(feature = "mysql")]
        {
            use d1_rs::backends::MySQLBackend;
            let _: Option<MySQLBackend> = None;
            println!("✅ MySQL feature gate active");
        }
        
        // SQLite should always be available
        use d1_rs::backends::SQLiteBackend;
        let _: Option<SQLiteBackend> = None;
        println!("✅ SQLite always available");
        
        Ok(())
    }

    #[tokio::test]
    async fn validate_query_equivalence() -> TestResult {
        use sea_query::{ColumnDef, Table, Iden, Alias as TableAlias, SqliteQueryBuilder};
        #[cfg(feature = "postgres")]
        use sea_query::PostgresQueryBuilder;
        #[cfg(feature = "mysql")]
        use sea_query::MysqlQueryBuilder;
        use crate::common::query_helpers::{build_insert_query, build_count_query, table, column};
        use d1_rs::backends::DatabaseBackend;
        
        println!("🔍 Validating query equivalence across databases...");
        
        let test_data = vec![
            ("Alice", 25, true),
            ("Bob", 30, false), 
            ("Charlie", 35, true),
        ];

        #[derive(Iden)]
        enum TestUsers {
            #[allow(dead_code)]
            Table,
            Id,
            Name,
            Age,
            Active,
        }
        
        // Test on all available databases
        let manager = TestDatabaseManager::new();
        for &dialect in manager.available_dialects() {
            println!("Testing query equivalence on {:?}", dialect);
            
            if !manager.is_available(dialect) {
                println!("⏭️  Skipping {:?} - not available", dialect);
                continue;
            }
            
            let client = manager.create_client(dialect).await?;
            
            // Create identical table structure using sea-query
            let create_table = Table::create()
                .table(TableAlias::new("test_users"))
                .if_not_exists()
                .col(ColumnDef::new(TestUsers::Id).integer().auto_increment().primary_key())
                .col(ColumnDef::new(TestUsers::Name).text().not_null())
                .col(ColumnDef::new(TestUsers::Age).integer().not_null())
                .col(ColumnDef::new(TestUsers::Active).boolean().not_null())
                .to_owned();
                
            let create_sql = match dialect {
                d1_rs::dialects::DatabaseDialect::SQLite => create_table.build(SqliteQueryBuilder),
                #[cfg(feature = "postgres")]
                d1_rs::dialects::DatabaseDialect::PostgreSQL => create_table.build(PostgresQueryBuilder),
                #[cfg(feature = "mysql")]
                d1_rs::dialects::DatabaseDialect::MySQL => create_table.build(MysqlQueryBuilder),
            };
            client.execute_schema(&create_sql).await?;
            
            // Insert test data using database-agnostic helpers
            for (name, age, active) in &test_data {
                let (insert_sql, insert_params) = build_insert_query(
                    table("test_users"),
                    vec![column("name"), column("age"), column("active")],
                    vec![
                        sea_query::Value::String(Some(Box::new(name.to_string()))),
                        sea_query::Value::Int(Some(*age)),
                        sea_query::Value::Bool(Some(*active)),
                    ],
                    dialect
                );
                client.execute_query(&insert_sql, &insert_params).await?;
            }
            
            // Verify data using database-agnostic count query
            let (count_sql, count_params) = build_count_query(table("test_users"), dialect);
            let count_result = client.execute_query(&count_sql, &count_params).await?;
            let count_rows = count_result.into_rows();
            
            // Extract count (handle different column name formats)
            let count = if let Some(row) = count_rows.first() {
                if let Some(serde_json::Value::Number(n)) = row.get("count")
                    .or_else(|| row.get("COUNT(*)"))
                    .or_else(|| row.get("COUNT")) {
                    n.as_i64().unwrap_or(0)
                } else {
                    0
                }
            } else {
                0
            };
            
            assert_eq!(count, test_data.len() as i64, 
                "Query equivalence failed for {:?}: expected {} rows, got {}", dialect, test_data.len(), count);
                
            println!("✅ Query equivalence validated for {:?}", dialect);
        }
        
        Ok(())
    }
}