use crate::{Result, D1RsError};
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