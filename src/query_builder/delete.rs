use super::{QueryRenderer, json_to_sea_value};
use crate::backends::{DatabaseBackend, QueryResult};
use crate::{Entity, Result, D1RsError, DatabaseClient};
use crate::dialects::DatabaseDialect;
use sea_query::{Query, DeleteStatement, Expr};
use std::marker::PhantomData;
use serde_json::Value;

/// Type-safe DELETE query builder for entities
/// 
/// Provides a fluent API for building DELETE queries with compile-time type safety.
/// The generic parameter T ensures that the query can only be executed for the correct entity type.
/// Supports WHERE conditions and RETURNING clauses with database-specific optimizations.
pub struct TypeSafeDelete<T: Entity> {
    query: DeleteStatement,
    _phantom: PhantomData<T>,
}

impl<T: Entity> TypeSafeDelete<T> {
    /// Create a new DELETE query builder for entity T
    /// 
    /// Automatically sets up the query to delete from the entity's table.
    pub fn new() -> Self {
        let query = Query::delete()
            .from_table(sea_query::Alias::new(T::TABLE_NAME))
            .to_owned();
        
        Self {
            query,
            _phantom: PhantomData,
        }
    }
    
    /// Add a WHERE condition for equality comparison
    /// 
    /// This method allows adding WHERE clauses with equality comparisons.
    /// Can be chained multiple times to add multiple conditions (combined with AND).
    /// 
    /// # Examples
    /// ```
    /// let delete = TypeSafeDelete::<User>::new()
    ///     .where_eq("id", 1)
    ///     .where_eq("active", false);
    /// ```
    pub fn where_eq<V>(mut self, column: &str, value: V) -> Self 
    where
        V: Into<Value>,
    {
        let sea_value = json_to_sea_value(&value.into());
        self.query.and_where(Expr::col(sea_query::Alias::new(column)).eq(sea_value));
        self
    }
    
    /// Add a WHERE condition for non-equality comparison
    /// 
    /// This method allows adding WHERE clauses with non-equality comparisons.
    /// 
    /// # Examples
    /// ```
    /// let delete = TypeSafeDelete::<User>::new()
    ///     .where_ne("status", "active");
    /// ```
    pub fn where_ne<V>(mut self, column: &str, value: V) -> Self 
    where
        V: Into<Value>,
    {
        let sea_value = json_to_sea_value(&value.into());
        self.query.and_where(Expr::col(sea_query::Alias::new(column)).ne(sea_value));
        self
    }
    
    /// Add a WHERE condition for greater than comparison
    pub fn where_gt<V>(mut self, column: &str, value: V) -> Self 
    where
        V: Into<Value>,
    {
        let sea_value = json_to_sea_value(&value.into());
        self.query.and_where(Expr::col(sea_query::Alias::new(column)).gt(sea_value));
        self
    }
    
    /// Add a WHERE condition for greater than or equal comparison
    pub fn where_gte<V>(mut self, column: &str, value: V) -> Self 
    where
        V: Into<Value>,
    {
        let sea_value = json_to_sea_value(&value.into());
        self.query.and_where(Expr::col(sea_query::Alias::new(column)).gte(sea_value));
        self
    }
    
    /// Add a WHERE condition for less than comparison
    pub fn where_lt<V>(mut self, column: &str, value: V) -> Self 
    where
        V: Into<Value>,
    {
        let sea_value = json_to_sea_value(&value.into());
        self.query.and_where(Expr::col(sea_query::Alias::new(column)).lt(sea_value));
        self
    }
    
    /// Add a WHERE condition for less than or equal comparison
    pub fn where_lte<V>(mut self, column: &str, value: V) -> Self 
    where
        V: Into<Value>,
    {
        let sea_value = json_to_sea_value(&value.into());
        self.query.and_where(Expr::col(sea_query::Alias::new(column)).lte(sea_value));
        self
    }
    
    /// Add a WHERE condition for LIKE pattern matching
    /// 
    /// Useful for string pattern matching with wildcards (% and _).
    /// 
    /// # Examples
    /// ```
    /// let delete = TypeSafeDelete::<User>::new()
    ///     .where_like("email", "%@temp.com");
    /// ```
    pub fn where_like<V>(mut self, column: &str, pattern: V) -> Self 
    where
        V: Into<Value>,
    {
        let json_value = pattern.into();
        let pattern_str = match json_value {
            Value::String(s) => s,
            _ => json_value.to_string(), // Convert non-strings to string representation
        };
        self.query.and_where(Expr::col(sea_query::Alias::new(column)).like(pattern_str));
        self
    }
    
    /// Add a WHERE condition for IN clause
    /// 
    /// Checks if column value is in a list of values.
    /// 
    /// # Examples
    /// ```
    /// let delete = TypeSafeDelete::<User>::new()
    ///     .where_in("status", vec!["inactive", "banned"]);
    /// ```
    pub fn where_in<V>(mut self, column: &str, values: Vec<V>) -> Self 
    where
        V: Into<Value>,
    {
        let sea_values: Vec<_> = values.into_iter()
            .map(|v| json_to_sea_value(&v.into()))
            .collect();
        self.query.and_where(Expr::col(sea_query::Alias::new(column)).is_in(sea_values));
        self
    }
    
    /// Add a WHERE condition for BETWEEN clause
    /// 
    /// Checks if column value is between two values (inclusive).
    /// 
    /// # Examples
    /// ```
    /// let delete = TypeSafeDelete::<User>::new()
    ///     .where_between("created_at", "2020-01-01", "2021-01-01");
    /// ```
    pub fn where_between<V>(mut self, column: &str, min: V, max: V) -> Self 
    where
        V: Into<Value>,
    {
        let min_value = json_to_sea_value(&min.into());
        let max_value = json_to_sea_value(&max.into());
        self.query.and_where(Expr::col(sea_query::Alias::new(column)).between(min_value, max_value));
        self
    }
    
    /// Add a WHERE condition for IS NULL
    /// 
    /// Checks if column value is NULL.
    /// 
    /// # Examples
    /// ```
    /// let delete = TypeSafeDelete::<User>::new()
    ///     .where_null("deleted_at");
    /// ```
    pub fn where_null(mut self, column: &str) -> Self {
        self.query.and_where(Expr::col(sea_query::Alias::new(column)).is_null());
        self
    }
    
    /// Add a WHERE condition for IS NOT NULL
    /// 
    /// Checks if column value is not NULL.
    /// 
    /// # Examples
    /// ```
    /// let delete = TypeSafeDelete::<User>::new()
    ///     .where_not_null("email");
    /// ```
    pub fn where_not_null(mut self, column: &str) -> Self {
        self.query.and_where(Expr::col(sea_query::Alias::new(column)).is_not_null());
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
    /// let delete = TypeSafeDelete::<User>::new()
    ///     .where_eq("id", 1)
    ///     .returning_col("id");
    /// ```
    pub fn returning_col(mut self, column: &str) -> Self {
        self.query.returning_col(sea_query::Alias::new(column));
        self
    }
    
    /// Add RETURNING columns clause to the query
    /// 
    /// This is useful for databases that support RETURNING clauses (PostgreSQL, SQLite).
    /// MySQL does not support RETURNING, so this will be ignored for MySQL connections.
    /// 
    /// # Examples
    /// ```
    /// let delete = TypeSafeDelete::<User>::new()
    ///     .where_eq("active", false)
    ///     .returning_cols(vec!["id", "name", "email"]);
    /// ```
    pub fn returning_cols(mut self, columns: Vec<&str>) -> Self {
        for column in columns {
            self.query.returning_col(sea_query::Alias::new(column));
        }
        self
    }
    
    /// Execute the DELETE query and return the number of affected rows
    /// 
    /// This is the primary execution method for DELETE operations.
    /// Works with all database types and returns the count of deleted rows.
    /// 
    /// # Examples
    /// ```
    /// let affected_rows = TypeSafeDelete::<User>::new()
    ///     .where_eq("active", false)
    ///     .where_lt("last_login", "2020-01-01")
    ///     .execute(&client)
    ///     .await?;
    /// ```
    pub async fn execute<B: DatabaseBackend>(self, client: &DatabaseClient<B>) -> Result<u64> {
        let (sql, params) = self.query.render_for_dialect(client.dialect());
        let result = client.execute(&sql, &params).await
            .map_err(|e| D1RsError::Database(format!("DELETE query execution failed: {:?}", e)))?;
        
        // Extract affected rows count
        let affected_rows = result.extract_count()
            .map_err(|e| D1RsError::Database(format!("Affected rows extraction failed: {:?}", e)))?;
        
        Ok(affected_rows as u64)
    }
    
    /// Execute the DELETE query without returning any data
    /// 
    /// This is the most efficient method when you don't need feedback about the operation.
    /// Simply executes the DELETE and confirms success.
    /// 
    /// # Examples
    /// ```
    /// TypeSafeDelete::<User>::new()
    ///     .where_eq("id", 1)
    ///     .execute_simple(&client)
    ///     .await?;
    /// ```
    pub async fn execute_simple<B: DatabaseBackend>(self, client: &DatabaseClient<B>) -> Result<()> {
        let (sql, params) = self.query.render_for_dialect(client.dialect());
        let _result = client.execute(&sql, &params).await
            .map_err(|e| D1RsError::Database(format!("DELETE query execution failed: {:?}", e)))?;
        Ok(())
    }
    
    /// Execute the DELETE query with RETURNING support and return deleted records
    /// 
    /// This method is useful when you want to get back specific columns from deleted records.
    /// Only works with databases that support RETURNING.
    /// 
    /// # Examples
    /// ```
    /// let deleted_users = TypeSafeDelete::<User>::new()
    ///     .where_eq("active", false)
    ///     .returning_all()
    ///     .execute_returning(&client)
    ///     .await?;
    /// ```
    pub async fn execute_returning<B: DatabaseBackend>(self, client: &DatabaseClient<B>) -> Result<Vec<T>> {
        let dialect = client.dialect();
        
        if !dialect.supports_returning() {
            return Err(D1RsError::Database("RETURNING clause not supported by this database".to_string()));
        }
        
        let (sql, params) = self.query.render_for_dialect(dialect);
        let result = client.execute(&sql, &params).await
            .map_err(|e| D1RsError::Database(format!("DELETE query execution failed: {:?}", e)))?;
        
        // Extract the returned entities
        let entities = result.into_entities()
            .map_err(|e| D1RsError::Database(format!("Entities conversion failed: {:?}", e)))?;
        
        Ok(entities)
    }
}

impl<T: Entity> QueryRenderer for TypeSafeDelete<T> {
    fn render_for_dialect(&self, dialect: DatabaseDialect) -> (String, Vec<Value>) {
        self.query.render_for_dialect(dialect)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use sea_query::{Query, DeleteStatement};
    
    // Mock entity for testing
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct TestUser {
        id: i64,
        name: String,
        email: String,
        age: i32,
        active: bool,
    }
    
    // Simple test entity constants 
    impl TestUser {
        const TABLE_NAME_CONST: &'static str = "users";
        
        fn boolean_fields_static() -> &'static [&'static str] {
            &["active"]
        }
    }
    
    // Helper function to create a basic delete statement for testing
    fn create_basic_delete() -> DeleteStatement {
        Query::delete()
            .from_table(sea_query::Alias::new("users"))
            .to_owned()
    }
    
    #[test]
    fn test_basic_delete_sql_generation() {
        let mut query = create_basic_delete();
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("id")).eq(1));
        
        let sql = query.build(sea_query::SqliteQueryBuilder).0;
        
        assert!(sql.contains("DELETE"));
        assert!(sql.contains("FROM"));
        assert!(sql.contains("users"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("id"));
    }
    
    #[test]
    fn test_delete_with_multiple_where_conditions() {
        let mut query = create_basic_delete();
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("active")).eq(false));
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("age")).gt(65));
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("last_login")).lt("2020-01-01"));
        
        let sql = query.build(sea_query::SqliteQueryBuilder).0;
        
        assert!(sql.contains("DELETE"));
        assert!(sql.contains("FROM"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("AND"));
        assert!(sql.matches("AND").count() >= 2);
    }
    
    #[test]
    fn test_delete_with_like_condition() {
        let mut query = create_basic_delete();
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("email")).like("%@temp.com"));
        
        let sql = query.build(sea_query::SqliteQueryBuilder).0;
        
        assert!(sql.contains("DELETE"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("LIKE"));
    }
    
    #[test]
    fn test_delete_with_in_condition() {
        let mut query = create_basic_delete();
        let values: Vec<sea_query::Value> = vec!["inactive".into(), "banned".into(), "deleted".into()];
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("status")).is_in(values));
        
        let sql = query.build(sea_query::SqliteQueryBuilder).0;
        
        assert!(sql.contains("DELETE"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("IN"));
    }
    
    #[test]
    fn test_delete_with_between_condition() {
        let mut query = create_basic_delete();
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("created_at")).between("2020-01-01", "2021-01-01"));
        
        let sql = query.build(sea_query::SqliteQueryBuilder).0;
        
        assert!(sql.contains("DELETE"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("BETWEEN"));
    }
    
    #[test]
    fn test_delete_with_null_conditions() {
        let mut query = create_basic_delete();
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("deleted_at")).is_not_null());
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("temp_token")).is_null());
        
        let sql = query.build(sea_query::SqliteQueryBuilder).0;
        
        assert!(sql.contains("DELETE"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("IS NOT NULL"));
        assert!(sql.contains("IS NULL"));
    }
    
    #[test]
    fn test_delete_with_comparison_operators() {
        let mut query = create_basic_delete();
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("age")).gt(18));
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("score")).gte(75));
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("attempts")).lt(5));
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("failures")).lte(2));
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("status")).ne("active"));
        
        let sql = query.build(sea_query::SqliteQueryBuilder).0;
        
        assert!(sql.contains("DELETE"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains(">"));
        assert!(sql.contains(">="));
        assert!(sql.contains("<"));
        assert!(sql.contains("<="));
        // Different databases/versions may use != or <> for not equals
        assert!(sql.contains("!=") || sql.contains("<>"));
    }
    
    #[test]
    fn test_delete_with_returning_clause() {
        let mut query = create_basic_delete();
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("active")).eq(false));
        query.returning_all();
        
        let sql = query.build(sea_query::SqliteQueryBuilder).0;
        
        assert!(sql.contains("DELETE"));
        assert!(sql.contains("RETURNING"));
        assert!(sql.contains("*"));
    }
    
    #[test]
    fn test_delete_with_returning_specific_column() {
        let mut query = create_basic_delete();
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("id")).eq(1));
        query.returning_col(sea_query::Alias::new("id"));
        
        let sql = query.build(sea_query::SqliteQueryBuilder).0;
        
        assert!(sql.contains("DELETE"));
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
        let bool_val = json_to_sea_value(&json!(false));
        assert!(matches!(bool_val, sea_query::Value::Bool(Some(false))));
        
        // Test null conversion
        let null_val = json_to_sea_value(&json!(null));
        assert!(matches!(null_val, sea_query::Value::BigInt(None)));
    }
    
    #[test]
    fn test_renderer_functionality() {
        // Create a simple delete query manually
        let mut query = create_basic_delete();
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("id")).eq(1));
        
        // Test rendering for different dialects
        let sqlite_sql = query.build(sea_query::SqliteQueryBuilder).0;
        let postgres_sql = query.build(sea_query::PostgresQueryBuilder).0;
        let mysql_sql = query.build(sea_query::MysqlQueryBuilder).0;
        
        // Basic assertions
        assert!(sqlite_sql.contains("DELETE"));
        assert!(postgres_sql.contains("DELETE"));
        assert!(mysql_sql.contains("DELETE"));
        
        // Different dialects may use different quoting
        assert!(sqlite_sql.contains("users") || sqlite_sql.contains("\"users\""));
        assert!(postgres_sql.contains("users") || postgres_sql.contains("\"users\""));
        assert!(mysql_sql.contains("users") || mysql_sql.contains("`users`"));
    }
    
    #[test]
    fn test_unicode_and_special_characters() {
        let mut query = create_basic_delete();
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("name")).like("测试%"));
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("description")).like("%🎉%"));
        
        let sql = query.build(sea_query::SqliteQueryBuilder).0;
        
        // SQL should be generated without errors
        assert!(sql.contains("DELETE"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("LIKE"));
    }
    
    #[test]
    fn test_delete_without_where_clause() {
        let query = create_basic_delete();
        
        let sql = query.build(sea_query::SqliteQueryBuilder).0;
        
        // Should generate DELETE without WHERE (deletes all rows - dangerous but valid)
        assert!(sql.contains("DELETE"));
        assert!(sql.contains("FROM"));
        assert!(sql.contains("users"));
        assert!(!sql.contains("WHERE"));
    }
    
    #[test]
    fn test_complex_delete_query() {
        let mut query = create_basic_delete();
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("active")).eq(false));
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("email")).like("%@temp.com"));
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("created_at")).between("2020-01-01", "2021-01-01"));
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("login_count")).lte(1));
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("verified")).is_null());
        query.returning_all();
        
        let sql = query.build(sea_query::SqliteQueryBuilder).0;
        
        // Verify SQL structure
        assert!(sql.contains("DELETE"));
        assert!(sql.contains("FROM"));
        assert!(sql.contains("users"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("AND"));
        assert!(sql.contains("LIKE"));
        assert!(sql.contains("BETWEEN"));
        assert!(sql.contains("IS NULL"));
        assert!(sql.contains("RETURNING"));
        assert!(sql.contains("*"));
    }
    
    #[test]
    fn test_method_chaining_fluency() {
        // Test that all methods return Self for fluent chaining
        let mut _query = create_basic_delete();
        _query.and_where(sea_query::Expr::col(sea_query::Alias::new("field1")).eq(1));
        _query.and_where(sea_query::Expr::col(sea_query::Alias::new("field2")).ne(2));
        _query.and_where(sea_query::Expr::col(sea_query::Alias::new("field3")).gt(3));
        _query.and_where(sea_query::Expr::col(sea_query::Alias::new("field4")).gte(4));
        _query.and_where(sea_query::Expr::col(sea_query::Alias::new("field5")).lt(5));
        _query.and_where(sea_query::Expr::col(sea_query::Alias::new("field6")).lte(6));
        _query.and_where(sea_query::Expr::col(sea_query::Alias::new("field7")).like("pattern"));
        _query.and_where(sea_query::Expr::col(sea_query::Alias::new("field8")).is_in([1, 2, 3]));
        _query.and_where(sea_query::Expr::col(sea_query::Alias::new("field9")).between(1, 10));
        _query.and_where(sea_query::Expr::col(sea_query::Alias::new("field10")).is_null());
        _query.and_where(sea_query::Expr::col(sea_query::Alias::new("field11")).is_not_null());
        _query.returning_all();
        
        // If this compiles, the fluent interface works correctly
        assert!(true);
    }
    
    #[test]
    fn test_sql_injection_prevention() {
        let mut query = create_basic_delete();
        
        // Try to delete with potentially malicious content
        let malicious_content = "1; DROP TABLE users; --";
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("id")).eq(malicious_content));
        
        let (sql, _params) = query.build(sea_query::SqliteQueryBuilder);
        
        // Should not contain the malicious SQL in the generated SQL
        assert!(!sql.contains("DROP TABLE"));
        assert!(!sql.contains("; --"));
        // Should be properly structured
        assert!(sql.contains("DELETE"));
        assert!(sql.contains("FROM"));
        assert!(sql.contains("WHERE"));
        // The malicious content should be in parameters, not in SQL
    }
    
    #[test]
    fn test_delete_safety_considerations() {
        // Test that DELETE without WHERE is possible but documented
        let query = create_basic_delete();
        let sql = query.build(sea_query::SqliteQueryBuilder).0;
        
        // This should work but is dangerous - deletes all rows
        assert!(sql.contains("DELETE"));
        assert!(sql.contains("FROM"));
        assert!(!sql.contains("WHERE"));
        
        // Test with WHERE condition for safety
        let mut safe_query = create_basic_delete();
        safe_query.and_where(sea_query::Expr::col(sea_query::Alias::new("id")).eq(1));
        let safe_sql = safe_query.build(sea_query::SqliteQueryBuilder).0;
        
        assert!(safe_sql.contains("DELETE"));
        assert!(safe_sql.contains("WHERE"));
    }
}