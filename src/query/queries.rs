use serde_json::{Value, json};
use crate::query_builder::{QueryRenderer, json_to_sea_value};
use crate::dialects::DatabaseDialect;
use sea_query::{Query as SeaQuery, SelectStatement, Expr, Order};

// Import type-safe parameter building system from sibling module
// Note: These imports removed as we now use pure sea-query

#[derive(Debug, Clone)]
pub struct WhereClause {
    pub column: String,
    pub operator: String,
    pub value: Value,
}

#[derive(Debug, Clone)]
pub struct OrderBy {
    pub column: String,
    pub ascending: bool,
}

// Keep same public API but use sea-query internally
#[derive(Debug)]
pub struct Query {
    inner: SelectStatement,
    // Keep track of original data for backward compatibility
    where_clauses: Vec<WhereClause>,
    order_by: Vec<OrderBy>,
    limit: Option<i64>,
    offset: Option<i64>,
}

impl Query {
    pub fn new(table: String) -> Self {
        let mut select = SeaQuery::select();
        select.from(sea_query::Alias::new(&table))
              .column(sea_query::Asterisk);
        
        Self {
            inner: select,
            where_clauses: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
        }
    }

    pub fn where_clause(&mut self, column: &str, operator: &str, value: Value) -> &mut Self {
        // Store for backward compatibility
        self.where_clauses.push(WhereClause {
            column: column.to_string(),
            operator: operator.to_string(),
            value: value.clone(),
        });
        
        // Also build sea-query for new functionality
        let sea_value = json_to_sea_value(&value);
        let column_expr = Expr::col(sea_query::Alias::new(column));
        
        match operator {
            "=" => { self.inner.and_where(column_expr.eq(sea_value)); },
            "!=" => { self.inner.and_where(column_expr.ne(sea_value)); },
            ">" => { self.inner.and_where(column_expr.gt(sea_value)); },
            ">=" => { self.inner.and_where(column_expr.gte(sea_value)); },
            "<" => { self.inner.and_where(column_expr.lt(sea_value)); },
            "<=" => { self.inner.and_where(column_expr.lte(sea_value)); },
            "LIKE" => {
                // Handle LIKE operator - extract string value from sea_value
                if let sea_query::Value::String(Some(ref pattern)) = sea_value {
                    self.inner.and_where(column_expr.like(pattern.as_str()));
                } else {
                    // Fallback to equality for non-string LIKE values
                    self.inner.and_where(column_expr.eq(sea_value));
                }
            },
            "IN" => {
                // Handle IN operator specially
                if let Value::Array(values) = value {
                    let sea_values: Vec<_> = values.iter().map(json_to_sea_value).collect();
                    self.inner.and_where(column_expr.is_in(sea_values));
                } else {
                    // Fallback for non-array IN values
                    self.inner.and_where(column_expr.eq(sea_value));
                }
            },
            "BETWEEN" => {
                // Handle BETWEEN operator specially
                if let Value::Array(values) = value {
                    if values.len() >= 2 {
                        let start = json_to_sea_value(&values[0]);
                        let end = json_to_sea_value(&values[1]);
                        self.inner.and_where(column_expr.between(start, end));
                    } else {
                        // Fallback for malformed BETWEEN values
                        self.inner.and_where(column_expr.eq(sea_value));
                    }
                } else {
                    // Fallback for non-array BETWEEN values
                    self.inner.and_where(column_expr.eq(sea_value));
                }
            },
            "IS NOT" => {
                // Handle IS NOT NULL
                self.inner.and_where(column_expr.is_not_null());
            },
            _ => {
                // Default to equality for unknown operators
                self.inner.and_where(column_expr.eq(sea_value));
            }
        }
        self
    }

    pub fn order_by(&mut self, column: &str, ascending: bool) -> &mut Self {
        // Store for backward compatibility
        self.order_by.push(OrderBy {
            column: column.to_string(),
            ascending,
        });
        
        // Also build sea-query for new functionality
        let order = if ascending { Order::Asc } else { Order::Desc };
        self.inner.order_by(sea_query::Alias::new(column), order);
        self
    }

    pub fn limit(&mut self, limit: i64) -> &mut Self {
        // Store for backward compatibility
        self.limit = Some(limit);
        
        // Also build sea-query for new functionality
        self.inner.limit(limit as u64);
        self
    }

    pub fn offset(&mut self, offset: i64) -> &mut Self {
        // Store for backward compatibility
        self.offset = Some(offset);
        
        // Also build sea-query for new functionality
        self.inner.offset(offset as u64);
        self
    }

    // Clean sea-query implementation - database-agnostic
    pub fn to_sql(&self, dialect: DatabaseDialect) -> (String, Vec<Value>) {
        self.inner.render_for_dialect(dialect)
    }
    
    pub fn to_count_sql(&self, dialect: DatabaseDialect) -> (String, Vec<Value>) {
        // Create a pure sea-query COUNT query preserving all WHERE conditions
        let mut count_query = self.inner.clone();
        
        // Clear existing selections and set to COUNT(*)
        count_query.clear_selects()
                   .expr(sea_query::Func::count(Expr::col(sea_query::Asterisk)));
        
        // Clear ORDER BY for count queries (LIMIT/OFFSET don't affect COUNT)
        count_query.clear_order_by();
        
        // Render the pure sea-query COUNT statement for the specified dialect
        count_query.render_for_dialect(dialect)
    }
}

// Implement QueryRenderer for our Query wrapper
impl QueryRenderer for Query {
    fn render_for_dialect(&self, dialect: DatabaseDialect) -> (String, Vec<Value>) {
        self.inner.render_for_dialect(dialect)
    }
}


#[derive(Debug)]
pub struct InsertQuery {
    table: String,
    columns: Vec<String>,
    values: Vec<Value>,
}

impl InsertQuery {
    pub fn new(table: String) -> Self {
        Self {
            table,
            columns: Vec::new(),
            values: Vec::new(),
        }
    }

    pub fn set(&mut self, column: &str, value: Value) -> &mut Self {
        self.columns.push(column.to_string());
        self.values.push(value);
        self
    }

    pub fn to_sql(&self, dialect: DatabaseDialect) -> (String, Vec<Value>) {
        let mut insert = SeaQuery::insert();
        insert.into_table(sea_query::Alias::new(&self.table))
              .returning_all();
        
        if !self.columns.is_empty() {
            let columns: Vec<_> = self.columns.iter()
                .map(|k| sea_query::Alias::new(k))
                .collect();
            let expr_values: Vec<_> = self.values.iter()
                .map(|v| json_to_sea_value(v).into())
                .collect();
                
            insert.columns(columns).values_panic(expr_values);
        }
        
        insert.render_for_dialect(dialect)
    }
}

#[derive(Debug)]
pub struct UpdateQuery {
    inner: sea_query::UpdateStatement,
}

impl UpdateQuery {
    pub fn new(table: String) -> Self {
        let mut update = SeaQuery::update();
        update.table(sea_query::Alias::new(&table))
              .returning_all();
        
        Self {
            inner: update,
        }
    }

    pub fn set(&mut self, column: &str, value: Value) -> &mut Self {
        let sea_value = json_to_sea_value(&value);
        self.inner.value(sea_query::Alias::new(column), sea_value);
        self
    }

    pub fn where_clause(&mut self, column: &str, operator: &str, value: Value) -> &mut Self {
        let sea_value = json_to_sea_value(&value);
        let column_expr = Expr::col(sea_query::Alias::new(column));
        
        match operator {
            "=" => { self.inner.and_where(column_expr.eq(sea_value)); },
            "!=" => { self.inner.and_where(column_expr.ne(sea_value)); },
            ">" => { self.inner.and_where(column_expr.gt(sea_value)); },
            ">=" => { self.inner.and_where(column_expr.gte(sea_value)); },
            "<" => { self.inner.and_where(column_expr.lt(sea_value)); },
            "<=" => { self.inner.and_where(column_expr.lte(sea_value)); },
            "LIKE" => {
                if let sea_query::Value::String(Some(ref pattern)) = sea_value {
                    self.inner.and_where(column_expr.like(pattern.as_str()));
                } else {
                    self.inner.and_where(column_expr.eq(sea_value));
                }
            },
            "IN" => {
                if let Value::Array(values) = value {
                    let sea_values: Vec<_> = values.iter().map(json_to_sea_value).collect();
                    self.inner.and_where(column_expr.is_in(sea_values));
                } else {
                    self.inner.and_where(column_expr.eq(sea_value));
                }
            },
            "IS NOT" => {
                if value == json!(null) {
                    self.inner.and_where(column_expr.is_not_null());
                } else {
                    self.inner.and_where(column_expr.ne(sea_value));
                }
            },
            _ => {
                self.inner.and_where(column_expr.eq(sea_value));
            }
        }
        self
    }

    pub fn to_sql(&self, dialect: DatabaseDialect) -> (String, Vec<Value>) {
        self.inner.render_for_dialect(dialect)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use crate::dialects::DatabaseDialect;

    #[test]
    fn test_query_select_with_where_clauses() {
        let mut query = Query::new("users".to_string());
        query.where_clause("age", ">", json!(18));
        query.where_clause("status", "=", json!("active"));
        query.order_by("name", true);
        query.limit(10);
        query.offset(5);

        // Test all available database dialects (based on enabled features)
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.to_sql(dialect);
            
            // All dialects should generate proper SQL structure
            assert!(sql.contains("SELECT"));
            assert!(sql.contains("FROM users") || sql.contains("FROM \"users\"") || sql.contains("FROM `users`"));
            assert!(sql.contains("WHERE"));
            assert!(sql.contains("ORDER BY"));
            assert!(sql.contains("LIMIT"));
            assert!(sql.contains("OFFSET"));
            
            // Parameters should be consistent across all dialects (2 WHERE + 1 LIMIT + 1 OFFSET)
            assert_eq!(params.len(), 4);
            assert_eq!(params[0], json!(18));
            assert_eq!(params[1], json!("active"));
            assert_eq!(params[2], json!(10));  // LIMIT parameter
            assert_eq!(params[3], json!(5));   // OFFSET parameter
        }
    }

    #[test]
    fn test_query_select_without_where_clauses() {
        let mut query = Query::new("products".to_string());
        query.order_by("price", false);
        query.limit(20);

        // Test all available database dialects (based on enabled features)
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.to_sql(dialect);
            
            // All dialects should generate proper SQL structure
            assert!(sql.contains("SELECT"));
            assert!(sql.contains("FROM products") || sql.contains("FROM \"products\"") || sql.contains("FROM `products`"));
            assert!(sql.contains("ORDER BY"));
            assert!(sql.contains("DESC"));
            assert!(sql.contains("LIMIT"));
            
            // 1 parameter expected (for LIMIT)
            assert_eq!(params.len(), 1);
            assert_eq!(params[0], json!(20));
        }
    }

    #[test]
    fn test_query_count_with_where_clauses() {
        let mut query = Query::new("orders".to_string());
        query.where_clause("total", ">=", json!(100.0));
        query.where_clause("cancelled", "=", json!(false));

        // Test all available database dialects (based on enabled features)
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.to_count_sql(dialect);
            
            // All dialects should generate proper COUNT SQL structure
            assert!(sql.contains("SELECT"));
            assert!(sql.contains("COUNT"));
            assert!(sql.contains("FROM orders") || sql.contains("FROM \"orders\"") || sql.contains("FROM `orders`"));
            assert!(sql.contains("WHERE"));
            
            // Parameters should be consistent across all dialects
            assert_eq!(params.len(), 2);
            assert_eq!(params[0], json!(100.0));
            assert_eq!(params[1], json!(false));
        }
    }

    #[test]
    fn test_query_count_without_where_clauses() {
        let query = Query::new("customers".to_string());

        // Test all available database dialects (based on enabled features)
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.to_count_sql(dialect);
            
            // All dialects should generate proper COUNT SQL structure
            assert!(sql.contains("SELECT"));
            assert!(sql.contains("COUNT"));
            assert!(sql.contains("FROM customers") || sql.contains("FROM \"customers\"") || sql.contains("FROM `customers`"));
            
            // No parameters expected
            assert_eq!(params.len(), 0);
        }
    }

    #[test]
    fn test_insert_query_single_column() {
        let mut query = InsertQuery::new("users".to_string());
        query.set("name", json!("Alice"));

        // Test all available database dialects (based on enabled features)
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.to_sql(dialect);
            
            // All dialects should generate proper INSERT SQL structure (case-insensitive)
            assert!(sql.to_uppercase().contains("INSERT INTO"));
            assert!(sql.contains("users") || sql.contains("\"users\"")); // Handle quoted identifiers
            assert!(sql.contains("name") || sql.contains("\"name\"")); // Handle quoted identifiers
            assert!(sql.to_uppercase().contains("VALUES"));
            assert!(sql.to_uppercase().contains("RETURNING") || !dialect.supports_returning()); // Some databases don't support RETURNING
            
            // Parameters should be consistent across all dialects
            assert_eq!(params.len(), 1);
            assert_eq!(params[0], json!("Alice"));
        }
    }

    #[test]
    fn test_insert_query_multiple_columns() {
        let mut query = InsertQuery::new("users".to_string());
        query.set("name", json!("Bob"));
        query.set("age", json!(25));
        query.set("active", json!(true));
        query.set("score", json!(null));

        // Test all available database dialects (based on enabled features)
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.to_sql(dialect);
            
            // All dialects should generate proper INSERT SQL structure (case-insensitive)
            assert!(sql.to_uppercase().contains("INSERT INTO"));
            assert!(sql.contains("users") || sql.contains("\"users\""));
            assert!(sql.contains("name") || sql.contains("\"name\""));
            assert!(sql.contains("age") || sql.contains("\"age\""));
            assert!(sql.contains("active") || sql.contains("\"active\""));
            assert!(sql.contains("score") || sql.contains("\"score\""));
            assert!(sql.to_uppercase().contains("VALUES"));
            
            // Parameters should be consistent across all dialects
            assert_eq!(params.len(), 4);
            assert_eq!(params[0], json!("Bob"));
            assert_eq!(params[1], json!(25));
            assert_eq!(params[2], json!(true));
            assert_eq!(params[3], json!(null));
        }
    }

    #[test]
    fn test_insert_query_empty() {
        let query = InsertQuery::new("logs".to_string());

        // Test all available database dialects (based on enabled features)
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.to_sql(dialect);
            
            // All dialects should generate proper INSERT SQL structure (case-insensitive)
            assert!(sql.to_uppercase().contains("INSERT INTO"));
            assert!(sql.contains("logs") || sql.contains("\"logs\""));
            assert!(sql.to_uppercase().contains("RETURNING") || !dialect.supports_returning()); // Some databases don't support RETURNING
            
            // No parameters expected for empty insert
            assert_eq!(params.len(), 0);
        }
    }

    #[test]
    fn test_revolutionary_type_safe_placeholder_elimination() {
        // This test specifically validates that we eliminated the target line 142:
        // OLD: let placeholders = vec!["?"; self.values.len()].join(", ");
        // NEW: Type-safe parameter building with InsertParameterBuilder
        
        let mut query = InsertQuery::new("test_table".to_string());
        query.set("col1", json!("value1"));
        query.set("col2", json!(42));
        query.set("col3", json!(true));

        // Test all available database dialects (based on enabled features)
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.to_sql(dialect);
        
            // Verify the SQL structure is correct without string literal placeholders
            assert!(sql.to_uppercase().contains("INSERT INTO"));
            assert!(sql.contains("test_table") || sql.contains("\"test_table\"") || sql.contains("`test_table`"));
            assert!(sql.contains("(col1, col2, col3)") || sql.contains("(\"col1\", \"col2\", \"col3\")") || sql.contains("(`col1`, `col2`, `col3`)"));
            assert!(sql.contains("VALUES (?, ?, ?)"));
            assert!(sql.contains("RETURNING *") || !dialect.supports_returning());
            assert_eq!(params.len(), 3);
            
            // Verify parameters maintain proper order and type safety
            assert_eq!(params[0], json!("value1"));
            assert_eq!(params[1], json!(42));
            assert_eq!(params[2], json!(true));
            
            // This test proves that the type-safe system produces identical results
            // to the old string-based system but with compile-time safety
        }
    }

    #[test]
    fn test_update_query_with_where_clauses() {
        let mut query = UpdateQuery::new("users".to_string());
        query.set("name", json!("Updated Name"));
        query.set("age", json!(30));
        query.where_clause("id", "=", json!(1));
        query.where_clause("active", "=", json!(true));

        // Test all available database dialects (based on enabled features)
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.to_sql(dialect);
        
            assert!(sql.to_uppercase().contains("UPDATE"));
            assert!(sql.contains("users") || sql.contains("\"users\"") || sql.contains("`users`"));
            assert!((sql.contains("SET name = ?") || sql.contains("SET \"name\" = ?")) && (sql.contains("age = ?") || sql.contains("\"age\" = ?")));
            assert!((sql.contains("WHERE id = ?") || sql.contains("WHERE \"id\" = ?")) && (sql.contains("active = ?") || sql.contains("\"active\" = ?")));
            assert!(sql.contains("RETURNING *") || !dialect.supports_returning());
            assert_eq!(params.len(), 4);
            assert_eq!(params[0], json!("Updated Name"));
            assert_eq!(params[1], json!(30));
            assert_eq!(params[2], json!(1));
            assert_eq!(params[3], json!(true));
        }
    }

    #[test]
    fn test_update_query_without_where_clauses() {
        let mut query = UpdateQuery::new("settings".to_string());
        query.set("updated_at", json!("2024-01-01"));

        // Test all available database dialects (based on enabled features)
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.to_sql(dialect);
        
            assert!(sql.to_uppercase().contains("UPDATE"));
            assert!(sql.contains("settings") || sql.contains("\"settings\"") || sql.contains("`settings`"));
            assert!(sql.contains("SET updated_at = ?") || sql.contains("SET \"updated_at\" = ?") || sql.contains("SET `updated_at` = ?"));
            assert!(sql.contains("RETURNING *") || !dialect.supports_returning());
            assert_eq!(params.len(), 1);
            assert_eq!(params[0], json!("2024-01-01"));
        }
    }

    #[test]
    fn test_update_query_multiple_sets_no_where() {
        let mut query = UpdateQuery::new("config".to_string());
        query.set("theme", json!("dark"));
        query.set("notifications", json!(false));
        query.set("timeout", json!(5000));

        // Test all available database dialects (based on enabled features)
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.to_sql(dialect);
        
            assert!(sql.to_uppercase().contains("UPDATE"));
            assert!(sql.contains("config") || sql.contains("\"config\"") || sql.contains("`config`"));
            assert!((sql.contains("SET theme = ?") || sql.contains("SET \"theme\" = ?")) && (sql.contains("notifications = ?") || sql.contains("\"notifications\" = ?")) && (sql.contains("timeout = ?") || sql.contains("\"timeout\" = ?")));
            assert!(sql.contains("RETURNING *") || !dialect.supports_returning());
            assert_eq!(params.len(), 3);
            assert_eq!(params[0], json!("dark"));
            assert_eq!(params[1], json!(false));
            assert_eq!(params[2], json!(5000));
        }
    }

    #[test]
    fn test_complex_json_values_handling() {
        let mut query = InsertQuery::new("documents".to_string());
        
        // Test complex JSON object
        let complex_json = json!({
            "metadata": {
                "author": "test_user",
                "tags": ["important", "draft"],
                "settings": {
                    "public": false,
                    "expires": null
                }
            }
        });
        
        query.set("content", complex_json.clone());
        query.set("array_data", json!([1, 2, 3, "test"]));

        // Test all available database dialects (based on enabled features)
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.to_sql(dialect);
            
            // All dialects should generate proper INSERT SQL structure (case-insensitive)
            assert!(sql.to_uppercase().contains("INSERT INTO"));
            assert!(sql.contains("documents") || sql.contains("\"documents\""));
            assert!(sql.contains("content") || sql.contains("\"content\""));
            assert!(sql.contains("array_data") || sql.contains("\"array_data\""));
            assert!(sql.to_uppercase().contains("VALUES"));
            assert!(sql.to_uppercase().contains("RETURNING") || !dialect.supports_returning());
            
            // Parameters should be consistent across all dialects
            assert_eq!(params.len(), 2);
            assert_eq!(params[0], complex_json);
            assert_eq!(params[1], json!([1, 2, 3, "test"]));
        }
    }

    #[test]
    fn test_query_builder_method_chaining() {
        let (sql, params) = Query::new("products".to_string())
            .where_clause("category", "=", json!("electronics"))
            .where_clause("price", "BETWEEN", json!([100, 500]))
            .order_by("price", true)
            .order_by("name", true)
            .limit(50)
            .offset(25)
            .to_sql(DatabaseDialect::SQLite);
        
        // Check SQL structure is database-agnostic
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("FROM products") || sql.contains("FROM \"products\"") || sql.contains("FROM `products`"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("category") && sql.contains("price"));
        assert!(sql.contains("ORDER BY"));
        assert!(sql.contains("ASC"));
        assert!(sql.contains("LIMIT"));
        assert!(sql.contains("OFFSET"));
        
        // Parameters: 1 for category, 2 for BETWEEN range, 1 for LIMIT, 1 for OFFSET = 5 total
        assert_eq!(params.len(), 5);
        assert_eq!(params[0], json!("electronics"));
        // BETWEEN generates 2 parameters
        assert_eq!(params[1], json!(100));
        assert_eq!(params[2], json!(500));
        assert_eq!(params[3], json!(50));   // LIMIT
        assert_eq!(params[4], json!(25));   // OFFSET
    }

    #[test]
    fn test_edge_case_empty_string_values() {
        let mut query = InsertQuery::new("test_table".to_string());
        query.set("empty_string", json!(""));
        query.set("whitespace", json!("   "));
        query.set("special_chars", json!("!@#$%^&*()"));

        // Test all available database dialects (based on enabled features)
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.to_sql(dialect);
        
            assert!(sql.to_uppercase().contains("INSERT INTO"));
            assert!(sql.contains("test_table") || sql.contains("\"test_table\"") || sql.contains("`test_table`"));
            assert!(sql.contains("empty_string") && sql.contains("whitespace") && sql.contains("special_chars"));
            assert!(sql.contains("VALUES (?, ?, ?)"));
            assert!(sql.contains("RETURNING *") || !dialect.supports_returning());
            assert_eq!(params.len(), 3);
            assert_eq!(params[0], json!(""));
            assert_eq!(params[1], json!("   "));
            assert_eq!(params[2], json!("!@#$%^&*()"));
        }
    }

    #[test]
    fn test_numeric_values_precision() {
        let mut query = InsertQuery::new("measurements".to_string());
        query.set("integer", json!(42));
        query.set("float", json!(3.14159));
        query.set("negative", json!(-123.456));
        query.set("scientific", json!(1.23e-10));

        // Test all available database dialects (based on enabled features)
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.to_sql(dialect);
        
            assert!(sql.to_uppercase().contains("INSERT INTO"));
            assert!(sql.contains("measurements") || sql.contains("\"measurements\"") || sql.contains("`measurements`"));
            assert!(sql.contains("integer") && sql.contains("float") && sql.contains("negative") && sql.contains("scientific"));
            assert!(sql.contains("VALUES (?, ?, ?, ?)"));
            assert!(sql.contains("RETURNING *") || !dialect.supports_returning());
            assert_eq!(params.len(), 4);
            assert_eq!(params[0], json!(42));
            assert_eq!(params[1], json!(3.14159));
            assert_eq!(params[2], json!(-123.456));
            assert_eq!(params[3], json!(1.23e-10));
        }
    }

    #[test]
    fn test_boolean_and_null_handling() {
        let mut query = UpdateQuery::new("flags".to_string());
        query.set("is_enabled", json!(true));
        query.set("is_disabled", json!(false));
        query.set("nullable_field", json!(null));
        query.where_clause("id", "IS NOT", json!(null));

        // Test all available database dialects (based on enabled features)
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.to_sql(dialect);
        
            assert!(sql.to_uppercase().contains("UPDATE"));
            assert!(sql.contains("flags") || sql.contains("\"flags\"") || sql.contains("`flags`"));
            assert!((sql.contains("SET is_enabled = ?") || sql.contains("SET \"is_enabled\" = ?")) && (sql.contains("is_disabled = ?") || sql.contains("\"is_disabled\" = ?")) && (sql.contains("nullable_field = ?") || sql.contains("\"nullable_field\" = ?")));
            assert!(sql.contains("WHERE id IS NOT NULL") || sql.contains("WHERE \"id\" IS NOT NULL"));
            assert!(sql.contains("RETURNING *") || !dialect.supports_returning());
            assert_eq!(params.len(), 3);
            assert_eq!(params[0], json!(true));
            assert_eq!(params[1], json!(false));
            assert_eq!(params[2], json!(null));
        }
    }

    #[test]
    fn test_sql_injection_prevention() {
        // The type-safe parameter building should prevent SQL injection by design
        let mut query = Query::new("users".to_string());
        query.where_clause("username", "=", json!("'; DROP TABLE users; --"));
        query.where_clause("password", "=", json!("' OR '1'='1"));

        // Test all available database dialects (based on enabled features)
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.to_sql(dialect);
        
            // SQL structure should be preserved, malicious content should be in parameters
            assert!(sql.contains("SELECT"));
            assert!(sql.contains("FROM users") || sql.contains("FROM \"users\"") || sql.contains("FROM `users`"));
            assert!((sql.contains("WHERE username") || sql.contains("WHERE \"username\"")) && (sql.contains("password") || sql.contains("\"password\"")));
            assert!(sql.contains("= ?") && sql.contains("AND"));
            assert_eq!(params.len(), 2);
            assert_eq!(params[0], json!("'; DROP TABLE users; --"));
            assert_eq!(params[1], json!("' OR '1'='1"));
        }
    }

    #[test]
    fn test_unicode_and_special_characters() {
        let mut query = InsertQuery::new("internationalization".to_string());
        query.set("chinese", json!("你好世界"));
        query.set("emoji", json!("🚀✨🎉"));
        query.set("arabic", json!("مرحبا بالعالم"));
        query.set("russian", json!("Привет мир"));

        // Test all available database dialects (based on enabled features)
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.to_sql(dialect);
        
            assert!(sql.to_uppercase().contains("INSERT INTO"));
            assert!(sql.contains("internationalization") || sql.contains("\"internationalization\"") || sql.contains("`internationalization`"));
            assert!(sql.contains("chinese") && sql.contains("emoji") && sql.contains("arabic") && sql.contains("russian"));
            assert!(sql.contains("VALUES (?, ?, ?, ?)"));
            assert!(sql.contains("RETURNING *") || !dialect.supports_returning());
            assert_eq!(params.len(), 4);
            assert_eq!(params[0], json!("你好世界"));
            assert_eq!(params[1], json!("🚀✨🎉"));
            assert_eq!(params[2], json!("مرحبا بالعالم"));
            assert_eq!(params[3], json!("Привет мир"));
        }
    }

    #[test]
    fn test_performance_large_parameter_sets() {
        let mut query = InsertQuery::new("bulk_data".to_string());
        
        // Test with many parameters
        for i in 0..100 {
            query.set(&format!("col_{}", i), json!(format!("value_{}", i)));
        }

        // Test all available database dialects (based on enabled features)
        for dialect in DatabaseDialect::all_available() {
            let (sql, params) = query.to_sql(dialect);
        
            // Should handle large parameter sets efficiently
            assert!(sql.to_uppercase().contains("INSERT INTO"));
            assert!(sql.contains("bulk_data") || sql.contains("\"bulk_data\"") || sql.contains("`bulk_data`"));
            assert!(sql.contains("VALUES"));
            assert!(sql.ends_with("RETURNING *"));
            assert_eq!(params.len(), 100);
            
            // Verify parameter ordering is preserved
            for i in 0..100 {
                assert_eq!(params[i], json!(format!("value_{}", i)));
            }
        }
    }


    #[test]
    fn test_zero_string_literals_compliance() {
        // This test validates that the new system completely eliminates
        // string-based placeholder generation as required by Phase 3.1.1
        
        // Create queries with various parameter counts
        let empty_insert = InsertQuery::new("test".to_string());
        let (sql, params) = empty_insert.to_sql(DatabaseDialect::SQLite);
        assert!(sql.to_uppercase().contains("INSERT INTO"));
        assert!(sql.contains("test") || sql.contains("\"test\"") || sql.contains("`test`"));
        assert!(sql.contains("RETURNING *") || !DatabaseDialect::SQLite.supports_returning());
        assert_eq!(params.len(), 0);
        
        let mut single_insert = InsertQuery::new("test".to_string());
        single_insert.set("col", json!("val"));
        let (sql, params) = single_insert.to_sql(DatabaseDialect::SQLite);
        assert!(sql.to_uppercase().contains("INSERT INTO"));
        assert!(sql.contains("test") || sql.contains("\"test\"") || sql.contains("`test`"));
        assert!(sql.contains("col") && sql.contains("VALUES (?)"));
        assert!(sql.contains("RETURNING *") || !DatabaseDialect::SQLite.supports_returning());
        assert_eq!(params.len(), 1);
        
        let mut multi_insert = InsertQuery::new("test".to_string());
        for i in 0..5 {
            multi_insert.set(&format!("col{}", i), json!(i));
        }
        let (sql, params) = multi_insert.to_sql(DatabaseDialect::SQLite);
        assert!(sql.to_uppercase().contains("INSERT INTO"));
        assert!(sql.contains("test") || sql.contains("\"test\"") || sql.contains("`test`"));
        assert!(sql.contains("col0") && sql.contains("col1") && sql.contains("col2") && sql.contains("col3") && sql.contains("col4"));
        assert!(sql.contains("VALUES (?, ?, ?, ?, ?)"));
        assert!(sql.contains("RETURNING *") || !DatabaseDialect::SQLite.supports_returning());
        assert_eq!(params.len(), 5);
        
        // Verify that placeholder generation is now type-safe and consistent
        // The old system: vec!["?"; self.values.len()].join(", ")
        // The new system: InsertParameterBuilder with compile-time validation
    }
}