use sea_query::{SelectStatement, InsertStatement, UpdateStatement, DeleteStatement, Value as SeaValue};
use crate::backends::DatabaseBackend;
use crate::db::DatabaseClient;
use crate::dialects::DatabaseDialect;
use serde_json::Value;

pub mod select;
pub mod insert;
pub mod update;
pub mod delete;

/// Convert serde_json::Value to sea_query::Value with comprehensive type mapping
pub fn json_to_sea_value(value: &Value) -> SeaValue {
    match value {
        Value::Null => SeaValue::BigInt(None),  // Use None variant for null
        Value::Bool(b) => SeaValue::Bool(Some(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                // Choose appropriate integer type based on size
                if i >= i32::MIN as i64 && i <= i32::MAX as i64 {
                    SeaValue::Int(Some(i as i32))
                } else {
                    SeaValue::BigInt(Some(i))
                }
            } else if let Some(u) = n.as_u64() {
                // Handle unsigned integers
                if u <= i64::MAX as u64 {
                    SeaValue::BigInt(Some(u as i64))
                } else {
                    // For very large unsigned integers, convert to string
                    SeaValue::String(Some(Box::new(u.to_string())))
                }
            } else if let Some(f) = n.as_f64() {
                SeaValue::Double(Some(f))
            } else {
                SeaValue::BigInt(None)  // Use None variant for invalid numbers
            }
        },
        Value::String(s) => SeaValue::String(Some(Box::new(s.clone()))),
        Value::Array(_) | Value::Object(_) => {
            // Convert complex types to JSON string for database storage
            SeaValue::String(Some(Box::new(value.to_string())))
        }
    }
}

/// Convert sea_query::Value back to serde_json::Value with comprehensive type mapping
pub fn sea_value_to_json(value: &SeaValue) -> Value {
    match value {
        SeaValue::Bool(Some(b)) => Value::Bool(*b),
        SeaValue::Bool(None) => Value::Null,
        SeaValue::TinyInt(Some(i)) => Value::Number((*i as i64).into()),
        SeaValue::TinyInt(None) => Value::Null,
        SeaValue::SmallInt(Some(i)) => Value::Number((*i as i64).into()),
        SeaValue::SmallInt(None) => Value::Null,
        SeaValue::Int(Some(i)) => Value::Number((*i as i64).into()),
        SeaValue::Int(None) => Value::Null,
        SeaValue::BigInt(Some(i)) => Value::Number((*i).into()),
        SeaValue::BigInt(None) => Value::Null,
        SeaValue::TinyUnsigned(Some(u)) => Value::Number((*u as i64).into()),
        SeaValue::TinyUnsigned(None) => Value::Null,
        SeaValue::SmallUnsigned(Some(u)) => Value::Number((*u as i64).into()),
        SeaValue::SmallUnsigned(None) => Value::Null,
        SeaValue::Unsigned(Some(u)) => Value::Number((*u as i64).into()),
        SeaValue::Unsigned(None) => Value::Null,
        SeaValue::BigUnsigned(Some(u)) => {
            if *u <= i64::MAX as u64 {
                Value::Number((*u as i64).into())
            } else {
                Value::String(u.to_string())
            }
        },
        SeaValue::BigUnsigned(None) => Value::Null,
        SeaValue::Float(Some(f)) => {
            serde_json::Number::from_f64(*f as f64)
                .map(Value::Number)
                .unwrap_or(Value::Null)
        },
        SeaValue::Float(None) => Value::Null,
        SeaValue::Double(Some(f)) => {
            serde_json::Number::from_f64(*f)
                .map(Value::Number)
                .unwrap_or(Value::Null)
        },
        SeaValue::Double(None) => Value::Null,
        SeaValue::String(Some(s)) => {
            // Try to parse as JSON first, fallback to string
            serde_json::from_str(s.as_ref()).unwrap_or_else(|_| Value::String(s.as_ref().clone()))
        },
        SeaValue::String(None) => Value::Null,
        SeaValue::Char(Some(c)) => Value::String(c.to_string()),
        SeaValue::Char(None) => Value::Null,
        SeaValue::Bytes(Some(bytes)) => {
            // Convert bytes to base64 string
            use base64::Engine;
            Value::String(base64::engine::general_purpose::STANDARD.encode(bytes.as_ref()))
        },
        SeaValue::Bytes(None) => Value::Null,
    }
}

/// Trait for rendering sea-query statements to SQL for specific database dialects
pub trait QueryRenderer {
    /// Render the query to SQL and parameters for a specific database dialect
    fn render_for_dialect(&self, dialect: DatabaseDialect) -> (String, Vec<Value>);
}

impl QueryRenderer for SelectStatement {
    fn render_for_dialect(&self, dialect: DatabaseDialect) -> (String, Vec<Value>) {
        match dialect {
            DatabaseDialect::SQLite => {
                let (sql, values) = self.build(sea_query::SqliteQueryBuilder);
                let json_values = values.iter().map(sea_value_to_json).collect();
                (sql, json_values)
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                let (sql, values) = self.build(sea_query::PostgresQueryBuilder);
                let json_values = values.iter().map(sea_value_to_json).collect();
                (sql, json_values)
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                let (sql, values) = self.build(sea_query::MysqlQueryBuilder);
                let json_values = values.iter().map(sea_value_to_json).collect();
                (sql, json_values)
            },
        }
    }
}

impl QueryRenderer for InsertStatement {
    fn render_for_dialect(&self, dialect: DatabaseDialect) -> (String, Vec<Value>) {
        match dialect {
            DatabaseDialect::SQLite => {
                let (sql, values) = self.build(sea_query::SqliteQueryBuilder);
                let json_values = values.iter().map(sea_value_to_json).collect();
                (sql, json_values)
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                let (sql, values) = self.build(sea_query::PostgresQueryBuilder);
                let json_values = values.iter().map(sea_value_to_json).collect();
                (sql, json_values)
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                let (sql, values) = self.build(sea_query::MysqlQueryBuilder);
                let json_values = values.iter().map(sea_value_to_json).collect();
                (sql, json_values)
            },
        }
    }
}

impl QueryRenderer for UpdateStatement {
    fn render_for_dialect(&self, dialect: DatabaseDialect) -> (String, Vec<Value>) {
        match dialect {
            DatabaseDialect::SQLite => {
                let (sql, values) = self.build(sea_query::SqliteQueryBuilder);
                let json_values = values.iter().map(sea_value_to_json).collect();
                (sql, json_values)
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                let (sql, values) = self.build(sea_query::PostgresQueryBuilder);
                let json_values = values.iter().map(sea_value_to_json).collect();
                (sql, json_values)
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                let (sql, values) = self.build(sea_query::MysqlQueryBuilder);
                let json_values = values.iter().map(sea_value_to_json).collect();
                (sql, json_values)
            },
        }
    }
}

impl QueryRenderer for DeleteStatement {
    fn render_for_dialect(&self, dialect: DatabaseDialect) -> (String, Vec<Value>) {
        match dialect {
            DatabaseDialect::SQLite => {
                let (sql, values) = self.build(sea_query::SqliteQueryBuilder);
                let json_values = values.iter().map(sea_value_to_json).collect();
                (sql, json_values)
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                let (sql, values) = self.build(sea_query::PostgresQueryBuilder);
                let json_values = values.iter().map(sea_value_to_json).collect();
                (sql, json_values)
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                let (sql, values) = self.build(sea_query::MysqlQueryBuilder);
                let json_values = values.iter().map(sea_value_to_json).collect();
                (sql, json_values)
            },
        }
    }
}

/// Extension trait for DatabaseClient to execute sea-query statements
#[async_trait::async_trait]
pub trait SeaQueryExecutor<B: DatabaseBackend> {
    /// Execute a sea-query statement that implements QueryRenderer
    async fn execute_sea_query<Q>(&self, query: Q) -> Result<B::QueryResult, B::Error>
    where
        Q: QueryRenderer + Send;
}

#[async_trait::async_trait]
impl<B: DatabaseBackend> SeaQueryExecutor<B> for DatabaseClient<B> {
    async fn execute_sea_query<Q>(&self, query: Q) -> Result<B::QueryResult, B::Error>
    where
        Q: QueryRenderer + Send,
    {
        let (sql, params) = query.render_for_dialect(self.dialect());
        self.execute(&sql, &params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Number;
    use sea_query::{Iden, Query};

    #[derive(Iden)]
    enum TestTable {
        Table,
        Id,
        Name,
        Age,
        Active,
    }

    #[test]
    fn test_json_to_sea_value_null() {
        let json_val = Value::Null;
        let sea_val = json_to_sea_value(&json_val);
        assert!(matches!(sea_val, SeaValue::BigInt(None)));
    }

    #[test]
    fn test_json_to_sea_value_bool() {
        let json_val = Value::Bool(true);
        let sea_val = json_to_sea_value(&json_val);
        assert!(matches!(sea_val, SeaValue::Bool(Some(true))));

        let json_val = Value::Bool(false);
        let sea_val = json_to_sea_value(&json_val);
        assert!(matches!(sea_val, SeaValue::Bool(Some(false))));
    }

    #[test]
    fn test_json_to_sea_value_integer() {
        // Small integer -> Int
        let json_val = Value::Number(Number::from(42));
        let sea_val = json_to_sea_value(&json_val);
        assert!(matches!(sea_val, SeaValue::Int(Some(42))));

        // Large integer -> BigInt
        let json_val = Value::Number(Number::from(9223372036854775807i64));
        let sea_val = json_to_sea_value(&json_val);
        assert!(matches!(sea_val, SeaValue::BigInt(Some(9223372036854775807))));
    }

    #[test]
    fn test_json_to_sea_value_float() {
        let json_val = Value::Number(Number::from_f64(3.14).unwrap());
        let sea_val = json_to_sea_value(&json_val);
        assert!(matches!(sea_val, SeaValue::Double(Some(val)) if (val - 3.14).abs() < f64::EPSILON));
    }

    #[test]
    fn test_json_to_sea_value_string() {
        let json_val = Value::String("hello".to_string());
        let sea_val = json_to_sea_value(&json_val);
        assert!(matches!(sea_val, SeaValue::String(Some(ref s)) if s.as_ref() == "hello"));
    }

    #[test]
    fn test_json_to_sea_value_array() {
        let json_val = Value::Array(vec![Value::Number(Number::from(1)), Value::Number(Number::from(2))]);
        let sea_val = json_to_sea_value(&json_val);
        assert!(matches!(sea_val, SeaValue::String(Some(ref s)) if s.as_ref() == "[1,2]"));
    }

    #[test]
    fn test_json_to_sea_value_object() {
        let mut obj = serde_json::Map::new();
        obj.insert("key".to_string(), Value::String("value".to_string()));
        let json_val = Value::Object(obj);
        let sea_val = json_to_sea_value(&json_val);
        assert!(matches!(sea_val, SeaValue::String(Some(ref s)) if s.contains("key") && s.contains("value")));
    }

    #[test]
    fn test_sea_value_to_json_null() {
        let sea_val = SeaValue::BigInt(None);
        let json_val = sea_value_to_json(&sea_val);
        assert_eq!(json_val, Value::Null);
    }

    #[test]
    fn test_sea_value_to_json_bool() {
        let sea_val = SeaValue::Bool(Some(true));
        let json_val = sea_value_to_json(&sea_val);
        assert_eq!(json_val, Value::Bool(true));

        let sea_val = SeaValue::Bool(None);
        let json_val = sea_value_to_json(&sea_val);
        assert_eq!(json_val, Value::Null);
    }

    #[test]
    fn test_sea_value_to_json_integers() {
        let sea_val = SeaValue::Int(Some(42));
        let json_val = sea_value_to_json(&sea_val);
        assert_eq!(json_val, Value::Number(Number::from(42)));

        let sea_val = SeaValue::BigInt(Some(9223372036854775807));
        let json_val = sea_value_to_json(&sea_val);
        assert_eq!(json_val, Value::Number(Number::from(9223372036854775807i64)));
    }

    #[test]
    fn test_sea_value_to_json_float() {
        let sea_val = SeaValue::Double(Some(3.14));
        let json_val = sea_value_to_json(&sea_val);
        if let Value::Number(n) = json_val {
            assert!((n.as_f64().unwrap() - 3.14).abs() < f64::EPSILON);
        } else {
            panic!("Expected number");
        }
    }

    #[test]
    fn test_sea_value_to_json_string() {
        let sea_val = SeaValue::String(Some(Box::new("hello".to_string())));
        let json_val = sea_value_to_json(&sea_val);
        assert_eq!(json_val, Value::String("hello".to_string()));
    }

    #[test]
    fn test_sea_value_to_json_string_with_json() {
        let sea_val = SeaValue::String(Some(Box::new(r#"{"key":"value"}"#.to_string())));
        let json_val = sea_value_to_json(&sea_val);
        
        if let Value::Object(obj) = json_val {
            assert_eq!(obj.get("key"), Some(&Value::String("value".to_string())));
        } else {
            panic!("Expected parsed JSON object");
        }
    }

    #[test]
    fn test_query_renderer_select_sqlite() {
        let query = Query::select()
            .from(TestTable::Table)
            .column(TestTable::Id)
            .column(TestTable::Name)
            .and_where(sea_query::Expr::col(TestTable::Id).eq(1))
            .to_owned();

        let (sql, params) = query.render_for_dialect(DatabaseDialect::SQLite);
        
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("FROM"));
        assert!(sql.contains("WHERE"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], Value::Number(Number::from(1)));
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn test_query_renderer_select_postgres() {
        let query = Query::select()
            .from(TestTable::Table)
            .column(TestTable::Id)
            .column(TestTable::Name)
            .and_where(sea_query::Expr::col(TestTable::Id).eq(1))
            .to_owned();

        let (sql, params) = query.render_for_dialect(DatabaseDialect::PostgreSQL);
        
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("FROM"));
        assert!(sql.contains("WHERE"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], Value::Number(Number::from(1)));
    }

    #[cfg(feature = "mysql")]
    #[test]
    fn test_query_renderer_select_mysql() {
        let query = Query::select()
            .from(TestTable::Table)
            .column(TestTable::Id)
            .column(TestTable::Name)
            .and_where(sea_query::Expr::col(TestTable::Id).eq(1))
            .to_owned();

        let (sql, params) = query.render_for_dialect(DatabaseDialect::MySQL);
        
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("FROM"));
        assert!(sql.contains("WHERE"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], Value::Number(Number::from(1)));
    }

    #[test]
    fn test_query_renderer_insert_sqlite() {
        let query = Query::insert()
            .into_table(TestTable::Table)
            .columns([TestTable::Name, TestTable::Age, TestTable::Active])
            .values_panic(vec![
                "John".into(),
                30.into(),
                true.into(),
            ])
            .to_owned();

        let (sql, params) = query.render_for_dialect(DatabaseDialect::SQLite);
        
        assert!(sql.contains("INSERT INTO"));
        assert!(sql.contains("VALUES"));
        assert_eq!(params.len(), 3);
        assert_eq!(params[0], Value::String("John".to_string()));
        assert_eq!(params[1], Value::Number(Number::from(30)));
        assert_eq!(params[2], Value::Bool(true));
    }

    #[test]
    fn test_query_renderer_update_sqlite() {
        let query = Query::update()
            .table(TestTable::Table)
            .value(TestTable::Name, "Jane")
            .value(TestTable::Age, 25)
            .and_where(sea_query::Expr::col(TestTable::Id).eq(1))
            .to_owned();

        let (sql, params) = query.render_for_dialect(DatabaseDialect::SQLite);
        
        assert!(sql.contains("UPDATE"));
        assert!(sql.contains("SET"));
        assert!(sql.contains("WHERE"));
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn test_query_renderer_delete_sqlite() {
        let query = Query::delete()
            .from_table(TestTable::Table)
            .and_where(sea_query::Expr::col(TestTable::Id).eq(1))
            .to_owned();

        let (sql, params) = query.render_for_dialect(DatabaseDialect::SQLite);
        
        assert!(sql.contains("DELETE FROM"));
        assert!(sql.contains("WHERE"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], Value::Number(Number::from(1)));
    }

    #[test]
    fn test_value_conversion_roundtrip() {
        let original_values = vec![
            Value::Null,
            Value::Bool(true),
            Value::Bool(false),
            Value::Number(Number::from(42)),
            Value::Number(Number::from(9223372036854775807i64)),
            Value::Number(Number::from_f64(3.14159).unwrap()),
            Value::String("hello world".to_string()),
        ];

        for original in original_values {
            let sea_val = json_to_sea_value(&original);
            let converted_back = sea_value_to_json(&sea_val);
            
            match (&original, &converted_back) {
                (Value::Number(n1), Value::Number(n2)) => {
                    // For numbers, check approximate equality for floats
                    if let (Some(f1), Some(f2)) = (n1.as_f64(), n2.as_f64()) {
                        assert!((f1 - f2).abs() < f64::EPSILON, "Float values don't match: {} vs {}", f1, f2);
                    } else {
                        assert_eq!(original, converted_back, "Number conversion failed");
                    }
                }
                _ => {
                    assert_eq!(original, converted_back, "Value conversion roundtrip failed");
                }
            }
        }
    }

    #[test]
    fn test_complex_query_with_multiple_conditions() {
        let query = Query::select()
            .from(TestTable::Table)
            .columns([TestTable::Id, TestTable::Name, TestTable::Age])
            .and_where(sea_query::Expr::col(TestTable::Age).gte(18))
            .and_where(sea_query::Expr::col(TestTable::Active).eq(true))
            .and_where(sea_query::Expr::col(TestTable::Name).like("%John%"))
            .order_by(TestTable::Age, sea_query::Order::Asc)
            .limit(10)
            .to_owned();

        let (sql, params) = query.render_for_dialect(DatabaseDialect::SQLite);
        
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("FROM"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("ORDER BY"));
        assert!(sql.contains("LIMIT"));
        assert_eq!(params.len(), 3);
        assert_eq!(params[0], Value::Number(Number::from(18)));
        assert_eq!(params[1], Value::Bool(true));
        assert_eq!(params[2], Value::String("%John%".to_string()));
    }
}