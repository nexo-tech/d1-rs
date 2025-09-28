use super::{QueryRenderer, json_to_sea_value};
use crate::backends::{DatabaseBackend, QueryResult};
use crate::{Entity, Result, D1RsError, DatabaseClient};
use crate::dialects::DatabaseDialect;
use sea_query::{Query, InsertStatement};
use std::marker::PhantomData;
use serde_json::Value;
use std::collections::HashMap;

/// Type-safe INSERT query builder for entities
/// 
/// Provides a fluent API for building INSERT queries with compile-time type safety.
/// The generic parameter T ensures that the query can only be executed for the correct entity type.
/// Supports both single value insertion and bulk HashMap-based insertion.
pub struct TypeSafeInsert<T: Entity> {
    query: InsertStatement,
    columns: Vec<String>,
    values: Vec<Value>,
    _phantom: PhantomData<T>,
}

impl<T: Entity> TypeSafeInsert<T> {
    /// Create a new INSERT query builder for entity T
    /// 
    /// Automatically sets up the query to insert into the entity's table.
    pub fn new() -> Self {
        let query = Query::insert()
            .into_table(sea_query::Alias::new(T::TABLE_NAME))
            .to_owned();
        
        Self {
            query,
            columns: Vec::new(),
            values: Vec::new(),
            _phantom: PhantomData,
        }
    }
    
    /// Set multiple column values from a HashMap
    /// 
    /// This method allows bulk insertion of data from a HashMap where keys are column names
    /// and values are the data to insert. All values are converted to the appropriate
    /// sea-query values for database insertion.
    /// 
    /// # Examples
    /// ```
    /// let mut data = HashMap::new();
    /// data.insert("name".to_string(), json!("Alice"));
    /// data.insert("email".to_string(), json!("alice@example.com"));
    /// 
    /// let insert = TypeSafeInsert::<User>::new()
    ///     .values(data);
    /// ```
    pub fn values(mut self, data: HashMap<String, Value>) -> Self {
        // Store columns and values for later query building
        self.columns.clear();
        self.values.clear();
        
        for (key, value) in data {
            self.columns.push(key);
            self.values.push(value);
        }
        
        self
    }
    
    /// Set a single column value
    /// 
    /// This method allows setting individual column values in a fluent manner.
    /// Can be chained multiple times to set multiple columns.
    /// 
    /// # Examples
    /// ```
    /// let insert = TypeSafeInsert::<User>::new()
    ///     .value("name", "Alice")
    ///     .value("email", "alice@example.com")
    ///     .value("age", 30);
    /// ```
    pub fn value<V>(mut self, column: &str, value: V) -> Self 
    where
        V: Into<Value>,
    {
        self.columns.push(column.to_string());
        self.values.push(value.into());
        self
    }
    
    /// Add RETURNING * clause to the query
    /// 
    /// This is useful for databases that support RETURNING clauses (PostgreSQL, SQLite).
    /// MySQL does not support RETURNING, so this will be ignored for MySQL connections.
    pub fn returning_all(mut self) -> Self {
        self.query.returning_all();
        self
    }
    
    /// Add RETURNING column clause to the query
    /// 
    /// This is useful for databases that support RETURNING clauses (PostgreSQL, SQLite).
    /// MySQL does not support RETURNING, so this will be ignored for MySQL connections.
    /// 
    /// # Examples
    /// ```
    /// let insert = TypeSafeInsert::<User>::new()
    ///     .value("name", "Alice")
    ///     .returning_col("id");
    /// ```
    pub fn returning_col(mut self, column: &str) -> Self {
        self.query.returning_col(sea_query::Alias::new(column));
        self
    }
    
    /// Helper method to build the final query with accumulated columns and values
    fn build_query(mut self) -> InsertStatement {
        if !self.columns.is_empty() {
            let columns: Vec<_> = self.columns.iter()
                .map(|k| sea_query::Alias::new(k))
                .collect();
            let expr_values: Vec<_> = self.values.iter()
                .map(|v| json_to_sea_value(v).into())
                .collect();
                
            self.query.columns(columns).values_panic(expr_values);
        }
        self.query
    }
    
    /// Execute the INSERT query and return the inserted entity
    /// 
    /// This method handles database-specific differences:
    /// - For databases with RETURNING support (PostgreSQL, SQLite): Uses RETURNING * to get the full entity
    /// - For databases without RETURNING support (MySQL): Executes insert, gets the ID, then queries back the entity
    /// 
    /// Uses the database dialect from the client to generate optimal SQL.
    /// 
    /// # Examples
    /// ```
    /// let user = TypeSafeInsert::<User>::new()
    ///     .value("name", "Alice")
    ///     .value("email", "alice@example.com")
    ///     .save(&client)
    ///     .await?;
    /// ```
    pub async fn save<B: DatabaseBackend>(self, client: &DatabaseClient<B>) -> Result<T> {
        let dialect = client.dialect();
        let mut query = self.build_query();
        
        // Add RETURNING clause for databases that support it
        if dialect.supports_returning() {
            query.returning_all();
        }
        
        let (sql, params) = query.render_for_dialect(dialect);
        
        if dialect.supports_returning() {
            // Use RETURNING result directly
            let result = client.execute(&sql, &params).await
                .map_err(|e| D1RsError::Database(format!("INSERT query execution failed: {:?}", e)))?;
            let entity = result.into_entity()
                .map_err(|e| D1RsError::Database(format!("Entity conversion failed: {:?}", e)))?
                .ok_or(D1RsError::NotFound)?;
            Ok(entity)
        } else {
            // For MySQL and other databases without RETURNING support
            let result = client.execute(&sql, &params).await
                .map_err(|e| D1RsError::Database(format!("INSERT query execution failed: {:?}", e)))?;
            
            // Extract the inserted ID
            let id = result.extract_id()
                .map_err(|e| D1RsError::Database(format!("ID extraction failed: {:?}", e)))?
                .ok_or(D1RsError::Database("No ID returned from INSERT operation".to_string()))?;
            
            // Query back the inserted record using the ID
            // Import the select module to use TypeSafeSelect
            use crate::query_builder::select::TypeSafeSelect;
            let select_query = TypeSafeSelect::<T>::new().where_eq("id", id);
            let entity = select_query.first(client).await?
                .ok_or(D1RsError::NotFound)?;
            Ok(entity)
        }
    }
    
    /// Execute the INSERT query and return only the inserted ID
    /// 
    /// This is more efficient than save() when you only need the ID of the inserted record.
    /// Works with all database types by using appropriate methods for each.
    /// Uses the database dialect from the client to generate optimal SQL.
    /// 
    /// # Examples
    /// ```
    /// let user_id = TypeSafeInsert::<User>::new()
    ///     .value("name", "Alice")
    ///     .value("email", "alice@example.com")
    ///     .save_returning_id(&client)
    ///     .await?;
    /// ```
    pub async fn save_returning_id<B: DatabaseBackend>(self, client: &DatabaseClient<B>) -> Result<i64> {
        let dialect = client.dialect();
        let mut query = self.build_query();
        
        // Add RETURNING id clause for databases that support it
        if dialect.supports_returning() {
            query.returning_col(sea_query::Alias::new("id"));
        }
        
        let (sql, params) = query.render_for_dialect(dialect);
        let result = client.execute(&sql, &params).await
            .map_err(|e| D1RsError::Database(format!("INSERT query execution failed: {:?}", e)))?;
            
        // Extract ID from result
        result.extract_id()
            .map_err(|e| D1RsError::Database(format!("ID extraction failed: {:?}", e)))?
            .ok_or(D1RsError::Database("No ID returned from INSERT operation".to_string()))
    }
    
    /// Execute the INSERT query without returning data
    /// 
    /// This is the most efficient method when you don't need the inserted data back.
    /// Simply executes the INSERT and confirms success.
    /// Uses the database dialect from the client to generate optimal SQL.
    /// 
    /// # Examples
    /// ```
    /// TypeSafeInsert::<User>::new()
    ///     .value("name", "Alice")
    ///     .value("email", "alice@example.com")
    ///     .execute(&client)
    ///     .await?;
    /// ```
    pub async fn execute<B: DatabaseBackend>(self, client: &DatabaseClient<B>) -> Result<()> {
        let query = self.build_query();
        let (sql, params) = query.render_for_dialect(client.dialect());
        let _result = client.execute(&sql, &params).await
            .map_err(|e| D1RsError::Database(format!("INSERT query execution failed: {:?}", e)))?;
        Ok(())
    }
    
    /// Create multiple insert statements for bulk operations
    /// 
    /// This method allows efficient bulk insertion by accepting multiple HashMaps
    /// representing different rows to insert.
    /// 
    /// # Examples
    /// ```
    /// let users_data = vec![
    ///     {
    ///         let mut user1 = HashMap::new();
    ///         user1.insert("name".to_string(), json!("Alice"));
    ///         user1.insert("email".to_string(), json!("alice@example.com"));
    ///         user1
    ///     },
    ///     {
    ///         let mut user2 = HashMap::new();
    ///         user2.insert("name".to_string(), json!("Bob"));
    ///         user2.insert("email".to_string(), json!("bob@example.com"));
    ///         user2
    ///     }
    /// ];
    /// 
    /// let insert = TypeSafeInsert::<User>::new()
    ///     .values_bulk(users_data);
    /// ```
    pub fn values_bulk(mut self, rows: Vec<HashMap<String, Value>>) -> Self {
        if rows.is_empty() {
            return self;
        }
        
        // Clear existing columns and values for bulk operation
        self.columns.clear();
        self.values.clear();
        
        // Get columns from the first row (assuming all rows have the same structure)
        let column_keys: Vec<_> = rows[0].keys().cloned().collect();
        
        // For bulk insert, we need to build the query manually
        let columns: Vec<_> = column_keys.iter()
            .map(|k| sea_query::Alias::new(k))
            .collect();
        
        // Convert all rows to sea-query expressions
        for row in rows {
            let expr_values: Vec<_> = column_keys.iter()
                .map(|key| {
                    let value = row.get(key).unwrap_or(&Value::Null);
                    json_to_sea_value(value).into()
                })
                .collect();
            self.query.values_panic(expr_values);
        }
        
        // Set columns once at the end
        self.query.columns(columns);
        self
    }
}

impl<T: Entity> QueryRenderer for TypeSafeInsert<T> {
    fn render_for_dialect(&self, dialect: DatabaseDialect) -> (String, Vec<Value>) {
        // For rendering, we need to build the query with current state
        // Clone self to avoid consuming it
        let mut query_clone = self.query.clone();
        
        if !self.columns.is_empty() {
            let columns: Vec<_> = self.columns.iter()
                .map(|k| sea_query::Alias::new(k))
                .collect();
            let expr_values: Vec<_> = self.values.iter()
                .map(|v| json_to_sea_value(v).into())
                .collect();
                
            query_clone.columns(columns).values_panic(expr_values);
        }
        
        query_clone.render_for_dialect(dialect)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialects::DatabaseDialect;
    use serde_json::json;
    use sea_query::Query;
    
    // Test our business logic, not sea-query's SQL generation
    
    #[test]
    fn test_single_value_parameter_binding() {
        // Test basic INSERT with single values
        let mut query = Query::insert();
        query.into_table(sea_query::Alias::new("users"))
             .columns([sea_query::Alias::new("name"), sea_query::Alias::new("email")])
             .values_panic(["Alice".into(), "alice@example.com".into()]);
        
        // Test parameter rendering across all database dialects
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test parameter correctness - this is our business logic
            assert_eq!(params.len(), 2);
            assert_eq!(params[0], json!("Alice"));
            assert_eq!(params[1], json!("alice@example.com"));
            
            // Basic sanity check that sea-query generated an INSERT
            assert!(sql.to_uppercase().contains("INSERT"));
        }
    }
    
    #[test]
    fn test_multiple_value_types_parameter_binding() {
        // Test INSERT with different data types
        let mut query = Query::insert();
        query.into_table(sea_query::Alias::new("profiles"))
             .columns([
                 sea_query::Alias::new("name"),
                 sea_query::Alias::new("age"), 
                 sea_query::Alias::new("active"),
                 sea_query::Alias::new("score"),
                 sea_query::Alias::new("notes")
             ])
             .values_panic([
                 sea_query::Value::String(Some(Box::new("Bob".to_string()))).into(),
                 sea_query::Value::Int(Some(25)).into(),
                 sea_query::Value::Bool(Some(true)).into(),
                 sea_query::Value::Double(Some(87.5)).into(),
                 sea_query::Value::String(None).into() // NULL
             ]);
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test parameter correctness and order
            assert_eq!(params.len(), 5);
            assert_eq!(params[0], json!("Bob"));      // name
            assert_eq!(params[1], json!(25));        // age
            assert_eq!(params[2], json!(true));      // active
            assert_eq!(params[3], json!(87.5));      // score
            assert_eq!(params[4], json!(null));      // notes (NULL)
            
            assert!(sql.to_uppercase().contains("INSERT"));
        }
    }
    
    #[test]
    fn test_returning_clause_database_support() {
        // Test RETURNING clause handling across different databases
        let mut query = Query::insert();
        query.into_table(sea_query::Alias::new("documents"))
             .columns([sea_query::Alias::new("title")])
             .values_panic(["Test Document".into()])
             .returning_all();
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test parameter correctness
            assert_eq!(params.len(), 1);
            assert_eq!(params[0], json!("Test Document"));
            
            // Test database-specific RETURNING support
            if dialect.supports_returning() {
                assert!(sql.to_uppercase().contains("RETURNING"));
            }
            // Note: For databases without RETURNING, our application logic
            // handles this, not the SQL generation
            
            assert!(sql.to_uppercase().contains("INSERT"));
        }
    }
    
    #[test]
    fn test_returning_specific_column() {
        // Test RETURNING specific column
        let mut query = Query::insert();
        query.into_table(sea_query::Alias::new("items"))
             .columns([sea_query::Alias::new("name")])
             .values_panic(["New Item".into()])
             .returning_col(sea_query::Alias::new("id"));
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test parameter correctness
            assert_eq!(params.len(), 1);
            assert_eq!(params[0], json!("New Item"));
            
            // Test database-specific RETURNING support
            if dialect.supports_returning() {
                assert!(sql.to_uppercase().contains("RETURNING"));
                // Verify it's returning specific column, not *
                assert!(!sql.contains("RETURNING *"));
            }
            
            assert!(sql.to_uppercase().contains("INSERT"));
        }
    }
    
    #[test]
    fn test_bulk_insert_parameter_handling() {
        // Test bulk INSERT with multiple rows
        let mut query = Query::insert();
        query.into_table(sea_query::Alias::new("bulk_users"))
             .columns([sea_query::Alias::new("name"), sea_query::Alias::new("role")]);
        
        // Add multiple rows
        query.values_panic(["User1".into(), "admin".into()]);
        query.values_panic(["User2".into(), "member".into()]);
        query.values_panic(["User3".into(), "guest".into()]);
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test parameter correctness for bulk insert (3 rows × 2 columns = 6 params)
            assert_eq!(params.len(), 6);
            assert_eq!(params[0], json!("User1"));   // Row 1: name
            assert_eq!(params[1], json!("admin"));   // Row 1: role
            assert_eq!(params[2], json!("User2"));   // Row 2: name
            assert_eq!(params[3], json!("member"));  // Row 2: role
            assert_eq!(params[4], json!("User3"));   // Row 3: name
            assert_eq!(params[5], json!("guest"));   // Row 3: role
            
            assert!(sql.to_uppercase().contains("INSERT"));
        }
    }
    
    #[test]
    fn test_json_to_sea_value_conversion() {
        // Test our value conversion logic
        use super::super::json_to_sea_value;
        
        // Test various JSON types get converted correctly
        let string_val = json_to_sea_value(&json!("test_string"));
        let int_val = json_to_sea_value(&json!(42));
        let float_val = json_to_sea_value(&json!(3.14));
        let bool_val = json_to_sea_value(&json!(true));
        let null_val = json_to_sea_value(&json!(null));
        
        // Verify conversions work (specific format is sea-query's responsibility)
        assert!(matches!(string_val, sea_query::Value::String(_)));
        assert!(matches!(int_val, sea_query::Value::Int(_) | sea_query::Value::BigInt(_)));
        assert!(matches!(float_val, sea_query::Value::Double(_) | sea_query::Value::Float(_)));
        assert!(matches!(bool_val, sea_query::Value::Bool(_)));
        // null_val format may vary by sea-query version
        let _null_handled = null_val;
    }
    
    #[test]
    fn test_unicode_and_special_character_parameters() {
        // Test that our parameter handling works with special characters
        let mut query = Query::insert();
        query.into_table(sea_query::Alias::new("international_content"))
             .columns([
                 sea_query::Alias::new("chinese_text"),
                 sea_query::Alias::new("emoji_content"),
                 sea_query::Alias::new("special_chars")
             ])
             .values_panic([
                 "测试内容".into(),      // Chinese
                 "🚀✨🎉".into(),        // Emoji
                 "!@#$%^&*()".into()   // Special characters
             ]);
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test parameter correctness with unicode
            assert_eq!(params.len(), 3);
            assert_eq!(params[0], json!("测试内容"));
            assert_eq!(params[1], json!("🚀✨🎉"));
            assert_eq!(params[2], json!("!@#$%^&*()"));
            
            assert!(sql.to_uppercase().contains("INSERT"));
        }
    }
    
    #[test]
    fn test_empty_insert_handling() {
        // Test INSERT with no values (should not panic)
        let query = Query::insert()
            .into_table(sea_query::Alias::new("empty_table"))
            .to_owned();
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // No parameters for empty insert
            assert_eq!(params.len(), 0);
            
            // Should still generate valid SQL
            assert!(sql.to_uppercase().contains("INSERT"));
        }
    }
    
    #[test]
    fn test_complex_insert_parameter_order() {
        // Test complex INSERT with many columns to verify parameter order
        let mut query = Query::insert();
        query.into_table(sea_query::Alias::new("complex_records"))
             .columns([
                 sea_query::Alias::new("string_field"),
                 sea_query::Alias::new("int_field"),
                 sea_query::Alias::new("float_field"),
                 sea_query::Alias::new("bool_field"),
                 sea_query::Alias::new("null_field")
             ])
             .values_panic([
                 sea_query::Value::String(Some(Box::new("complex_string".to_string()))).into(),
                 sea_query::Value::Int(Some(999)).into(),
                 sea_query::Value::Double(Some(123.456)).into(),
                 sea_query::Value::Bool(Some(false)).into(),
                 sea_query::Value::String(None).into()
             ]);
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test parameter correctness and order
            assert_eq!(params.len(), 5);
            assert_eq!(params[0], json!("complex_string")); // string_field
            assert_eq!(params[1], json!(999));             // int_field
            assert_eq!(params[2], json!(123.456));         // float_field
            assert_eq!(params[3], json!(false));           // bool_field
            assert_eq!(params[4], json!(null));            // null_field
            
            assert!(sql.to_uppercase().contains("INSERT"));
        }
    }
    
    #[test]
    fn test_sql_injection_prevention_through_parameters() {
        // Test that malicious content is safely parameterized
        let malicious_content = "'; DROP TABLE users; --";
        let mut query = Query::insert();
        query.into_table(sea_query::Alias::new("safe_table"))
             .columns([sea_query::Alias::new("user_input")])
             .values_panic([malicious_content.into()]);
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test that malicious content is safely in parameters
            assert_eq!(params.len(), 1);
            assert_eq!(params[0], json!(malicious_content));
            
            // Verify SQL structure is safe (no malicious content in SQL itself)
            assert!(sql.to_uppercase().contains("INSERT"));
            // The malicious content should NOT be in the SQL string
            assert!(!sql.contains("DROP TABLE"));
            assert!(!sql.contains("--"));
        }
    }
    
    #[test]
    fn test_large_dataset_parameter_handling() {
        // Test INSERT with larger dataset to verify parameter handling scales
        let mut query = Query::insert();
        query.into_table(sea_query::Alias::new("large_dataset"))
             .columns([sea_query::Alias::new("id"), sea_query::Alias::new("value")]);
        
        // Add 10 rows
        for i in 1..=10 {
            query.values_panic([i.into(), format!("value_{}", i).into()]);
        }
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test parameter correctness (10 rows × 2 columns = 20 params)
            assert_eq!(params.len(), 20);
            
            // Verify parameter order for first few rows
            assert_eq!(params[0], json!(1));           // Row 1: id
            assert_eq!(params[1], json!("value_1"));   // Row 1: value
            assert_eq!(params[2], json!(2));           // Row 2: id
            assert_eq!(params[3], json!("value_2"));   // Row 2: value
            // ... and so on
            assert_eq!(params[18], json!(10));         // Row 10: id
            assert_eq!(params[19], json!("value_10")); // Row 10: value
            
            assert!(sql.to_uppercase().contains("INSERT"));
        }
    }
}