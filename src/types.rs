use chrono::{DateTime, Utc, NaiveDateTime};
use serde_json::Value;
use crate::{Result, D1RsError};
use base64::{Engine as _, engine::general_purpose};

pub trait SqlType {
    fn to_sql_value(&self) -> Value;
    fn from_sql_value(value: &Value) -> Result<Self> where Self: Sized;
    fn sql_type_name() -> &'static str;
}

impl SqlType for i32 {
    fn to_sql_value(&self) -> Value {
        serde_json::json!(*self)
    }

    fn from_sql_value(value: &Value) -> Result<Self> {
        match value {
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(i as i32)
                } else if let Some(i) = n.as_u64() {
                    Ok(i as i32)
                } else {
                    Err(D1RsError::SerializationError("Expected integer".to_string()))
                }
            },
            _ => Err(D1RsError::SerializationError("Expected number for i32".to_string())),
        }
    }

    fn sql_type_name() -> &'static str {
        "INTEGER"
    }
}

impl SqlType for i64 {
    fn to_sql_value(&self) -> Value {
        // D1 doesn't support bigint, so we force to i32 and use json! macro
        serde_json::json!(*self as i32)
    }

    fn from_sql_value(value: &Value) -> Result<Self> {
        match value {
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(i)
                } else if let Some(i) = n.as_u64() {
                    Ok(i as i64)
                } else {
                    Err(D1RsError::SerializationError("Expected integer".to_string()))
                }
            },
            _ => Err(D1RsError::SerializationError("Expected number for i64".to_string())),
        }
    }

    fn sql_type_name() -> &'static str {
        "INTEGER"
    }
}

impl SqlType for String {
    fn to_sql_value(&self) -> Value {
        Value::String(self.clone())
    }

    fn from_sql_value(value: &Value) -> Result<Self> {
        match value {
            Value::String(s) => Ok(s.clone()),
            Value::Null => Ok(String::new()),
            _ => Err(D1RsError::SerializationError("Expected string".to_string())),
        }
    }

    fn sql_type_name() -> &'static str {
        "TEXT"
    }
}

impl SqlType for bool {
    fn to_sql_value(&self) -> Value {
        Value::Number(if *self { 1 } else { 0 }.into())
    }

    fn from_sql_value(value: &Value) -> Result<Self> {
        match value {
            Value::Number(n) => Ok(n.as_i64().unwrap_or(0) != 0),
            Value::Bool(b) => Ok(*b),
            _ => Err(D1RsError::SerializationError("Expected boolean".to_string())),
        }
    }

    fn sql_type_name() -> &'static str {
        "INTEGER"
    }
}

impl SqlType for DateTime<Utc> {
    fn to_sql_value(&self) -> Value {
        Value::String(self.format("%Y-%m-%d %H:%M:%S").to_string())
    }

    fn from_sql_value(value: &Value) -> Result<Self> {
        match value {
            Value::String(s) => {
                if let Ok(naive) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
                    Ok(DateTime::from_naive_utc_and_offset(naive, Utc))
                } else {
                    Err(D1RsError::SerializationError("Invalid datetime format".to_string()))
                }
            },
            _ => Err(D1RsError::SerializationError("Expected string for datetime".to_string())),
        }
    }

    fn sql_type_name() -> &'static str {
        "DATETIME"
    }
}

impl<T: SqlType> SqlType for Option<T> {
    fn to_sql_value(&self) -> Value {
        match self {
            Some(val) => val.to_sql_value(),
            None => Value::Null,
        }
    }

    fn from_sql_value(value: &Value) -> Result<Self> {
        match value {
            Value::Null => Ok(None),
            _ => Ok(Some(T::from_sql_value(value)?)),
        }
    }

    fn sql_type_name() -> &'static str {
        T::sql_type_name()
    }
}
/// REVOLUTIONARY: Binary data support for Phase 3.3
/// Implements SqlType for Vec<u8> to handle binary data seamlessly
impl SqlType for Vec<u8> {
    fn to_sql_value(&self) -> Value {
        // Encode binary data as base64 for JSON transport
        let encoded = general_purpose::STANDARD.encode(self);
        Value::String(encoded)
    }

    fn from_sql_value(value: &Value) -> Result<Self> {
        match value {
            Value::String(s) => {
                // Decode base64 string back to binary data
                general_purpose::STANDARD.decode(s)
                    .map_err(|e| D1RsError::SerializationError(format!("Invalid base64 binary data: {}", e)))
            },
            Value::Null => Ok(Vec::new()),
            _ => Err(D1RsError::SerializationError("Expected string for binary data".to_string())),
        }
    }

    fn sql_type_name() -> &'static str {
        "BLOB"
    }
}
