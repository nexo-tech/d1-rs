// MySQL backend stub - Task 2.6 will implement the full MySQL backend
// This is a minimal stub to make feature-gated exports compile

#[cfg(feature = "mysql")]
mod mysql_stub {
    use async_trait::async_trait;
    use crate::backends::{DatabaseBackend, QueryResult};
    use crate::dialects::DatabaseDialect;
    use crate::D1RsError;
    use serde_json::Value;
    
    /// MySQL backend stub - full implementation in Task 2.6
    #[derive(Clone)]
    pub struct MySQLBackend {
        // Stub: actual implementation will be added in Task 2.6
    }
    
    /// MySQL query result stub - full implementation in Task 2.6
    #[derive(Debug)]
    pub struct MySQLQueryResult {
        // Stub: actual implementation will be added in Task 2.6
        rows: Vec<Value>,
    }
    
    impl MySQLBackend {
        /// Stub constructor - will be properly implemented in Task 2.6
        pub async fn new(_url: &str) -> Result<Self, D1RsError> {
            // Stub: Task 2.6 will implement actual MySQL connection
            Err(D1RsError::Database("MySQL backend not yet implemented - see Task 2.6".to_string()))
        }
    }
    
    impl QueryResult for MySQLQueryResult {
        type Error = D1RsError;
        
        fn rows(&self) -> &[Value] {
            &self.rows
        }
        
        fn into_rows(self) -> Vec<Value> {
            self.rows
        }
        
        // Stub implementations - Task 2.6 will provide full implementations
        fn into_entities<T>(self) -> std::result::Result<Vec<T>, Self::Error>
        where
            T: serde::de::DeserializeOwned + crate::Entity,
        {
            Err(D1RsError::Database("MySQL backend not yet implemented - see Task 2.6".to_string()))
        }
        
        fn into_entity<T>(self) -> std::result::Result<Option<T>, Self::Error>
        where
            T: serde::de::DeserializeOwned + crate::Entity,
        {
            Err(D1RsError::Database("MySQL backend not yet implemented - see Task 2.6".to_string()))
        }
        
        fn into_simple_entities<T>(self) -> std::result::Result<Vec<T>, Self::Error>
        where
            T: serde::de::DeserializeOwned,
        {
            Err(D1RsError::Database("MySQL backend not yet implemented - see Task 2.6".to_string()))
        }
        
        fn into_simple_entity<T>(self) -> std::result::Result<Option<T>, Self::Error>
        where
            T: serde::de::DeserializeOwned,
        {
            Err(D1RsError::Database("MySQL backend not yet implemented - see Task 2.6".to_string()))
        }
    }
    
    #[async_trait]
    impl DatabaseBackend for MySQLBackend {
        type QueryResult = MySQLQueryResult;
        type Error = D1RsError;
        
        async fn execute_query(
            &self, 
            _sql: &str, 
            _params: &[Value]
        ) -> std::result::Result<Self::QueryResult, Self::Error> {
            Err(D1RsError::Database("MySQL backend not yet implemented - see Task 2.6".to_string()))
        }
        
        async fn execute_schema(&self, _sql: &str) -> std::result::Result<(), Self::Error> {
            Err(D1RsError::Database("MySQL backend not yet implemented - see Task 2.6".to_string()))
        }
        
        fn dialect(&self) -> DatabaseDialect {
            DatabaseDialect::MySQL
        }
        
        fn connection_info(&self) -> String {
            "mysql://stub-not-yet-implemented".to_string()
        }
        
        async fn ping(&self) -> std::result::Result<(), Self::Error> {
            Err(D1RsError::Database("MySQL backend not yet implemented - see Task 2.6".to_string()))
        }
    }
}

#[cfg(feature = "mysql")]
pub use mysql_stub::{MySQLBackend, MySQLQueryResult};