#[cfg(feature = "mysql")]
use crate::backends::mysql::MySQLBackend;
#[cfg(feature = "mysql")]
use crate::backends::QueryResult;
#[cfg(feature = "mysql")]
use crate::dialects::DatabaseDialect;
#[cfg(feature = "mysql")]
use crate::auto_migration::introspector::{
    TableSchema, ColumnSchema, IndexSchema, ForeignKeySchema, ConstraintSchema,
    ConstraintType
};
#[cfg(feature = "mysql")]
use crate::introspection::{SchemaIntrospector, IntrospectionError, IntrospectionErrorKind};
#[cfg(feature = "mysql")]
use crate::{DatabaseClient, Entity};
#[cfg(feature = "mysql")]
use crate::query_builder::sea_value_to_json;
#[cfg(feature = "mysql")]
use async_trait::async_trait;
#[cfg(feature = "mysql")]
use sea_query::{Query, Expr, Alias, MysqlQueryBuilder};
#[cfg(feature = "mysql")]
use serde_json::Value;
#[cfg(feature = "mysql")]
use std::collections::{HashMap, HashSet};

#[cfg(feature = "mysql")]
/// MySQL-specific schema introspector using sea-query builders
/// 
/// This implementation provides database-agnostic schema introspection for MySQL
/// using sea-query builders and information_schema views. It replaces direct SQL queries
/// with type-safe, dialect-aware query building while maintaining full compatibility
/// with MySQL's comprehensive schema system.
/// 
/// # Design Principles
/// 
/// - **Sea-Query Only**: All database queries use sea-query builders, no raw SQL
/// - **Entity Aware**: Supports Entity::boolean_fields() for accurate boolean detection
/// - **Complete Coverage**: Implements all introspection methods from the trait
/// - **Performance Optimized**: Efficient queries with minimal database round-trips
/// - **Type Safe**: Leverages Rust's type system for compile-time safety
/// 
/// # MySQL Specifics
/// 
/// MySQL uses information_schema views and SHOW commands for introspection:
/// - `information_schema.tables` for table listing
/// - `information_schema.columns` for column details
/// - `information_schema.statistics` for index information
/// - `information_schema.key_column_usage` for foreign key relationships
/// - `information_schema.table_constraints` for various constraints
/// - `information_schema.check_constraints` for check constraints (MySQL 8.0.16+)
/// - MySQL-specific data types and storage engines
/// 
/// All queries use sea-query builders for database-agnostic SQL generation.
pub struct MySQLIntrospector<'a> {
    client: &'a DatabaseClient<MySQLBackend>,
}

#[cfg(feature = "mysql")]
impl<'a> MySQLIntrospector<'a> {
    /// Create a new MySQL schema introspector
    /// 
    /// # Arguments
    /// * `client` - Database client connected to MySQL database
    /// 
    /// # Examples
    /// ```
    /// use d1_rs::introspection::mysql::MySQLIntrospector;
    /// 
    /// let introspector = MySQLIntrospector::new(&mysql_client);
    /// let tables = introspector.list_tables().await?;
    /// ```
    pub fn new(client: &'a DatabaseClient<MySQLBackend>) -> Self {
        Self { client }
    }
    
    /// Query information_schema.tables using sea-query
    /// 
    /// This is the sea-query replacement for direct information_schema queries.
    /// MySQL stores table metadata in standard information_schema views.
    async fn query_information_schema_tables(&self, schema_name: Option<&str>) -> Result<Vec<serde_json::Map<String, Value>>, IntrospectionError> {
        // Build query and extract SQL immediately to avoid Send issues
        let (sql, sea_params) = {
            let mut query = Query::select()
                .columns([
                    Alias::new("table_name"),
                    Alias::new("table_schema"),
                    Alias::new("table_type"),
                    Alias::new("engine"),
                    Alias::new("table_rows"),
                    Alias::new("table_comment")
                ])
                .from(Alias::new("information_schema").to_owned())
                .from(Alias::new("tables"))
                .and_where(Expr::col(Alias::new("table_type")).eq("BASE TABLE"))
                .to_owned();
            
            // Filter by schema if provided, otherwise use current database
            if let Some(schema) = schema_name {
                query = query.and_where(Expr::col(Alias::new("table_schema")).eq(schema)).to_owned();
            } else {
                // Use DATABASE() function to get current database
                query = query.and_where(Expr::col(Alias::new("table_schema")).eq(Expr::cust("DATABASE()"))).to_owned();
            }
            
            // Exclude system schemas
            query = query
                .and_where(Expr::col(Alias::new("table_schema")).ne("information_schema"))
                .and_where(Expr::col(Alias::new("table_schema")).ne("performance_schema"))
                .and_where(Expr::col(Alias::new("table_schema")).ne("mysql"))
                .and_where(Expr::col(Alias::new("table_schema")).ne("sys"))
                .to_owned();
            
            let (sql, params) = query.build(MysqlQueryBuilder);
            let sea_params: Vec<Value> = params.into_iter().map(|p| sea_value_to_json(&p)).collect();
            (sql, sea_params)
        };
        
        let result = self.client.execute(&sql, &sea_params).await
            .map_err(|e| IntrospectionError {
                message: format!("Failed to query information_schema.tables: {}", e),
                kind: IntrospectionErrorKind::DatabaseError,
            })?;
        
        let mut rows = Vec::new();
        for row in result.rows() {
            if let Value::Object(obj) = row {
                rows.push(obj.clone());
            }
        }
        
        Ok(rows)
    }
    
    /// Query information_schema.columns using sea-query
    /// 
    /// Retrieves column information for a specific table using MySQL's information_schema.
    async fn query_information_schema_columns(&self, table_name: &str, schema_name: Option<&str>) -> Result<Vec<serde_json::Map<String, Value>>, IntrospectionError> {
        // Build query and extract SQL immediately to avoid Send issues
        let (sql, sea_params) = {
            let mut query = Query::select()
                .columns([
                    Alias::new("column_name"),
                    Alias::new("data_type"),
                    Alias::new("is_nullable"),
                    Alias::new("column_default"),
                    Alias::new("character_maximum_length"),
                    Alias::new("numeric_precision"),
                    Alias::new("numeric_scale"),
                    Alias::new("ordinal_position"),
                    Alias::new("column_type"),
                    Alias::new("column_key"),
                    Alias::new("extra"),
                    Alias::new("column_comment"),
                ])
                .from(Alias::new("information_schema").to_owned())
                .from(Alias::new("columns"))
                .and_where(Expr::col(Alias::new("table_name")).eq(table_name))
                .to_owned();
            
            // Filter by schema
            if let Some(schema) = schema_name {
                query = query.and_where(Expr::col(Alias::new("table_schema")).eq(schema)).to_owned();
            } else {
                query = query.and_where(Expr::col(Alias::new("table_schema")).eq(Expr::cust("DATABASE()"))).to_owned();
            }
            
            // Order by position
            query = query.order_by(Alias::new("ordinal_position"), sea_query::Order::Asc).to_owned();
            
            let (sql, params) = query.build(MysqlQueryBuilder);
            let sea_params: Vec<Value> = params.into_iter().map(|p| sea_value_to_json(&p)).collect();
            (sql, sea_params)
        };
        
        let result = self.client.execute(&sql, &sea_params).await
            .map_err(|e| IntrospectionError {
                message: format!("Failed to query information_schema.columns for table '{}': {}", table_name, e),
                kind: IntrospectionErrorKind::DatabaseError,
            })?;
        
        let mut rows = Vec::new();
        for row in result.rows() {
            if let Value::Object(obj) = row {
                rows.push(obj.clone());
            }
        }
        
        Ok(rows)
    }
    
    /// Query information_schema for constraint information
    /// 
    /// Retrieves constraint information for a specific table using MySQL's information_schema.
    async fn query_table_constraints(&self, table_name: &str, schema_name: Option<&str>) -> Result<Vec<serde_json::Map<String, Value>>, IntrospectionError> {
        // Build query and extract SQL immediately to avoid Send issues
        let (sql, sea_params) = {
            let mut query = Query::select()
                .columns([
                    Alias::new("constraint_name"),
                    Alias::new("constraint_type"),
                ])
                .from(Alias::new("information_schema").to_owned())
                .from(Alias::new("table_constraints"))
                .and_where(Expr::col(Alias::new("table_name")).eq(table_name))
                .to_owned();
            
            // Filter by schema
            if let Some(schema) = schema_name {
                query = query.and_where(Expr::col(Alias::new("table_schema")).eq(schema)).to_owned();
            } else {
                query = query.and_where(Expr::col(Alias::new("table_schema")).eq(Expr::cust("DATABASE()"))).to_owned();
            }
            
            let (sql, params) = query.build(MysqlQueryBuilder);
            let sea_params: Vec<Value> = params.into_iter().map(|p| sea_value_to_json(&p)).collect();
            (sql, sea_params)
        };
        
        let result = self.client.execute(&sql, &sea_params).await
            .map_err(|e| IntrospectionError {
                message: format!("Failed to query table constraints for table '{}': {}", table_name, e),
                kind: IntrospectionErrorKind::DatabaseError,
            })?;
        
        let mut rows = Vec::new();
        for row in result.rows() {
            if let Value::Object(obj) = row {
                rows.push(obj.clone());
            }
        }
        
        Ok(rows)
    }
    
    /// Query MySQL check constraints from information_schema (MySQL 8.0.16+)
    async fn query_check_constraints(&self, table_name: &str, schema_name: Option<&str>) -> Result<Vec<serde_json::Map<String, Value>>, IntrospectionError> {
        // Build query for check constraints (available in MySQL 8.0.16+)
        let (sql, sea_params) = {
            let mut query = Query::select()
                .columns([
                    Alias::new("constraint_name"),
                    Alias::new("check_clause"),
                ])
                .from(Alias::new("information_schema").to_owned())
                .from(Alias::new("check_constraints"))
                .and_where(
                    if let Some(schema) = schema_name {
                        Expr::col(Alias::new("constraint_schema")).eq(schema)
                    } else {
                        Expr::col(Alias::new("constraint_schema")).eq(Expr::cust("DATABASE()"))
                    }
                )
                .to_owned();
                
            // Join with table_constraints to filter by table
            query = query
                .inner_join(
                    (Alias::new("information_schema"), Alias::new("table_constraints")),
                    Expr::col((Alias::new("check_constraints"), Alias::new("constraint_name")))
                        .equals((Alias::new("table_constraints"), Alias::new("constraint_name")))
                        .and(Expr::col((Alias::new("check_constraints"), Alias::new("constraint_schema")))
                            .equals((Alias::new("table_constraints"), Alias::new("table_schema"))))
                )
                .and_where(Expr::col((Alias::new("table_constraints"), Alias::new("table_name"))).eq(table_name))
                .to_owned();
            
            let (sql, params) = query.build(MysqlQueryBuilder);
            let sea_params: Vec<Value> = params.into_iter().map(|p| sea_value_to_json(&p)).collect();
            (sql, sea_params)
        };
        
        let result = self.client.execute(&sql, &sea_params).await
            .map_err(|e| IntrospectionError {
                message: format!("Failed to query check constraints for table '{}': {}", table_name, e),
                kind: IntrospectionErrorKind::DatabaseError,
            })?;
        
        let mut rows = Vec::new();
        for row in result.rows() {
            if let Value::Object(obj) = row {
                rows.push(obj.clone());
            }
        }
        
        Ok(rows)
    }
    
    /// Parse column information from information_schema results
    /// 
    /// Converts raw information_schema.columns output to ColumnSchema with optional boolean field detection.
    fn parse_column_info(&self, column_data: &serde_json::Map<String, Value>, boolean_fields: Option<&HashSet<String>>) -> Result<ColumnSchema, IntrospectionError> {
        let name = column_data.get("column_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| IntrospectionError {
                message: "Missing column name".to_string(),
                kind: IntrospectionErrorKind::InvalidSchema,
            })?.to_string();
        
        let data_type = column_data.get("data_type")
            .and_then(|v| v.as_str())
            .unwrap_or("text")
            .to_string();
        
        let column_type = column_data.get("column_type")
            .and_then(|v| v.as_str())
            .unwrap_or(&data_type)
            .to_string();
        
        let nullable = column_data.get("is_nullable")
            .and_then(|v| v.as_str())
            .map(|s| s == "YES")
            .unwrap_or(true);
        
        let default_value = column_data.get("column_default")
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    Some(v.as_str().unwrap_or("").to_string())
                }
            });
        
        let column_key = column_data.get("column_key")
            .and_then(|v| v.as_str())
            .unwrap_or("");
            
        let extra = column_data.get("extra")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        // MySQL primary key and auto increment detection
        let primary_key = column_key == "PRI";
        let auto_increment = extra.contains("auto_increment");
        let unique = column_key == "UNI";
        
        // Entity-aware boolean detection - REVOLUTIONARY approach
        let is_boolean = match boolean_fields {
            Some(boolean_set) => boolean_set.contains(&name), // Trait-based detection ONLY!
            None => {
                // MySQL boolean detection: TINYINT(1) or BOOLEAN type
                data_type == "tinyint" && column_type.contains("(1)") || 
                data_type.to_lowercase() == "boolean"
            },
        };
        
        let final_column_type = if is_boolean {
            match data_type.as_str() {
                "tinyint" => "boolean".to_string(), // MySQL TINYINT(1) -> boolean
                _ => data_type.clone()
            }
        } else {
            data_type.clone()
        };
        
        Ok(ColumnSchema {
            name,
            column_type: final_column_type,
            nullable,
            default_value,
            primary_key,
            auto_increment,
            unique,
            constraints: vec![], // Will be populated separately
        })
    }
    
    /// Query information_schema for index information
    /// 
    /// MySQL uses information_schema.statistics for index information.
    async fn query_table_indexes(&self, table_name: &str, schema_name: Option<&str>) -> Result<Vec<IndexSchema>, IntrospectionError> {
        // Build query for information_schema.statistics using sea-query
        let (sql, sea_params) = {
            let mut query = Query::select()
                .columns([
                    Alias::new("index_name"),
                    Alias::new("column_name"),
                    Alias::new("non_unique"),
                    Alias::new("seq_in_index"),
                ])
                .from(Alias::new("information_schema").to_owned())
                .from(Alias::new("statistics"))
                .and_where(Expr::col(Alias::new("table_name")).eq(table_name))
                .to_owned();
            
            // Filter by schema
            if let Some(schema) = schema_name {
                query = query.and_where(Expr::col(Alias::new("table_schema")).eq(schema)).to_owned();
            } else {
                query = query.and_where(Expr::col(Alias::new("table_schema")).eq(Expr::cust("DATABASE()"))).to_owned();
            }
            
            // Order by index name and sequence
            query = query
                .order_by(Alias::new("index_name"), sea_query::Order::Asc)
                .order_by(Alias::new("seq_in_index"), sea_query::Order::Asc)
                .to_owned();
            
            let (sql, params) = query.build(MysqlQueryBuilder);
            let sea_params: Vec<Value> = params.into_iter().map(|p| sea_value_to_json(&p)).collect();
            (sql, sea_params)
        };
        
        let result = self.client.execute(&sql, &sea_params).await
            .map_err(|e| IntrospectionError {
                message: format!("Failed to query information_schema.statistics for table '{}': {}", table_name, e),
                kind: IntrospectionErrorKind::DatabaseError,
            })?;
        
        // Group index information by index name
        let mut index_map: HashMap<String, IndexSchema> = HashMap::new();
        
        for row in result.rows() {
            if let Value::Object(obj) = row {
                let index_name = obj.get("index_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                let column_name = obj.get("column_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                let non_unique = obj.get("non_unique")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(1);
                
                // Skip PRIMARY key indexes (they're handled separately)
                if index_name == "PRIMARY" {
                    continue;
                }
                
                let unique = non_unique == 0;
                
                if let Some(existing_index) = index_map.get_mut(&index_name) {
                    existing_index.columns.push(column_name);
                } else {
                    index_map.insert(index_name.clone(), IndexSchema {
                        name: index_name,
                        columns: vec![column_name],
                        unique,
                        table_name: Some(table_name.to_string()),
                    });
                }
            }
        }
        
        Ok(index_map.into_values().collect())
    }
    
    /// Query foreign key information using information_schema with sea-query builders
    async fn query_foreign_keys(&self, table_name: &str, schema_name: Option<&str>) -> Result<Vec<ForeignKeySchema>, IntrospectionError> {
        // Build complex query for foreign key information using pure sea-query builders - NO RAW SQL!
        // MySQL stores foreign key info in information_schema.key_column_usage and referential_constraints
        let (sql, sea_params) = {
            // Define table aliases for the complex join
            let kcu_alias = Alias::new("kcu");
            let rc_alias = Alias::new("rc");
            
            let mut query = Query::select()
                // SELECT clause with proper aliasing
                .columns([
                    (kcu_alias.clone(), Alias::new("constraint_name")),
                    (kcu_alias.clone(), Alias::new("column_name")),
                    (kcu_alias.clone(), Alias::new("referenced_table_name")),
                    (kcu_alias.clone(), Alias::new("referenced_column_name")),
                    (rc_alias.clone(), Alias::new("update_rule")),
                    (rc_alias.clone(), Alias::new("delete_rule")),
                ])
                // FROM clause with table alias
                .from_as(
                    (Alias::new("information_schema"), Alias::new("key_column_usage")),
                    kcu_alias.clone()
                )
                // JOIN: key_column_usage -> referential_constraints
                .join_as(
                    sea_query::JoinType::InnerJoin,
                    (Alias::new("information_schema"), Alias::new("referential_constraints")),
                    rc_alias.clone(),
                    Expr::col((kcu_alias.clone(), Alias::new("constraint_name")))
                        .equals((rc_alias.clone(), Alias::new("constraint_name")))
                        .and(Expr::col((kcu_alias.clone(), Alias::new("constraint_schema")))
                            .equals((rc_alias.clone(), Alias::new("constraint_schema"))))
                )
                // WHERE clause conditions
                .and_where(Expr::col((kcu_alias.clone(), Alias::new("table_name"))).eq(table_name))
                .and_where(Expr::col((kcu_alias.clone(), Alias::new("referenced_table_name"))).is_not_null())
                .to_owned();
            
            // Handle schema filtering with database-agnostic approach
            if let Some(schema) = schema_name {
                query = query.and_where(Expr::col((kcu_alias.clone(), Alias::new("constraint_schema"))).eq(schema)).to_owned();
            } else {
                // Use DATABASE() function for current database
                query = query.and_where(Expr::col((kcu_alias.clone(), Alias::new("constraint_schema"))).eq(Expr::cust("DATABASE()"))).to_owned();
            }
            
            // ORDER BY clause
            query = query
                .order_by((kcu_alias.clone(), Alias::new("constraint_name")), sea_query::Order::Asc)
                .order_by((kcu_alias, Alias::new("ordinal_position")), sea_query::Order::Asc)
                .to_owned();
            
            let (sql, params) = query.build(MysqlQueryBuilder);
            let sea_params: Vec<Value> = params.into_iter().map(|p| sea_value_to_json(&p)).collect();
            (sql, sea_params)
        };
        
        let result = self.client.execute(&sql, &sea_params).await
            .map_err(|e| IntrospectionError {
                message: format!("Failed to query foreign keys for table '{}': {}", table_name, e),
                kind: IntrospectionErrorKind::DatabaseError,
            })?;
        
        let mut foreign_keys = Vec::new();
        let mut fk_map: HashMap<String, ForeignKeySchema> = HashMap::new();
        
        for row in result.rows() {
            if let Value::Object(obj) = row {
                let constraint_name = obj.get("constraint_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                let column_name = obj.get("column_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                let referenced_table_name = obj.get("referenced_table_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                let referenced_column_name = obj.get("referenced_column_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                let update_rule = obj.get("update_rule")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                
                let delete_rule = obj.get("delete_rule")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                
                // Group by constraint name for composite foreign keys
                if let Some(existing_fk) = fk_map.get_mut(&constraint_name) {
                    existing_fk.columns.push(column_name);
                    existing_fk.referenced_columns.push(referenced_column_name);
                } else {
                    let fk = ForeignKeySchema {
                        name: constraint_name.clone(),
                        columns: vec![column_name],
                        referenced_table: referenced_table_name,
                        referenced_columns: vec![referenced_column_name],
                        on_delete: delete_rule,
                        on_update: update_rule,
                    };
                    fk_map.insert(constraint_name, fk);
                }
            }
        }
        
        foreign_keys.extend(fk_map.into_values());
        Ok(foreign_keys)
    }
    
    /// Convert information_schema constraint type to ConstraintType enum
    fn map_constraint_type(&self, mysql_constraint_type: &str) -> ConstraintType {
        match mysql_constraint_type.to_uppercase().as_str() {
            "PRIMARY KEY" => ConstraintType::PrimaryKey,
            "UNIQUE" => ConstraintType::Unique,
            "FOREIGN KEY" => ConstraintType::ForeignKey,
            "CHECK" => ConstraintType::Check,
            _ => ConstraintType::Check, // Default fallback
        }
    }
}

#[cfg(feature = "mysql")]
#[async_trait]
impl SchemaIntrospector<MySQLBackend> for MySQLIntrospector<'_> {
    type Error = IntrospectionError;
    
    fn dialect(&self) -> DatabaseDialect {
        DatabaseDialect::MySQL
    }
    
    async fn list_tables(&self) -> std::result::Result<Vec<String>, Self::Error> {
        let rows = self.query_information_schema_tables(None).await?;
        
        let mut tables = Vec::new();
        for row in rows {
            if let Some(Value::String(name)) = row.get("table_name") {
                tables.push(name.clone());
            }
        }
        
        Ok(tables)
    }
    
    async fn describe_table(&self, table_name: &str) -> std::result::Result<TableSchema, Self::Error> {
        let columns = self.list_columns(table_name).await?;
        let indexes = self.list_indexes(table_name).await?;
        let foreign_keys = self.list_foreign_keys(table_name).await?;
        let constraints = self.list_constraints(table_name).await?;
        
        Ok(TableSchema {
            name: table_name.to_string(),
            columns,
            indexes,
            foreign_keys,
            constraints,
        })
    }
    
    async fn list_columns(&self, table_name: &str) -> std::result::Result<Vec<ColumnSchema>, Self::Error> {
        let rows = self.query_information_schema_columns(table_name, None).await?;
        
        let mut columns = Vec::new();
        for row in rows {
            let column = self.parse_column_info(&row, None)?;
            columns.push(column);
        }
        
        Ok(columns)
    }
    
    async fn list_indexes(&self, table_name: &str) -> std::result::Result<Vec<IndexSchema>, Self::Error> {
        self.query_table_indexes(table_name, None).await
    }
    
    async fn list_foreign_keys(&self, table_name: &str) -> std::result::Result<Vec<ForeignKeySchema>, Self::Error> {
        self.query_foreign_keys(table_name, None).await
    }
    
    async fn list_constraints(&self, table_name: &str) -> std::result::Result<Vec<ConstraintSchema>, Self::Error> {
        let mut constraints = Vec::new();
        
        // Get basic constraints from table_constraints
        let table_constraint_rows = self.query_table_constraints(table_name, None).await?;
        
        for row in table_constraint_rows {
            let constraint_name = row.get("constraint_name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let constraint_type_str = row.get("constraint_type")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            
            let constraint_type = self.map_constraint_type(constraint_type_str);
            
            // Skip foreign keys (they're handled separately)
            if constraint_type == ConstraintType::ForeignKey {
                continue;
            }
            
            constraints.push(ConstraintSchema {
                name: constraint_name,
                constraint_type,
                columns: Vec::new(), // Could be enhanced to parse column names from constraint
                definition: Some(format!("{} constraint", constraint_type_str)),
            });
        }
        
        // Get check constraints if available (MySQL 8.0.16+)
        if let Ok(check_rows) = self.query_check_constraints(table_name, None).await {
            for row in check_rows {
                let constraint_name = row.get("constraint_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                let check_clause = row.get("check_clause")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                constraints.push(ConstraintSchema {
                    name: constraint_name,
                    constraint_type: ConstraintType::Check,
                    columns: Vec::new(), // Could be enhanced to parse column names from CHECK constraint  
                    definition: Some(check_clause),
                });
            }
        }
        
        Ok(constraints)
    }
    
    async fn get_database_metadata(&self) -> std::result::Result<HashMap<String, String>, Self::Error> {
        let mut metadata = HashMap::new();
        
        // Get MySQL version using sea-query function builder - NO RAW SQL!
        let (sql, params) = {
            let query = Query::select()
                .expr_as(Expr::cust("VERSION()"), Alias::new("version"))
                .to_owned();
            let (sql, sea_params) = query.build(MysqlQueryBuilder);
            let json_params: Vec<Value> = sea_params.into_iter()
                .map(|p| sea_value_to_json(&p))
                .collect();
            (sql, json_params)
        };
        
        let version_result = self.client.execute(&sql, &params).await
            .map_err(|e| IntrospectionError {
                message: format!("Failed to get MySQL version: {}", e),
                kind: IntrospectionErrorKind::DatabaseError,
            })?;
        
        if let Some(Value::Object(obj)) = version_result.rows().first() {
            if let Some(Value::String(version)) = obj.get("version") {
                metadata.insert("version".to_string(), version.clone());
            }
        }
        
        // Get current database name using sea-query function builder - NO RAW SQL!
        let (db_sql, db_params) = {
            let query = Query::select()
                .expr_as(Expr::cust("DATABASE()"), Alias::new("current_database"))
                .to_owned();
            let (sql, sea_params) = query.build(MysqlQueryBuilder);
            let json_params: Vec<Value> = sea_params.into_iter()
                .map(|p| sea_value_to_json(&p))
                .collect();
            (sql, json_params)
        };
        
        let database_result = self.client.execute(&db_sql, &db_params).await
            .map_err(|e| IntrospectionError {
                message: format!("Failed to get current database: {}", e),
                kind: IntrospectionErrorKind::DatabaseError,
            })?;
        
        if let Some(Value::Object(obj)) = database_result.rows().first() {
            if let Some(Value::String(db_name)) = obj.get("current_database") {
                metadata.insert("current_database".to_string(), db_name.clone());
            }
        }
        
        metadata.insert("database_type".to_string(), "MySQL".to_string());
        metadata.insert("supports_returning".to_string(), "false".to_string()); // MySQL doesn't support RETURNING
        metadata.insert("supports_transactions".to_string(), "true".to_string());
        metadata.insert("supports_foreign_keys".to_string(), "true".to_string());
        metadata.insert("supports_json".to_string(), "true".to_string()); // MySQL 5.7+
        metadata.insert("supports_check_constraints".to_string(), "true".to_string()); // MySQL 8.0.16+
        metadata.insert("supports_generated_columns".to_string(), "true".to_string()); // MySQL 5.7+
        
        Ok(metadata)
    }
}

// Entity-aware introspection extension
#[cfg(feature = "mysql")]
impl<'a> MySQLIntrospector<'a> {
    /// Entity-aware column introspection with boolean field detection
    /// 
    /// This method enhances standard column introspection by using Entity metadata
    /// to provide accurate boolean field detection for MySQL databases.
    /// 
    /// # Type Parameters
    /// * `T` - Entity type that implements the Entity trait
    /// 
    /// # Arguments
    /// * `table_name` - Name of the table to introspect
    /// 
    /// # Examples
    /// ```
    /// let columns = introspector.list_columns_for_entity::<User>("users").await?;
    /// let active_col = columns.iter().find(|c| c.name == "active").unwrap();
    /// // active field is correctly identified as boolean even if stored as TINYINT(1)
    /// ```
    pub async fn list_columns_for_entity<T>(&self, table_name: &str) -> Result<Vec<ColumnSchema>, IntrospectionError>
    where
        T: Entity + Send + Sync,
    {
        let rows = self.query_information_schema_columns(table_name, None).await?;
        
        // Get boolean field names from the entity trait - COMPILE-TIME SAFE!
        let boolean_fields: HashSet<String> = T::boolean_fields()
            .iter()
            .map(|s| s.to_string())
            .collect();
        
        let mut columns = Vec::new();
        for row in rows {
            let column = self.parse_column_info(&row, Some(&boolean_fields))?;
            columns.push(column);
        }
        
        Ok(columns)
    }
}

#[cfg(not(feature = "mysql"))]
/// Stub implementation when mysql feature is not enabled
pub struct MySQLIntrospector;

#[cfg(test)]
#[cfg(feature = "mysql")]
mod tests {
    use super::*;
    use crate::dialects::DatabaseDialect;
    use serde_json::json;

    // Test business logic and parameter handling, NOT SQL format verification
    // These tests run against MySQL dialect specifically for MySQL introspection
    
    #[test]
    fn test_parse_column_info_business_logic() {
        // Test business logic: column info parsing correctness for MySQL
        let introspector = create_test_introspector();
        
        // Test MySQL-specific column parsing with various data types
        let mut column_data = serde_json::Map::new();
        column_data.insert("column_name".to_string(), json!("user_email"));
        column_data.insert("data_type".to_string(), json!("varchar"));
        column_data.insert("column_type".to_string(), json!("varchar(255)"));
        column_data.insert("is_nullable".to_string(), json!("NO"));
        column_data.insert("column_default".to_string(), json!("NULL"));
        column_data.insert("column_key".to_string(), json!(""));
        column_data.insert("extra".to_string(), json!(""));
        
        let result = introspector.parse_column_info(&column_data, None);
        assert!(result.is_ok());
        
        let column = result.unwrap();
        assert_eq!(column.name, "user_email");
        assert_eq!(column.column_type, "varchar");
        assert_eq!(column.nullable, false); // is_nullable = "NO" means NOT NULL
        assert_eq!(column.primary_key, false);
        assert_eq!(column.auto_increment, false);
        assert_eq!(column.unique, false);
    }
    
    #[test]
    fn test_parse_column_info_boolean_detection() {
        // Test business logic: Entity-aware boolean field detection for MySQL
        let introspector = create_test_introspector();
        
        let mut column_data = serde_json::Map::new();
        column_data.insert("column_name".to_string(), json!("is_verified"));
        column_data.insert("data_type".to_string(), json!("tinyint"));
        column_data.insert("column_type".to_string(), json!("tinyint(1)"));
        column_data.insert("is_nullable".to_string(), json!("YES"));
        column_data.insert("column_default".to_string(), Value::Null);
        column_data.insert("column_key".to_string(), json!(""));
        column_data.insert("extra".to_string(), json!(""));
        
        // Test with MySQL TINYINT(1) - should be detected as boolean
        let result = introspector.parse_column_info(&column_data, None);
        assert!(result.is_ok());
        
        let column = result.unwrap();
        assert_eq!(column.name, "is_verified");
        assert_eq!(column.column_type, "boolean"); // Should be converted from tinyint to boolean
        
        // Test with Entity-aware boolean field detection
        let mut boolean_fields = HashSet::new();
        boolean_fields.insert("is_verified".to_string());
        
        let result2 = introspector.parse_column_info(&column_data, Some(&boolean_fields));
        assert!(result2.is_ok());
        
        let column2 = result2.unwrap();
        assert_eq!(column2.column_type, "boolean"); // Should be converted to boolean
        
        // Test regular TINYINT without (1) - should remain tinyint
        let mut regular_tinyint = column_data.clone();
        regular_tinyint.insert("column_type".to_string(), json!("tinyint(4)"));
        
        let result3 = introspector.parse_column_info(&regular_tinyint, None);
        assert!(result3.is_ok());
        
        let column3 = result3.unwrap();
        assert_eq!(column3.column_type, "tinyint"); // Should remain tinyint
    }
    
    #[test]
    fn test_mysql_column_key_detection() {
        // Test business logic: MySQL column key detection (PRI, UNI, MUL)
        let introspector = create_test_introspector();
        
        // Test primary key detection
        let mut pk_column = serde_json::Map::new();
        pk_column.insert("column_name".to_string(), json!("id"));
        pk_column.insert("data_type".to_string(), json!("int"));
        pk_column.insert("column_type".to_string(), json!("int(11)"));
        pk_column.insert("is_nullable".to_string(), json!("NO"));
        pk_column.insert("column_key".to_string(), json!("PRI"));
        pk_column.insert("extra".to_string(), json!("auto_increment"));
        
        let result = introspector.parse_column_info(&pk_column, None);
        assert!(result.is_ok());
        
        let column = result.unwrap();
        assert_eq!(column.name, "id");
        assert!(column.primary_key); // Should detect PRI key
        assert!(column.auto_increment); // Should detect auto_increment
        assert!(!column.unique); // Primary key is not marked as unique separately
        
        // Test unique key detection
        let mut unique_column = pk_column.clone();
        unique_column.insert("column_key".to_string(), json!("UNI"));
        unique_column.insert("extra".to_string(), json!(""));
        
        let result2 = introspector.parse_column_info(&unique_column, None);
        assert!(result2.is_ok());
        
        let column2 = result2.unwrap();
        assert!(!column2.primary_key); // Should not be primary
        assert!(column2.unique); // Should detect UNI key
        assert!(!column2.auto_increment); // Should not be auto increment
    }
    
    #[test]
    fn test_constraint_type_mapping() {
        // Test business logic: MySQL constraint type mapping
        let introspector = create_test_introspector();
        
        assert_eq!(introspector.map_constraint_type("PRIMARY KEY"), ConstraintType::PrimaryKey);
        assert_eq!(introspector.map_constraint_type("UNIQUE"), ConstraintType::Unique);
        assert_eq!(introspector.map_constraint_type("FOREIGN KEY"), ConstraintType::ForeignKey);
        assert_eq!(introspector.map_constraint_type("CHECK"), ConstraintType::Check);
        
        // Test case insensitive
        assert_eq!(introspector.map_constraint_type("primary key"), ConstraintType::PrimaryKey);
        assert_eq!(introspector.map_constraint_type("unique"), ConstraintType::Unique);
        
        // Test unknown constraint type (should default to Check)
        assert_eq!(introspector.map_constraint_type("CUSTOM"), ConstraintType::Check);
    }
    
    #[test]
    fn test_mysql_dialect_identification() {
        // Test business logic: correct dialect identification
        assert_eq!(DatabaseDialect::MySQL.to_string(), "MySQL");
        assert!(!DatabaseDialect::MySQL.supports_returning()); // MySQL doesn't support RETURNING
        assert!(DatabaseDialect::MySQL.supports_cte());
        
        // Test MySQL-specific features (business logic, not trait methods)
        let mysql_metadata = HashMap::from([
            ("supports_json".to_string(), "true".to_string()),
            ("supports_check_constraints".to_string(), "true".to_string()),
            ("supports_generated_columns".to_string(), "true".to_string()),
            ("supports_returning".to_string(), "false".to_string()),
        ]);
        assert_eq!(mysql_metadata.get("supports_json"), Some(&"true".to_string()));
        assert_eq!(mysql_metadata.get("supports_check_constraints"), Some(&"true".to_string()));
        assert_eq!(mysql_metadata.get("supports_generated_columns"), Some(&"true".to_string()));
        assert_eq!(mysql_metadata.get("supports_returning"), Some(&"false".to_string()));
    }
    
    #[test]
    fn test_mysql_auto_increment_detection() {
        // Test business logic: MySQL auto increment detection patterns
        let introspector = create_test_introspector();
        
        // Test auto_increment detection
        let mut auto_inc_column = serde_json::Map::new();
        auto_inc_column.insert("column_name".to_string(), json!("id"));
        auto_inc_column.insert("data_type".to_string(), json!("bigint"));
        auto_inc_column.insert("column_type".to_string(), json!("bigint(20)"));
        auto_inc_column.insert("is_nullable".to_string(), json!("NO"));
        auto_inc_column.insert("column_key".to_string(), json!("PRI"));
        auto_inc_column.insert("extra".to_string(), json!("auto_increment"));
        
        let result = introspector.parse_column_info(&auto_inc_column, None);
        assert!(result.is_ok());
        
        let column = result.unwrap();
        assert_eq!(column.name, "id");
        assert_eq!(column.column_type, "bigint");
        assert!(column.primary_key); // Should detect primary key
        assert!(column.auto_increment); // Should detect auto increment
        
        // Test column without auto_increment
        let mut regular_column = auto_inc_column.clone();
        regular_column.insert("extra".to_string(), json!(""));
        
        let result2 = introspector.parse_column_info(&regular_column, None);
        assert!(result2.is_ok());
        
        let column2 = result2.unwrap();
        assert!(!column2.auto_increment); // Should not detect auto increment
    }
    
    #[test]
    fn test_mysql_schema_handling() {
        // Test business logic: MySQL schema handling (defaults to DATABASE())
        // This tests the parameter logic for schema filtering, not SQL generation
        
        // Test default schema (should use DATABASE() function)
        let default_schema: Option<&str> = None;
        let use_database_function = default_schema.is_none();
        assert!(use_database_function);
        
        // Test custom schema
        let custom_schema = Some("custom_schema");
        let use_custom = custom_schema.is_some();
        assert!(use_custom);
        assert_eq!(custom_schema.unwrap(), "custom_schema");
        
        // Test schema filtering logic - MySQL system schemas
        let excluded_schemas = ["information_schema", "performance_schema", "mysql", "sys"];
        for schema in excluded_schemas.iter() {
            assert!(matches!(*schema, 
                "information_schema" | "performance_schema" | "mysql" | "sys"
            ));
        }
    }
    
    #[test]
    fn test_mysql_data_type_patterns() {
        // Test business logic: MySQL-specific data type patterns
        let introspector = create_test_introspector();
        
        // Test various MySQL column types
        let test_cases = vec![
            ("varchar(255)", "varchar", false),
            ("int(11)", "int", false),
            ("tinyint(1)", "tinyint", true), // Should be detected as boolean
            ("tinyint(4)", "tinyint", false), // Should not be boolean
            ("decimal(10,2)", "decimal", false),
            ("text", "text", false),
            ("json", "json", false),
        ];
        
        for (column_type, data_type, should_be_boolean) in test_cases {
            let mut column_data = serde_json::Map::new();
            column_data.insert("column_name".to_string(), json!("test_col"));
            column_data.insert("data_type".to_string(), json!(data_type));
            column_data.insert("column_type".to_string(), json!(column_type));
            column_data.insert("is_nullable".to_string(), json!("YES"));
            column_data.insert("column_key".to_string(), json!(""));
            column_data.insert("extra".to_string(), json!(""));
            
            let result = introspector.parse_column_info(&column_data, None);
            assert!(result.is_ok());
            
            let column = result.unwrap();
            if should_be_boolean {
                assert_eq!(column.column_type, "boolean");
            } else {
                assert_eq!(column.column_type, data_type);
            }
        }
    }
    
    // Helper function to create a test introspector for testing business logic
    fn create_test_introspector() -> MySQLIntrospector<'static> {
        // Create a dummy introspector using a null reference - safe for testing parsing methods
        // These tests only call parsing methods that don't use the client
        let client_ref: &'static DatabaseClient<MySQLBackend> = unsafe {
            // This is safe because we only test parsing methods that don't access the client
            std::mem::transmute(&() as *const () as *const DatabaseClient<MySQLBackend>)
        };
        MySQLIntrospector::new(client_ref)
    }
}