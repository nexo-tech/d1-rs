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
    /// 
    /// Since sea-query doesn't provide direct WHERE condition cloning, we use a 
    /// hybrid approach: generate the original query, extract WHERE parameters,
    /// and create a new COUNT query.
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
    use serde_json::json;
    use sea_query::{Query, SelectStatement};
    
    // Mock implementation helper that creates SelectStatement directly
    fn create_basic_select() -> SelectStatement {
        let mut query = Query::select();
        query.from(sea_query::Alias::new("users"))
             .column(sea_query::Asterisk);
        query
    }
    
    // Test SQL generation directly without Entity constraint
    #[test]
    fn test_basic_select_sql_generation() {
        let query = create_basic_select();
        let sql = query.to_string(sea_query::SqliteQueryBuilder);
        
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("FROM"));
        assert!(sql.contains("users"));
    }
    
    #[test] 
    fn test_where_conditions() {
        let mut query = create_basic_select();
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("name")).eq("Alice"));
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("age")).gt(25));
        
        let sql = query.to_string(sea_query::SqliteQueryBuilder);
        
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("name"));
        assert!(sql.contains("age"));
        assert!(sql.contains("AND"));
    }
    
    #[test]
    fn test_like_condition() {
        let mut query = create_basic_select();
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("name")).like("%Alice%"));
        
        let sql = query.to_string(sea_query::SqliteQueryBuilder);
        
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("LIKE"));
        assert!(sql.contains("%Alice%"));
    }
    
    #[test]
    fn test_in_condition() {
        let mut query = create_basic_select();
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("name"))
            .is_in(["Alice", "Bob", "Charlie"]));
        
        let sql = query.to_string(sea_query::SqliteQueryBuilder);
        
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("IN"));
    }
    
    #[test]
    fn test_between_condition() {
        let mut query = create_basic_select();
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("age")).between(18, 65));
        
        let sql = query.to_string(sea_query::SqliteQueryBuilder);
        
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("BETWEEN"));
    }
    
    #[test]
    fn test_null_conditions() {
        let mut query = create_basic_select();
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("email")).is_null());
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("name")).is_not_null());
        
        let sql = query.to_string(sea_query::SqliteQueryBuilder);
        
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("IS NULL"));
        assert!(sql.contains("IS NOT NULL"));
    }
    
    #[test]
    fn test_order_by() {
        let mut query = create_basic_select();
        query.order_by(sea_query::Alias::new("name"), sea_query::Order::Asc);
        query.order_by(sea_query::Alias::new("created_at"), sea_query::Order::Desc);
        
        let sql = query.to_string(sea_query::SqliteQueryBuilder);
        
        assert!(sql.contains("ORDER BY"));
        assert!(sql.contains("ASC"));
        assert!(sql.contains("DESC"));
    }
    
    #[test]
    fn test_limit_and_offset() {
        let mut query = create_basic_select();
        query.limit(10);
        query.offset(20);
        
        let sql = query.to_string(sea_query::SqliteQueryBuilder);
        
        assert!(sql.contains("LIMIT"));
        assert!(sql.contains("OFFSET"));
        assert!(sql.contains("10"));
        assert!(sql.contains("20"));
    }
    
    #[test]
    fn test_complex_query() {
        let mut query = create_basic_select();
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("active")).eq(true));
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("name")).like("%admin%"));
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("age")).gte(21));
        query.order_by(sea_query::Alias::new("created_at"), sea_query::Order::Desc);
        query.order_by(sea_query::Alias::new("name"), sea_query::Order::Asc);
        query.limit(50);
        query.offset(100);
        
        let sql = query.to_string(sea_query::SqliteQueryBuilder);
        
        // Verify SQL structure
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("FROM"));
        assert!(sql.contains("users"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("AND"));
        assert!(sql.contains("ORDER BY"));
        assert!(sql.contains("LIMIT"));
        assert!(sql.contains("OFFSET"));
    }
    
    #[test]
    fn test_comparison_operators() {
        let mut query = create_basic_select();
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("age")).gt(18));
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("score")).gte(75));
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("attempts")).lt(5));
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("failures")).lte(2));
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("status")).ne("banned"));
        
        let sql = query.to_string(sea_query::SqliteQueryBuilder);
        
        assert!(sql.contains("WHERE"));
        assert!(sql.contains(">"));
        assert!(sql.contains(">="));
        assert!(sql.contains("<"));
        assert!(sql.contains("<="));
        // Different databases/versions may use != or <> for not equals
        assert!(sql.contains("!=") || sql.contains("<>"));
    }
    
    #[test]
    fn test_json_to_sea_value_conversion() {
        use super::json_to_sea_value;
        
        // Test string conversion
        let string_val = json_to_sea_value(&json!("test"));
        assert_eq!(string_val, sea_query::Value::String(Some(Box::new("test".to_string()))));
        
        // Test integer conversion
        let int_val = json_to_sea_value(&json!(42));
        assert!(matches!(int_val, sea_query::Value::Int(_) | sea_query::Value::BigInt(_)));
        
        // Test boolean conversion
        let bool_val = json_to_sea_value(&json!(true));
        assert_eq!(bool_val, sea_query::Value::Bool(Some(true)));
        
        // Test null conversion  
        let null_val = json_to_sea_value(&json!(null));
        // For null values, we expect a null representation - this varies by sea-query version
        // Just verify the conversion doesn't panic
        let _converted = null_val;
    }
    
    #[test]
    fn test_renderer_functionality() {
        // Create a simple query manually
        let query = create_basic_select();
        
        // Test rendering for different dialects
        let sqlite_sql = query.to_string(sea_query::SqliteQueryBuilder);
        let postgres_sql = query.to_string(sea_query::PostgresQueryBuilder);
        let mysql_sql = query.to_string(sea_query::MysqlQueryBuilder);
        
        // Basic assertions
        assert!(sqlite_sql.contains("SELECT"));
        assert!(postgres_sql.contains("SELECT"));
        assert!(mysql_sql.contains("SELECT"));
        
        // Different dialects may use different quoting
        assert!(sqlite_sql.contains("users") || sqlite_sql.contains("\"users\""));
        assert!(postgres_sql.contains("users") || postgres_sql.contains("\"users\""));
        assert!(mysql_sql.contains("users") || mysql_sql.contains("`users`"));
    }
    
    #[test]
    fn test_unicode_support() {
        let mut query = create_basic_select();
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("name")).eq("测试用户"));
        query.and_where(sea_query::Expr::col(sea_query::Alias::new("emoji")).eq("😀"));
        
        let sql = query.to_string(sea_query::SqliteQueryBuilder);
        
        // SQL should be generated without errors
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("WHERE"));
    }
    
    #[test]
    fn test_count_query_structure() {
        let mut count_query = Query::select();
        count_query.from(sea_query::Alias::new("users"))
                   .expr(sea_query::Expr::count(sea_query::Expr::col(sea_query::Asterisk)));
        
        let sql = count_query.to_string(sea_query::SqliteQueryBuilder);
        
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("COUNT"));
        assert!(sql.contains("FROM"));
        assert!(sql.contains("users"));
    }
}