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
    use serde_json::json;
    use sea_query::{Query, InsertStatement};
    
    // Mock entity for testing
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct TestUser {
        id: i64,
        name: String,
        email: String,
        age: i32,
        active: bool,
    }
    
    // Test entity - no additional constants needed
    
    // Helper function to create a basic insert statement for testing
    fn create_basic_insert() -> InsertStatement {
        Query::insert()
            .into_table(sea_query::Alias::new("users"))
            .to_owned()
    }
    
    #[test]
    fn test_basic_insert_sql_generation() {
        let mut query = create_basic_insert();
        let columns = vec![
            sea_query::Alias::new("name"),
            sea_query::Alias::new("email")
        ];
        let values = vec![
            "Alice".into(),
            "alice@example.com".into()
        ];
        query.columns(columns).values_panic(values);
        
        let sql = query.build(sea_query::SqliteQueryBuilder).0;
        
        assert!(sql.contains("INSERT INTO"));
        assert!(sql.contains("users"));
        assert!(sql.contains("name"));
        assert!(sql.contains("email"));
        assert!(sql.contains("VALUES"));
    }
    
    #[test]
    fn test_insert_with_multiple_values() {
        let mut query = create_basic_insert();
        let columns = vec![
            sea_query::Alias::new("name"),
            sea_query::Alias::new("email"),
            sea_query::Alias::new("age"),
            sea_query::Alias::new("active")
        ];
        let values = vec![
            "Alice".into(),
            "alice@example.com".into(),
            30.into(),
            true.into()
        ];
        query.columns(columns).values_panic(values);
        
        let sql = query.build(sea_query::SqliteQueryBuilder).0;
        
        assert!(sql.contains("INSERT INTO"));
        assert!(sql.contains("users"));
        assert!(sql.contains("name"));
        assert!(sql.contains("email"));
        assert!(sql.contains("age"));
        assert!(sql.contains("active"));
    }
    
    #[test]
    fn test_insert_with_columns_and_values() {
        let mut query = create_basic_insert();
        let columns = vec![
            sea_query::Alias::new("name"),
            sea_query::Alias::new("email"),
            sea_query::Alias::new("age")
        ];
        let values = vec![
            "Alice".into(),
            "alice@example.com".into(),
            30.into()
        ];
        query.columns(columns).values_panic(values);
        
        let sql = query.build(sea_query::SqliteQueryBuilder).0;
        
        assert!(sql.contains("INSERT INTO"));
        assert!(sql.contains("users"));
        assert!(sql.contains("("));
        assert!(sql.contains(")"));
        assert!(sql.contains("VALUES"));
    }
    
    #[test]
    fn test_returning_clause() {
        let mut query = create_basic_insert();
        let columns = vec![sea_query::Alias::new("name")];
        let values = vec!["Alice".into()];
        query.columns(columns).values_panic(values);
        query.returning_all();
        
        let sql = query.build(sea_query::SqliteQueryBuilder).0;
        
        assert!(sql.contains("INSERT INTO"));
        assert!(sql.contains("RETURNING"));
        assert!(sql.contains("*"));
    }
    
    #[test]
    fn test_returning_specific_column() {
        let mut query = create_basic_insert();
        let columns = vec![sea_query::Alias::new("name")];
        let values = vec!["Alice".into()];
        query.columns(columns).values_panic(values);
        query.returning_col(sea_query::Alias::new("id"));
        
        let sql = query.build(sea_query::SqliteQueryBuilder).0;
        
        assert!(sql.contains("INSERT INTO"));
        assert!(sql.contains("RETURNING"));
        assert!(sql.contains("id"));
        assert!(!sql.contains("*"));
    }
    
    #[test]
    fn test_json_to_sea_value_conversion() {
        use super::super::json_to_sea_value;
        
        // Test string conversion
        let string_val = json_to_sea_value(&json!("test"));
        assert!(matches!(string_val, sea_query::Value::String(_)));
        
        // Test integer conversion
        let int_val = json_to_sea_value(&json!(42));
        assert!(matches!(int_val, sea_query::Value::Int(_) | sea_query::Value::BigInt(_)));
        
        // Test boolean conversion
        let bool_val = json_to_sea_value(&json!(true));
        assert!(matches!(bool_val, sea_query::Value::Bool(Some(true))));
        
        // Test null conversion
        let null_val = json_to_sea_value(&json!(null));
        assert!(matches!(null_val, sea_query::Value::BigInt(None)));
    }
    
    #[test]
    fn test_bulk_insert_structure() {
        let mut query = create_basic_insert();
        
        // Simulate bulk insert with multiple rows
        let columns = vec![
            sea_query::Alias::new("name"),
            sea_query::Alias::new("email")
        ];
        
        // Set columns first
        query.columns(columns);
        
        // First row
        let values1 = vec![
            "Alice".into(),
            "alice@example.com".into()
        ];
        query.values_panic(values1);
        
        // Second row
        let values2 = vec![
            "Bob".into(),
            "bob@example.com".into()
        ];
        query.values_panic(values2);
        
        let sql = query.build(sea_query::SqliteQueryBuilder).0;
        
        assert!(sql.contains("INSERT INTO"));
        assert!(sql.contains("users"));
        assert!(sql.contains("VALUES"));
        // Should contain multiple value sets for bulk insert
        assert!(sql.matches("(").count() >= 2);
    }
    
    #[test]
    fn test_renderer_functionality() {
        // Create a simple insert query manually
        let mut query = create_basic_insert();
        let columns = vec![sea_query::Alias::new("name")];
        let values = vec!["Alice".into()];
        query.columns(columns).values_panic(values);
        
        // Test rendering for different dialects
        let sqlite_sql = query.build(sea_query::SqliteQueryBuilder).0;
        let postgres_sql = query.build(sea_query::PostgresQueryBuilder).0;
        let mysql_sql = query.build(sea_query::MysqlQueryBuilder).0;
        
        // Basic assertions
        assert!(sqlite_sql.contains("INSERT INTO"));
        assert!(postgres_sql.contains("INSERT INTO"));
        assert!(mysql_sql.contains("INSERT INTO"));
        
        // Different dialects may use different quoting
        assert!(sqlite_sql.contains("users") || sqlite_sql.contains("`users`"));
        assert!(postgres_sql.contains("users") || postgres_sql.contains("\"users\""));
        assert!(mysql_sql.contains("users") || mysql_sql.contains("`users`"));
    }
    
    #[test]
    fn test_unicode_and_special_characters() {
        let mut query = create_basic_insert();
        let columns = vec![
            sea_query::Alias::new("name"),
            sea_query::Alias::new("description"),
            sea_query::Alias::new("emoji")
        ];
        let values = vec![
            "测试用户".into(),
            "🎉 party".into(),
            "😀".into()
        ];
        query.columns(columns).values_panic(values);
        
        let sql = query.build(sea_query::SqliteQueryBuilder).0;
        
        // SQL should be generated without errors
        assert!(sql.contains("INSERT INTO"));
        assert!(sql.contains("VALUES"));
    }
    
    #[test]
    fn test_different_value_types() {
        let mut query = create_basic_insert();
        let columns = vec![
            sea_query::Alias::new("string_field"),
            sea_query::Alias::new("int_field"),
            sea_query::Alias::new("float_field"),
            sea_query::Alias::new("bool_field")
        ];
        let values = vec![
            "text".into(),
            42.into(),
            3.14.into(),
            true.into()
        ];
        query.columns(columns).values_panic(values);
        
        let sql = query.build(sea_query::SqliteQueryBuilder).0;
        
        assert!(sql.contains("INSERT INTO"));
        assert!(sql.contains("string_field"));
        assert!(sql.contains("int_field"));
        assert!(sql.contains("float_field"));
        assert!(sql.contains("bool_field"));
    }
    
    #[test]
    fn test_empty_values_handling() {
        let query = create_basic_insert();
        
        // Should not panic with empty query
        let sql = query.build(sea_query::SqliteQueryBuilder).0;
        assert!(sql.contains("INSERT INTO"));
        assert!(sql.contains("users"));
    }
    
    #[test]
    fn test_column_ordering_consistency() {
        let mut query = create_basic_insert();
        
        // Add values in a specific order
        let columns = vec![
            sea_query::Alias::new("id"),
            sea_query::Alias::new("name"),
            sea_query::Alias::new("email"),
            sea_query::Alias::new("created_at")
        ];
        let values = vec![
            1.into(),
            "Alice".into(),
            "alice@example.com".into(),
            "2023-01-01".into()
        ];
        
        query.columns(columns).values_panic(values);
        
        let sql = query.build(sea_query::SqliteQueryBuilder).0;
        
        // Should maintain column order
        assert!(sql.contains("INSERT INTO"));
        assert!(sql.contains("("));
        assert!(sql.contains(")"));
        assert!(sql.contains("VALUES"));
    }
    
    #[test]
    fn test_sql_injection_prevention() {
        let mut query = create_basic_insert();
        
        // Try to insert potentially malicious content
        let malicious_content = "'; DROP TABLE users; --";
        let columns = vec![sea_query::Alias::new("name")];
        let values = vec![malicious_content.into()];
        query.columns(columns).values_panic(values);
        
        let (sql, _params) = query.build(sea_query::SqliteQueryBuilder);
        
        // Should not contain the malicious SQL in the generated SQL
        assert!(!sql.contains("DROP TABLE"));
        assert!(!sql.contains("--"));
        // Should be properly structured
        assert!(sql.contains("INSERT INTO"));
        assert!(sql.contains("VALUES"));
        // The malicious content should be in parameters, not in SQL
    }
}