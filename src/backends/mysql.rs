use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

// MySQL backend implementation with complete sqlx integration
#[cfg(feature = "mysql")]
mod mysql_impl {
    use super::*;
    use crate::backends::{DatabaseBackend, QueryResult};
    use crate::dialects::DatabaseDialect;
    use crate::{D1RsError, Entity};
    use async_trait::async_trait;
    use serde_json::{Map, Number};
    use sqlx::mysql::{MySqlPool, MySqlPoolOptions, MySqlRow};
    use sqlx::{Column, Row, TypeInfo};
    use std::time::Duration;

    /// MySQL database backend using sqlx connection pooling
    #[derive(Clone)]
    pub struct MySQLBackend {
        pool: Arc<MySqlPool>,
    }

    /// Query result implementation for MySQL using serde_json::Value storage
    #[derive(Debug)]
    pub struct MySQLQueryResult {
        rows: Vec<Value>,
    }

    impl MySQLBackend {
        /// Create a new MySQL backend with connection URL
        pub async fn new(url: &str) -> Result<Self, D1RsError> {
            let pool = MySqlPool::connect(url)
                .await
                .map_err(|e| D1RsError::Database(format!("MySQL connection failed: {}", e)))?;

            Ok(Self {
                pool: Arc::new(pool),
            })
        }

        /// Create MySQL backend from configuration with pool settings
        pub async fn from_config(config: &crate::backends::DatabaseConfig) -> Result<Self, D1RsError> {
            let pool_options = if let Some(pool_config) = &config.pool_config {
                MySqlPoolOptions::new()
                    .max_connections(pool_config.max_connections)
                    .min_connections(pool_config.min_connections)
                    .acquire_timeout(pool_config.connect_timeout)
                    .idle_timeout(pool_config.idle_timeout)
                    .max_lifetime(pool_config.max_lifetime)
            } else {
                MySqlPoolOptions::new()
                    .max_connections(10)
                    .min_connections(1)
                    .acquire_timeout(Duration::from_secs(30))
            };

            let pool = pool_options
                .connect(&config.url)
                .await
                .map_err(|e| D1RsError::Database(format!("MySQL pool creation failed: {}", e)))?;

            Ok(Self {
                pool: Arc::new(pool),
            })
        }

        /// Get current pool statistics
        pub fn pool_size(&self) -> u32 {
            self.pool.size()
        }

        /// Get number of idle connections in pool
        pub fn idle_connections(&self) -> usize {
            self.pool.num_idle()
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

        fn len(&self) -> usize {
            self.rows.len()
        }

        fn is_empty(&self) -> bool {
            self.rows.is_empty()
        }

        fn into_entities<T>(self) -> Result<Vec<T>, Self::Error>
        where
            T: serde::de::DeserializeOwned + Entity,
        {
            self.rows
                .into_iter()
                .map(|row| {
                    // Apply Entity conversion for MySQL-specific type handling
                    let converted = T::convert_from_sqlite(row);
                    serde_json::from_value(converted)
                        .map_err(|e| D1RsError::SerializationError(e.to_string()))
                })
                .collect()
        }

        fn into_entity<T>(mut self) -> Result<Option<T>, Self::Error>
        where
            T: serde::de::DeserializeOwned + Entity,
        {
            if let Some(row) = self.rows.pop() {
                let converted = T::convert_from_sqlite(row);
                let entity = serde_json::from_value(converted)
                    .map_err(|e| D1RsError::SerializationError(e.to_string()))?;
                Ok(Some(entity))
            } else {
                Ok(None)
            }
        }

        fn into_simple_entities<T>(self) -> Result<Vec<T>, Self::Error>
        where
            T: serde::de::DeserializeOwned,
        {
            self.rows
                .into_iter()
                .map(|row| {
                    serde_json::from_value(row)
                        .map_err(|e| D1RsError::SerializationError(e.to_string()))
                })
                .collect()
        }

        fn into_simple_entity<T>(mut self) -> Result<Option<T>, Self::Error>
        where
            T: serde::de::DeserializeOwned,
        {
            if let Some(row) = self.rows.pop() {
                let entity = serde_json::from_value(row)
                    .map_err(|e| D1RsError::SerializationError(e.to_string()))?;
                Ok(Some(entity))
            } else {
                Ok(None)
            }
        }

        fn extract_count(&self) -> Result<i64, Self::Error> {
            if let Some(row) = self.rows.first() {
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

        fn extract_id(&self) -> Result<Option<i64>, Self::Error> {
            if let Some(row) = self.rows.first() {
                if let Value::Object(obj) = row {
                    // Try to find an ID column by common names
                    for key in ["id", "ID", "rowid", "ROWID", "oid"] {
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

        fn extract_strings(&self) -> Result<Vec<String>, Self::Error> {
            let mut strings = Vec::new();

            for row in &self.rows {
                if let Value::Object(obj) = row {
                    for value in obj.values() {
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

        fn into_hashmap(self) -> Result<Option<HashMap<String, Value>>, Self::Error>
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

        fn column_names(&self) -> Vec<String> {
            if let Some(Value::Object(obj)) = self.rows.first() {
                obj.keys().cloned().collect()
            } else {
                Vec::new()
            }
        }

        fn has_column(&self, column_name: &str) -> bool {
            if let Some(Value::Object(obj)) = self.rows.first() {
                obj.contains_key(column_name)
            } else {
                false
            }
        }
    }

    #[async_trait]
    impl DatabaseBackend for MySQLBackend {
        type QueryResult = MySQLQueryResult;
        type Error = D1RsError;

        async fn execute_query(
            &self,
            sql: &str,
            params: &[Value],
        ) -> Result<Self::QueryResult, Self::Error> {
            // Build the query with parameter placeholders
            let mut query = sqlx::query(sql);

            // Bind parameters in order
            for param in params {
                query = bind_json_value_to_query(query, param)?;
            }

            // Execute the query and fetch all rows
            let rows = query
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| D1RsError::Database(format!("MySQL query failed: {}", e)))?;

            // Convert MySQL rows to serde_json::Value format
            let json_rows = rows
                .into_iter()
                .map(convert_mysql_row_to_json)
                .collect::<Result<Vec<_>, _>>()?;

            Ok(MySQLQueryResult { rows: json_rows })
        }

        async fn execute_schema(&self, sql: &str) -> Result<(), Self::Error> {
            sqlx::query(sql)
                .execute(&*self.pool)
                .await
                .map_err(|e| D1RsError::Database(format!("MySQL schema execution failed: {}", e)))?;
            Ok(())
        }

        fn dialect(&self) -> DatabaseDialect {
            DatabaseDialect::MySQL
        }

        fn connection_info(&self) -> String {
            format!(
                "MySQL pool: {}/{} connections ({} idle)",
                self.pool.size(),
                10, // max_connections would need to be stored to display accurately
                self.pool.num_idle()
            )
        }

        async fn ping(&self) -> Result<(), Self::Error> {
            sqlx::query("SELECT 1")
                .execute(&*self.pool)
                .await
                .map_err(|e| D1RsError::Database(format!("MySQL ping failed: {}", e)))?;
            Ok(())
        }
    }

    /// Convert a MySQL row to serde_json::Value format
    fn convert_mysql_row_to_json(row: MySqlRow) -> Result<Value, D1RsError> {
        let mut json_obj = Map::new();

        for (i, column) in row.columns().iter().enumerate() {
            let column_name = column.name().to_string();
            let type_info = column.type_info();

            // Handle different MySQL types and convert to serde_json::Value
            let json_value = match type_info.name() {
                "BOOLEAN" | "TINYINT" => {
                    // MySQL boolean is often TINYINT(1)
                    if let Ok(val) = row.try_get::<Option<bool>, _>(i) {
                        val.map(Value::Bool).unwrap_or(Value::Null)
                    } else if let Ok(val) = row.try_get::<Option<i8>, _>(i) {
                        val.map(|v| Value::Bool(v != 0)).unwrap_or(Value::Null)
                    } else {
                        Value::Null
                    }
                }
                "SMALLINT" | "MEDIUMINT" | "INT" | "BIGINT" => {
                    if let Ok(val) = row.try_get::<Option<i64>, _>(i) {
                        val.map(|n| Value::Number(Number::from(n)))
                            .unwrap_or(Value::Null)
                    } else {
                        Value::Null
                    }
                }
                "FLOAT" | "DOUBLE" | "DECIMAL" => {
                    if let Ok(val) = row.try_get::<Option<f64>, _>(i) {
                        val.and_then(|f| Number::from_f64(f).map(Value::Number))
                            .unwrap_or(Value::Null)
                    } else {
                        Value::Null
                    }
                }
                "VARCHAR" | "CHAR" | "TEXT" | "LONGTEXT" | "MEDIUMTEXT" | "TINYTEXT" => {
                    if let Ok(val) = row.try_get::<Option<String>, _>(i) {
                        val.map(Value::String).unwrap_or(Value::Null)
                    } else {
                        Value::Null
                    }
                }
                "DATE" | "DATETIME" | "TIMESTAMP" | "TIME" | "YEAR" => {
                    // Handle MySQL date/time types as strings
                    if let Ok(val) = row.try_get::<Option<String>, _>(i) {
                        val.map(Value::String).unwrap_or(Value::Null)
                    } else {
                        Value::Null
                    }
                }
                "JSON" => {
                    // Handle MySQL JSON type
                    if let Ok(val) = row.try_get::<Option<String>, _>(i) {
                        val.and_then(|json_str| {
                            Some(serde_json::from_str(&json_str).unwrap_or_else(|_| {
                                Value::String(json_str)
                            }))
                        })
                        .unwrap_or(Value::Null)
                    } else {
                        Value::Null
                    }
                }
                "BINARY" | "VARBINARY" | "BLOB" | "LONGBLOB" | "MEDIUMBLOB" | "TINYBLOB" => {
                    // Handle binary data as base64 string
                    if let Ok(val) = row.try_get::<Option<Vec<u8>>, _>(i) {
                        val.map(|bytes| {
                            use base64::Engine;
                            Value::String(base64::engine::general_purpose::STANDARD.encode(bytes))
                        })
                        .unwrap_or(Value::Null)
                    } else {
                        Value::Null
                    }
                }
                _ => {
                    // For unknown types, try to get as string
                    if let Ok(val) = row.try_get::<Option<String>, _>(i) {
                        val.map(Value::String).unwrap_or(Value::Null)
                    } else {
                        Value::Null
                    }
                }
            };

            json_obj.insert(column_name, json_value);
        }

        Ok(Value::Object(json_obj))
    }

    /// Bind a serde_json::Value to a sqlx query
    fn bind_json_value_to_query<'a>(
        query: sqlx::query::Query<'a, sqlx::MySql, sqlx::mysql::MySqlArguments>,
        value: &Value,
    ) -> Result<sqlx::query::Query<'a, sqlx::MySql, sqlx::mysql::MySqlArguments>, D1RsError> {
        match value {
            Value::Null => Ok(query.bind(None::<String>)),
            Value::Bool(b) => Ok(query.bind(*b)),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(query.bind(i))
                } else if let Some(u) = n.as_u64() {
                    Ok(query.bind(u as i64))
                } else if let Some(f) = n.as_f64() {
                    Ok(query.bind(f))
                } else {
                    Ok(query.bind(n.to_string()))
                }
            }
            Value::String(s) => Ok(query.bind(s.clone())),
            Value::Array(_) | Value::Object(_) => {
                // Bind complex types as JSON
                let json_str = serde_json::to_string(value)
                    .map_err(|e| D1RsError::SerializationError(e.to_string()))?;
                Ok(query.bind(json_str))
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::backends::DatabaseConfig;
        use std::env;

        fn get_test_url() -> Option<String> {
            env::var("MYSQL_TEST_URL").ok()
        }

        #[tokio::test]
        async fn test_mysql_backend_creation() {
            if let Some(url) = get_test_url() {
                let backend = MySQLBackend::new(&url).await;
                assert!(backend.is_ok(), "Failed to create MySQL backend");
            }
        }

        #[tokio::test]
        async fn test_mysql_backend_from_config() {
            if let Some(url) = get_test_url() {
                let config = DatabaseConfig::from_url(&url).expect("Failed to create config");
                let backend = MySQLBackend::from_config(&config).await;
                assert!(backend.is_ok(), "Failed to create MySQL backend from config");
            }
        }

        #[tokio::test]
        async fn test_mysql_backend_ping() {
            if let Some(url) = get_test_url() {
                let backend = MySQLBackend::new(&url).await.expect("Failed to create backend");
                let result = backend.ping().await;
                assert!(result.is_ok(), "MySQL ping failed: {:?}", result.err());
            }
        }

        #[tokio::test]
        async fn test_mysql_backend_basic_query() {
            if let Some(url) = get_test_url() {
                let backend = MySQLBackend::new(&url).await.expect("Failed to create backend");
                let result = backend.execute_query("SELECT 1 as test_value", &[]).await;
                assert!(result.is_ok(), "Basic query failed: {:?}", result.err());

                let query_result = result.unwrap();
                assert_eq!(query_result.len(), 1);
                assert!(query_result.has_column("test_value"));
            }
        }

        #[tokio::test]
        async fn test_mysql_backend_parameter_binding() {
            if let Some(url) = get_test_url() {
                let backend = MySQLBackend::new(&url).await.expect("Failed to create backend");
                let params = vec![
                    Value::String("test".to_string()),
                    Value::Number(Number::from(42)),
                    Value::Bool(true),
                ];

                let result = backend
                    .execute_query("SELECT ? as str_val, ? as num_val, ? as bool_val", &params)
                    .await;
                assert!(result.is_ok(), "Parameter binding failed: {:?}", result.err());

                let query_result = result.unwrap();
                assert_eq!(query_result.len(), 1);
                assert!(query_result.has_column("str_val"));
                assert!(query_result.has_column("num_val"));
                assert!(query_result.has_column("bool_val"));
            }
        }

        #[tokio::test]
        async fn test_mysql_backend_schema_operations() {
            if let Some(url) = get_test_url() {
                let backend = MySQLBackend::new(&url).await.expect("Failed to create backend");
                
                use sea_query::{MysqlQueryBuilder, Query, Table, ColumnDef, Alias};
                
                // Create a test table using sea-query
                let table_name = format!("test_mysql_schema_{}", chrono::Utc::now().timestamp_millis());
                let create_table = Table::create()
                    .table(Alias::new(&table_name))
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Alias::new("id"))
                            .integer()
                            .not_null()
                            .primary_key()
                    )
                    .col(
                        ColumnDef::new(Alias::new("name"))
                            .string_len(255)
                    )
                    .build(MysqlQueryBuilder);
                    
                let create_result = backend.execute_schema(&create_table).await;
                assert!(create_result.is_ok(), "Schema creation failed: {:?}", create_result.err());

                // Insert data using sea-query
                let (insert_sql, insert_values) = Query::insert()
                    .into_table(Alias::new(&table_name))
                    .columns([Alias::new("id"), Alias::new("name")])
                    .values_panic([1.into(), "test".into()])
                    .build(MysqlQueryBuilder);
                    
                // Convert sea-query Values to d1-rs Value array
                let insert_params: Vec<Value> = insert_values.0.into_iter().map(|v| match v {
                    sea_query::Value::Int(Some(i)) => Value::Number(Number::from(i)),
                    sea_query::Value::String(Some(s)) => Value::String(s.to_string()),
                    _ => Value::Null,
                }).collect();
                    
                let insert_result = backend.execute_query(&insert_sql, &insert_params).await;
                assert!(insert_result.is_ok(), "Insert failed: {:?}", insert_result.err());

                // Query the data back using sea-query
                let (select_sql, _) = Query::select()
                    .from(Alias::new(&table_name))
                    .columns([Alias::new("id"), Alias::new("name")])
                    .build(MysqlQueryBuilder);
                    
                let select_result = backend.execute_query(&select_sql, &[]).await;
                assert!(select_result.is_ok(), "Select failed: {:?}", select_result.err());

                let query_result = select_result.unwrap();
                assert_eq!(query_result.len(), 1);
                assert!(query_result.has_column("id"));
                assert!(query_result.has_column("name"));
                
                // Clean up using sea-query
                let drop_stmt = Table::drop()
                    .table(Alias::new(&table_name))
                    .if_exists()
                    .build(MysqlQueryBuilder);
                let _ = backend.execute_schema(&drop_stmt).await;
            }
        }

        #[tokio::test]
        async fn test_mysql_query_result_extract_methods() {
            if let Some(url) = get_test_url() {
                let backend = MySQLBackend::new(&url).await.expect("Failed to create backend");
                
                // Test count extraction
                let count_result = backend
                    .execute_query("SELECT 42 as count", &[])
                    .await
                    .expect("Count query failed");
                
                let count = count_result.extract_count().expect("Failed to extract count");
                assert_eq!(count, 42);

                // Test ID extraction
                let id_result = backend
                    .execute_query("SELECT 123 as id", &[])
                    .await
                    .expect("ID query failed");
                
                let id = id_result.extract_id().expect("Failed to extract ID");
                assert_eq!(id, Some(123));

                // Test string extraction
                let string_result = backend
                    .execute_query("SELECT 'hello' as greeting, 'world' as target", &[])
                    .await
                    .expect("String query failed");
                
                let strings = string_result.extract_strings().expect("Failed to extract strings");
                assert!(strings.contains(&"hello".to_string()));
                assert!(strings.contains(&"world".to_string()));
            }
        }

        #[tokio::test]
        async fn test_mysql_query_result_column_operations() {
            if let Some(url) = get_test_url() {
                let backend = MySQLBackend::new(&url).await.expect("Failed to create backend");
                let result = backend
                    .execute_query("SELECT 'test' as col1, 42 as col2, true as col3", &[])
                    .await
                    .expect("Query failed");

                let column_names = result.column_names();
                assert!(column_names.contains(&"col1".to_string()));
                assert!(column_names.contains(&"col2".to_string()));
                assert!(column_names.contains(&"col3".to_string()));

                assert!(result.has_column("col1"));
                assert!(result.has_column("col2"));
                assert!(result.has_column("col3"));
                assert!(!result.has_column("nonexistent"));
            }
        }

        #[tokio::test]
        async fn test_mysql_query_result_into_hashmap() {
            if let Some(url) = get_test_url() {
                let backend = MySQLBackend::new(&url).await.expect("Failed to create backend");
                let result = backend
                    .execute_query("SELECT 'value1' as key1, 'value2' as key2", &[])
                    .await
                    .expect("Query failed");

                let hashmap = result.into_hashmap().expect("Failed to convert to hashmap");
                assert!(hashmap.is_some());

                let map = hashmap.unwrap();
                assert_eq!(map.len(), 2);
                assert_eq!(map.get("key1"), Some(&Value::String("value1".to_string())));
                assert_eq!(map.get("key2"), Some(&Value::String("value2".to_string())));
            }
        }

        #[tokio::test]
        async fn test_mysql_query_result_empty_result() {
            if let Some(url) = get_test_url() {
                let backend = MySQLBackend::new(&url).await.expect("Failed to create backend");
                let result = backend
                    .execute_query("SELECT 1 as test WHERE 1 = 0", &[]) // Always false condition
                    .await
                    .expect("Query failed");

                assert!(result.is_empty());
                assert_eq!(result.len(), 0);
                
                let count = result.extract_count().expect("Failed to extract count");
                assert_eq!(count, 0);
                
                let id = result.extract_id().expect("Failed to extract ID");
                assert_eq!(id, None);
            }
        }
    }
}

#[cfg(feature = "mysql")]
pub use mysql_impl::{MySQLBackend, MySQLQueryResult};