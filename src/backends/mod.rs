use serde_json::Value;
use std::collections::HashMap;
use async_trait::async_trait;
use crate::Entity;
use crate::dialects::DatabaseDialect;

// Database configuration structs
pub mod config;
pub use config::{DatabaseConfig, PoolConfig, SslConfig, SslMode, TimeoutConfig, ConfigError};

// SQLite backend implementation
pub mod sqlite;
pub use sqlite::{SQLiteBackend, SQLiteQueryResult};

/// Core trait for database query results
/// 
/// This trait abstracts over different database backend query results,
/// providing a unified interface for data access and type conversion.
pub trait QueryResult: Send + Sync {
    /// Associated error type for this query result
    type Error: std::error::Error + Send + Sync + 'static;
    
    /// Get a reference to the raw rows data
    /// 
    /// Returns the underlying row data as a slice of JSON values
    fn rows(&self) -> &[Value];
    
    /// Consume the result and return the raw rows data
    /// 
    /// Takes ownership of the result and returns the rows vector
    fn into_rows(self) -> Vec<Value>;
    
    /// Get the number of rows in the result
    /// 
    /// Equivalent to `rows().len()` but may be optimized in implementations
    fn len(&self) -> usize {
        self.rows().len()
    }
    
    /// Check if the result contains no rows
    /// 
    /// Equivalent to `rows().is_empty()` but may be optimized in implementations
    fn is_empty(&self) -> bool {
        self.rows().is_empty()
    }
    
    /// Convert rows into a vector of entities with proper type conversion
    /// 
    /// This method handles entity-specific conversions like boolean fields
    /// and datetime formatting that are required for proper deserialization.
    fn into_entities<T>(self) -> std::result::Result<Vec<T>, Self::Error>
    where
        T: serde::de::DeserializeOwned + Entity;
        
    /// Convert the first row into an entity with proper type conversion
    /// 
    /// Returns None if there are no rows, otherwise converts the first row
    /// using entity-specific conversion logic.
    fn into_entity<T>(self) -> std::result::Result<Option<T>, Self::Error>
    where
        T: serde::de::DeserializeOwned + Entity;
        
    /// Convert rows into simple types without entity-specific conversions
    /// 
    /// This method is for types that don't implement Entity and don't need
    /// special conversion logic (e.g., simple structs, migration records).
    fn into_simple_entities<T>(self) -> std::result::Result<Vec<T>, Self::Error>
    where
        T: serde::de::DeserializeOwned;
        
    /// Convert the first row into a simple type without entity-specific conversions
    /// 
    /// Returns None if there are no rows, otherwise converts the first row
    /// using standard JSON deserialization.
    fn into_simple_entity<T>(self) -> std::result::Result<Option<T>, Self::Error>
    where
        T: serde::de::DeserializeOwned;
        
    /// Extract count value from a COUNT query result
    /// 
    /// Expects the result to contain a single row with a single numeric column
    /// representing a count value. Returns 0 if no valid count is found.
    fn extract_count(&self) -> std::result::Result<i64, Self::Error> {
        if let Some(row) = self.rows().first() {
            if let Value::Object(obj) = row {
                // Try to find a count column by common names
                for key in ["count", "COUNT(*)", "count(*)", "COUNT", "c"] {
                    if let Some(Value::Number(n)) = obj.get(key) {
                        if let Some(count) = n.as_i64() {
                            return Ok(count);
                        }
                    }
                }
                
                // If no named count column, try the first numeric value
                for value in obj.values() {
                    if let Value::Number(n) = value {
                        if let Some(count) = n.as_i64() {
                            return Ok(count);
                        }
                    }
                }
            }
        }
        Ok(0)
    }
    
    /// Extract ID value from an INSERT or similar operation result
    /// 
    /// Looks for common ID column names and returns the first valid ID found.
    /// Returns None if no valid ID is found in the result.
    fn extract_id(&self) -> std::result::Result<Option<i64>, Self::Error> {
        if let Some(row) = self.rows().first() {
            if let Value::Object(obj) = row {
                // Try to find an ID column by common names
                for key in ["id", "ID", "rowid", "ROWID", "last_insert_rowid()"] {
                    if let Some(value) = obj.get(key) {
                        match value {
                            Value::Number(n) => {
                                if let Some(id) = n.as_i64() {
                                    return Ok(Some(id));
                                }
                            }
                            Value::String(s) => {
                                if let Ok(id) = s.parse::<i64>() {
                                    return Ok(Some(id));
                                }
                            }
                            _ => continue,
                        }
                    }
                }
            }
        }
        Ok(None)
    }
    
    /// Extract string values from the result rows
    /// 
    /// Collects all string values from the first column of each row.
    /// Useful for queries that return lists of names, identifiers, etc.
    fn extract_strings(&self) -> std::result::Result<Vec<String>, Self::Error> {
        let mut strings = Vec::new();
        
        for row in self.rows() {
            if let Value::Object(obj) = row {
                // Get the first value from the row
                if let Some((_, value)) = obj.iter().next() {
                    match value {
                        Value::String(s) => strings.push(s.clone()),
                        Value::Number(n) => strings.push(n.to_string()),
                        Value::Bool(b) => strings.push(b.to_string()),
                        Value::Null => strings.push("NULL".to_string()),
                        _ => strings.push(value.to_string()),
                    }
                }
            } else if let Value::String(s) = row {
                strings.push(s.clone());
            }
        }
        
        Ok(strings)
    }
    
    /// Convert the first row to a HashMap for dynamic field access
    /// 
    /// Useful when you need to access fields dynamically without knowing
    /// the structure at compile time.
    fn into_hashmap(self) -> std::result::Result<Option<HashMap<String, Value>>, Self::Error>
    where
        Self: Sized,
    {
        let rows = self.into_rows();
        if let Some(row) = rows.into_iter().next() {
            if let Value::Object(obj) = row {
                let hashmap: HashMap<String, Value> = obj.into_iter().collect();
                return Ok(Some(hashmap));
            }
        }
        Ok(None)
    }
    
    /// Get the column names from the first row
    /// 
    /// Returns the keys from the first row object, representing column names.
    /// Returns empty vector if there are no rows or the first row is not an object.
    fn column_names(&self) -> Vec<String> {
        if let Some(Value::Object(obj)) = self.rows().first() {
            obj.keys().cloned().collect()
        } else {
            Vec::new()
        }
    }
    
    /// Check if a specific column exists in the result
    /// 
    /// Checks the first row for the presence of the specified column name.
    fn has_column(&self, column_name: &str) -> bool {
        if let Some(Value::Object(obj)) = self.rows().first() {
            obj.contains_key(column_name)
        } else {
            false
        }
    }
}

/// Backend-agnostic error types for QueryResult operations
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Connection error: {0}")]
    Connection(String),
    
    #[error("Query error: {0}")]
    Query(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Type conversion error: {0}")]
    TypeConversion(String),
    
    #[error("Invalid data: {0}")]
    InvalidData(String),
}

/// Core trait for database backend implementations
/// 
/// This trait defines the interface that all database backends must implement,
/// providing a unified API for query execution, schema operations, and metadata access.
/// It supports multiple databases (SQLite, PostgreSQL, MySQL) through feature flags
/// and conditional compilation.
#[async_trait]
pub trait DatabaseBackend: Send + Sync + Clone {
    /// Associated query result type that implements QueryResult
    type QueryResult: QueryResult;
    
    /// Associated error type for backend operations
    type Error: std::error::Error + Send + Sync + 'static;
    
    /// Execute a parameterized query and return results
    /// 
    /// This method handles SELECT, INSERT, UPDATE, and DELETE operations
    /// with proper parameter binding to prevent SQL injection.
    /// 
    /// # Arguments
    /// * `sql` - The SQL query string with parameter placeholders
    /// * `params` - Parameter values to bind to the query
    /// 
    /// # Returns
    /// Query results wrapped in the backend-specific QueryResult type
    async fn execute_query(
        &self, 
        sql: &str, 
        params: &[Value]
    ) -> std::result::Result<Self::QueryResult, Self::Error>;
    
    /// Execute schema operations (DDL statements)
    /// 
    /// This method handles CREATE, ALTER, DROP, and other schema modification
    /// operations that don't return result sets. It's separated from execute_query
    /// for clear distinction between data and schema operations.
    /// 
    /// # Arguments
    /// * `sql` - The DDL statement to execute
    /// 
    /// # Returns
    /// Success or error indication
    async fn execute_schema(&self, sql: &str) -> std::result::Result<(), Self::Error>;
    
    /// Get the database dialect for this backend
    /// 
    /// Returns the DatabaseDialect enum value indicating which database
    /// this backend connects to (SQLite, PostgreSQL, or MySQL).
    /// This is used for dialect-specific SQL generation and feature detection.
    fn dialect(&self) -> DatabaseDialect;
    
    /// Get connection information for debugging and monitoring
    /// 
    /// Returns a human-readable string describing the connection,
    /// such as database path for SQLite or connection string details
    /// for PostgreSQL/MySQL (without sensitive credentials).
    fn connection_info(&self) -> String;
    
    /// Perform a health check on the database connection
    /// 
    /// Executes a simple query to verify the connection is alive
    /// and the database is responding. This is useful for connection
    /// pooling and service health monitoring.
    async fn ping(&self) -> std::result::Result<(), Self::Error>;
}

// Implement QueryResult for the existing D1QueryResult to maintain full compatibility
impl QueryResult for crate::db::D1QueryResult {
    type Error = crate::D1RsError;
    
    fn rows(&self) -> &[Value] {
        &self.rows
    }
    
    fn into_rows(self) -> Vec<Value> {
        self.rows
    }
    
    fn len(&self) -> usize {
        self.rows.len()
    }
    
    fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    
    fn into_entities<T>(self) -> std::result::Result<Vec<T>, Self::Error>
    where
        T: serde::de::DeserializeOwned + Entity,
    {
        self.rows
            .into_iter()
            .map(|row| {
                let converted = T::convert_from_sqlite(row);
                serde_json::from_value(converted)
                    .map_err(|e| crate::D1RsError::SerializationError(e.to_string()))
            })
            .collect()
    }
    
    fn into_entity<T>(mut self) -> std::result::Result<Option<T>, Self::Error>
    where
        T: serde::de::DeserializeOwned + Entity,
    {
        if let Some(row) = self.rows.pop() {
            let converted = T::convert_from_sqlite(row);
            let entity = serde_json::from_value(converted)
                .map_err(|e| crate::D1RsError::SerializationError(e.to_string()))?;
            Ok(Some(entity))
        } else {
            Ok(None)
        }
    }
    
    fn into_simple_entities<T>(self) -> std::result::Result<Vec<T>, Self::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.rows
            .into_iter()
            .map(|row| {
                serde_json::from_value(row)
                    .map_err(|e| crate::D1RsError::SerializationError(e.to_string()))
            })
            .collect()
    }
    
    fn into_simple_entity<T>(mut self) -> std::result::Result<Option<T>, Self::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        if let Some(row) = self.rows.pop() {
            let entity = serde_json::from_value(row)
                .map_err(|e| crate::D1RsError::SerializationError(e.to_string()))?;
            Ok(Some(entity))
        } else {
            Ok(None)
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Create a test implementation of QueryResult for testing
    #[derive(Debug)]
    struct TestQueryResult {
        rows: Vec<Value>,
    }

    impl QueryResult for TestQueryResult {
        type Error = crate::D1RsError;

        fn rows(&self) -> &[Value] {
            &self.rows
        }

        fn into_rows(self) -> Vec<Value> {
            self.rows
        }

        fn into_entities<T>(self) -> std::result::Result<Vec<T>, Self::Error>
        where
            T: serde::de::DeserializeOwned + Entity,
        {
            self.rows
                .into_iter()
                .map(|row| {
                    let converted = T::convert_from_sqlite(row);
                    serde_json::from_value(converted)
                        .map_err(|e| crate::D1RsError::SerializationError(e.to_string()))
                })
                .collect()
        }

        fn into_entity<T>(mut self) -> std::result::Result<Option<T>, Self::Error>
        where
            T: serde::de::DeserializeOwned + Entity,
        {
            if let Some(row) = self.rows.pop() {
                let converted = T::convert_from_sqlite(row);
                let entity = serde_json::from_value(converted)
                    .map_err(|e| crate::D1RsError::SerializationError(e.to_string()))?;
                Ok(Some(entity))
            } else {
                Ok(None)
            }
        }

        fn into_simple_entities<T>(self) -> std::result::Result<Vec<T>, Self::Error>
        where
            T: serde::de::DeserializeOwned,
        {
            self.rows
                .into_iter()
                .map(|row| {
                    serde_json::from_value(row)
                        .map_err(|e| crate::D1RsError::SerializationError(e.to_string()))
                })
                .collect()
        }

        fn into_simple_entity<T>(mut self) -> std::result::Result<Option<T>, Self::Error>
        where
            T: serde::de::DeserializeOwned,
        {
            if let Some(row) = self.rows.pop() {
                let entity = serde_json::from_value(row)
                    .map_err(|e| crate::D1RsError::SerializationError(e.to_string()))?;
                Ok(Some(entity))
            } else {
                Ok(None)
            }
        }
    }

    #[test]
    fn test_query_result_basic_operations() {
        let result = TestQueryResult {
            rows: vec![
                json!({"id": 1, "name": "Alice"}),
                json!({"id": 2, "name": "Bob"}),
            ],
        };

        assert_eq!(result.len(), 2);
        assert!(!result.is_empty());
        assert_eq!(result.rows().len(), 2);
        
        let column_names = result.column_names();
        assert!(column_names.contains(&"id".to_string()));
        assert!(column_names.contains(&"name".to_string()));
        
        assert!(result.has_column("id"));
        assert!(result.has_column("name"));
        assert!(!result.has_column("nonexistent"));
    }

    #[test]
    fn test_query_result_empty() {
        let result = TestQueryResult {
            rows: vec![],
        };

        assert_eq!(result.len(), 0);
        assert!(result.is_empty());
        assert_eq!(result.rows().len(), 0);
        assert_eq!(result.column_names().len(), 0);
        assert!(!result.has_column("any"));
    }

    #[test]
    fn test_extract_count() {
        // Test with named count column
        let result = TestQueryResult {
            rows: vec![json!({"count": 42})],
        };
        assert_eq!(result.extract_count().unwrap(), 42);

        // Test with COUNT(*) column
        let result = TestQueryResult {
            rows: vec![json!({"COUNT(*)": 100})],
        };
        assert_eq!(result.extract_count().unwrap(), 100);

        // Test with no rows
        let result = TestQueryResult {
            rows: vec![],
        };
        assert_eq!(result.extract_count().unwrap(), 0);

        // Test with first numeric value
        let result = TestQueryResult {
            rows: vec![json!({"total_users": 25, "active_users": 15})],
        };
        let count = result.extract_count().unwrap();
        assert!(count == 25 || count == 15); // Either is acceptable as first numeric
    }

    #[test]
    fn test_extract_id() {
        // Test with id column
        let result = TestQueryResult {
            rows: vec![json!({"id": 123, "name": "test"})],
        };
        assert_eq!(result.extract_id().unwrap(), Some(123));

        // Test with ROWID column
        let result = TestQueryResult {
            rows: vec![json!({"ROWID": 456})],
        };
        assert_eq!(result.extract_id().unwrap(), Some(456));

        // Test with string ID
        let result = TestQueryResult {
            rows: vec![json!({"id": "789"})],
        };
        assert_eq!(result.extract_id().unwrap(), Some(789));

        // Test with no ID
        let result = TestQueryResult {
            rows: vec![json!({"name": "test"})],
        };
        assert_eq!(result.extract_id().unwrap(), None);

        // Test with no rows
        let result = TestQueryResult {
            rows: vec![],
        };
        assert_eq!(result.extract_id().unwrap(), None);
    }

    #[test]
    fn test_extract_strings() {
        let result = TestQueryResult {
            rows: vec![
                json!({"name": "Alice"}),
                json!({"name": "Bob"}),
                json!({"name": "Charlie"}),
            ],
        };
        
        let strings = result.extract_strings().unwrap();
        assert_eq!(strings, vec!["Alice", "Bob", "Charlie"]);

        // Test with mixed types
        let result = TestQueryResult {
            rows: vec![
                json!({"value": "text"}),
                json!({"value": 42}),
                json!({"value": true}),
                json!({"value": null}),
            ],
        };
        
        let strings = result.extract_strings().unwrap();
        assert_eq!(strings, vec!["text", "42", "true", "NULL"]);

        // Test with empty result
        let result = TestQueryResult {
            rows: vec![],
        };
        
        let strings = result.extract_strings().unwrap();
        assert!(strings.is_empty());
    }

    #[test]
    fn test_into_hashmap() {
        let result = TestQueryResult {
            rows: vec![json!({"id": 1, "name": "Alice", "active": true})],
        };

        let hashmap = result.into_hashmap().unwrap().unwrap();
        assert_eq!(hashmap.get("id").unwrap(), &json!(1));
        assert_eq!(hashmap.get("name").unwrap(), &json!("Alice"));
        assert_eq!(hashmap.get("active").unwrap(), &json!(true));

        // Test with empty result
        let result = TestQueryResult {
            rows: vec![],
        };
        
        let hashmap = result.into_hashmap().unwrap();
        assert!(hashmap.is_none());
    }

    #[test]
    fn test_into_rows() {
        let expected_rows = vec![
            json!({"id": 1, "name": "Alice"}),
            json!({"id": 2, "name": "Bob"}),
        ];

        let result = TestQueryResult {
            rows: expected_rows.clone(),
        };

        let actual_rows = result.into_rows();
        assert_eq!(actual_rows, expected_rows);
    }

    #[derive(Debug, serde::Deserialize, PartialEq)]
    struct TestRecord {
        id: i64,
        name: String,
    }

    #[test]
    fn test_into_simple_entities() {
        let result = TestQueryResult {
            rows: vec![
                json!({"id": 1, "name": "Alice"}),
                json!({"id": 2, "name": "Bob"}),
            ],
        };

        let records: Vec<TestRecord> = result.into_simple_entities().unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0], TestRecord { id: 1, name: "Alice".to_string() });
        assert_eq!(records[1], TestRecord { id: 2, name: "Bob".to_string() });
    }

    #[test]
    fn test_into_simple_entity() {
        let result = TestQueryResult {
            rows: vec![json!({"id": 1, "name": "Alice"})],
        };

        let record: Option<TestRecord> = result.into_simple_entity().unwrap();
        assert_eq!(record, Some(TestRecord { id: 1, name: "Alice".to_string() }));

        // Test with empty result
        let result = TestQueryResult {
            rows: vec![],
        };

        let record: Option<TestRecord> = result.into_simple_entity().unwrap();
        assert_eq!(record, None);
    }

    #[test]
    fn test_backend_error_display() {
        let error = BackendError::Database("connection failed".to_string());
        assert_eq!(error.to_string(), "Database error: connection failed");

        let error = BackendError::Query("syntax error".to_string());
        assert_eq!(error.to_string(), "Query error: syntax error");

        let error = BackendError::Serialization("invalid JSON".to_string());
        assert_eq!(error.to_string(), "Serialization error: invalid JSON");
    }

    #[test]
    fn test_backend_error_all_variants() {
        let errors = vec![
            BackendError::Database("db error".to_string()),
            BackendError::Connection("conn error".to_string()),
            BackendError::Query("query error".to_string()),
            BackendError::Serialization("serde error".to_string()),
            BackendError::TypeConversion("type error".to_string()),
            BackendError::InvalidData("invalid data".to_string()),
        ];

        let expected = vec![
            "Database error: db error",
            "Connection error: conn error",
            "Query error: query error",
            "Serialization error: serde error",
            "Type conversion error: type error",
            "Invalid data: invalid data",
        ];

        for (error, expected_msg) in errors.iter().zip(expected.iter()) {
            assert_eq!(&error.to_string(), expected_msg);
        }
    }

    // Mock DatabaseBackend implementation for testing
    #[derive(Clone)]
    struct MockDatabaseBackend {
        pub dialect: DatabaseDialect,
        pub connection_info: String,
        pub should_fail_query: bool,
        pub should_fail_schema: bool,
        pub should_fail_ping: bool,
        pub mock_rows: Vec<Value>,
    }

    impl MockDatabaseBackend {
        fn new() -> Self {
            Self {
                dialect: DatabaseDialect::SQLite,
                connection_info: "mock://test-database".to_string(),
                should_fail_query: false,
                should_fail_schema: false,
                should_fail_ping: false,
                mock_rows: vec![],
            }
        }

        fn with_dialect(mut self, dialect: DatabaseDialect) -> Self {
            self.dialect = dialect;
            self
        }

        fn with_mock_data(mut self, rows: Vec<Value>) -> Self {
            self.mock_rows = rows;
            self
        }

        fn with_query_failure(mut self) -> Self {
            self.should_fail_query = true;
            self
        }

        fn with_schema_failure(mut self) -> Self {
            self.should_fail_schema = true;
            self
        }

        fn with_ping_failure(mut self) -> Self {
            self.should_fail_ping = true;
            self
        }
    }

    #[async_trait]
    impl DatabaseBackend for MockDatabaseBackend {
        type QueryResult = TestQueryResult;
        type Error = BackendError;

        async fn execute_query(
            &self, 
            sql: &str, 
            params: &[Value]
        ) -> std::result::Result<Self::QueryResult, Self::Error> {
            if self.should_fail_query {
                return Err(BackendError::Query("Mock query failure".to_string()));
            }

            // Simple mock: return different data based on SQL content
            let rows = if sql.contains("SELECT COUNT") {
                vec![json!({"count": params.len()})]
            } else if sql.contains("SELECT") {
                self.mock_rows.clone()
            } else if sql.contains("INSERT") || sql.contains("UPDATE") || sql.contains("DELETE") {
                vec![json!({"changes": 1, "last_insert_rowid": 42})]
            } else {
                vec![]
            };

            Ok(TestQueryResult { rows })
        }

        async fn execute_schema(&self, _sql: &str) -> std::result::Result<(), Self::Error> {
            if self.should_fail_schema {
                Err(BackendError::Database("Mock schema failure".to_string()))
            } else {
                Ok(())
            }
        }

        fn dialect(&self) -> DatabaseDialect {
            self.dialect
        }

        fn connection_info(&self) -> String {
            self.connection_info.clone()
        }

        async fn ping(&self) -> std::result::Result<(), Self::Error> {
            if self.should_fail_ping {
                Err(BackendError::Connection("Mock ping failure".to_string()))
            } else {
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn test_database_backend_trait_compilation() {
        // This test ensures the trait can be compiled and used
        let backend = MockDatabaseBackend::new();
        
        // Test dialect
        assert_eq!(backend.dialect(), DatabaseDialect::SQLite);
        
        // Test connection info
        assert_eq!(backend.connection_info(), "mock://test-database");
        
        // Test successful ping
        let result = backend.ping().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_database_backend_query_execution() {
        let backend = MockDatabaseBackend::new()
            .with_mock_data(vec![
                json!({"id": 1, "name": "Alice"}),
                json!({"id": 2, "name": "Bob"}),
            ]);

        // Test SELECT query
        let result = backend.execute_query("SELECT * FROM users", &[]).await;
        assert!(result.is_ok());
        let query_result = result.unwrap();
        assert_eq!(query_result.len(), 2);
        
        // Test INSERT query
        let result = backend.execute_query(
            "INSERT INTO users (name) VALUES (?)", 
            &[json!("Charlie")]
        ).await;
        assert!(result.is_ok());
        
        // Test COUNT query with parameters
        let result = backend.execute_query(
            "SELECT COUNT(*) FROM users WHERE active = ?", 
            &[json!(true), json!(false)]
        ).await;
        assert!(result.is_ok());
        let query_result = result.unwrap();
        let count = query_result.extract_count().unwrap();
        assert_eq!(count, 2); // Should equal params.len()
    }

    #[tokio::test]
    async fn test_database_backend_query_failure() {
        let backend = MockDatabaseBackend::new().with_query_failure();

        let result = backend.execute_query("SELECT * FROM users", &[]).await;
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        assert_eq!(error.to_string(), "Query error: Mock query failure");
    }

    #[tokio::test]
    async fn test_database_backend_schema_operations() {
        let backend = MockDatabaseBackend::new();

        // Test successful schema operation
        let result = backend.execute_schema("CREATE TABLE test (id INTEGER)").await;
        assert!(result.is_ok());

        // Test schema failure
        let failing_backend = backend.with_schema_failure();
        let result = failing_backend.execute_schema("DROP TABLE test").await;
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        assert_eq!(error.to_string(), "Database error: Mock schema failure");
    }

    #[tokio::test]
    async fn test_database_backend_ping_health_check() {
        let backend = MockDatabaseBackend::new();

        // Test successful ping
        let result = backend.ping().await;
        assert!(result.is_ok());

        // Test ping failure
        let failing_backend = backend.with_ping_failure();
        let result = failing_backend.ping().await;
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        assert_eq!(error.to_string(), "Connection error: Mock ping failure");
    }

    #[tokio::test]
    async fn test_database_backend_different_dialects() {
        let sqlite_backend = MockDatabaseBackend::new()
            .with_dialect(DatabaseDialect::SQLite);
        assert_eq!(sqlite_backend.dialect(), DatabaseDialect::SQLite);

        #[cfg(feature = "postgres")]
        {
            let postgres_backend = MockDatabaseBackend::new()
                .with_dialect(DatabaseDialect::PostgreSQL);
            assert_eq!(postgres_backend.dialect(), DatabaseDialect::PostgreSQL);
        }

        #[cfg(feature = "mysql")]
        {
            let mysql_backend = MockDatabaseBackend::new()
                .with_dialect(DatabaseDialect::MySQL);
            assert_eq!(mysql_backend.dialect(), DatabaseDialect::MySQL);
        }
    }

    #[tokio::test]
    async fn test_database_backend_clone_and_send_sync() {
        let backend = MockDatabaseBackend::new();
        
        // Test Clone
        let cloned = backend.clone();
        assert_eq!(backend.dialect(), cloned.dialect());
        assert_eq!(backend.connection_info(), cloned.connection_info());

        // Test Send + Sync by spawning a task
        let handle = tokio::spawn(async move {
            cloned.ping().await.unwrap();
        });
        
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_database_backend_parameter_binding() {
        let backend = MockDatabaseBackend::new();

        // Test with various parameter types
        let params = vec![
            json!("string_value"),
            json!(42),
            json!(true),
            json!(null),
            json!({"nested": "object"}),
            json!([1, 2, 3]),
        ];

        let result = backend.execute_query(
            "SELECT * FROM test WHERE col1 = ? AND col2 = ? AND col3 = ? AND col4 = ? AND col5 = ? AND col6 = ?", 
            &params
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_database_backend_integration_with_query_result() {
        let backend = MockDatabaseBackend::new()
            .with_mock_data(vec![
                json!({"id": 1, "name": "Integration Test", "active": true}),
            ]);

        let result = backend.execute_query("SELECT * FROM users", &[]).await;
        assert!(result.is_ok());

        let query_result = result.unwrap();
        
        // Test QueryResult integration
        assert!(!query_result.is_empty());
        assert_eq!(query_result.len(), 1);
        assert!(query_result.has_column("id"));
        assert!(query_result.has_column("name"));
        assert!(query_result.has_column("active"));
        
        let column_names = query_result.column_names();
        assert!(column_names.contains(&"id".to_string()));
        assert!(column_names.contains(&"name".to_string()));
        assert!(column_names.contains(&"active".to_string()));
    }
}