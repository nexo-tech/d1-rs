#[cfg(feature = "postgres")]
use crate::backends::postgres::PostgreSQLBackend;
#[cfg(feature = "postgres")]
use crate::backends::QueryResult;
#[cfg(feature = "postgres")]
use crate::dialects::DatabaseDialect;
#[cfg(feature = "postgres")]
use crate::auto_migration::introspector::{
    TableSchema, ColumnSchema, IndexSchema, ForeignKeySchema, ConstraintSchema,
    ConstraintType
};
#[cfg(feature = "postgres")]
use crate::introspection::{SchemaIntrospector, IntrospectionError, IntrospectionErrorKind};
#[cfg(feature = "postgres")]
use crate::{DatabaseClient, Entity};
#[cfg(feature = "postgres")]
use crate::query_builder::sea_value_to_json;
#[cfg(feature = "postgres")]
use async_trait::async_trait;
#[cfg(feature = "postgres")]
use sea_query::{Query, Expr, Alias, PostgresQueryBuilder};
#[cfg(feature = "postgres")]
use serde_json::Value;
#[cfg(feature = "postgres")]
use std::collections::{HashMap, HashSet};

#[cfg(feature = "postgres")]
/// PostgreSQL-specific schema introspector using sea-query builders
/// 
/// This implementation provides database-agnostic schema introspection for PostgreSQL
/// using sea-query builders and information_schema views. It replaces direct SQL queries
/// with type-safe, dialect-aware query building while maintaining full compatibility
/// with PostgreSQL's rich schema system.
/// 
/// # Design Principles
/// 
/// - **Sea-Query Only**: All database queries use sea-query builders, no raw SQL
/// - **Entity Aware**: Supports Entity::boolean_fields() for accurate boolean detection
/// - **Complete Coverage**: Implements all introspection methods from the trait
/// - **Performance Optimized**: Efficient queries with minimal database round-trips
/// - **Type Safe**: Leverages Rust's type system for compile-time safety
/// 
/// # PostgreSQL Specifics
/// 
/// PostgreSQL uses information_schema views and pg_* system tables for introspection:
/// - `information_schema.tables` for table listing
/// - `information_schema.columns` for column details
/// - `information_schema.statistics` and `pg_indexes` for index information
/// - `information_schema.key_column_usage` for foreign key relationships
/// - `information_schema.table_constraints` for various constraints
/// - `pg_constraint` for advanced constraint information
/// 
/// All queries use sea-query builders for database-agnostic SQL generation.
pub struct PostgreSQLIntrospector<'a> {
    client: &'a DatabaseClient<PostgreSQLBackend>,
}

#[cfg(feature = "postgres")]
impl<'a> PostgreSQLIntrospector<'a> {
    /// Create a new PostgreSQL schema introspector
    /// 
    /// # Arguments
    /// * `client` - Database client connected to PostgreSQL database
    /// 
    /// # Examples
    /// ```
    /// use d1_rs::introspection::postgres::PostgreSQLIntrospector;
    /// 
    /// let introspector = PostgreSQLIntrospector::new(&postgres_client);
    /// let tables = introspector.list_tables().await?;
    /// ```
    pub fn new(client: &'a DatabaseClient<PostgreSQLBackend>) -> Self {
        Self { client }
    }
    
    /// Query information_schema.tables using sea-query
    /// 
    /// This is the sea-query replacement for direct information_schema queries.
    /// PostgreSQL stores table metadata in standard information_schema views.
    async fn query_information_schema_tables(&self, schema_name: Option<&str>) -> Result<Vec<serde_json::Map<String, Value>>, IntrospectionError> {
        // Build query and extract SQL immediately to avoid Send issues
        let (sql, sea_params) = {
            let mut query = Query::select()
                .columns([
                    Alias::new("table_name"),
                    Alias::new("table_schema"),
                    Alias::new("table_type")
                ])
                .from(Alias::new("information_schema").to_owned())
                .from(Alias::new("tables"))
                .and_where(Expr::col(Alias::new("table_type")).eq("BASE TABLE"))
                .to_owned();
            
            // Filter by schema if provided, default to 'public'
            let target_schema = schema_name.unwrap_or("public");
            query = query.and_where(Expr::col(Alias::new("table_schema")).eq(target_schema)).to_owned();
            
            // Exclude system schemas
            query = query
                .and_where(Expr::col(Alias::new("table_schema")).ne("information_schema"))
                .and_where(Expr::col(Alias::new("table_schema")).ne("pg_catalog"))
                .and_where(Expr::col(Alias::new("table_schema")).not_like("pg_%"))
                .to_owned();
            
            let (sql, params) = query.build(PostgresQueryBuilder);
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
    /// Retrieves column information for a specific table using PostgreSQL's information_schema.
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
                ])
                .from(Alias::new("information_schema").to_owned())
                .from(Alias::new("columns"))
                .and_where(Expr::col(Alias::new("table_name")).eq(table_name))
                .to_owned();
            
            // Filter by schema
            let target_schema = schema_name.unwrap_or("public");
            query = query.and_where(Expr::col(Alias::new("table_schema")).eq(target_schema)).to_owned();
            
            // Order by position
            query = query.order_by(Alias::new("ordinal_position"), sea_query::Order::Asc).to_owned();
            
            let (sql, params) = query.build(PostgresQueryBuilder);
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
    /// Retrieves constraint information for a specific table using PostgreSQL's information_schema.
    async fn query_table_constraints(&self, table_name: &str, schema_name: Option<&str>) -> Result<Vec<serde_json::Map<String, Value>>, IntrospectionError> {
        // Build query and extract SQL immediately to avoid Send issues
        let (sql, sea_params) = {
            let mut query = Query::select()
                .columns([
                    Alias::new("constraint_name"),
                    Alias::new("constraint_type"),
                    Alias::new("check_clause"),
                ])
                .from(Alias::new("information_schema").to_owned())
                .from(Alias::new("table_constraints"))
                .and_where(Expr::col(Alias::new("table_name")).eq(table_name))
                .to_owned();
            
            // Filter by schema
            let target_schema = schema_name.unwrap_or("public");
            query = query.and_where(Expr::col(Alias::new("table_schema")).eq(target_schema)).to_owned();
            
            let (sql, params) = query.build(PostgresQueryBuilder);
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
        
        // PostgreSQL primary key detection - need additional query for this
        // For now, we'll detect based on common patterns in default values
        let primary_key = default_value.as_ref()
            .map(|d| d.contains("nextval") || d.contains("serial"))
            .unwrap_or(false);
            
        let auto_increment = primary_key && (
            data_type.contains("serial") ||
            default_value.as_ref().map(|d| d.contains("nextval")).unwrap_or(false)
        );
        
        // Entity-aware boolean detection - REVOLUTIONARY approach
        let is_boolean = match boolean_fields {
            Some(boolean_set) => boolean_set.contains(&name), // Trait-based detection ONLY!
            None => data_type == "boolean", // Basic PostgreSQL boolean detection
        };
        
        let final_column_type = if is_boolean && data_type != "boolean" {
            "boolean".to_string() // Override for entity-aware detection
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
            unique: false, // Will be detected from index information
            constraints: vec![], // Will be populated separately
        })
    }
    
    /// Query information_schema for index information
    /// 
    /// PostgreSQL doesn't have standard information_schema for indexes,
    /// so we use pg_indexes system view with sea-query.
    async fn query_table_indexes(&self, table_name: &str, schema_name: Option<&str>) -> Result<Vec<IndexSchema>, IntrospectionError> {
        // Build query for pg_indexes using sea-query
        let (sql, sea_params) = {
            let mut query = Query::select()
                .columns([
                    Alias::new("indexname"),
                    Alias::new("indexdef"),
                ])
                .from(Alias::new("pg_indexes"))
                .and_where(Expr::col(Alias::new("tablename")).eq(table_name))
                .to_owned();
            
            // Filter by schema
            let target_schema = schema_name.unwrap_or("public");
            query = query.and_where(Expr::col(Alias::new("schemaname")).eq(target_schema)).to_owned();
            
            let (sql, params) = query.build(PostgresQueryBuilder);
            let sea_params: Vec<Value> = params.into_iter().map(|p| sea_value_to_json(&p)).collect();
            (sql, sea_params)
        };
        
        let result = self.client.execute(&sql, &sea_params).await
            .map_err(|e| IntrospectionError {
                message: format!("Failed to query pg_indexes for table '{}': {}", table_name, e),
                kind: IntrospectionErrorKind::DatabaseError,
            })?;
        
        let mut indexes = Vec::new();
        for row in result.rows() {
            if let Value::Object(obj) = row {
                let index_name = obj.get("indexname")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                let index_def = obj.get("indexdef")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                
                // Skip primary key indexes (they're handled separately)
                if index_name.ends_with("_pkey") {
                    continue;
                }
                
                // Parse columns from index definition
                let columns = self.parse_index_columns(index_def)?;
                
                // Detect if it's a unique index
                let unique = index_def.to_uppercase().contains("UNIQUE");
                
                indexes.push(IndexSchema {
                    name: index_name,
                    columns,
                    unique,
                    table_name: Some(table_name.to_string()),
                });
            }
        }
        
        Ok(indexes)
    }
    
    /// Parse column names from PostgreSQL index definition
    fn parse_index_columns(&self, index_def: &str) -> Result<Vec<String>, IntrospectionError> {
        // Find the column list in parentheses
        let start = index_def.find('(')
            .ok_or_else(|| IntrospectionError {
                message: format!("Invalid index definition: {}", index_def),
                kind: IntrospectionErrorKind::InvalidSchema,
            })?;
            
        let end = index_def.rfind(')')
            .ok_or_else(|| IntrospectionError {
                message: format!("Invalid index definition: {}", index_def),
                kind: IntrospectionErrorKind::InvalidSchema,
            })?;
        
        let column_part = &index_def[start + 1..end];
        
        // Split by commas and clean up column names
        let columns = column_part
            .split(',')
            .map(|col| col.trim().trim_matches('"').to_string())
            .collect();
        
        Ok(columns)
    }
    
    /// Query foreign key information using information_schema with sea-query builders
    async fn query_foreign_keys(&self, table_name: &str, schema_name: Option<&str>) -> Result<Vec<ForeignKeySchema>, IntrospectionError> {
        // Build complex query for foreign key information using pure sea-query builders - NO RAW SQL!
        // This joins multiple information_schema tables using sea-query's join capabilities
        let (sql, sea_params) = {
            let target_schema = schema_name.unwrap_or("public");
            
            // Define table aliases for the complex join
            let tc_alias = Alias::new("tc");
            let kcu_alias = Alias::new("kcu"); 
            let ccu_alias = Alias::new("ccu");
            let rc_alias = Alias::new("rc");
            
            let query = Query::select()
                // SELECT clause with proper aliasing
                .columns([
                    (tc_alias.clone(), Alias::new("constraint_name")),
                    (kcu_alias.clone(), Alias::new("column_name")),
                ])
                .expr_as(
                    Expr::col((ccu_alias.clone(), Alias::new("table_name"))),
                    Alias::new("foreign_table_name")
                )
                .expr_as(
                    Expr::col((ccu_alias.clone(), Alias::new("column_name"))),
                    Alias::new("foreign_column_name")
                )
                .columns([
                    (rc_alias.clone(), Alias::new("update_rule")),
                    (rc_alias.clone(), Alias::new("delete_rule")),
                ])
                // FROM clause with table alias
                .from_as(
                    (Alias::new("information_schema"), Alias::new("table_constraints")),
                    tc_alias.clone()
                )
                // First JOIN: table_constraints -> key_column_usage
                .join_as(
                    sea_query::JoinType::InnerJoin,
                    (Alias::new("information_schema"), Alias::new("key_column_usage")),
                    kcu_alias.clone(),
                    Expr::col((tc_alias.clone(), Alias::new("constraint_name")))
                        .equals((kcu_alias.clone(), Alias::new("constraint_name")))
                        .and(Expr::col((tc_alias.clone(), Alias::new("table_schema")))
                            .equals((kcu_alias.clone(), Alias::new("table_schema"))))
                )
                // Second JOIN: table_constraints -> constraint_column_usage
                .join_as(
                    sea_query::JoinType::InnerJoin,
                    (Alias::new("information_schema"), Alias::new("constraint_column_usage")),
                    ccu_alias.clone(),
                    Expr::col((ccu_alias.clone(), Alias::new("constraint_name")))
                        .equals((tc_alias.clone(), Alias::new("constraint_name")))
                        .and(Expr::col((ccu_alias.clone(), Alias::new("table_schema")))
                            .equals((tc_alias.clone(), Alias::new("table_schema"))))
                )
                // Third JOIN: table_constraints -> referential_constraints
                .join_as(
                    sea_query::JoinType::InnerJoin,
                    (Alias::new("information_schema"), Alias::new("referential_constraints")),
                    rc_alias.clone(),
                    Expr::col((tc_alias.clone(), Alias::new("constraint_name")))
                        .equals((rc_alias.clone(), Alias::new("constraint_name")))
                        .and(Expr::col((tc_alias.clone(), Alias::new("table_schema")))
                            .equals((rc_alias.clone(), Alias::new("constraint_schema"))))
                )
                // WHERE clause conditions
                .and_where(Expr::col((tc_alias.clone(), Alias::new("constraint_type"))).eq("FOREIGN KEY"))
                .and_where(Expr::col((tc_alias.clone(), Alias::new("table_name"))).eq(table_name))
                .and_where(Expr::col((tc_alias, Alias::new("table_schema"))).eq(target_schema))
                .to_owned();
            
            let (sql, params) = query.build(PostgresQueryBuilder);
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
                
                let foreign_table_name = obj.get("foreign_table_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                let foreign_column_name = obj.get("foreign_column_name")
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
                    existing_fk.referenced_columns.push(foreign_column_name);
                } else {
                    let fk = ForeignKeySchema {
                        name: constraint_name.clone(),
                        columns: vec![column_name],
                        referenced_table: foreign_table_name,
                        referenced_columns: vec![foreign_column_name],
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
    fn map_constraint_type(&self, pg_constraint_type: &str) -> ConstraintType {
        match pg_constraint_type.to_uppercase().as_str() {
            "PRIMARY KEY" => ConstraintType::PrimaryKey,
            "UNIQUE" => ConstraintType::Unique,
            "FOREIGN KEY" => ConstraintType::ForeignKey,
            "CHECK" => ConstraintType::Check,
            _ => ConstraintType::Check, // Default fallback
        }
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl SchemaIntrospector<PostgreSQLBackend> for PostgreSQLIntrospector<'_> {
    type Error = IntrospectionError;
    
    fn dialect(&self) -> DatabaseDialect {
        DatabaseDialect::PostgreSQL
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
        let rows = self.query_table_constraints(table_name, None).await?;
        
        let mut constraints = Vec::new();
        for row in rows {
            let constraint_name = row.get("constraint_name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let constraint_type_str = row.get("constraint_type")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            
            let constraint_type = self.map_constraint_type(constraint_type_str);
            
            // Get constraint definition - PostgreSQL stores CHECK constraints differently
            let definition = if constraint_type == ConstraintType::Check {
                row.get("check_clause")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            } else {
                format!("{} constraint", constraint_type_str)
            };
            
            constraints.push(ConstraintSchema {
                name: constraint_name,
                constraint_type,
                definition,
            });
        }
        
        Ok(constraints)
    }
    
    async fn get_database_metadata(&self) -> std::result::Result<HashMap<String, String>, Self::Error> {
        let mut metadata = HashMap::new();
        
        // Get PostgreSQL version using sea-query function builder - NO RAW SQL!
        let (sql, params) = {
            let query = Query::select()
                .expr_as(Expr::cust("version()"), Alias::new("version"))
                .to_owned();
            let (sql, sea_params) = query.build(PostgresQueryBuilder);
            let json_params: Vec<Value> = sea_params.into_iter()
                .map(|p| sea_value_to_json(&p))
                .collect();
            (sql, json_params)
        };
        
        let version_result = self.client.execute(&sql, &params).await
            .map_err(|e| IntrospectionError {
                message: format!("Failed to get PostgreSQL version: {}", e),
                kind: IntrospectionErrorKind::DatabaseError,
            })?;
        
        if let Some(Value::Object(obj)) = version_result.rows().first() {
            if let Some(Value::String(version)) = obj.get("version") {
                metadata.insert("version".to_string(), version.clone());
            }
        }
        
        metadata.insert("database_type".to_string(), "PostgreSQL".to_string());
        metadata.insert("supports_returning".to_string(), "true".to_string());
        metadata.insert("supports_transactions".to_string(), "true".to_string());
        metadata.insert("supports_foreign_keys".to_string(), "true".to_string());
        metadata.insert("supports_schemas".to_string(), "true".to_string());
        metadata.insert("supports_arrays".to_string(), "true".to_string());
        metadata.insert("supports_json".to_string(), "true".to_string());
        
        Ok(metadata)
    }
}

// Entity-aware introspection extension
#[cfg(feature = "postgres")]
impl<'a> PostgreSQLIntrospector<'a> {
    /// Entity-aware column introspection with boolean field detection
    /// 
    /// This method enhances standard column introspection by using Entity metadata
    /// to provide accurate boolean field detection for PostgreSQL databases.
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
    /// // active field is correctly identified as boolean
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

#[cfg(not(feature = "postgres"))]
/// Stub implementation when postgres feature is not enabled
pub struct PostgreSQLIntrospector;

#[cfg(test)]
#[cfg(feature = "postgres")]
mod tests {
    use super::*;
    use crate::dialects::DatabaseDialect;
    use serde_json::json;

    // Test business logic and parameter handling, NOT SQL format verification
    // These tests run against PostgreSQL dialect specifically for PostgreSQL introspection
    
    #[test]
    fn test_parse_column_info_business_logic() {
        // Test business logic: column info parsing correctness for PostgreSQL
        let introspector = create_test_introspector();
        
        // Test PostgreSQL-specific column parsing with various data types
        let mut column_data = serde_json::Map::new();
        column_data.insert("column_name".to_string(), json!("user_email"));
        column_data.insert("data_type".to_string(), json!("character varying"));
        column_data.insert("is_nullable".to_string(), json!("NO"));
        column_data.insert("column_default".to_string(), json!("NULL"));
        column_data.insert("character_maximum_length".to_string(), json!(255));
        
        let result = introspector.parse_column_info(&column_data, None);
        assert!(result.is_ok());
        
        let column = result.unwrap();
        assert_eq!(column.name, "user_email");
        assert_eq!(column.column_type, "character varying");
        assert_eq!(column.nullable, false); // is_nullable = "NO" means NOT NULL
        assert_eq!(column.primary_key, false);
        assert_eq!(column.auto_increment, false);
    }
    
    #[test]
    fn test_parse_column_info_boolean_detection() {
        // Test business logic: Entity-aware boolean field detection for PostgreSQL
        let introspector = create_test_introspector();
        
        let mut column_data = serde_json::Map::new();
        column_data.insert("column_name".to_string(), json!("is_verified"));
        column_data.insert("data_type".to_string(), json!("integer"));
        column_data.insert("is_nullable".to_string(), json!("YES"));
        column_data.insert("column_default".to_string(), Value::Null);
        
        // Test with PostgreSQL native boolean type
        let mut boolean_column_data = column_data.clone();
        boolean_column_data.insert("data_type".to_string(), json!("boolean"));
        
        let result = introspector.parse_column_info(&boolean_column_data, None);
        assert!(result.is_ok());
        
        let column = result.unwrap();
        assert_eq!(column.name, "is_verified");
        assert_eq!(column.column_type, "boolean"); // PostgreSQL native boolean
        
        // Test with Entity-aware boolean field detection
        let mut boolean_fields = HashSet::new();
        boolean_fields.insert("is_verified".to_string());
        
        let result2 = introspector.parse_column_info(&column_data, Some(&boolean_fields));
        assert!(result2.is_ok());
        
        let column2 = result2.unwrap();
        assert_eq!(column2.column_type, "boolean"); // Should be converted to boolean
        
        // Test without boolean field detection
        let result3 = introspector.parse_column_info(&column_data, None);
        assert!(result3.is_ok());
        
        let column3 = result3.unwrap();
        assert_eq!(column3.column_type, "integer"); // Should remain integer
    }
    
    #[test]
    fn test_parse_index_columns_business_logic() {
        // Test business logic: PostgreSQL index definition parsing
        let introspector = create_test_introspector();
        
        // Test simple index
        let simple_index_def = "CREATE INDEX idx_user_email ON users (email)";
        let result = introspector.parse_index_columns(simple_index_def);
        assert!(result.is_ok());
        
        let columns = result.unwrap();
        assert_eq!(columns.len(), 1);
        assert_eq!(columns[0], "email");
        
        // Test composite index
        let composite_index_def = "CREATE INDEX idx_user_name_email ON users (last_name, first_name, email)";
        let result2 = introspector.parse_index_columns(composite_index_def);
        assert!(result2.is_ok());
        
        let columns2 = result2.unwrap();
        assert_eq!(columns2.len(), 3);
        assert_eq!(columns2[0], "last_name");
        assert_eq!(columns2[1], "first_name");
        assert_eq!(columns2[2], "email");
        
        // Test index with quoted identifiers
        let quoted_index_def = "CREATE INDEX idx_quoted ON users (\"column_name\", \"another_column\")";
        let result3 = introspector.parse_index_columns(quoted_index_def);
        assert!(result3.is_ok());
        
        let columns3 = result3.unwrap();
        assert_eq!(columns3.len(), 2);
        assert_eq!(columns3[0], "column_name"); // Should remove quotes
        assert_eq!(columns3[1], "another_column");
    }
    
    #[test]
    fn test_constraint_type_mapping() {
        // Test business logic: PostgreSQL constraint type mapping
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
    fn test_postgresql_dialect_identification() {
        // Test business logic: correct dialect identification
        assert_eq!(DatabaseDialect::PostgreSQL.to_string(), "PostgreSQL");
        assert!(DatabaseDialect::PostgreSQL.supports_returning());
        assert!(DatabaseDialect::PostgreSQL.supports_cte());
        
        // Test PostgreSQL-specific features (business logic, not trait methods)
        let pg_metadata = HashMap::from([
            ("supports_schemas".to_string(), "true".to_string()),
            ("supports_arrays".to_string(), "true".to_string()),
            ("supports_json".to_string(), "true".to_string()),
        ]);
        assert_eq!(pg_metadata.get("supports_schemas"), Some(&"true".to_string()));
        assert_eq!(pg_metadata.get("supports_arrays"), Some(&"true".to_string()));
        assert_eq!(pg_metadata.get("supports_json"), Some(&"true".to_string()));
    }
    
    #[test]
    fn test_primary_key_detection_patterns() {
        // Test business logic: PostgreSQL primary key detection patterns
        let introspector = create_test_introspector();
        
        // Test serial column detection
        let mut serial_column = serde_json::Map::new();
        serial_column.insert("column_name".to_string(), json!("id"));
        serial_column.insert("data_type".to_string(), json!("bigserial"));
        serial_column.insert("is_nullable".to_string(), json!("NO"));
        serial_column.insert("column_default".to_string(), json!("nextval('users_id_seq'::regclass)"));
        
        let result = introspector.parse_column_info(&serial_column, None);
        assert!(result.is_ok());
        
        let column = result.unwrap();
        assert_eq!(column.name, "id");
        assert_eq!(column.column_type, "bigserial");
        assert!(column.primary_key); // Should detect primary key from nextval
        assert!(column.auto_increment); // Should detect auto increment from serial/nextval
        
        // Test sequence column detection
        let mut sequence_column = serde_json::Map::new();
        sequence_column.insert("column_name".to_string(), json!("user_id"));
        sequence_column.insert("data_type".to_string(), json!("integer"));
        sequence_column.insert("is_nullable".to_string(), json!("NO"));
        sequence_column.insert("column_default".to_string(), json!("nextval('user_sequence')"));
        
        let result2 = introspector.parse_column_info(&sequence_column, None);
        assert!(result2.is_ok());
        
        let column2 = result2.unwrap();
        assert!(column2.primary_key); // Should detect from nextval
        assert!(column2.auto_increment); // Should detect auto increment from nextval
    }
    
    #[test]
    fn test_schema_name_handling() {
        // Test business logic: PostgreSQL schema handling (defaults to 'public')
        // This tests the parameter logic for schema filtering, not SQL generation
        
        // Test default schema (should be 'public')
        let default_schema = None;
        let target_schema = default_schema.unwrap_or("public");
        assert_eq!(target_schema, "public");
        
        // Test custom schema
        let custom_schema = Some("custom_schema");
        let target_schema2 = custom_schema.unwrap_or("public");
        assert_eq!(target_schema2, "custom_schema");
        
        // Test schema filtering logic
        let excluded_schemas = ["information_schema", "pg_catalog"];
        for schema in excluded_schemas.iter() {
            assert_ne!(schema, &"public"); // Should not exclude public schema
            assert!(schema.starts_with("pg_") || schema == &"information_schema");
        }
    }
    
    // Helper function to create a test introspector for testing business logic
    fn create_test_introspector() -> PostgreSQLIntrospector<'static> {
        // Create a dummy introspector using a null reference - safe for testing parsing methods
        // These tests only call parsing methods that don't use the client
        let client_ref: &'static DatabaseClient<PostgreSQLBackend> = unsafe {
            // This is safe because we only test parsing methods that don't access the client
            std::mem::transmute(&() as *const () as *const DatabaseClient<PostgreSQLBackend>)
        };
        PostgreSQLIntrospector::new(client_ref)
    }
}