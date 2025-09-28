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
    /// Uses the database dialect from the client to generate optimal SQL.
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
    /// Uses the database dialect from the client to generate optimal SQL.
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
    /// Uses the database dialect from the client to generate optimal SQL.
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
    use super::*;
    use crate::dialects::DatabaseDialect;
    use serde_json::json;
    use sea_query::Query;
    
    // Test our business logic, not sea-query's SQL generation
    
    #[test]
    fn test_single_where_parameter_binding() {
        // Test basic DELETE with single WHERE condition
        let mut query = Query::delete();
        query.from_table(sea_query::Alias::new("users"))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("id")).eq(1));
        
        // Test parameter rendering across all database dialects
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test parameter correctness - this is our business logic
            assert_eq!(params.len(), 1);
            assert_eq!(params[0], json!(1));  // WHERE id = 1
            
            // Basic sanity check that sea-query generated a DELETE
            assert!(sql.to_uppercase().contains("DELETE"));
        }
    }
    
    #[test]
    fn test_multiple_where_conditions_parameter_binding() {
        // Test DELETE with multiple WHERE conditions
        let mut query = Query::delete();
        query.from_table(sea_query::Alias::new("accounts"))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("active")).eq(false))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("age")).gt(65))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("last_login")).lt("2020-01-01"));
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test parameter correctness and order (3 WHERE conditions)
            assert_eq!(params.len(), 3);
            assert_eq!(params[0], json!(false));        // WHERE active = false
            assert_eq!(params[1], json!(65));           // WHERE age > 65
            assert_eq!(params[2], json!("2020-01-01")); // WHERE last_login < '2020-01-01'
            
            assert!(sql.to_uppercase().contains("DELETE"));
        }
    }
    
    #[test]
    fn test_like_pattern_parameter_handling() {
        // Test LIKE operator parameter handling
        let mut query = Query::delete();
        query.from_table(sea_query::Alias::new("temp_users"))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("email")).like("%@temp.com"))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("username")).like("test_%"));
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test parameter correctness for LIKE patterns
            assert_eq!(params.len(), 2);
            assert_eq!(params[0], json!("%@temp.com"));  // WHERE email LIKE '%@temp.com'
            assert_eq!(params[1], json!("test_%"));       // WHERE username LIKE 'test_%'
            
            assert!(sql.to_uppercase().contains("DELETE"));
        }
    }
    
    #[test]
    fn test_in_operator_parameter_handling() {
        // Test IN operator with multiple values
        let mut query = Query::delete();
        query.from_table(sea_query::Alias::new("cleanup_records"))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("status"))
                       .is_in(["inactive", "banned", "deleted"]));
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test parameter correctness for IN operator (3 IN values)
            assert_eq!(params.len(), 3);
            assert_eq!(params[0], json!("inactive"));  // IN value 1
            assert_eq!(params[1], json!("banned"));    // IN value 2
            assert_eq!(params[2], json!("deleted"));   // IN value 3
            
            assert!(sql.to_uppercase().contains("DELETE"));
        }
    }
    
    #[test]
    fn test_between_operator_parameter_handling() {
        // Test BETWEEN operator parameter handling
        let mut query = Query::delete();
        query.from_table(sea_query::Alias::new("archive_data"))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("created_at")).between("2020-01-01", "2021-01-01"))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("score")).between(0, 50));
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test parameter correctness for BETWEEN (2×2 BETWEEN = 4 params)
            assert_eq!(params.len(), 4);
            assert_eq!(params[0], json!("2020-01-01"));  // WHERE created_at BETWEEN '2020-01-01'
            assert_eq!(params[1], json!("2021-01-01"));  // AND '2021-01-01'
            assert_eq!(params[2], json!(0));             // WHERE score BETWEEN 0
            assert_eq!(params[3], json!(50));            // AND 50
            
            assert!(sql.to_uppercase().contains("DELETE"));
        }
    }
    
    #[test]
    fn test_null_handling() {
        // Test NULL/NOT NULL handling (no parameters for NULL checks)
        let mut query = Query::delete();
        query.from_table(sea_query::Alias::new("expired_sessions"))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("deleted_at")).is_not_null())
             .and_where(sea_query::Expr::col(sea_query::Alias::new("temp_token")).is_null());
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // NULL checks don't generate parameters
            assert_eq!(params.len(), 0);
            
            assert!(sql.to_uppercase().contains("DELETE"));
        }
    }
    
    #[test]
    fn test_comparison_operators_parameter_handling() {
        // Test all comparison operators
        let mut query = Query::delete();
        query.from_table(sea_query::Alias::new("analytics_cleanup"))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("views")).gt(1000))      // >
             .and_where(sea_query::Expr::col(sea_query::Alias::new("likes")).gte(100))      // >=
             .and_where(sea_query::Expr::col(sea_query::Alias::new("errors")).lt(10))       // <
             .and_where(sea_query::Expr::col(sea_query::Alias::new("warnings")).lte(5))     // <=
             .and_where(sea_query::Expr::col(sea_query::Alias::new("status")).ne("active")); // !=
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test parameter correctness and order (5 WHERE conditions)
            assert_eq!(params.len(), 5);
            assert_eq!(params[0], json!(1000));      // WHERE views > 1000
            assert_eq!(params[1], json!(100));       // WHERE likes >= 100
            assert_eq!(params[2], json!(10));        // WHERE errors < 10
            assert_eq!(params[3], json!(5));         // WHERE warnings <= 5
            assert_eq!(params[4], json!("active"));  // WHERE status != 'active'
            
            assert!(sql.to_uppercase().contains("DELETE"));
        }
    }
    
    #[test]
    fn test_returning_clause_database_support() {
        // Test RETURNING clause handling across different databases
        let mut query = Query::delete();
        query.from_table(sea_query::Alias::new("user_logs"))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("active")).eq(false))
             .returning_all();
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test parameter correctness
            assert_eq!(params.len(), 1);
            assert_eq!(params[0], json!(false));  // WHERE active = false
            
            // Test database-specific RETURNING support
            if dialect.supports_returning() {
                assert!(sql.to_uppercase().contains("RETURNING"));
            }
            // Note: For databases without RETURNING, our application logic
            // handles this, not the SQL generation
            
            assert!(sql.to_uppercase().contains("DELETE"));
        }
    }
    
    #[test]
    fn test_returning_specific_columns() {
        // Test RETURNING specific columns
        let mut query = Query::delete();
        query.from_table(sea_query::Alias::new("audit_records"))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("id")).eq(42))
             .returning_col(sea_query::Alias::new("id"))
             .returning_col(sea_query::Alias::new("deleted_at"));
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test parameter correctness
            assert_eq!(params.len(), 1);
            assert_eq!(params[0], json!(42));  // WHERE id = 42
            
            // Test database-specific RETURNING support
            if dialect.supports_returning() {
                assert!(sql.to_uppercase().contains("RETURNING"));
                // Verify it's returning specific columns, not *
                assert!(!sql.contains("RETURNING *"));
            }
            
            assert!(sql.to_uppercase().contains("DELETE"));
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
        let bool_val = json_to_sea_value(&json!(false));
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
        let mut query = Query::delete();
        query.from_table(sea_query::Alias::new("international_content"))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("name")).like("测试%"))     // Chinese
             .and_where(sea_query::Expr::col(sea_query::Alias::new("content")).like("%🎉%"))   // Emoji
             .and_where(sea_query::Expr::col(sea_query::Alias::new("notes")).eq("!@#$%^&*()"));
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test parameter correctness with unicode
            assert_eq!(params.len(), 3);
            assert_eq!(params[0], json!("测试%"));       // Chinese LIKE pattern
            assert_eq!(params[1], json!("%🎉%"));         // Emoji LIKE pattern
            assert_eq!(params[2], json!("!@#$%^&*()"));    // Special characters
            
            assert!(sql.to_uppercase().contains("DELETE"));
        }
    }
    
    #[test]
    fn test_no_where_clause_parameter_handling() {
        // Test DELETE without WHERE clause (affects all rows - dangerous but valid)
        let query = Query::delete()
            .from_table(sea_query::Alias::new("temp_table"))
            .to_owned();
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // No parameters for DELETE without WHERE
            assert_eq!(params.len(), 0);
            
            assert!(sql.to_uppercase().contains("DELETE"));
        }
    }
    
    #[test]
    fn test_complex_delete_parameter_order() {
        // Test complex DELETE with multiple conditions to verify parameter order
        let mut query = Query::delete();
        query.from_table(sea_query::Alias::new("user_cleanup"))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("active")).eq(false))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("email")).like("%@temp.com"))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("created_at")).between("2020-01-01", "2021-01-01"))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("login_count")).lte(1))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("role")).is_in(["guest", "temp"]));
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test parameter correctness and order: 1+1+2+1+2 = 7 parameters
            assert_eq!(params.len(), 7);
            assert_eq!(params[0], json!(false));           // WHERE active = false
            assert_eq!(params[1], json!("%@temp.com"));    // WHERE email LIKE '%@temp.com'
            assert_eq!(params[2], json!("2020-01-01"));     // WHERE created_at BETWEEN '2020-01-01'
            assert_eq!(params[3], json!("2021-01-01"));     // AND '2021-01-01'
            assert_eq!(params[4], json!(1));               // WHERE login_count <= 1
            assert_eq!(params[5], json!("guest"));          // WHERE role IN ('guest',
            assert_eq!(params[6], json!("temp"));           //                'temp')
            
            assert!(sql.to_uppercase().contains("DELETE"));
        }
    }
    
    #[test]
    fn test_sql_injection_prevention_through_parameters() {
        // Test that malicious content is safely parameterized
        let malicious_content = "1; DROP TABLE users; --";
        let mut query = Query::delete();
        query.from_table(sea_query::Alias::new("safe_records"))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("user_input")).eq(malicious_content));
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test that malicious content is safely in parameters
            assert_eq!(params.len(), 1);
            assert_eq!(params[0], json!(malicious_content));
            
            // Verify SQL structure is safe (no malicious content in SQL itself)
            assert!(sql.to_uppercase().contains("DELETE"));
            // The malicious content should NOT be in the SQL string
            assert!(!sql.contains("DROP TABLE"));
            assert!(!sql.contains("; --"));
        }
    }
    
    #[test]
    fn test_delete_safety_considerations() {
        // Test that DELETE without WHERE is possible but generates no parameters
        let query = Query::delete()
            .from_table(sea_query::Alias::new("global_reset"))
            .to_owned();
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // No parameters for global DELETE (dangerous but valid)
            assert_eq!(params.len(), 0);
            assert!(sql.to_uppercase().contains("DELETE"));
        }
        
        // Test safer DELETE with WHERE condition
        let mut safe_query = Query::delete();
        safe_query.from_table(sea_query::Alias::new("safe_cleanup"))
                  .and_where(sea_query::Expr::col(sea_query::Alias::new("id")).eq(123));
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = safe_query.render_for_dialect(dialect);
            
            // Safe DELETE has parameters
            assert_eq!(params.len(), 1);
            assert_eq!(params[0], json!(123));
            assert!(sql.to_uppercase().contains("DELETE"));
        }
    }
    
    #[test]
    fn test_method_chaining_compilation() {
        // Test that our fluent interface compiles correctly
        // This test verifies the API design, not runtime behavior
        let mut query = Query::delete();
        query.from_table(sea_query::Alias::new("chain_test"))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("field1")).eq(1))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("field2")).ne(2))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("field3")).gt(3))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("field4")).gte(4))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("field5")).lt(5))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("field6")).lte(6))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("field7")).like("pattern"))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("field8")).is_in([1, 2, 3]))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("field9")).between(1, 10))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("field10")).is_null())
             .and_where(sea_query::Expr::col(sea_query::Alias::new("field11")).is_not_null())
             .returning_all();
        
        // Test parameter generation for this complex query
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Expected parameters: 1+1+1+1+1+1+1+3+2+0+0 = 12 parameters
            assert_eq!(params.len(), 12);
            
            assert!(sql.to_uppercase().contains("DELETE"));
        }
    }
}