use crate::{Result, D1RsError};
use crate::backends::{DatabaseBackend, QueryResult};
use crate::dialects::DatabaseDialect;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
use worker::wasm_bindgen::JsValue;
#[cfg(target_arch = "wasm32")]
use worker::d1::D1Database;

#[cfg(not(target_arch = "wasm32"))]
use rusqlite::{Connection, params_from_iter};
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct D1Client {
    #[cfg(target_arch = "wasm32")]
    db: Arc<D1Database>,
    #[cfg(not(target_arch = "wasm32"))]
    db: Arc<Mutex<Connection>>,
}

impl D1Client {
    #[cfg(target_arch = "wasm32")]
    pub fn new(db: D1Database) -> Self {
        Self { db: Arc::new(db) }
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new_sqlite(conn: Connection) -> Self {
        Self { db: Arc::new(Mutex::new(conn)) }
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| D1RsError::Database(e.to_string()))?;
        Ok(Self::new_sqlite(conn))
    }
    

    pub async fn execute(&self, sql: &str, params: &[Value]) -> Result<D1QueryResult> {
        #[cfg(target_arch = "wasm32")]
        {
            self.execute_d1(sql, params).await
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.execute_sqlite(sql, params).await
        }
    }
    
    #[cfg(target_arch = "wasm32")]
    async fn execute_d1(&self, sql: &str, params: &[Value]) -> Result<D1QueryResult> {
        let mut stmt = self.db.prepare(sql);
        
        // Convert all parameters to JsValue and bind them all at once
        let js_params: Vec<JsValue> = params.iter().map(|param| {
            match param {
                Value::Null => JsValue::NULL,
                Value::Bool(b) => (*b as i32).into(),
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        // D1 doesn't support BigInt, so convert to i32 like we do in types.rs
                        (i as i32).into()
                    } else if let Some(f) = n.as_f64() {
                        f.into()
                    } else {
                        JsValue::NULL
                    }
                },
                Value::String(s) => s.as_str().into(),
                _ => JsValue::NULL,
            }
        }).collect();
        
        if !js_params.is_empty() {
            stmt = stmt.bind(&js_params).map_err(|e| D1RsError::Database(format!("{:?}", e)))?;
        }
        
        let result = stmt.all().await.map_err(|e| D1RsError::Database(format!("{:?}", e)))?;
        let rows = result.results::<serde_json::Value>().map_err(|e| D1RsError::Database(format!("{:?}", e)))?;
        
        Ok(D1QueryResult { rows })
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    async fn execute_sqlite(&self, sql: &str, params: &[Value]) -> Result<D1QueryResult> {
        use rusqlite::types::ToSqlOutput;
        
        let conn = self.db.lock().await;
        
        // Convert JSON values to rusqlite parameters
        let sqlite_params: Vec<ToSqlOutput> = params.iter().map(|param| {
            match param {
                Value::Null => ToSqlOutput::Owned(rusqlite::types::Value::Null),
                Value::Bool(b) => ToSqlOutput::Owned(rusqlite::types::Value::Integer(*b as i64)),
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        ToSqlOutput::Owned(rusqlite::types::Value::Integer(i))
                    } else if let Some(f) = n.as_f64() {
                        ToSqlOutput::Owned(rusqlite::types::Value::Real(f))
                    } else {
                        ToSqlOutput::Owned(rusqlite::types::Value::Null)
                    }
                },
                Value::String(s) => ToSqlOutput::Owned(rusqlite::types::Value::Text(s.clone())),
                _ => ToSqlOutput::Owned(rusqlite::types::Value::Null),
            }
        }).collect();
        
        // Check if it's a SELECT query
        let is_select = sql.trim_start().to_uppercase().starts_with("SELECT");
        
        if is_select {
            let mut stmt = conn.prepare(sql)
                .map_err(|e| D1RsError::Database(e.to_string()))?;
            
            let column_names: Vec<String> = stmt.column_names()
                .iter()
                .map(|s| s.to_string())
                .collect();
            
            let rows_result = stmt.query_map(params_from_iter(sqlite_params.iter()), |row| {
                let mut obj = serde_json::Map::new();
                
                for (i, name) in column_names.iter().enumerate() {
                    let value = if let Ok(v) = row.get::<_, Option<i64>>(i) {
                        v.map(|n| Value::Number(n.into())).unwrap_or(Value::Null)
                    } else if let Ok(v) = row.get::<_, Option<f64>>(i) {
                        v.map(|n| serde_json::Number::from_f64(n).map(Value::Number).unwrap_or(Value::Null)).unwrap_or(Value::Null)
                    } else if let Ok(v) = row.get::<_, Option<String>>(i) {
                        v.map(Value::String).unwrap_or(Value::Null)
                    } else {
                        Value::Null
                    };
                    obj.insert(name.clone(), value);
                }
                
                Ok(Value::Object(obj))
            }).map_err(|e| D1RsError::Database(e.to_string()))?;
            
            let rows: Result<Vec<Value>> = rows_result
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| D1RsError::Database(e.to_string()));
            
            Ok(D1QueryResult { rows: rows? })
        } else {
            // For queries with RETURNING, use query_map directly without separate execute
            if sql.contains("RETURNING") {
                let mut stmt = conn.prepare(sql)
                    .map_err(|e| D1RsError::Database(e.to_string()))?;
                
                let column_names: Vec<String> = stmt.column_names()
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
                
                let rows_result = stmt.query_map(params_from_iter(sqlite_params.iter()), |row| {
                    let mut obj = serde_json::Map::new();
                    
                    for (i, name) in column_names.iter().enumerate() {
                        let value = if let Ok(v) = row.get::<_, Option<i64>>(i) {
                            v.map(|n| Value::Number(n.into())).unwrap_or(Value::Null)
                        } else if let Ok(v) = row.get::<_, Option<f64>>(i) {
                            v.map(|n| serde_json::Number::from_f64(n).map(Value::Number).unwrap_or(Value::Null)).unwrap_or(Value::Null)
                        } else if let Ok(v) = row.get::<_, Option<String>>(i) {
                            v.map(Value::String).unwrap_or(Value::Null)
                        } else {
                            Value::Null
                        };
                        obj.insert(name.clone(), value);
                    }
                    
                    Ok(Value::Object(obj))
                }).map_err(|e| D1RsError::Database(e.to_string()))?;
                
                let rows: Result<Vec<Value>> = rows_result
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(|e| D1RsError::Database(e.to_string()));
                
                Ok(D1QueryResult { rows: rows? })
            } else if sql.trim_start().to_uppercase().starts_with("INSERT") {
                // For INSERT without RETURNING
                conn.execute(sql, params_from_iter(sqlite_params.iter()))
                    .map_err(|e| D1RsError::Database(e.to_string()))?;
                // Get last insert rowid for simple INSERTs
                let id = conn.last_insert_rowid();
                let mut obj = serde_json::Map::new();
                obj.insert("id".to_string(), Value::Number(id.into()));
                Ok(D1QueryResult { rows: vec![Value::Object(obj)] })
            } else {
                // For UPDATE/DELETE without RETURNING
                conn.execute(sql, params_from_iter(sqlite_params.iter()))
                    .map_err(|e| D1RsError::Database(e.to_string()))?;
                Ok(D1QueryResult { rows: vec![] })
            }
        }
    }

    pub async fn execute_returning_one(&self, sql: &str, params: &[Value]) -> Result<Option<HashMap<String, Value>>> {
        let result = self.execute(sql, params).await?;
        
        if let Some(row) = result.rows.into_iter().next() {
            if let Value::Object(obj) = row {
                let hashmap: HashMap<String, Value> = obj.into_iter().collect();
                return Ok(Some(hashmap));
            }
        }
        
        Ok(None)
    }

    pub async fn execute_returning_count(&self, sql: &str, params: &[Value]) -> Result<i64> {
        let result = self.execute(sql, params).await?;
        
        if let Some(row) = result.rows.into_iter().next() {
            if let Value::Object(obj) = row {
                if let Some(count) = obj.values().next() {
                    if let Value::Number(n) = count {
                        if let Some(i) = n.as_i64() {
                            return Ok(i);
                        }
                    }
                }
            }
        }
        
        Ok(0)
    }
}

// Generic DatabaseClient that works with any backend
#[derive(Clone)]
pub struct DatabaseClient<B: DatabaseBackend> {
    backend: B,
    dialect: DatabaseDialect,
}

impl<B: DatabaseBackend> DatabaseClient<B> {
    pub fn new(backend: B) -> Self {
        let dialect = backend.dialect();
        Self { backend, dialect }
    }
    
    pub fn dialect(&self) -> DatabaseDialect {
        self.dialect
    }
    
    // Direct query execution (for raw SQL during migration period)
    pub async fn execute(&self, sql: &str, params: &[Value]) -> std::result::Result<B::QueryResult, B::Error> {
        self.backend.execute_query(sql, params).await
    }
    
    pub async fn execute_returning_one(&self, sql: &str, params: &[Value]) -> std::result::Result<Option<HashMap<String, Value>>, B::Error> {
        let result = self.execute(sql, params).await?;
        
        // Convert first row to HashMap
        if let Some(row) = result.rows().first() {
            if let Value::Object(obj) = row {
                let hashmap: HashMap<String, Value> = obj.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                return Ok(Some(hashmap));
            }
        }
        Ok(None)
    }
    
    pub async fn execute_returning_count(&self, sql: &str, params: &[Value]) -> std::result::Result<i64, B::Error> 
    where
        B::Error: From<<<B as DatabaseBackend>::QueryResult as QueryResult>::Error>,
    {
        let result = self.execute(sql, params).await?;
        // Convert QueryResult error to Backend error using From trait
        result.extract_count().map_err(B::Error::from)
    }
    
    // Schema operations
    pub async fn execute_schema(&self, sql: &str) -> std::result::Result<(), B::Error> {
        self.backend.execute_schema(sql).await
    }
    
    // Health check
    pub async fn ping(&self) -> std::result::Result<(), B::Error> {
        self.backend.ping().await
    }
}

// Type alias for SQLite backend (most common case)
pub type SQLiteClient = DatabaseClient<crate::backends::SQLiteBackend>;

#[derive(Debug)]
pub struct D1QueryResult {
    pub rows: Vec<Value>,
}

impl D1QueryResult {
    pub fn into_entities<T>(self) -> Result<Vec<T>> 
    where 
        T: serde::de::DeserializeOwned + crate::Entity
    {
        self.rows
            .into_iter()
            .map(|row| Self::deserialize_with_boolean_conversion(row))
            .collect()
    }

    pub fn into_entity<T>(mut self) -> Result<Option<T>> 
    where 
        T: serde::de::DeserializeOwned + crate::Entity
    {
        if let Some(row) = self.rows.pop() {
            let entity = Self::deserialize_with_boolean_conversion(row)?;
            Ok(Some(entity))
        } else {
            Ok(None)
        }
    }
    
    /// Simple deserialization for non-Entity types (like MigrationRecord)
    pub fn into_simple_entities<T: serde::de::DeserializeOwned>(self) -> Result<Vec<T>> {
        self.rows
            .into_iter()
            .map(|row| {
                serde_json::from_value(row)
                    .map_err(|e| D1RsError::SerializationError(e.to_string()))
            })
            .collect()
    }

    pub fn into_simple_entity<T: serde::de::DeserializeOwned>(mut self) -> Result<Option<T>> {
        if let Some(row) = self.rows.pop() {
            let entity = serde_json::from_value(row)
                .map_err(|e| D1RsError::SerializationError(e.to_string()))?;
            Ok(Some(entity))
        } else {
            Ok(None)
        }
    }

    /// Smart deserialization using Entity metadata for complete SQLite conversion
    fn deserialize_with_boolean_conversion<T>(value: Value) -> Result<T> 
    where 
        T: serde::de::DeserializeOwned + crate::Entity 
    {
        // ALWAYS apply Entity conversion first (boolean + datetime + other SQLite conversions)
        let converted_value = T::convert_from_sqlite(value);
        
        // Then deserialize the converted data
        serde_json::from_value(converted_value)
            .map_err(|e| D1RsError::SerializationError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::SQLiteBackend;
    
    #[tokio::test]
    async fn test_database_client_creation() {
        let backend = SQLiteBackend::new_in_memory().await.expect("Failed to create backend");
        let client = DatabaseClient::new(backend);
        
        assert_eq!(client.dialect(), DatabaseDialect::SQLite);
    }
    
    #[tokio::test]
    async fn test_database_client_execute() {
        let backend = SQLiteBackend::new_in_memory().await.expect("Failed to create backend");
        let client = DatabaseClient::new(backend);
        
        // Create a test table
        let create_sql = "CREATE TABLE test_users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)";
        client.execute_schema(create_sql).await.expect("Failed to create table");
        
        // Insert test data
        let insert_sql = "INSERT INTO test_users (name, email) VALUES (?, ?) RETURNING *";
        let params = vec![
            serde_json::json!("John Doe"),
            serde_json::json!("john@example.com"),
        ];
        
        let result = client.execute(insert_sql, &params).await.expect("Failed to insert");
        assert_eq!(result.len(), 1);
        assert!(!result.is_empty());
    }
    
    #[tokio::test]
    async fn test_database_client_execute_returning_one() {
        let backend = SQLiteBackend::new_in_memory().await.expect("Failed to create backend");
        let client = DatabaseClient::new(backend);
        
        // Create a test table
        let create_sql = "CREATE TABLE test_users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)";
        client.execute_schema(create_sql).await.expect("Failed to create table");
        
        // Insert test data
        let insert_sql = "INSERT INTO test_users (name, email) VALUES (?, ?) RETURNING *";
        let params = vec![
            serde_json::json!("Jane Smith"),
            serde_json::json!("jane@example.com"),
        ];
        
        let result = client.execute_returning_one(insert_sql, &params).await.expect("Failed to insert");
        assert!(result.is_some());
        
        let user = result.unwrap();
        assert_eq!(user.get("name"), Some(&serde_json::json!("Jane Smith")));
        assert_eq!(user.get("email"), Some(&serde_json::json!("jane@example.com")));
        assert!(user.contains_key("id"));
    }
    
    #[tokio::test]
    async fn test_database_client_execute_returning_count() {
        let backend = SQLiteBackend::new_in_memory().await.expect("Failed to create backend");
        let client = DatabaseClient::new(backend);
        
        // Create a test table
        let create_sql = "CREATE TABLE test_users (id INTEGER PRIMARY KEY, name TEXT, active BOOLEAN DEFAULT 1)";
        client.execute_schema(create_sql).await.expect("Failed to create table");
        
        // Insert test data
        let insert_sql = "INSERT INTO test_users (name) VALUES (?), (?), (?)";
        let params = vec![
            serde_json::json!("User 1"),
            serde_json::json!("User 2"),
            serde_json::json!("User 3"),
        ];
        client.execute(insert_sql, &params).await.expect("Failed to insert");
        
        // Count active users
        let count_sql = "SELECT COUNT(*) as count FROM test_users WHERE active = ?";
        let count_params = vec![serde_json::json!(true)];
        
        let count = client.execute_returning_count(count_sql, &count_params).await.expect("Failed to count");
        assert_eq!(count, 3);
    }
    
    #[tokio::test]
    async fn test_database_client_ping() {
        let backend = SQLiteBackend::new_in_memory().await.expect("Failed to create backend");
        let client = DatabaseClient::new(backend);
        
        // Ping should work for SQLite
        client.ping().await.expect("Failed to ping database");
    }
    
    #[tokio::test]
    async fn test_database_client_complex_queries() {
        let backend = SQLiteBackend::new_in_memory().await.expect("Failed to create backend");
        let client = DatabaseClient::new(backend);
        
        // Create test tables
        let create_users = "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT UNIQUE)";
        let create_posts = "CREATE TABLE posts (id INTEGER PRIMARY KEY, user_id INTEGER, title TEXT, content TEXT, FOREIGN KEY(user_id) REFERENCES users(id))";
        
        client.execute_schema(create_users).await.expect("Failed to create users table");
        client.execute_schema(create_posts).await.expect("Failed to create posts table");
        
        // Insert test user
        let insert_user = "INSERT INTO users (name, email) VALUES (?, ?) RETURNING id";
        let user_params = vec![
            serde_json::json!("Alice"),
            serde_json::json!("alice@example.com"),
        ];
        
        let user_result = client.execute_returning_one(insert_user, &user_params).await.expect("Failed to insert user");
        let user_id = user_result.unwrap().get("id").unwrap().as_i64().unwrap();
        
        // Insert test posts
        let insert_post = "INSERT INTO posts (user_id, title, content) VALUES (?, ?, ?), (?, ?, ?)";
        let post_params = vec![
            serde_json::json!(user_id),
            serde_json::json!("First Post"),
            serde_json::json!("This is the first post"),
            serde_json::json!(user_id),
            serde_json::json!("Second Post"),
            serde_json::json!("This is the second post"),
        ];
        
        client.execute(insert_post, &post_params).await.expect("Failed to insert posts");
        
        // Query with JOIN
        let join_query = "SELECT u.name, u.email, p.title, p.content FROM users u JOIN posts p ON u.id = p.user_id WHERE u.id = ?";
        let join_params = vec![serde_json::json!(user_id)];
        
        let join_result = client.execute(join_query, &join_params).await.expect("Failed to execute join query");
        assert_eq!(join_result.len(), 2); // Should return 2 posts for Alice
        
        // Verify the joined data
        let rows = join_result.rows();
        assert_eq!(rows[0].get("name"), Some(&serde_json::json!("Alice")));
        assert_eq!(rows[1].get("name"), Some(&serde_json::json!("Alice")));
    }
    
    #[tokio::test]
    async fn test_sqlite_client_type_alias() {
        let backend = SQLiteBackend::new_in_memory().await.expect("Failed to create backend");
        let client: SQLiteClient = DatabaseClient::new(backend);
        
        assert_eq!(client.dialect(), DatabaseDialect::SQLite);
        
        // Test basic functionality through type alias
        let create_sql = "CREATE TABLE alias_test (id INTEGER PRIMARY KEY, value TEXT)";
        client.execute_schema(create_sql).await.expect("Failed to create table");
        
        let insert_sql = "INSERT INTO alias_test (value) VALUES (?) RETURNING *";
        let params = vec![serde_json::json!("test value")];
        
        let result = client.execute_returning_one(insert_sql, &params).await.expect("Failed to insert");
        assert!(result.is_some());
        
        let record = result.unwrap();
        assert_eq!(record.get("value"), Some(&serde_json::json!("test value")));
    }
}