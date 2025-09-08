use crate::{Result, D1OrmError};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use worker::wasm_bindgen::JsValue;
use worker::d1::D1Database;

#[derive(Clone)]
pub struct D1Client {
    db: Arc<D1Database>,
}

impl D1Client {
    pub fn new(db: D1Database) -> Self {
        Self { db: Arc::new(db) }
    }

    pub async fn execute(&self, sql: &str, params: &[Value]) -> Result<D1QueryResult> {
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
            stmt = stmt.bind(&js_params).map_err(|e| D1OrmError::Database(format!("{:?}", e)))?;
        }
        
        let result = stmt.all().await.map_err(|e| D1OrmError::Database(format!("{:?}", e)))?;
        let rows = result.results::<serde_json::Value>().map_err(|e| D1OrmError::Database(format!("{:?}", e)))?;
        
        Ok(D1QueryResult { rows })
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

pub struct D1QueryResult {
    pub rows: Vec<Value>,
}

impl D1QueryResult {
    pub fn into_entities<T: serde::de::DeserializeOwned>(self) -> Result<Vec<T>> {
        self.rows
            .into_iter()
            .map(|row| serde_json::from_value(row).map_err(|e| D1OrmError::SerializationError(e.to_string())))
            .collect()
    }

    pub fn into_entity<T: serde::de::DeserializeOwned>(mut self) -> Result<Option<T>> {
        if let Some(row) = self.rows.pop() {
            let entity = serde_json::from_value(row).map_err(|e| D1OrmError::SerializationError(e.to_string()))?;
            Ok(Some(entity))
        } else {
            Ok(None)
        }
    }
}