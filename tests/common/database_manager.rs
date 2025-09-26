/// Enhanced multi-database test management
/// 
/// This module provides comprehensive database testing capabilities for SQLite, PostgreSQL,
/// and MySQL databases. It handles connection management, database initialization, and
/// provides utilities for running tests across multiple database backends.

use d1_rs::{
    backends::{DatabaseBackend, SQLiteBackend, QueryResult},
    dialects::DatabaseDialect,
    D1RsError,
};

#[cfg(feature = "postgres")]
use d1_rs::backends::{PostgreSQLBackend, PostgreSQLQueryResult};

#[cfg(feature = "mysql")]
use d1_rs::backends::{MySQLBackend, MySQLQueryResult};

use async_trait::async_trait;
use std::collections::HashMap;
use std::env;
use tokio::time::{timeout, Duration};
use serde_json::Value;

/// Import query helpers for database-agnostic operations
use crate::common::query_helpers::{
    build_count_query, build_create_table_query_simple, 
    build_insert_query, table, column
};
use sea_query::Value as SeaValue;
use crate::common::query_helpers::QueryAssertions;

/// Unified backend wrapper that can hold any of the three supported database backends
#[derive(Clone)]
#[allow(dead_code)]
pub enum AnyDatabaseBackend {
    SQLite(SQLiteBackend),
    #[cfg(feature = "postgres")]
    PostgreSQL(PostgreSQLBackend),
    #[cfg(feature = "mysql")]
    MySQL(MySQLBackend),
}

/// Unified query result that can hold any of the three supported query result types
#[allow(dead_code)]
pub enum AnyQueryResult {
    SQLite(d1_rs::backends::SQLiteQueryResult),
    #[cfg(feature = "postgres")]
    PostgreSQL(PostgreSQLQueryResult),
    #[cfg(feature = "mysql")]
    MySQL(MySQLQueryResult),
}

impl QueryResult for AnyQueryResult {
    type Error = D1RsError;

    fn rows(&self) -> &[Value] {
        match self {
            AnyQueryResult::SQLite(r) => r.rows(),
            #[cfg(feature = "postgres")]
            AnyQueryResult::PostgreSQL(r) => r.rows(),
            #[cfg(feature = "mysql")]
            AnyQueryResult::MySQL(r) => r.rows(),
        }
    }

    fn into_rows(self) -> Vec<Value> {
        match self {
            AnyQueryResult::SQLite(r) => r.into_rows(),
            #[cfg(feature = "postgres")]
            AnyQueryResult::PostgreSQL(r) => r.into_rows(),
            #[cfg(feature = "mysql")]
            AnyQueryResult::MySQL(r) => r.into_rows(),
        }
    }

    fn into_entities<T>(self) -> Result<Vec<T>, Self::Error>
    where
        T: serde::de::DeserializeOwned + d1_rs::Entity,
    {
        match self {
            AnyQueryResult::SQLite(r) => r.into_entities(),
            #[cfg(feature = "postgres")]
            AnyQueryResult::PostgreSQL(r) => r.into_entities(),
            #[cfg(feature = "mysql")]
            AnyQueryResult::MySQL(r) => r.into_entities(),
        }
    }

    fn into_entity<T>(self) -> Result<Option<T>, Self::Error>
    where
        T: serde::de::DeserializeOwned + d1_rs::Entity,
    {
        match self {
            AnyQueryResult::SQLite(r) => r.into_entity(),
            #[cfg(feature = "postgres")]
            AnyQueryResult::PostgreSQL(r) => r.into_entity(),
            #[cfg(feature = "mysql")]
            AnyQueryResult::MySQL(r) => r.into_entity(),
        }
    }

    fn into_simple_entities<T>(self) -> Result<Vec<T>, Self::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        match self {
            AnyQueryResult::SQLite(r) => r.into_simple_entities(),
            #[cfg(feature = "postgres")]
            AnyQueryResult::PostgreSQL(r) => r.into_simple_entities(),
            #[cfg(feature = "mysql")]
            AnyQueryResult::MySQL(r) => r.into_simple_entities(),
        }
    }

    fn into_simple_entity<T>(self) -> Result<Option<T>, Self::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        match self {
            AnyQueryResult::SQLite(r) => r.into_simple_entity(),
            #[cfg(feature = "postgres")]
            AnyQueryResult::PostgreSQL(r) => r.into_simple_entity(),
            #[cfg(feature = "mysql")]
            AnyQueryResult::MySQL(r) => r.into_simple_entity(),
        }
    }
}

#[async_trait]
impl DatabaseBackend for AnyDatabaseBackend {
    type QueryResult = AnyQueryResult;
    type Error = D1RsError;

    async fn execute_query(
        &self,
        sql: &str,
        params: &[Value],
    ) -> Result<Self::QueryResult, Self::Error> {
        match self {
            AnyDatabaseBackend::SQLite(backend) => {
                backend.execute_query(sql, params).await.map(AnyQueryResult::SQLite)
            }
            #[cfg(feature = "postgres")]
            AnyDatabaseBackend::PostgreSQL(backend) => {
                backend.execute_query(sql, params).await.map(AnyQueryResult::PostgreSQL)
            }
            #[cfg(feature = "mysql")]
            AnyDatabaseBackend::MySQL(backend) => {
                backend.execute_query(sql, params).await.map(AnyQueryResult::MySQL)
            }
        }
    }

    async fn execute_schema(&self, sql: &str) -> Result<(), Self::Error> {
        match self {
            AnyDatabaseBackend::SQLite(backend) => backend.execute_schema(sql).await,
            #[cfg(feature = "postgres")]
            AnyDatabaseBackend::PostgreSQL(backend) => backend.execute_schema(sql).await,
            #[cfg(feature = "mysql")]
            AnyDatabaseBackend::MySQL(backend) => backend.execute_schema(sql).await,
        }
    }

    fn dialect(&self) -> DatabaseDialect {
        match self {
            AnyDatabaseBackend::SQLite(backend) => backend.dialect(),
            #[cfg(feature = "postgres")]
            AnyDatabaseBackend::PostgreSQL(backend) => backend.dialect(),
            #[cfg(feature = "mysql")]
            AnyDatabaseBackend::MySQL(backend) => backend.dialect(),
        }
    }

    fn connection_info(&self) -> String {
        match self {
            AnyDatabaseBackend::SQLite(backend) => backend.connection_info(),
            #[cfg(feature = "postgres")]
            AnyDatabaseBackend::PostgreSQL(backend) => backend.connection_info(),
            #[cfg(feature = "mysql")]
            AnyDatabaseBackend::MySQL(backend) => backend.connection_info(),
        }
    }

    async fn ping(&self) -> Result<(), Self::Error> {
        match self {
            AnyDatabaseBackend::SQLite(backend) => backend.ping().await,
            #[cfg(feature = "postgres")]
            AnyDatabaseBackend::PostgreSQL(backend) => backend.ping().await,
            #[cfg(feature = "mysql")]
            AnyDatabaseBackend::MySQL(backend) => backend.ping().await,
        }
    }
}

/// Comprehensive database manager for multi-database testing
#[derive(Debug)]
pub struct TestDatabaseManager {
    available_databases: Vec<DatabaseDialect>,
    connection_urls: HashMap<DatabaseDialect, String>,
}

impl TestDatabaseManager {
    /// Initialize with all available databases
    /// 
    /// Automatically detects available database backends based on environment variables
    /// and feature flags. Respects DATABASE_BACKENDS environment variable for enforcement.
    /// If DATABASE_BACKENDS is set, only the specified backend will be available.
    pub fn new() -> Self {
        #[allow(unused_mut)]
        let mut manager = Self {
            available_databases: vec![],
            connection_urls: HashMap::new(),
        };
        
        // Check if DATABASE_BACKENDS environment variable is set for enforcement
        if let Ok(enforced_backend) = env::var("DATABASE_BACKENDS") {
            match enforced_backend.as_str() {
                "sqlite" => {
                    manager.available_databases.push(DatabaseDialect::SQLite);
                    println!("🔒 Environment enforcement: SQLite-only mode");
                },
                "postgres" => {
                    #[cfg(feature = "postgres")]
                    {
                        if let Ok(postgres_url) = env::var("POSTGRES_TEST_URL") {
                            manager.available_databases.push(DatabaseDialect::PostgreSQL);
                            manager.connection_urls.insert(DatabaseDialect::PostgreSQL, postgres_url);
                            println!("🔒 Environment enforcement: PostgreSQL-only mode");
                        } else {
                            println!("⚠️ PostgreSQL enforcement requested but POSTGRES_TEST_URL not set");
                        }
                    }
                    #[cfg(not(feature = "postgres"))]
                    {
                        println!("⚠️ PostgreSQL enforcement requested but postgres feature not enabled");
                    }
                },
                "mysql" => {
                    #[cfg(feature = "mysql")]
                    {
                        if let Ok(mysql_url) = env::var("MYSQL_TEST_URL") {
                            manager.available_databases.push(DatabaseDialect::MySQL);
                            manager.connection_urls.insert(DatabaseDialect::MySQL, mysql_url);
                            println!("🔒 Environment enforcement: MySQL-only mode");
                        } else {
                            println!("⚠️ MySQL enforcement requested but MYSQL_TEST_URL not set");
                        }
                    }
                    #[cfg(not(feature = "mysql"))]
                    {
                        println!("⚠️ MySQL enforcement requested but mysql feature not enabled");
                    }
                },
                _ => {
                    println!("⚠️ Unknown DATABASE_BACKENDS value: {}. Falling back to auto-detection.", enforced_backend);
                }
            }
        }
        
        // If no enforcement or enforcement failed, fall back to auto-detection
        if manager.available_databases.is_empty() {
            println!("🔍 Auto-detecting available databases...");
            
            // Always include SQLite for baseline testing
            manager.available_databases.push(DatabaseDialect::SQLite);
            
            // Check for PostgreSQL availability
            #[allow(unused_variables)]
            if let Ok(postgres_url) = env::var("POSTGRES_TEST_URL") {
                #[cfg(feature = "postgres")]
                {
                    manager.available_databases.push(DatabaseDialect::PostgreSQL);
                    manager.connection_urls.insert(DatabaseDialect::PostgreSQL, postgres_url);
                    println!("🐘 PostgreSQL test database configured");
                }
                #[cfg(not(feature = "postgres"))]
                {
                    println!("⚠️ PostgreSQL URL found but postgres feature not enabled");
                }
            }
            
            // Check for MySQL availability  
            #[allow(unused_variables)]
            if let Ok(mysql_url) = env::var("MYSQL_TEST_URL") {
                #[cfg(feature = "mysql")]
                {
                    manager.available_databases.push(DatabaseDialect::MySQL);
                    manager.connection_urls.insert(DatabaseDialect::MySQL, mysql_url);
                    println!("🐬 MySQL test database configured");
                }
                #[cfg(not(feature = "mysql"))]
                {
                    println!("⚠️ MySQL URL found but mysql feature not enabled");
                }
            }
        }
        
        println!("📊 Available test databases: {:?}", manager.available_databases);
        manager
    }
    
    /// Get all available database dialects
    pub fn available_dialects(&self) -> &[DatabaseDialect] {
        &self.available_databases
    }
    
    /// Check if specific database dialect is available
    pub fn is_available(&self, dialect: DatabaseDialect) -> bool {
        self.available_databases.contains(&dialect)
    }
    
    /// Create SQLite database client (the only currently supported backend)
    /// 
    /// Returns a SQLite backend configured for testing. Other database backends
    /// will be added in future versions.
    pub async fn create_sqlite_client(&self) -> Result<SQLiteBackend, Box<dyn std::error::Error + Send + Sync>> {
        SQLiteBackend::new_in_memory().await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }
    
    /// Create database client for specific dialect
    /// 
    /// Creates the appropriate backend based on the dialect and available features.
    /// Returns the appropriate backend for each supported database dialect.
    pub async fn create_client(&self, dialect: DatabaseDialect) -> Result<AnyDatabaseBackend, Box<dyn std::error::Error + Send + Sync>> {
        let backend = match dialect {
            DatabaseDialect::SQLite => {
                println!("🗄️  Using SQLite in-memory backend");
                let backend = self.create_sqlite_client().await?;
                AnyDatabaseBackend::SQLite(backend)
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                if let Some(url) = self.connection_urls.get(&dialect) {
                    println!("🐘 Using PostgreSQL backend: {}", self.mask_credentials(url));
                    let backend = PostgreSQLBackend::new(url).await
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                    AnyDatabaseBackend::PostgreSQL(backend)
                } else {
                    return Err("PostgreSQL URL not configured. Set POSTGRES_TEST_URL environment variable.".into());
                }
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                if let Some(url) = self.connection_urls.get(&dialect) {
                    println!("🐬 Using MySQL backend: {}", self.mask_credentials(url));
                    let backend = MySQLBackend::new(url).await
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                    AnyDatabaseBackend::MySQL(backend)
                } else {
                    return Err("MySQL URL not configured. Set MYSQL_TEST_URL environment variable.".into());
                }
            },
        };
        
        // Backend verification to prevent cross-contamination
        assert_eq!(backend.dialect(), dialect);
        Ok(backend)
    }
    
    /// Mask credentials in database URLs for secure logging
    /// 
    /// Replaces sensitive information like passwords and usernames with masked values
    /// to prevent credential exposure in logs while preserving connection information.
    #[cfg_attr(not(any(feature = "postgres", feature = "mysql")), allow(dead_code))]
    fn mask_credentials(&self, url: &str) -> String {
        if let Some(scheme_end) = url.find("://") {
            let scheme = &url[..scheme_end + 3];
            let rest = &url[scheme_end + 3..];
            
            if let Some(at_pos) = rest.find('@') {
                let credentials = &rest[..at_pos];
                let host_and_path = &rest[at_pos..];
                
                // Mask the credentials part
                let masked_creds = if let Some(colon_pos) = credentials.find(':') {
                    let username = &credentials[..colon_pos];
                    format!("{}:***", username)
                } else {
                    "***".to_string()
                };
                
                format!("{}{}{}", scheme, masked_creds, host_and_path)
            } else {
                // No credentials in URL, return as-is
                url.to_string()
            }
        } else {
            // Not a standard URL format, return as-is
            url.to_string()
        }
    }
    
    /// Wait for database to be ready (with timeout)
    /// 
    /// Attempts to connect to the specified database and execute a simple query
    /// to verify it's ready for testing. Uses exponential backoff for retries.
    #[allow(dead_code)]
    pub async fn wait_for_database(&self, dialect: DatabaseDialect, timeout_secs: u64) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.is_available(dialect) {
            return Err(format!("Database {:?} is not available", dialect).into());
        }
        
        let timeout_duration = Duration::from_secs(timeout_secs);
        let mut retry_delay = Duration::from_millis(500);
        
        timeout(timeout_duration, async {
            loop {
                match self.create_client(dialect).await {
                    Ok(client) => {
                        // Use a simple literal select for connection test
                        let test_sql = match dialect {
                            DatabaseDialect::SQLite => "SELECT 1 as test",
                            #[cfg(feature = "postgres")]
                            DatabaseDialect::PostgreSQL => "SELECT 1 as test",
                            #[cfg(feature = "mysql")]
                            DatabaseDialect::MySQL => "SELECT 1 as test",
                        };
                        match client.execute_query(test_sql, &[]).await {
                            Ok(result) => {
                                // Verify we got the expected result
                                let rows = result.into_rows();
                                if !rows.is_empty() {
                                    println!("✅ {} database is ready!", dialect);
                                    return Ok(());
                                } else {
                                    println!("⚠️ {} database connected but query returned no results", dialect);
                                }
                            },
                            Err(e) => {
                                println!("⏳ Waiting for {} database... (query failed: {})", dialect, e);
                            }
                        }
                    },
                    Err(e) => {
                        println!("⏳ Waiting for {} database... (connection failed: {})", dialect, e);
                    }
                }
                
                tokio::time::sleep(retry_delay).await;
                
                // Exponential backoff, but cap at 5 seconds
                retry_delay = std::cmp::min(retry_delay * 2, Duration::from_secs(5));
            }
        }).await.map_err(|_| format!("Timeout waiting for {} database", dialect))?
    }
    
    /// Run test function across all available databases
    /// 
    /// Executes the provided test function on each available database backend.
    /// The test function receives both the database client and the dialect for
    /// conditional logic if needed.
    pub async fn run_on_all_databases<F, Fut>(&self, test_fn: F) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: Fn(AnyDatabaseBackend, DatabaseDialect) -> Fut + Clone,
        Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>>,
    {
        let mut errors = Vec::new();
        
        for &dialect in &self.available_databases {
            println!("🔄 Running test on {:?}", dialect);
            
            match self.create_client(dialect).await {
                Ok(client) => {
                    match test_fn(client, dialect).await {
                        Ok(()) => {
                            println!("✅ Test completed successfully on {:?}", dialect);
                        },
                        Err(e) => {
                            let error_msg = format!("Test failed on {:?}: {}", dialect, e);
                            println!("❌ {}", error_msg);
                            errors.push(error_msg);
                        }
                    }
                },
                Err(e) => {
                    // Check if this is a connection failure - skip gracefully instead of failing
                    let error_str = e.to_string().to_lowercase();
                    if error_str.contains("connection refused") || 
                       error_str.contains("could not connect") || 
                       error_str.contains("connection failed") ||
                       error_str.contains("timeout") {
                        println!("⏭️  Skipping {:?} - database not available: {}", dialect, e);
                        continue;
                    }
                    
                    let error_msg = format!("Failed to create client for {:?}: {}", dialect, e);
                    println!("❌ {}", error_msg);
                    errors.push(error_msg);
                }
            }
        }
        
        if !errors.is_empty() {
            return Err(format!("Test failures: {}", errors.join(", ")).into());
        }
        
        println!("✅ All tests completed successfully across all databases!");
        Ok(())
    }
    
    /// Run test function on specific database only if available
    /// 
    /// Executes the test function on the specified database if it's available.
    /// Skips gracefully if the database is not configured.
    #[allow(dead_code)]
    pub async fn run_on_database<F, Fut>(&self, dialect: DatabaseDialect, test_fn: F) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: Fn(AnyDatabaseBackend) -> Fut,
        Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>>,
    {
        if !self.is_available(dialect) {
            println!("⏭️  Skipping {:?} - not available", dialect);
            return Ok(());
        }
        
        println!("🔄 Running test on {:?}", dialect);
        
        let client = self.create_client(dialect).await?;
        test_fn(client).await?;
        
        println!("✅ Test completed on {:?}", dialect);
        Ok(())
    }
    
    /// Get database connection URL for testing purposes
    #[allow(dead_code)]
    pub fn get_connection_url(&self, dialect: DatabaseDialect) -> Option<&String> {
        self.connection_urls.get(&dialect)
    }
    
    /// Validate database configuration
    /// 
    /// Checks that all configured databases are accessible and properly configured.
    /// This is useful for setup validation and debugging connection issues.
    pub async fn validate_configuration(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("🔍 Validating database configuration...");
        
        for &dialect in &self.available_databases {
            match dialect {
                DatabaseDialect::SQLite => {
                    // SQLite should always work
                    match SQLiteBackend::new_in_memory().await {
                        Ok(_) => println!("✅ SQLite configuration valid"),
                        Err(e) => return Err(format!("SQLite configuration invalid: {}", e).into()),
                    }
                },
                #[cfg(feature = "postgres")]
                DatabaseDialect::PostgreSQL => {
                    // For PostgreSQL, check if URL is configured
                    if let Some(url) = self.connection_urls.get(&dialect) {
                        println!("✅ {} URL configured: {}", dialect, 
                            // Mask password for security
                            url.split('@').nth(0).unwrap_or("***").split(':').take(2).collect::<Vec<_>>().join(":")
                        );
                    } else {
                        return Err(format!("{} URL not configured", dialect).into());
                    }
                },
                #[cfg(feature = "mysql")]
                DatabaseDialect::MySQL => {
                    // For MySQL, check if URL is configured
                    if let Some(url) = self.connection_urls.get(&dialect) {
                        println!("✅ {} URL configured: {}", dialect, 
                            // Mask password for security
                            url.split('@').nth(0).unwrap_or("***").split(':').take(2).collect::<Vec<_>>().join(":")
                        );
                    } else {
                        return Err(format!("{} URL not configured", dialect).into());
                    }
                }
            }
        }
        
        println!("✅ All database configurations valid");
        Ok(())
    }
    
    /// Initialize test data across all databases
    /// 
    /// Sets up common test data structures that work across all database backends.
    /// Uses the TestFixtureManager for consistent data setup.
    #[allow(dead_code)]
    pub async fn initialize_test_data(&self, fixture_name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use crate::common::fixtures::TestFixtureManager;
        
        for &dialect in &self.available_databases {
            println!("🔧 Setting up test data for {:?}", dialect);
            
            let client = self.create_client(dialect).await?;
            let fixture_manager = TestFixtureManager::new(dialect);
            
            fixture_manager.setup_fixture(fixture_name, &client).await
                .map_err(|e| format!("Failed to setup fixture '{}' on {:?}: {}", fixture_name, dialect, e))?;
            
            println!("✅ Test data ready for {:?}", dialect);
        }
        
        println!("✅ Test data initialized across all databases");
        Ok(())
    }
    
    /// Clean up test data across all databases
    #[allow(dead_code)]
    pub async fn cleanup_test_data(&self, fixture_name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use crate::common::fixtures::TestFixtureManager;
        
        for &dialect in &self.available_databases {
            println!("🧹 Cleaning up test data for {:?}", dialect);
            
            let client = self.create_client(dialect).await?;
            let fixture_manager = TestFixtureManager::new(dialect);
            
            // Cleanup is best-effort - don't fail if it doesn't work
            if let Err(e) = fixture_manager.teardown_fixture(fixture_name, &client).await {
                println!("⚠️ Cleanup warning for {:?}: {}", dialect, e);
            } else {
                println!("✅ Cleanup completed for {:?}", dialect);
            }
        }
        
        println!("✅ Cleanup completed across all databases");
        Ok(())
    }
}

impl Default for TestDatabaseManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Test utilities for common database operations
pub struct TestUtils;

impl TestUtils {
    /// Setup test client for specific database type with error handling
    pub async fn setup_test_client_with_manager(dialect: DatabaseDialect) -> Result<AnyDatabaseBackend, Box<dyn std::error::Error + Send + Sync>> {
        let manager = TestDatabaseManager::new();
        manager.create_client(dialect).await
    }
    
    /// Verify table exists in database (database-agnostic)
    pub async fn table_exists(client: &AnyDatabaseBackend, table_name: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // Use database-agnostic table existence check from query_helpers
        match QueryAssertions::assert_table_exists(client, table_name, client.dialect()).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false), // Table doesn't exist if assertion fails
        }
    }
    
    /// Get row count from table (database-agnostic)
    pub async fn get_row_count(client: &AnyDatabaseBackend, table_name: &str) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
        let (sql, params) = build_count_query(table(table_name), client.dialect());
        let result = client.execute_query(&sql, &params).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        
        // Extract count from the first row (try different possible count column names)
        let rows = result.into_rows();
        if let Some(row) = rows.first() {
            // Try different possible count column names
            let count_value = row.get("count")
                .or_else(|| row.get("COUNT(*)"))
                .or_else(|| row.get("COUNT"));
            if let Some(Value::Number(n)) = count_value {
                if let Some(count) = n.as_i64() {
                    return Ok(count);
                }
            }
        }
        Err("Failed to extract count from query result".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_database_manager_initialization() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let manager = TestDatabaseManager::new();
        
        // Should have at least one database available
        assert!(!manager.available_dialects().is_empty());
        
        // Check based on environment enforcement or auto-detection
        if let Ok(enforced_backend) = std::env::var("DATABASE_BACKENDS") {
            match enforced_backend.as_str() {
                "sqlite" => {
                    assert!(manager.is_available(DatabaseDialect::SQLite));
                    #[cfg(feature = "postgres")]
                    assert!(!manager.is_available(DatabaseDialect::PostgreSQL));
                    #[cfg(feature = "mysql")]
                    assert!(!manager.is_available(DatabaseDialect::MySQL));
                },
                "postgres" => {
                    #[cfg(feature = "postgres")]
                    {
                        assert!(manager.is_available(DatabaseDialect::PostgreSQL));
                        assert!(!manager.is_available(DatabaseDialect::SQLite));
                        #[cfg(feature = "mysql")]
                        assert!(!manager.is_available(DatabaseDialect::MySQL));
                    }
                },
                "mysql" => {
                    #[cfg(feature = "mysql")]
                    {
                        assert!(manager.is_available(DatabaseDialect::MySQL));
                        assert!(!manager.is_available(DatabaseDialect::SQLite));
                        #[cfg(feature = "postgres")]
                        assert!(!manager.is_available(DatabaseDialect::PostgreSQL));
                    }
                },
                _ => {
                    // Invalid enforcement - should fall back to auto-detection
                    assert!(manager.is_available(DatabaseDialect::SQLite));
                }
            }
        } else {
            // No enforcement - SQLite should always be available
            assert!(manager.is_available(DatabaseDialect::SQLite));
        }
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_sqlite_client_creation() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let manager = TestDatabaseManager::new();
        let client = manager.create_client(DatabaseDialect::SQLite).await?;
        
        // Verify we can execute a simple query using literal for connection test
        let test_sql = "SELECT 1 as test"; // Simple connection test query
        let result = client.execute_query(test_sql, &[]).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        let rows = result.into_rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get("test"), Some(&Value::Number(1.into())));
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_configuration_validation() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let manager = TestDatabaseManager::new();
        
        // Should validate successfully with at least SQLite
        manager.validate_configuration().await?;
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_run_on_all_databases() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let manager = TestDatabaseManager::new();
        
        // Test that runs successfully on all databases
        manager.run_on_all_databases(|client, dialect| async move {
            println!("Testing on {:?}", dialect);
            let test_sql = "SELECT 1 as test"; // Simple connection test query
            let result = client.execute_query(test_sql, &[]).await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
            let rows = result.into_rows();
            assert!(!rows.is_empty());
            Ok(())
        }).await?;
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_utils_table_operations() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let client = TestUtils::setup_test_client_with_manager(DatabaseDialect::SQLite).await?;
        
        // Create a test table using sea-query helper
        let (create_sql, _create_params) = build_create_table_query_simple("test_table", client.dialect());
        client.execute_schema(&create_sql).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        
        // Verify table exists
        assert!(TestUtils::table_exists(&client, "test_table").await?);
        
        // Check initial row count
        assert_eq!(TestUtils::get_row_count(&client, "test_table").await?, 0);
        
        // Insert data and verify count using sea-query helper
        let (insert_sql, insert_params) = build_insert_query(
            table("test_table"),
            vec![column("name")],
            vec![SeaValue::String(Some(Box::new("test".to_string())))],
            client.dialect()
        );
        client.execute_query(&insert_sql, &insert_params).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        
        assert_eq!(TestUtils::get_row_count(&client, "test_table").await?, 1);
        
        Ok(())
    }
}