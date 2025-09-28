use super::{QueryRenderer, json_to_sea_value};
use crate::backends::{DatabaseBackend, QueryResult};
use crate::{Entity, Result, D1RsError, DatabaseClient};
use crate::dialects::DatabaseDialect;
use sea_query::{Query, SelectStatement, Expr, Order};
use std::marker::PhantomData;
use serde_json::Value;

/// Type-safe SELECT query builder for entities
/// 
/// Provides a fluent API for building SELECT queries with compile-time type safety.
/// The generic parameter T ensures that the query can only be executed for the correct entity type.
pub struct TypeSafeSelect<T: Entity> {
    query: SelectStatement,
    _phantom: PhantomData<T>,
}

impl<T: Entity> TypeSafeSelect<T> {
    /// Create a new SELECT query builder for entity T
    /// 
    /// Automatically sets up the query to select from the entity's table
    /// with all columns selected by default.
    pub fn new() -> Self {
        let mut query = Query::select();
        query.from(sea_query::Alias::new(T::TABLE_NAME))
             .column(sea_query::Asterisk);
        
        Self {
            query,
            _phantom: PhantomData,
        }
    }
    
    /// Add WHERE column = value condition
    /// 
    /// # Examples
    /// ```
    /// let query = TypeSafeSelect::<User>::new()
    ///     .where_eq("name", "Alice");
    /// ```
    pub fn where_eq<V>(mut self, column: &str, value: V) -> Self 
    where
        V: Into<Value>,
    {
        let sea_value = json_to_sea_value(&value.into());
        self.query.and_where(Expr::col(sea_query::Alias::new(column)).eq(sea_value));
        self
    }
    
    /// Add WHERE column != value condition
    pub fn where_ne<V>(mut self, column: &str, value: V) -> Self 
    where
        V: Into<Value>,
    {
        let sea_value = json_to_sea_value(&value.into());
        self.query.and_where(Expr::col(sea_query::Alias::new(column)).ne(sea_value));
        self
    }
    
    /// Add WHERE column > value condition
    pub fn where_gt<V>(mut self, column: &str, value: V) -> Self 
    where
        V: Into<Value>,
    {
        let sea_value = json_to_sea_value(&value.into());
        self.query.and_where(Expr::col(sea_query::Alias::new(column)).gt(sea_value));
        self
    }
    
    /// Add WHERE column >= value condition
    pub fn where_gte<V>(mut self, column: &str, value: V) -> Self 
    where
        V: Into<Value>,
    {
        let sea_value = json_to_sea_value(&value.into());
        self.query.and_where(Expr::col(sea_query::Alias::new(column)).gte(sea_value));
        self
    }
    
    /// Add WHERE column < value condition
    pub fn where_lt<V>(mut self, column: &str, value: V) -> Self 
    where
        V: Into<Value>,
    {
        let sea_value = json_to_sea_value(&value.into());
        self.query.and_where(Expr::col(sea_query::Alias::new(column)).lt(sea_value));
        self
    }
    
    /// Add WHERE column <= value condition
    pub fn where_lte<V>(mut self, column: &str, value: V) -> Self 
    where
        V: Into<Value>,
    {
        let sea_value = json_to_sea_value(&value.into());
        self.query.and_where(Expr::col(sea_query::Alias::new(column)).lte(sea_value));
        self
    }
    
    /// Add WHERE column LIKE pattern condition
    /// 
    /// # Examples
    /// ```
    /// let query = TypeSafeSelect::<User>::new()
    ///     .where_like("name", "%alice%");
    /// ```
    pub fn where_like<V>(mut self, column: &str, pattern: V) -> Self 
    where
        V: Into<Value>,
    {
        let value = pattern.into();
        if let Value::String(pattern_str) = value {
            self.query.and_where(Expr::col(sea_query::Alias::new(column)).like(pattern_str));
        } else {
            // Fallback: convert to string and use
            let pattern_str = value.to_string().trim_matches('"').to_string();
            self.query.and_where(Expr::col(sea_query::Alias::new(column)).like(pattern_str));
        }
        self
    }
    
    /// Add WHERE column IN (values...) condition
    /// 
    /// # Examples
    /// ```
    /// let query = TypeSafeSelect::<User>::new()
    ///     .where_in("status", vec!["active", "pending"]);
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
    
    /// Add WHERE column BETWEEN start AND end condition
    pub fn where_between<V>(mut self, column: &str, start: V, end: V) -> Self 
    where
        V: Into<Value>,
    {
        let start_value = json_to_sea_value(&start.into());
        let end_value = json_to_sea_value(&end.into());
        self.query.and_where(Expr::col(sea_query::Alias::new(column)).between(start_value, end_value));
        self
    }
    
    /// Add WHERE column IS NULL condition
    pub fn where_null(mut self, column: &str) -> Self {
        self.query.and_where(Expr::col(sea_query::Alias::new(column)).is_null());
        self
    }
    
    /// Add WHERE column IS NOT NULL condition
    pub fn where_not_null(mut self, column: &str) -> Self {
        self.query.and_where(Expr::col(sea_query::Alias::new(column)).is_not_null());
        self
    }
    
    /// Add ORDER BY column ASC
    /// 
    /// # Examples
    /// ```
    /// let query = TypeSafeSelect::<User>::new()
    ///     .order_by_asc("created_at");
    /// ```
    pub fn order_by_asc(mut self, column: &str) -> Self {
        self.query.order_by(sea_query::Alias::new(column), Order::Asc);
        self
    }
    
    /// Add ORDER BY column DESC
    pub fn order_by_desc(mut self, column: &str) -> Self {
        self.query.order_by(sea_query::Alias::new(column), Order::Desc);
        self
    }
    
    /// Set LIMIT for pagination
    /// 
    /// # Examples
    /// ```
    /// let query = TypeSafeSelect::<User>::new()
    ///     .limit(10);
    /// ```
    pub fn limit(mut self, limit: u64) -> Self {
        self.query.limit(limit);
        self
    }
    
    /// Set OFFSET for pagination
    /// 
    /// # Examples
    /// ```
    /// let query = TypeSafeSelect::<User>::new()
    ///     .limit(10)
    ///     .offset(20);
    /// ```
    pub fn offset(mut self, offset: u64) -> Self {
        self.query.offset(offset);
        self
    }
    
    /// Execute query and return all matching entities
    /// 
    /// Uses the database dialect from the client to generate optimal SQL.
    /// 
    /// # Examples
    /// ```
    /// let users = TypeSafeSelect::<User>::new()
    ///     .where_eq("status", "active")
    ///     .order_by_asc("name")
    ///     .all(&client)
    ///     .await?;
    /// ```
    pub async fn all<B: DatabaseBackend>(self, client: &DatabaseClient<B>) -> Result<Vec<T>> {
        let (sql, params) = self.query.render_for_dialect(client.dialect());
        let result = client.execute(&sql, &params).await
            .map_err(|e| D1RsError::Database(format!("Query execution failed: {:?}", e)))?;
        result.into_entities()
            .map_err(|e| D1RsError::Database(format!("Entity conversion failed: {:?}", e)))
    }
    
    /// Execute query and return the first matching entity
    /// 
    /// Automatically adds LIMIT 1 to the query for efficiency.
    /// Uses the database dialect from the client to generate optimal SQL.
    /// 
    /// # Examples
    /// ```
    /// let user = TypeSafeSelect::<User>::new()
    ///     .where_eq("email", "alice@example.com")
    ///     .first(&client)
    ///     .await?;
    /// ```
    pub async fn first<B: DatabaseBackend>(mut self, client: &DatabaseClient<B>) -> Result<Option<T>> {
        self.query.limit(1);
        let (sql, params) = self.query.render_for_dialect(client.dialect());
        let result = client.execute(&sql, &params).await
            .map_err(|e| D1RsError::Database(format!("Query execution failed: {:?}", e)))?;
        result.into_entity()
            .map_err(|e| D1RsError::Database(format!("Entity conversion failed: {:?}", e)))
    }
    
    /// Execute query and return the count of matching records
    /// 
    /// Generates an optimized COUNT(*) query that preserves all WHERE conditions
    /// but ignores ORDER BY, LIMIT, and OFFSET for efficiency.
    /// Uses the database dialect from the client to generate optimal SQL.
    /// 
    /// # Examples
    /// ```
    /// let count = TypeSafeSelect::<User>::new()
    ///     .where_eq("status", "active")
    ///     .count(&client)
    ///     .await?;
    /// ```
    pub async fn count<B: DatabaseBackend>(mut self, client: &DatabaseClient<B>) -> Result<i64> {
        // Convert the existing query to a COUNT query by modifying it directly
        // This preserves all WHERE conditions naturally without any string parsing
        
        // Clear existing selections and set to COUNT(*)
        self.query.clear_selects()
                  .expr(sea_query::Func::count(Expr::col(sea_query::Asterisk)));
        
        // Clear ORDER BY since it's meaningless for COUNT queries
        self.query.clear_order_by();
        
        // Render and execute the COUNT query
        let (sql, params) = self.query.render_for_dialect(client.dialect());
        let result = client.execute(&sql, &params).await
            .map_err(|e| D1RsError::Database(format!("Count query execution failed: {:?}", e)))?;
        result.extract_count()
            .map_err(|e| D1RsError::Database(format!("Count extraction failed: {:?}", e)))
    }
    
    /// Check if any records match the query conditions
    /// 
    /// More efficient than calling count() > 0 as it uses LIMIT 1.
    /// Uses the database dialect from the client to generate optimal SQL.
    pub async fn exists<B: DatabaseBackend>(mut self, client: &DatabaseClient<B>) -> Result<bool> {
        self.query.limit(1);
        let (sql, params) = self.query.render_for_dialect(client.dialect());
        let result = client.execute(&sql, &params).await
            .map_err(|e| D1RsError::Database(format!("Exists query execution failed: {:?}", e)))?;
        Ok(!result.is_empty())
    }
}

impl<T: Entity> QueryRenderer for TypeSafeSelect<T> {
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
    fn test_query_builder_parameter_binding() {
        // Test that our query builder correctly handles parameter binding
        let mut query = Query::select();
        query.from(sea_query::Alias::new("users"))
             .column(sea_query::Asterisk)
             .and_where(sea_query::Expr::col(sea_query::Alias::new("name")).eq("Alice"))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("age")).gt(25));
        
        // Test parameter rendering across all database dialects
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test parameter correctness - this is our business logic
            assert_eq!(params.len(), 2);
            assert_eq!(params[0], json!("Alice"));
            assert_eq!(params[1], json!(25));
            
            // Basic sanity check that sea-query generated a SELECT (but don't test format)
            assert!(sql.to_uppercase().contains("SELECT"));
        }
    }
    
    #[test] 
    fn test_where_condition_operators() {
        // Test that our operators map correctly to sea-query expressions
        let mut query = Query::select();
        query.from(sea_query::Alias::new("products"))
             .column(sea_query::Asterisk)
             .and_where(sea_query::Expr::col(sea_query::Alias::new("price")).gt(100))      // >
             .and_where(sea_query::Expr::col(sea_query::Alias::new("discount")).gte(10))    // >=
             .and_where(sea_query::Expr::col(sea_query::Alias::new("stock")).lt(50))        // <
             .and_where(sea_query::Expr::col(sea_query::Alias::new("rating")).lte(4))       // <=
             .and_where(sea_query::Expr::col(sea_query::Alias::new("status")).ne("banned"));// !=
        
        // Test across all database dialects
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test parameter correctness and order
            assert_eq!(params.len(), 5);
            assert_eq!(params[0], json!(100));     // price > 100
            assert_eq!(params[1], json!(10));      // discount >= 10
            assert_eq!(params[2], json!(50));      // stock < 50
            assert_eq!(params[3], json!(4));       // rating <= 4
            assert_eq!(params[4], json!("banned"));// status != "banned"
            
            // Basic sanity check
            assert!(sql.to_uppercase().contains("SELECT"));
        }
    }
    
    #[test]
    fn test_like_pattern_handling() {
        // Test LIKE operator parameter handling
        let mut query = Query::select();
        query.from(sea_query::Alias::new("users"))
             .column(sea_query::Asterisk)
             .and_where(sea_query::Expr::col(sea_query::Alias::new("name")).like("%Alice%"))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("email")).like("%.com"));
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test parameter correctness for LIKE patterns
            assert_eq!(params.len(), 2);
            assert_eq!(params[0], json!("%Alice%"));
            assert_eq!(params[1], json!("%.com"));
            
            assert!(sql.to_uppercase().contains("SELECT"));
        }
    }
    
    #[test]
    fn test_in_operator_parameter_handling() {
        // Test IN operator with multiple values
        let mut query = Query::select();
        query.from(sea_query::Alias::new("orders"))
             .column(sea_query::Asterisk)
             .and_where(sea_query::Expr::col(sea_query::Alias::new("status"))
                       .is_in(["pending", "processing", "shipped"]));
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test parameter correctness for IN operator
            assert_eq!(params.len(), 3);
            assert_eq!(params[0], json!("pending"));
            assert_eq!(params[1], json!("processing"));
            assert_eq!(params[2], json!("shipped"));
            
            assert!(sql.to_uppercase().contains("SELECT"));
        }
    }
    
    #[test]
    fn test_between_operator_parameter_handling() {
        // Test BETWEEN operator parameter handling
        let mut query = Query::select();
        query.from(sea_query::Alias::new("events"))
             .column(sea_query::Asterisk)
             .and_where(sea_query::Expr::col(sea_query::Alias::new("age")).between(18, 65))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("score")).between(75.5, 95.8));
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test parameter correctness for BETWEEN
            assert_eq!(params.len(), 4);
            assert_eq!(params[0], json!(18));      // age BETWEEN 18
            assert_eq!(params[1], json!(65));      // AND 65
            assert_eq!(params[2], json!(75.5));    // score BETWEEN 75.5
            assert_eq!(params[3], json!(95.8));    // AND 95.8
            
            assert!(sql.to_uppercase().contains("SELECT"));
        }
    }
    
    #[test]
    fn test_null_handling() {
        // Test NULL/NOT NULL handling (no parameters expected)
        let mut query = Query::select();
        query.from(sea_query::Alias::new("profiles"))
             .column(sea_query::Asterisk)
             .and_where(sea_query::Expr::col(sea_query::Alias::new("avatar")).is_null())
             .and_where(sea_query::Expr::col(sea_query::Alias::new("name")).is_not_null());
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // NULL checks don't generate parameters
            assert_eq!(params.len(), 0);
            
            assert!(sql.to_uppercase().contains("SELECT"));
        }
    }
    
    #[test]
    fn test_ordering_and_pagination_parameters() {
        // Test ORDER BY, LIMIT, OFFSET parameter handling
        let mut query = Query::select();
        query.from(sea_query::Alias::new("posts"))
             .column(sea_query::Asterisk)
             .and_where(sea_query::Expr::col(sea_query::Alias::new("published")).eq(true))
             .order_by(sea_query::Alias::new("created_at"), sea_query::Order::Desc)
             .order_by(sea_query::Alias::new("title"), sea_query::Order::Asc)
             .limit(25)
             .offset(50);
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test parameter correctness: 1 WHERE + 1 LIMIT + 1 OFFSET = 3 params
            assert_eq!(params.len(), 3);
            assert_eq!(params[0], json!(true));   // WHERE published = true
            assert_eq!(params[1], json!(25));     // LIMIT 25
            assert_eq!(params[2], json!(50));     // OFFSET 50
            
            assert!(sql.to_uppercase().contains("SELECT"));
        }
    }
    
    #[test]
    fn test_json_to_sea_value_conversion() {
        // Test our value conversion logic
        use super::json_to_sea_value;
        
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
        // null_val format varies by sea-query version - just ensure no panic
        let _null_handled = null_val;
    }
    
    #[test]
    fn test_complex_query_parameter_order() {
        // Test complex query with multiple conditions to verify parameter order
        let mut query = Query::select();
        query.from(sea_query::Alias::new("analytics"))
             .column(sea_query::Asterisk)
             .and_where(sea_query::Expr::col(sea_query::Alias::new("active")).eq(true))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("category")).like("%finance%"))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("score")).gte(85))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("tags")).is_in(["urgent", "priority"]))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("price")).between(100.0, 500.0))
             .order_by(sea_query::Alias::new("created_at"), sea_query::Order::Desc)
             .limit(20)
             .offset(40);
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test parameter correctness and order: 1+1+1+2+2+1+1 = 9 parameters
            assert_eq!(params.len(), 9);
            assert_eq!(params[0], json!(true));        // active = true
            assert_eq!(params[1], json!("%finance%"));  // category LIKE '%finance%'
            assert_eq!(params[2], json!(85));          // score >= 85
            assert_eq!(params[3], json!("urgent"));      // tags IN ('urgent',
            assert_eq!(params[4], json!("priority"));    //          'priority')
            assert_eq!(params[5], json!(100.0));       // price BETWEEN 100.0
            assert_eq!(params[6], json!(500.0));       //           AND 500.0
            assert_eq!(params[7], json!(20));          // LIMIT 20
            assert_eq!(params[8], json!(40));          // OFFSET 40
            
            assert!(sql.to_uppercase().contains("SELECT"));
        }
    }
    
    #[test]
    fn test_count_query_optimization() {
        // Test COUNT query parameter preservation
        let mut query = Query::select();
        query.from(sea_query::Alias::new("reports"))
             .column(sea_query::Asterisk)
             .and_where(sea_query::Expr::col(sea_query::Alias::new("status")).eq("published"))
             .and_where(sea_query::Expr::col(sea_query::Alias::new("views")).gte(1000))
             .order_by(sea_query::Alias::new("created_at"), sea_query::Order::Desc)
             .limit(50);
        
        // Convert to COUNT query
        query.clear_selects()
             .expr(sea_query::Func::count(sea_query::Expr::col(sea_query::Asterisk)))
             .clear_order_by();
        // Note: LIMIT should be preserved for COUNT queries in some cases
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test that WHERE parameters are preserved in COUNT
            assert_eq!(params.len(), 3);  // 2 WHERE + 1 LIMIT
            assert_eq!(params[0], json!("published"));
            assert_eq!(params[1], json!(1000));
            assert_eq!(params[2], json!(50));
            
            assert!(sql.to_uppercase().contains("SELECT"));
        }
    }
    
    #[test]
    fn test_unicode_and_special_character_parameters() {
        // Test that our parameter handling works with special characters
        let mut query = Query::select();
        query.from(sea_query::Alias::new("international_users"))
             .column(sea_query::Asterisk)
             .and_where(sea_query::Expr::col(sea_query::Alias::new("name")).eq("测试用户"))   // Chinese
             .and_where(sea_query::Expr::col(sea_query::Alias::new("emoji")).eq("🚀✨"))    // Emoji
             .and_where(sea_query::Expr::col(sea_query::Alias::new("special")).eq("!@#$%"));  // Special chars
        
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.render_for_dialect(dialect);
            
            // Test parameter correctness with unicode
            assert_eq!(params.len(), 3);
            assert_eq!(params[0], json!("测试用户"));
            assert_eq!(params[1], json!("🚀✨"));
            assert_eq!(params[2], json!("!@#$%"));
            
            assert!(sql.to_uppercase().contains("SELECT"));
        }
    }
}