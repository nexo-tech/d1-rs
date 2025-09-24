use super::{DatabaseBackend, QueryResult};
use crate::dialects::DatabaseDialect;
use crate::{D1RsError, Entity};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
use worker::d1::D1Database;
#[cfg(not(target_arch = "wasm32"))]
use {rusqlite::Connection, tokio::sync::Mutex};

/// SQLite backend implementation that wraps existing D1Client logic
/// 
/// This backend provides a DatabaseBackend trait implementation for SQLite,
/// supporting both WASM (Cloudflare D1) and native (rusqlite) targets.
/// It leverages the existing D1Client infrastructure to maintain compatibility
/// while providing a clean DatabaseBackend interface.
#[derive(Clone)]
pub struct SQLiteBackend {
    #[cfg(target_arch = "wasm32")]
    db: Arc<D1Database>,
    #[cfg(not(target_arch = "wasm32"))]
    db: Arc<Mutex<Connection>>,
}

impl SQLiteBackend {
    /// Create a new SQLiteBackend from a D1Database (WASM target)
    #[cfg(target_arch = "wasm32")]
    pub fn new(db: D1Database) -> Self {
        Self { db: Arc::new(db) }
    }
    
    /// Create a new SQLiteBackend from a rusqlite Connection (native target)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new_sqlite(conn: Connection) -> Self {
        Self { db: Arc::new(Mutex::new(conn)) }
    }
    
    /// Create an in-memory SQLite database for testing (native target)
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn new_in_memory() -> Result<Self, D1RsError> {
        let conn = rusqlite::Connection::open_in_memory()
            .map_err(|e| D1RsError::Database(e.to_string()))?;
        Ok(Self::new_sqlite(conn))
    }
}

/// SQLite-specific QueryResult that wraps existing D1QueryResult
/// 
/// This provides a clean interface while delegating to the existing
/// D1QueryResult implementation to maintain full compatibility.
#[derive(Debug)]
pub struct SQLiteQueryResult {
    inner: crate::db::D1QueryResult,
}

impl SQLiteQueryResult {
    /// Create a new SQLiteQueryResult from D1QueryResult
    pub fn new(inner: crate::db::D1QueryResult) -> Self {
        Self { inner }
    }
}

impl QueryResult for SQLiteQueryResult {
    type Error = D1RsError;
    
    fn rows(&self) -> &[Value] {
        &self.inner.rows
    }
    
    fn into_rows(self) -> Vec<Value> {
        self.inner.rows
    }
    
    fn len(&self) -> usize {
        self.inner.rows.len()
    }
    
    fn is_empty(&self) -> bool {
        self.inner.rows.is_empty()
    }
    
    fn into_entities<T>(self) -> std::result::Result<Vec<T>, Self::Error>
    where
        T: serde::de::DeserializeOwned + Entity,
    {
        // Use existing D1QueryResult implementation
        self.inner.into_entities()
    }
    
    fn into_entity<T>(mut self) -> std::result::Result<Option<T>, Self::Error>
    where
        T: serde::de::DeserializeOwned + Entity,
    {
        if let Some(row) = self.inner.rows.pop() {
            let converted = T::convert_from_sqlite(row);
            let entity = serde_json::from_value(converted)
                .map_err(|e| D1RsError::SerializationError(e.to_string()))?;
            Ok(Some(entity))
        } else {
            Ok(None)
        }
    }
    
    fn into_simple_entities<T>(self) -> std::result::Result<Vec<T>, Self::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.inner.rows
            .into_iter()
            .map(|row| {
                serde_json::from_value(row)
                    .map_err(|e| D1RsError::SerializationError(e.to_string()))
            })
            .collect()
    }
    
    fn into_simple_entity<T>(mut self) -> std::result::Result<Option<T>, Self::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        if let Some(row) = self.inner.rows.pop() {
            let entity = serde_json::from_value(row)
                .map_err(|e| D1RsError::SerializationError(e.to_string()))?;
            Ok(Some(entity))
        } else {
            Ok(None)
        }
    }
    
    fn extract_count(&self) -> std::result::Result<i64, Self::Error> {
        if let Some(row) = self.rows().first() {
            if let Value::Object(obj) = row {
                // Try different possible count column names
                for key in ["count", "COUNT(*)", "count(*)", "COUNT(*)"] {
                    if let Some(value) = obj.get(key) {
                        if let Value::Number(n) = value {
                            if let Some(i) = n.as_i64() {
                                return Ok(i);
                            }
                        }
                    }
                }
                
                // If no named count, try first numeric value
                if let Some(value) = obj.values().next() {
                    if let Value::Number(n) = value {
                        if let Some(i) = n.as_i64() {
                            return Ok(i);
                        }
                    }
                }
            }
        }
        Ok(0)
    }
    
    fn extract_id(&self) -> std::result::Result<Option<i64>, Self::Error> {
        if let Some(row) = self.rows().first() {
            if let Value::Object(obj) = row {
                // Try different possible ID column names
                for key in ["id", "rowid", "last_insert_rowid", "last_insert_rowid()"] {
                    if let Some(value) = obj.get(key) {
                        if let Value::Number(n) = value {
                            if let Some(i) = n.as_i64() {
                                return Ok(Some(i));
                            }
                        }
                    }
                }
            }
        }
        Ok(None)
    }
    
    fn extract_strings(&self) -> std::result::Result<Vec<String>, Self::Error> {
        let mut strings = Vec::new();
        for row in self.rows() {
            if let Value::Object(obj) = row {
                for value in obj.values() {
                    if let Value::String(s) = value {
                        strings.push(s.clone());
                    }
                }
            } else if let Value::String(s) = row {
                strings.push(s.clone());
            }
        }
        Ok(strings)
    }
    
    fn into_hashmap(self) -> std::result::Result<Option<HashMap<String, Value>>, Self::Error> {
        if let Some(row) = self.inner.rows.into_iter().next() {
            if let Value::Object(obj) = row {
                let hashmap: HashMap<String, Value> = obj.into_iter().collect();
                return Ok(Some(hashmap));
            }
        }
        Ok(None)
    }
    
    fn column_names(&self) -> Vec<String> {
        if let Some(row) = self.rows().first() {
            if let Value::Object(obj) = row {
                return obj.keys().cloned().collect();
            }
        }
        vec![]
    }
    
    fn has_column(&self, name: &str) -> bool {
        if let Some(row) = self.rows().first() {
            if let Value::Object(obj) = row {
                return obj.contains_key(name);
            }
        }
        false
    }
}

#[async_trait]
impl DatabaseBackend for SQLiteBackend {
    type QueryResult = SQLiteQueryResult;
    type Error = D1RsError;
    
    async fn execute_query(
        &self, 
        sql: &str, 
        params: &[Value]
    ) -> std::result::Result<Self::QueryResult, Self::Error> {
        #[cfg(target_arch = "wasm32")]
        {
            // For WASM, use existing D1Client logic
            let client = crate::db::D1Client::new((*self.db).clone());
            let result = client.execute(sql, params).await?;
            Ok(SQLiteQueryResult::new(result))
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            // For native, implement directly using the connection
            use rusqlite::params_from_iter;
            
            let db = self.db.lock().await;
            
            // Convert JSON Values to rusqlite-compatible parameters
            let rusqlite_params: Vec<rusqlite::types::Value> = params.iter()
                .map(|v| match v {
                    Value::Null => rusqlite::types::Value::Null,
                    Value::Bool(b) => rusqlite::types::Value::Integer(if *b { 1 } else { 0 }),
                    Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            rusqlite::types::Value::Integer(i)
                        } else if let Some(f) = n.as_f64() {
                            rusqlite::types::Value::Real(f)
                        } else {
                            rusqlite::types::Value::Text(n.to_string())
                        }
                    },
                    Value::String(s) => rusqlite::types::Value::Text(s.clone()),
                    _ => rusqlite::types::Value::Text(v.to_string()),
                })
                .collect();
            
            // Check if it's a SELECT query or has RETURNING clause to determine how to handle the result
            let sql_upper = sql.trim_start().to_uppercase();
            let is_query_with_results = sql_upper.starts_with("SELECT") || sql_upper.contains("RETURNING");
            
            if is_query_with_results {
                // For SELECT queries or queries with RETURNING clause, collect all rows
                let mut stmt = db.prepare(sql)
                    .map_err(|e| D1RsError::Database(e.to_string()))?;
                    
                let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
                
                let rows_result = stmt.query_map(params_from_iter(rusqlite_params), |row| {
                    let mut obj = serde_json::Map::new();
                    for (i, col_name) in column_names.iter().enumerate() {
                        let value: rusqlite::types::Value = row.get(i)?;
                        let json_value = match value {
                            rusqlite::types::Value::Null => Value::Null,
                            rusqlite::types::Value::Integer(i) => Value::Number(serde_json::Number::from(i)),
                            rusqlite::types::Value::Real(f) => {
                                if let Some(num) = serde_json::Number::from_f64(f) {
                                    Value::Number(num)
                                } else {
                                    Value::String(f.to_string())
                                }
                            },
                            rusqlite::types::Value::Text(s) => Value::String(s),
                            rusqlite::types::Value::Blob(b) => {
                                use base64::{Engine, engine::general_purpose::STANDARD};
                                Value::String(STANDARD.encode(&b))
                            },
                        };
                        obj.insert(col_name.clone(), json_value);
                    }
                    Ok(Value::Object(obj))
                }).map_err(|e| D1RsError::Database(e.to_string()))?;
                
                let rows: Result<Vec<Value>, D1RsError> = rows_result
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| D1RsError::Database(e.to_string()));
                
                let d1_result = crate::db::D1QueryResult { rows: rows? };
                Ok(SQLiteQueryResult::new(d1_result))
            } else {
                // For non-SELECT queries (INSERT, UPDATE, DELETE), execute and return metadata
                let mut stmt = db.prepare(sql)
                    .map_err(|e| D1RsError::Database(e.to_string()))?;
                    
                let changes = stmt.execute(params_from_iter(rusqlite_params))
                    .map_err(|e| D1RsError::Database(e.to_string()))?;
                    
                let last_insert_rowid = db.last_insert_rowid();
                
                // Return metadata as a single row
                let mut obj = serde_json::Map::new();
                obj.insert("changes".to_string(), Value::Number(serde_json::Number::from(changes)));
                obj.insert("last_insert_rowid".to_string(), Value::Number(serde_json::Number::from(last_insert_rowid)));
                
                let d1_result = crate::db::D1QueryResult { 
                    rows: vec![Value::Object(obj)]
                };
                Ok(SQLiteQueryResult::new(d1_result))
            }
        }
    }
    
    async fn execute_schema(&self, sql: &str) -> std::result::Result<(), Self::Error> {
        // Schema operations (DDL) typically don't return data, just execute
        self.execute_query(sql, &[]).await?;
        Ok(())
    }
    
    fn dialect(&self) -> DatabaseDialect {
        DatabaseDialect::SQLite
    }
    
    fn connection_info(&self) -> String {
        #[cfg(target_arch = "wasm32")]
        return "D1Database (Cloudflare Workers)".to_string();
        
        #[cfg(not(target_arch = "wasm32"))]
        return "SQLite (rusqlite)".to_string();
    }
    
    async fn ping(&self) -> std::result::Result<(), Self::Error> {
        // Simple health check query
        self.execute_query("SELECT 1", &[]).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_sqlite_backend_creation() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let backend = SQLiteBackend::new_in_memory().await;
            assert!(backend.is_ok());
            
            let backend = backend.unwrap();
            assert_eq!(backend.dialect(), DatabaseDialect::SQLite);
            assert_eq!(backend.connection_info(), "SQLite (rusqlite)");
        }
    }
    
    #[tokio::test]
    async fn test_sqlite_backend_ping() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let backend = SQLiteBackend::new_in_memory().await.unwrap();
            let result = backend.ping().await;
            assert!(result.is_ok());
        }
    }
    
    #[tokio::test]
    async fn test_sqlite_backend_basic_query() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let backend = SQLiteBackend::new_in_memory().await.unwrap();
            
            // Test basic SELECT query
            let result = backend.execute_query("SELECT 1 as test_value", &[]).await;
            assert!(result.is_ok());
            
            let query_result = result.unwrap();
            assert!(!query_result.is_empty());
            assert_eq!(query_result.len(), 1);
            
            let rows = query_result.rows();
            assert_eq!(rows.len(), 1);
        }
    }
    
    #[tokio::test]
    async fn test_sqlite_backend_schema_operations() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let backend = SQLiteBackend::new_in_memory().await.unwrap();
            
            // Test table creation
            let result = backend.execute_schema(
                "CREATE TABLE test_table (id INTEGER PRIMARY KEY, name TEXT)"
            ).await;
            assert!(result.is_ok());
            
            // Test data insertion
            let result = backend.execute_query(
                "INSERT INTO test_table (name) VALUES (?)",
                &[json!("test_name")]
            ).await;
            assert!(result.is_ok());
            
            // Test data retrieval
            let result = backend.execute_query(
                "SELECT * FROM test_table WHERE name = ?",
                &[json!("test_name")]
            ).await;
            assert!(result.is_ok());
            
            let query_result = result.unwrap();
            assert_eq!(query_result.len(), 1);
            assert!(query_result.has_column("id"));
            assert!(query_result.has_column("name"));
        }
    }
    
    #[tokio::test]
    async fn test_sqlite_query_result_extract_methods() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let backend = SQLiteBackend::new_in_memory().await.unwrap();
            
            // Test count extraction
            let result = backend.execute_query("SELECT 42 as count", &[]).await;
            assert!(result.is_ok());
            
            let query_result = result.unwrap();
            let count = query_result.extract_count();
            assert!(count.is_ok());
            assert_eq!(count.unwrap(), 42);
            
            // Test ID extraction
            let result = backend.execute_query("SELECT 123 as id", &[]).await;
            assert!(result.is_ok());
            
            let query_result = result.unwrap();
            let id = query_result.extract_id();
            assert!(id.is_ok());
            assert_eq!(id.unwrap(), Some(123));
            
            // Test string extraction
            let result = backend.execute_query("SELECT 'test1' as col1, 'test2' as col2", &[]).await;
            assert!(result.is_ok());
            
            let query_result = result.unwrap();
            let strings = query_result.extract_strings();
            assert!(strings.is_ok());
            
            let strings = strings.unwrap();
            assert_eq!(strings.len(), 2);
            assert!(strings.contains(&"test1".to_string()));
            assert!(strings.contains(&"test2".to_string()));
        }
    }
    
    #[tokio::test]
    async fn test_sqlite_query_result_into_hashmap() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let backend = SQLiteBackend::new_in_memory().await.unwrap();
            
            let result = backend.execute_query(
                "SELECT 'value1' as key1, 'value2' as key2", 
                &[]
            ).await;
            assert!(result.is_ok());
            
            let query_result = result.unwrap();
            let hashmap = query_result.into_hashmap();
            assert!(hashmap.is_ok());
            
            let hashmap = hashmap.unwrap();
            assert!(hashmap.is_some());
            
            let map = hashmap.unwrap();
            assert_eq!(map.len(), 2);
            assert_eq!(map.get("key1"), Some(&json!("value1")));
            assert_eq!(map.get("key2"), Some(&json!("value2")));
        }
    }
    
    #[tokio::test]
    async fn test_sqlite_query_result_column_operations() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let backend = SQLiteBackend::new_in_memory().await.unwrap();
            
            let result = backend.execute_query(
                "SELECT 1 as id, 'test' as name, 42.5 as score", 
                &[]
            ).await;
            assert!(result.is_ok());
            
            let query_result = result.unwrap();
            
            // Test column names
            let column_names = query_result.column_names();
            assert_eq!(column_names.len(), 3);
            assert!(column_names.contains(&"id".to_string()));
            assert!(column_names.contains(&"name".to_string()));
            assert!(column_names.contains(&"score".to_string()));
            
            // Test has_column
            assert!(query_result.has_column("id"));
            assert!(query_result.has_column("name"));
            assert!(query_result.has_column("score"));
            assert!(!query_result.has_column("nonexistent"));
        }
    }

    #[tokio::test]
    async fn test_sqlite_query_result_empty_result() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let backend = SQLiteBackend::new_in_memory().await.unwrap();
            
            // Create table first
            backend.execute_schema("CREATE TABLE empty_test (id INTEGER)").await.unwrap();
            
            // Query empty table
            let result = backend.execute_query("SELECT * FROM empty_test", &[]).await;
            assert!(result.is_ok());
            
            let query_result = result.unwrap();
            assert!(query_result.is_empty());
            assert_eq!(query_result.len(), 0);
            
            // Test extraction methods on empty result
            assert_eq!(query_result.extract_count().unwrap(), 0);
            assert_eq!(query_result.extract_id().unwrap(), None);
            assert!(query_result.extract_strings().unwrap().is_empty());
            assert!(query_result.column_names().is_empty());
            assert!(!query_result.has_column("id"));
        }
    }
    
    #[tokio::test]
    async fn test_sqlite_backend_parameter_binding() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let backend = SQLiteBackend::new_in_memory().await.unwrap();
            
            // Create test table
            backend.execute_schema(
                "CREATE TABLE param_test (id INTEGER, name TEXT, active BOOLEAN, score REAL)"
            ).await.unwrap();
            
            // Insert with parameters
            let result = backend.execute_query(
                "INSERT INTO param_test (name, active, score) VALUES (?, ?, ?)",
                &[json!("test_user"), json!(true), json!(98.5)]
            ).await;
            assert!(result.is_ok());
            
            // Query with parameters
            let result = backend.execute_query(
                "SELECT * FROM param_test WHERE name = ? AND active = ?",
                &[json!("test_user"), json!(true)]
            ).await;
            assert!(result.is_ok());
            
            let query_result = result.unwrap();
            assert_eq!(query_result.len(), 1);
            
            let hashmap = query_result.into_hashmap().unwrap().unwrap();
            assert_eq!(hashmap.get("name"), Some(&json!("test_user")));
            assert_eq!(hashmap.get("active"), Some(&json!(1))); // SQLite stores boolean as integer
            assert_eq!(hashmap.get("score"), Some(&json!(98.5)));
        }
    }
}