use async_trait::async_trait;
use gluesql::prelude::*;
use gluesql::core::ast::{ColumnDef, DataType};
use gluesql::core::data::{Schema, Row, CustomFunction as CustomFunc};
use gluesql::core::store::{
    DataRow, Store, StoreMut, RowIter, Index, IndexMut, Metadata, 
    CustomFunction, CustomFunctionMut, AlterTable, Transaction
};
use std::sync::Arc;
use worker::d1::D1Database;
use serde_json::json;
use wasm_bindgen::JsValue;
use futures::stream;

#[derive(Clone)]
pub struct D1Storage {
    db: Arc<D1Database>,
}

impl D1Storage {
    pub fn new(db: D1Database) -> Self {
        Self { db: Arc::new(db) }
    }

    async fn table_exists(&self, table_name: &str) -> Result<bool> {
        let sql = "SELECT name FROM sqlite_master WHERE type='table' AND name=?";
        let stmt = self.db.prepare(sql).bind(&[table_name.into()]).unwrap();
        
        match stmt.first::<serde_json::Value>(None).await {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(e) => Err(gluesql::core::error::Error::StorageMsg(format!("Failed to check table existence: {:?}", e))),
        }
    }

    async fn get_schema(&self, table_name: &str) -> Result<Schema> {
        let sql = format!("PRAGMA table_info({})", table_name);
        let stmt = self.db.prepare(&sql);
        
        let result = stmt.all().await
            .map_err(|e| gluesql::core::error::Error::StorageMsg(format!("Failed to get table info: {:?}", e)))?;
        
        let rows = result.results::<serde_json::Value>()
            .map_err(|e| gluesql::core::error::Error::StorageMsg(format!("Failed to parse table info: {:?}", e)))?;
        
        let column_defs: Vec<ColumnDef> = rows.iter()
            .map(|row| {
                let name = row["name"].as_str().unwrap_or("").to_string();
                let sql_type = row["type"].as_str().unwrap_or("TEXT").to_string();
                let nullable = row["notnull"].as_i64().unwrap_or(0) == 0;
                
                let data_type = match sql_type.to_uppercase().as_str() {
                    "INTEGER" | "INT" => DataType::Int,
                    "TEXT" | "VARCHAR" => DataType::Text,
                    "REAL" | "FLOAT" | "DOUBLE" => DataType::Float,
                    "BOOLEAN" | "BOOL" => DataType::Boolean,
                    "DATETIME" => DataType::Timestamp,
                    _ => DataType::Text,
                };
                
                ColumnDef {
                    name: name.clone(),
                    data_type,
                    nullable,
                    default: None,
                    unique: None,
                }
            })
            .collect();
        
        Ok(Schema {
            table_name: table_name.to_string(),
            column_defs: Some(column_defs),
            indexes: vec![],
            engine: None,
        })
    }

    async fn execute_query(&self, sql: &str, params: Vec<serde_json::Value>) -> Result<Vec<Row>> {
        let mut stmt = self.db.prepare(sql);
        
        for param in params {
            stmt = match param {
                serde_json::Value::Null => stmt.bind(&[JsValue::NULL]).unwrap(),
                serde_json::Value::Bool(b) => stmt.bind(&[(b as i32).into()]).unwrap(),
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        stmt.bind(&[i.into()]).unwrap()
                    } else if let Some(f) = n.as_f64() {
                        stmt.bind(&[f.into()]).unwrap()
                    } else {
                        stmt.bind(&[JsValue::NULL]).unwrap()
                    }
                },
                serde_json::Value::String(s) => stmt.bind(&[s.as_str().into()]).unwrap(),
                _ => stmt.bind(&[JsValue::NULL]).unwrap(),
            };
        }
        
        let result = stmt.all().await
            .map_err(|e| gluesql::core::error::Error::StorageMsg(format!("Query execution failed: {:?}", e)))?;
        
        let json_rows = result.results::<serde_json::Value>()
            .map_err(|e| gluesql::core::error::Error::StorageMsg(format!("Failed to parse results: {:?}", e)))?;
        
        let rows: Vec<Row> = json_rows.iter()
            .map(|json_row| {
                let values: Vec<Value> = if let serde_json::Value::Object(obj) = json_row {
                    obj.values().map(|v| json_value_to_glue_value(v)).collect()
                } else {
                    vec![]
                };
                let columns: Vec<String> = if let serde_json::Value::Object(obj) = json_row {
                    obj.keys().cloned().collect()
                } else {
                    vec![]
                };
                Row::Vec { columns: columns.into(), values }
            })
            .collect();
        
        Ok(rows)
    }
}

fn json_value_to_glue_value(json: &serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::I64(i)
            } else if let Some(f) = n.as_f64() {
                Value::F64(f)
            } else {
                Value::Null
            }
        },
        serde_json::Value::String(s) => Value::Str(s.clone()),
        _ => Value::Null,
    }
}

fn glue_value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::I8(i) => json!(*i),
        Value::I16(i) => json!(*i),
        Value::I32(i) => json!(*i),
        Value::I64(i) => json!(*i),
        Value::I128(i) => json!(*i),
        Value::U8(u) => json!(*u),
        Value::U16(u) => json!(*u),
        Value::U32(u) => json!(*u),
        Value::U64(u) => json!(*u),
        Value::U128(u) => json!(*u),
        Value::F32(f) => json!(*f),
        Value::F64(f) => json!(*f),
        Value::Str(s) => json!(s),
        Value::Bytea(b) => json!(String::from_utf8_lossy(b)),
        Value::Date(d) => json!(d.to_string()),
        Value::Time(t) => json!(t.to_string()),
        Value::Timestamp(ts) => json!(ts.to_string()),
        Value::Interval(i) => json!(format!("{:?}", i)),
        Value::Uuid(u) => json!(u.to_string()),
        Value::Map(m) => {
            let obj: serde_json::Map<String, serde_json::Value> = m.iter()
                .map(|(k, v)| (k.clone(), glue_value_to_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        },
        Value::List(l) => {
            let arr: Vec<serde_json::Value> = l.iter()
                .map(glue_value_to_json)
                .collect();
            serde_json::Value::Array(arr)
        },
        Value::Decimal(d) => json!(d.to_string()),
        Value::Point(p) => json!(format!("POINT({} {})", p.x, p.y)),
        Value::Inet(addr) => json!(addr.to_string()),
    }
}

#[async_trait(?Send)]
impl Store for D1Storage {
    async fn fetch_all_schemas(&self) -> Result<Vec<Schema>> {
        let sql = "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'";
        let stmt = self.db.prepare(sql);
        
        let result = stmt.all().await
            .map_err(|e| gluesql::core::error::Error::StorageMsg(format!("Failed to fetch table names: {:?}", e)))?;
        
        let tables = result.results::<serde_json::Value>()
            .map_err(|e| gluesql::core::error::Error::StorageMsg(format!("Failed to parse table names: {:?}", e)))?;
        
        let mut schemas = Vec::new();
        for table in tables {
            if let Some(name) = table["name"].as_str() {
                schemas.push(self.get_schema(name).await?);
            }
        }
        
        Ok(schemas)
    }

    async fn fetch_schema(&self, table_name: &str) -> Result<Option<Schema>> {
        if self.table_exists(table_name).await? {
            Ok(Some(self.get_schema(table_name).await?))
        } else {
            Ok(None)
        }
    }

    async fn fetch_data(&self, table_name: &str, key: &Key) -> Result<Option<DataRow>> {
        let schema = match self.fetch_schema(table_name).await? {
            Some(s) => s,
            None => return Ok(None),
        };
        
        let primary_key = if let Some(ref columns) = schema.column_defs {
            columns.first()
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "id".to_string())
        } else {
            "id".to_string()
        };
        
        let sql = format!("SELECT * FROM {} WHERE {} = ? LIMIT 1", table_name, primary_key);
        let key_value = match key {
            Key::I64(i) => json!(*i),
            Key::Str(s) => json!(s),
            _ => return Ok(None),
        };
        
        let rows = self.execute_query(&sql, vec![key_value]).await?;
        
        if let Some(row) = rows.into_iter().next() {
            let values = match row {
                Row::Vec { values, .. } => values,
                _ => vec![],
            };
            Ok(Some(DataRow::Vec(values)))
        } else {
            Ok(None)
        }
    }

    async fn scan_data(&self, table_name: &str) -> Result<RowIter> {
        let sql = format!("SELECT * FROM {}", table_name);
        let rows = self.execute_query(&sql, vec![]).await?;
        
        let data_rows: Vec<Result<(Key, DataRow)>> = rows.into_iter()
            .enumerate()
            .map(|(idx, row)| {
                let key = Key::I64(idx as i64);
                let values = match row {
                    Row::Vec { values, .. } => values,
                    _ => vec![],
                };
                Ok((key, DataRow::Vec(values)))
            })
            .collect();
        
        Ok(Box::pin(stream::iter(data_rows)))
    }
}

#[async_trait(?Send)]
impl StoreMut for D1Storage {
    async fn insert_schema(&mut self, schema: &Schema) -> Result<()> {
        let mut sql = format!("CREATE TABLE IF NOT EXISTS {} (", schema.table_name);
        
        if let Some(ref columns) = schema.column_defs {
            for (idx, col) in columns.iter().enumerate() {
                if idx > 0 {
                    sql.push_str(", ");
                }
                
                sql.push_str(&col.name);
                sql.push(' ');
                
                let sql_type = match col.data_type {
                    gluesql::core::ast::DataType::Int => "INTEGER",
                    gluesql::core::ast::DataType::Float => "REAL",
                    gluesql::core::ast::DataType::Text => "TEXT",
                    gluesql::core::ast::DataType::Boolean => "INTEGER",
                    gluesql::core::ast::DataType::Timestamp => "DATETIME",
                    _ => "TEXT",
                };
                sql.push_str(sql_type);
                
                if !col.nullable {
                    sql.push_str(" NOT NULL");
                }
                
                if col.unique.is_some() {
                    sql.push_str(" UNIQUE");
                }
            }
        }
        
        sql.push(')');
        
        self.db.prepare(&sql).run().await
            .map_err(|e| gluesql::core::error::Error::StorageMsg(format!("Failed to create table: {:?}", e)))?;
        
        Ok(())
    }

    async fn delete_schema(&mut self, table_name: &str) -> Result<()> {
        let sql = format!("DROP TABLE IF EXISTS {}", table_name);
        
        self.db.prepare(&sql).run().await
            .map_err(|e| gluesql::core::error::Error::StorageMsg(format!("Failed to drop table: {:?}", e)))?;
        
        Ok(())
    }

    async fn append_data(&mut self, table_name: &str, rows: Vec<DataRow>) -> Result<()> {
        let schema = match self.fetch_schema(table_name).await? {
            Some(s) => s,
            None => return Err(gluesql::core::error::Error::StorageMsg(format!("Table {} not found", table_name))),
        };
        
        for data_row in rows {
            let mut sql = format!("INSERT INTO {} (", table_name);
            let mut placeholders = String::new();
            let mut params = Vec::new();
            
            let values = match &data_row {
                DataRow::Vec(values) => values,
                _ => continue, // Skip non-Vec rows
            };
            
            if let Some(ref columns) = schema.column_defs {
                for (idx, (col, value)) in columns.iter().zip(values.iter()).enumerate() {
                    if idx > 0 {
                        sql.push_str(", ");
                        placeholders.push_str(", ");
                    }
                    sql.push_str(&col.name);
                    placeholders.push('?');
                    params.push(glue_value_to_json(value));
                }
            }
            
            sql.push_str(") VALUES (");
            sql.push_str(&placeholders);
            sql.push(')');
            
            let mut stmt = self.db.prepare(&sql);
            for param in params {
                stmt = match param {
                    serde_json::Value::Null => stmt.bind(&[JsValue::NULL]).unwrap(),
                    serde_json::Value::Bool(b) => stmt.bind(&[(b as i32).into()]).unwrap(),
                    serde_json::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            stmt.bind(&[i.into()]).unwrap()
                        } else if let Some(f) = n.as_f64() {
                            stmt.bind(&[f.into()]).unwrap()
                        } else {
                            stmt.bind(&[JsValue::NULL]).unwrap()
                        }
                    },
                    serde_json::Value::String(s) => stmt.bind(&[s.as_str().into()]).unwrap(),
                    _ => stmt.bind(&[JsValue::NULL]).unwrap(),
                };
            }
            
            stmt.run().await
                .map_err(|e| gluesql::core::error::Error::StorageMsg(format!("Failed to insert data: {:?}", e)))?;
        }
        
        Ok(())
    }

    async fn insert_data(&mut self, table_name: &str, rows: Vec<(Key, DataRow)>) -> Result<()> {
        let data_rows: Vec<DataRow> = rows.into_iter().map(|(_, row)| row).collect();
        self.append_data(table_name, data_rows).await
    }

    async fn delete_data(&mut self, table_name: &str, keys: Vec<Key>) -> Result<()> {
        let schema = match self.fetch_schema(table_name).await? {
            Some(s) => s,
            None => return Err(gluesql::core::error::Error::StorageMsg(format!("Table {} not found", table_name))),
        };
        
        let primary_key = if let Some(ref columns) = schema.column_defs {
            columns.first()
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "id".to_string())
        } else {
            "id".to_string()
        };
        
        for key in keys {
            let sql = format!("DELETE FROM {} WHERE {} = ?", table_name, primary_key);
            let key_value = match key {
                Key::I64(i) => json!(i),
                Key::Str(s) => json!(s),
                _ => continue,
            };
            
            let mut stmt = self.db.prepare(&sql);
            stmt = match key_value {
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        stmt.bind(&[i.into()]).unwrap()
                    } else {
                        continue;
                    }
                },
                serde_json::Value::String(s) => stmt.bind(&[s.as_str().into()]).unwrap(),
                _ => continue,
            };
            
            stmt.run().await
                .map_err(|e| gluesql::core::error::Error::StorageMsg(format!("Failed to delete data: {:?}", e)))?;
        }
        
        Ok(())
    }
}

// Implement required traits for GlueSQL compatibility
#[async_trait(?Send)]
impl Index for D1Storage {}

#[async_trait(?Send)]
impl IndexMut for D1Storage {
    async fn create_index(
        &mut self,
        _table_name: &str,
        _index_name: &str,
        _column: &gluesql::core::ast::OrderByExpr,
    ) -> Result<()> {
        // D1 doesn't support custom indexes in this implementation
        Ok(())
    }

    async fn drop_index(&mut self, _table_name: &str, _index_name: &str) -> Result<()> {
        // D1 doesn't support custom indexes in this implementation
        Ok(())
    }
}

#[async_trait(?Send)]
impl Metadata for D1Storage {
    async fn scan_table_meta(&self) -> Result<Box<dyn std::iter::Iterator<Item = Result<(String, std::collections::HashMap<String, Value>)>>>> {
        // Return basic table metadata
        let rows: Vec<Result<(String, std::collections::HashMap<String, Value>)>> = vec![];
        Ok(Box::new(rows.into_iter()))
    }
}

#[async_trait(?Send)]
impl CustomFunction for D1Storage {
    async fn fetch_function(&self, _func_name: &str) -> Result<Option<&CustomFunc>> {
        // No custom functions supported
        Ok(None)
    }

    async fn fetch_all_functions(&self) -> Result<Vec<&CustomFunc>> {
        // No custom functions supported
        Ok(vec![])
    }
}

#[async_trait(?Send)]
impl CustomFunctionMut for D1Storage {
    async fn insert_function(&mut self, _func: CustomFunc) -> Result<()> {
        // No custom functions supported
        Ok(())
    }

    async fn delete_function(&mut self, _func_name: &str) -> Result<()> {
        // No custom functions supported
        Ok(())
    }
}

#[async_trait(?Send)]
impl AlterTable for D1Storage {
    async fn rename_schema(&mut self, _table_name: &str, _new_table_name: &str) -> Result<()> {
        // Basic rename table support could be added here
        Err(gluesql::core::error::Error::StorageMsg("ALTER TABLE not supported".to_string()))
    }

    async fn rename_column(
        &mut self,
        _table_name: &str,
        _old_column_name: &str,
        _new_column_name: &str,
    ) -> Result<()> {
        // Column rename not supported
        Err(gluesql::core::error::Error::StorageMsg("ALTER TABLE not supported".to_string()))
    }

    async fn add_column(&mut self, _table_name: &str, _column_def: &ColumnDef) -> Result<()> {
        // Add column not supported
        Err(gluesql::core::error::Error::StorageMsg("ALTER TABLE not supported".to_string()))
    }

    async fn drop_column(&mut self, _table_name: &str, _column_name: &str, _if_exists: bool) -> Result<()> {
        // Drop column not supported
        Err(gluesql::core::error::Error::StorageMsg("ALTER TABLE not supported".to_string()))
    }
}

#[async_trait(?Send)]
impl Transaction for D1Storage {
    async fn begin(&mut self, _is_read_only: bool) -> Result<bool> {
        // D1 doesn't support transactions in this implementation
        Ok(false)
    }

    async fn rollback(&mut self) -> Result<()> {
        // D1 doesn't support transactions in this implementation
        Ok(())
    }

    async fn commit(&mut self) -> Result<()> {
        // D1 doesn't support transactions in this implementation
        Ok(())
    }
}