use crate::{D1RsError, DatabaseClient, Entity, D1Client};
use crate::backends::{DatabaseBackend, QueryResult};
use crate::dialects::DatabaseDialect;
use crate::query_builder::sea_value_to_json;
use sea_query::{Query, Expr, Alias, SqliteQueryBuilder};
#[cfg(feature = "postgres")]
use sea_query::{PostgresQueryBuilder, JoinType};
#[cfg(feature = "mysql")]
use sea_query::MysqlQueryBuilder;
use serde_json::Value;
use std::collections::HashSet;

type Result<T> = std::result::Result<T, D1RsError>;

/// Database-agnostic schema introspector that works across SQLite, PostgreSQL, and MySQL
/// 
/// This introspector uses pure sea-query builders and provides a unified interface
/// for schema introspection across different database backends. Each database uses
/// its most efficient introspection method:
/// 
/// - **SQLite**: sqlite_master table queries
/// - **PostgreSQL**: information_schema.columns queries (feature-gated)
/// - **MySQL**: information_schema.COLUMNS queries (feature-gated)
/// 
/// All queries are built using sea-query builders for type safety and database portability.
pub struct SchemaIntrospector<'a, B: DatabaseBackend> {
    client: &'a DatabaseClient<B>,
}

impl<'a, B: DatabaseBackend> SchemaIntrospector<'a, B>
where
    D1RsError: From<B::Error>,
{
    /// Create a new database-agnostic schema introspector
    pub fn new(client: &'a DatabaseClient<B>) -> Self {
        Self { client }
    }

    /// Create from D1Client using adapter pattern for backward compatibility
    pub fn from_d1_client(client: &'a D1Client) -> D1ClientAdapter<'a> {
        D1ClientAdapter { client }
    }

    /// Introspect the complete database schema
    pub async fn introspect_database(&self) -> Result<DatabaseSchema> {
        let table_names = self.introspect_tables().await?;
        let mut tables = Vec::new();
        
        for table_name in table_names {
            let table_schema = self.introspect_table(&table_name).await?;
            tables.push(table_schema);
        }
        
        Ok(DatabaseSchema {
            tables,
            dialect: self.client.dialect(),
        })
    }

    /// Get all table names using database-specific queries
    pub async fn introspect_tables(&self) -> Result<Vec<String>> {
        match self.client.dialect() {
            DatabaseDialect::SQLite => {
                let (sql, params) = Query::select()
                    .column(Alias::new("name"))
                    .from(Alias::new("sqlite_master"))
                    .and_where(Expr::col(Alias::new("type")).eq("table"))
                    .and_where(Expr::col(Alias::new("name")).not_like("sqlite_%"))
                    .build(SqliteQueryBuilder);

                let sea_params: Vec<Value> = params.into_iter().map(|p| sea_value_to_json(&p)).collect();
                let result = self.client.execute(&sql, &sea_params).await?;

                let mut tables = Vec::new();
                for row in result.rows() {
                    if let Value::Object(obj) = row {
                        if let Some(Value::String(name)) = obj.get("name") {
                            tables.push(name.clone());
                        }
                    }
                }
                Ok(tables)
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                let (sql, params) = Query::select()
                    .column(Alias::new("table_name"))
                    .from(Alias::new("information_schema.tables"))
                    .and_where(Expr::col(Alias::new("table_schema")).eq("public"))
                    .and_where(Expr::col(Alias::new("table_type")).eq("BASE TABLE"))
                    .build(PostgresQueryBuilder);

                let sea_params: Vec<Value> = params.into_iter().map(|p| sea_value_to_json(&p)).collect();
                let result = self.client.execute(&sql, &sea_params).await?;

                let mut tables = Vec::new();
                for row in result.rows() {
                    if let Value::Object(obj) = row {
                        if let Some(Value::String(name)) = obj.get("table_name") {
                            tables.push(name.clone());
                        }
                    }
                }
                Ok(tables)
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                let (sql, params) = Query::select()
                    .column(Alias::new("TABLE_NAME"))
                    .from(Alias::new("information_schema.TABLES"))
                    .and_where(Expr::col(Alias::new("TABLE_SCHEMA")).eq("DATABASE()"))
                    .and_where(Expr::col(Alias::new("TABLE_TYPE")).eq("BASE TABLE"))
                    .build(MysqlQueryBuilder);

                let sea_params: Vec<Value> = params.into_iter().map(|p| sea_value_to_json(&p)).collect();
                let result = self.client.execute(&sql, &sea_params).await?;

                let mut tables = Vec::new();
                for row in result.rows() {
                    if let Value::Object(obj) = row {
                        if let Some(Value::String(name)) = obj.get("TABLE_NAME") {
                            tables.push(name.clone());
                        }
                    }
                }
                Ok(tables)
            },
        }
    }

    /// Introspect a specific table's complete schema
    pub async fn introspect_table(&self, table_name: &str) -> Result<TableSchema> {
        let columns = self.introspect_columns(table_name).await?;
        let indexes = self.introspect_indexes(table_name).await?;
        let foreign_keys = self.introspect_foreign_keys(table_name).await?;
        let constraints = self.introspect_table_constraints(table_name).await?;
        
        Ok(TableSchema {
            name: table_name.to_string(),
            columns,
            indexes,
            foreign_keys,
            constraints,
        })
    }

    /// Introspect foreign keys for a table using database-agnostic approach
    pub async fn introspect_foreign_keys(&self, table_name: &str) -> Result<Vec<ForeignKeySchema>> {
        let mut foreign_keys = Vec::new();
        
        match self.client.dialect() {
            DatabaseDialect::SQLite => {
                #[cfg(feature = "sqlite")]
                {
                    // For SQLite, get the CREATE statement and parse foreign keys from it
                    let (sql, params) = Query::select()
                        .column(Alias::new("sql"))
                        .from(Alias::new("sqlite_master"))
                        .and_where(Expr::col(Alias::new("type")).eq("table"))
                        .and_where(Expr::col(Alias::new("name")).eq(table_name))
                        .build(SqliteQueryBuilder);
                    
                    let sea_params: Vec<Value> = params.into_iter().map(|p| sea_value_to_json(&p)).collect();
                    let result = self.client.execute(&sql, &sea_params).await?;
                    
                    // Parse foreign keys from CREATE TABLE statement
                    for row in result.rows() {
                        if let Value::Object(obj) = row {
                            if let Some(Value::String(create_sql)) = obj.get("sql") {
                                foreign_keys.extend(self.parse_sqlite_foreign_keys(create_sql)?);
                            }
                        }
                    }
                }
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                let (sql, params) = Query::select()
                    .columns([
                        Alias::new("kcu.constraint_name"),
                        Alias::new("kcu.column_name"),
                        Alias::new("ccu.table_name AS referenced_table"),
                        Alias::new("ccu.column_name AS referenced_column"),
                        Alias::new("rc.delete_rule"),
                        Alias::new("rc.update_rule"),
                    ])
                    .from(Alias::new("information_schema.key_column_usage AS kcu"))
                    .join(
                        JoinType::InnerJoin,
                        Alias::new("information_schema.constraint_column_usage AS ccu"),
                        Expr::col((Alias::new("kcu"), Alias::new("constraint_name")))
                            .eq(Expr::col((Alias::new("ccu"), Alias::new("constraint_name"))))
                    )
                    .join(
                        JoinType::InnerJoin,
                        Alias::new("information_schema.referential_constraints AS rc"),
                        Expr::col((Alias::new("kcu"), Alias::new("constraint_name")))
                            .eq(Expr::col((Alias::new("rc"), Alias::new("constraint_name"))))
                    )
                    .and_where(Expr::col((Alias::new("kcu"), Alias::new("table_name"))).eq(table_name))
                    .build(PostgresQueryBuilder);
                
                let sea_params: Vec<Value> = params.into_iter().map(|p| sea_value_to_json(&p)).collect();
                let result = self.client.execute(&sql, &sea_params).await?;
                
                for row in result.rows() {
                    if let Value::Object(obj) = row {
                        if let (
                            Some(Value::String(name)),
                            Some(Value::String(column)),
                            Some(Value::String(ref_table)),
                            Some(Value::String(ref_column))
                        ) = (
                            obj.get("constraint_name"),
                            obj.get("column_name"),
                            obj.get("referenced_table"),
                            obj.get("referenced_column")
                        ) {
                            foreign_keys.push(ForeignKeySchema {
                                name: name.clone(),
                                columns: vec![column.clone()],
                                referenced_table: ref_table.clone(),
                                referenced_columns: vec![ref_column.clone()],
                                on_delete: Some(obj.get("delete_rule").and_then(|v| v.as_str()).unwrap_or("NO ACTION").to_string()),
                                on_update: Some(obj.get("update_rule").and_then(|v| v.as_str()).unwrap_or("NO ACTION").to_string()),
                            });
                        }
                    }
                }
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                let (sql, params) = Query::select()
                    .columns([
                        Alias::new("CONSTRAINT_NAME"),
                        Alias::new("COLUMN_NAME"),
                        Alias::new("REFERENCED_TABLE_NAME"),
                        Alias::new("REFERENCED_COLUMN_NAME"),
                        Alias::new("DELETE_RULE"),
                        Alias::new("UPDATE_RULE"),
                    ])
                    .from(Alias::new("information_schema.KEY_COLUMN_USAGE"))
                    .and_where(Expr::col(Alias::new("TABLE_NAME")).eq(table_name))
                    .and_where(Expr::col(Alias::new("REFERENCED_TABLE_NAME")).is_not_null())
                    .build(MysqlQueryBuilder);
                
                let sea_params: Vec<Value> = params.into_iter().map(|p| sea_value_to_json(&p)).collect();
                let result = self.client.execute(&sql, &sea_params).await?;
                
                for row in result.rows() {
                    if let Value::Object(obj) = row {
                        if let (
                            Some(Value::String(name)),
                            Some(Value::String(column)),
                            Some(Value::String(ref_table)),
                            Some(Value::String(ref_column))
                        ) = (
                            obj.get("CONSTRAINT_NAME"),
                            obj.get("COLUMN_NAME"),
                            obj.get("REFERENCED_TABLE_NAME"),
                            obj.get("REFERENCED_COLUMN_NAME")
                        ) {
                            foreign_keys.push(ForeignKeySchema {
                                name: name.clone(),
                                columns: vec![column.clone()],
                                referenced_table: ref_table.clone(),
                                referenced_columns: vec![ref_column.clone()],
                                on_delete: Some(obj.get("DELETE_RULE").and_then(|v| v.as_str()).unwrap_or("NO ACTION").to_string()),
                                on_update: Some(obj.get("UPDATE_RULE").and_then(|v| v.as_str()).unwrap_or("NO ACTION").to_string()),
                            });
                        }
                    }
                }
            },
        }
        
        Ok(foreign_keys)
    }

    /// Introspect columns for a table without entity awareness
    pub async fn introspect_columns(&self, table_name: &str) -> Result<Vec<ColumnSchema>> {
        self.introspect_columns_impl(table_name, None).await
    }

    /// Introspect columns with entity-aware boolean detection
    pub async fn introspect_columns_with_entity<T: Entity>(&self, table_name: &str) -> Result<Vec<ColumnSchema>> {
        let boolean_fields: HashSet<String> = T::boolean_fields()
            .iter()
            .map(|s| s.to_string())
            .collect();
        
        self.introspect_columns_impl(table_name, Some(&boolean_fields)).await
    }

    /// Database-agnostic column introspection implementation
    async fn introspect_columns_impl(&self, table_name: &str, boolean_fields: Option<&HashSet<String>>) -> Result<Vec<ColumnSchema>> {
        match self.client.dialect() {
            DatabaseDialect::SQLite => {
                // SQLite: Query sqlite_master for CREATE TABLE statement
                let (sql, params) = Query::select()
                    .column(Alias::new("sql"))
                    .from(Alias::new("sqlite_master"))
                    .and_where(Expr::col(Alias::new("type")).eq("table"))
                    .and_where(Expr::col(Alias::new("name")).eq(table_name))
                    .build(SqliteQueryBuilder);

                let sea_params: Vec<Value> = params.into_iter().map(|p| sea_value_to_json(&p)).collect();
                let result = self.client.execute(&sql, &sea_params).await?;

                for row in result.rows() {
                    if let Value::Object(obj) = row {
                        if let Some(Value::String(create_sql)) = obj.get("sql") {
                            return self.parse_create_table_columns(create_sql, boolean_fields);
                        }
                    }
                }
                Ok(Vec::new())
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                // PostgreSQL: Query information_schema.columns
                let (sql, params) = Query::select()
                    .columns([
                        Alias::new("column_name"),
                        Alias::new("data_type"),
                        Alias::new("is_nullable"),
                        Alias::new("column_default"),
                    ])
                    .from(Alias::new("information_schema.columns"))
                    .and_where(Expr::col(Alias::new("table_name")).eq(table_name))
                    .order_by(Alias::new("ordinal_position"), sea_query::Order::Asc)
                    .build(PostgresQueryBuilder);

                let sea_params: Vec<Value> = params.into_iter().map(|p| sea_value_to_json(&p)).collect();
                let result = self.client.execute(&sql, &sea_params).await?;

                let mut columns = Vec::new();
                for row in result.rows() {
                    if let Value::Object(obj) = row {
                        if let Ok(column) = self.parse_postgres_column_info(obj.clone(), boolean_fields) {
                            columns.push(column);
                        }
                    }
                }
                Ok(columns)
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                // MySQL: Query information_schema.COLUMNS
                let (sql, params) = Query::select()
                    .columns([
                        Alias::new("COLUMN_NAME"),
                        Alias::new("DATA_TYPE"),
                        Alias::new("IS_NULLABLE"),
                        Alias::new("COLUMN_DEFAULT"),
                        Alias::new("COLUMN_KEY"),
                        Alias::new("EXTRA"),
                    ])
                    .from(Alias::new("information_schema.COLUMNS"))
                    .and_where(Expr::col(Alias::new("TABLE_NAME")).eq(table_name))
                    .order_by(Alias::new("ORDINAL_POSITION"), sea_query::Order::Asc)
                    .build(MysqlQueryBuilder);

                let sea_params: Vec<Value> = params.into_iter().map(|p| sea_value_to_json(&p)).collect();
                let result = self.client.execute(&sql, &sea_params).await?;

                let mut columns = Vec::new();
                for row in result.rows() {
                    if let Value::Object(obj) = row {
                        if let Ok(column) = self.parse_mysql_column_info(obj.clone(), boolean_fields) {
                            columns.push(column);
                        }
                    }
                }
                Ok(columns)
            },
        }
    }

    /// Introspect indexes for a table using database-agnostic approach
    pub async fn introspect_indexes(&self, table_name: &str) -> Result<Vec<IndexSchema>> {
        let mut indexes = Vec::new();
        
        match self.client.dialect() {
            DatabaseDialect::SQLite => {
                // SQLite: Query sqlite_master for indexes
                let (sql, params) = Query::select()
                    .columns([
                        Alias::new("name"),
                        Alias::new("sql"),
                    ])
                    .from(Alias::new("sqlite_master"))
                    .and_where(Expr::col(Alias::new("type")).eq("index"))
                    .and_where(Expr::col(Alias::new("tbl_name")).eq(table_name))
                    .and_where(Expr::col(Alias::new("name")).not_like("sqlite_%")) // Exclude auto-generated indexes
                    .build(SqliteQueryBuilder);
                
                let sea_params: Vec<Value> = params.into_iter().map(|p| sea_value_to_json(&p)).collect();
                let result = self.client.execute(&sql, &sea_params).await?;
                
                for row in result.rows() {
                    if let Value::Object(obj) = row {
                        if let (
                            Some(Value::String(name)),
                            Some(Value::String(create_sql))
                        ) = (
                            obj.get("name"),
                            obj.get("sql")
                        ) {
                            if let Some(index) = self.parse_sqlite_index_sql(name, create_sql, table_name)? {
                                indexes.push(index);
                            }
                        }
                    }
                }
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                let (sql, params) = Query::select()
                    .columns([
                        Alias::new("indexname"),
                        Alias::new("indexdef"),
                    ])
                    .from(Alias::new("pg_indexes"))
                    .and_where(Expr::col(Alias::new("tablename")).eq(table_name))
                    .and_where(Expr::col(Alias::new("schemaname")).eq("public"))
                    .build(PostgresQueryBuilder);
                
                let sea_params: Vec<Value> = params.into_iter().map(|p| sea_value_to_json(&p)).collect();
                let result = self.client.execute(&sql, &sea_params).await?;
                
                for row in result.rows() {
                    if let Value::Object(obj) = row {
                        if let (
                            Some(Value::String(name)),
                            Some(Value::String(definition))
                        ) = (
                            obj.get("indexname"),
                            obj.get("indexdef")
                        ) {
                            if let Some(index) = self.parse_postgres_index_definition(name, definition, table_name)? {
                                indexes.push(index);
                            }
                        }
                    }
                }
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                let (sql, params) = Query::select()
                    .columns([
                        Alias::new("INDEX_NAME"),
                        Alias::new("COLUMN_NAME"),
                        Alias::new("NON_UNIQUE"),
                        Alias::new("SEQ_IN_INDEX"),
                    ])
                    .from(Alias::new("information_schema.STATISTICS"))
                    .and_where(Expr::col(Alias::new("TABLE_NAME")).eq(table_name))
                    .and_where(Expr::col(Alias::new("TABLE_SCHEMA")).eq("DATABASE()"))
                    .and_where(Expr::col(Alias::new("INDEX_NAME")).ne("PRIMARY")) // Exclude primary key indexes
                    .order_by(Alias::new("INDEX_NAME"), sea_query::Order::Asc)
                    .order_by(Alias::new("SEQ_IN_INDEX"), sea_query::Order::Asc)
                    .build(MysqlQueryBuilder);
                
                let sea_params: Vec<Value> = params.into_iter().map(|p| sea_value_to_json(&p)).collect();
                let result = self.client.execute(&sql, &sea_params).await?;
                
                indexes = self.group_mysql_index_columns(result.rows(), table_name)?;
            },
        }
        
        Ok(indexes)
    }


    /// Introspect table constraints (placeholder implementation)
    pub async fn introspect_table_constraints(&self, _table_name: &str) -> Result<Vec<ConstraintSchema>> {
        // Database-agnostic constraint introspection will be implemented in future tasks
        Ok(Vec::new())
    }

    /// Parse CREATE TABLE SQL for column definitions (SQLite)
    fn parse_create_table_columns(&self, create_sql: &str, boolean_fields: Option<&HashSet<String>>) -> Result<Vec<ColumnSchema>> {
        let mut columns = Vec::new();
        
        // Extract column definitions from CREATE TABLE statement
        if let Some(start) = create_sql.find('(') {
            if let Some(end) = create_sql.rfind(')') {
                let columns_part = &create_sql[start + 1..end];
                
                // First pass: collect all parts (columns and constraints)
                let mut parts = Vec::new();
                let mut current_part = String::new();
                let mut paren_depth = 0;
                let mut in_quotes = false;
                let mut quote_char = '"';
                
                for ch in columns_part.chars() {
                    match ch {
                        '"' | '\'' if !in_quotes => {
                            in_quotes = true;
                            quote_char = ch;
                            current_part.push(ch);
                        }
                        ch if in_quotes && ch == quote_char => {
                            in_quotes = false;
                            current_part.push(ch);
                        }
                        '(' if !in_quotes => {
                            paren_depth += 1;
                            current_part.push(ch);
                        }
                        ')' if !in_quotes => {
                            paren_depth -= 1;
                            current_part.push(ch);
                        }
                        ',' if !in_quotes && paren_depth == 0 => {
                            if !current_part.trim().is_empty() {
                                parts.push(current_part.trim().to_string());
                            }
                            current_part.clear();
                        }
                        _ => {
                            current_part.push(ch);
                        }
                    }
                }
                
                // Handle the last part
                if !current_part.trim().is_empty() {
                    parts.push(current_part.trim().to_string());
                }
                
                // Second pass: identify table-level constraints (particularly PRIMARY KEY)
                let mut primary_key_columns = Vec::new();
                let mut column_parts = Vec::new();
                
                for part in parts {
                    let part_upper = part.to_uppercase();
                    if part_upper.trim().starts_with("PRIMARY KEY") {
                        // Extract columns from PRIMARY KEY (col1, col2, ...)
                        if let Some(paren_start) = part.find('(') {
                            if let Some(paren_end) = part.rfind(')') {
                                let pk_cols = &part[paren_start + 1..paren_end];
                                primary_key_columns = pk_cols
                                    .split(',')
                                    .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                                    .collect();
                            }
                        }
                    } else if !part_upper.trim().starts_with("FOREIGN KEY") && 
                              !part_upper.trim().starts_with("UNIQUE") && 
                              !part_upper.trim().starts_with("CHECK") && 
                              !part_upper.trim().starts_with("CONSTRAINT") {
                        // This is a column definition
                        column_parts.push(part);
                    }
                }
                
                // Third pass: parse column definitions and apply table-level PRIMARY KEY
                for part in column_parts {
                    if let Ok(mut column) = self.parse_column_definition(&part, boolean_fields) {
                        // Check if this column is part of the table-level PRIMARY KEY
                        if primary_key_columns.contains(&column.name) {
                            column.primary_key = true;
                            column.nullable = false; // Primary key columns are not nullable
                        }
                        columns.push(column);
                    }
                }
            }
        }
        
        Ok(columns)
    }

    /// Parse a single column definition from CREATE TABLE SQL
    fn parse_column_definition(&self, definition: &str, boolean_fields: Option<&HashSet<String>>) -> Result<ColumnSchema> {
        // Skip table constraints
        let def_upper = definition.to_uppercase();
        if def_upper.trim().starts_with("PRIMARY KEY") ||
           def_upper.trim().starts_with("FOREIGN KEY") ||
           def_upper.trim().starts_with("UNIQUE") ||
           def_upper.trim().starts_with("CHECK") ||
           def_upper.trim().starts_with("CONSTRAINT") {
            return Err(D1RsError::Database("Not a column definition".to_string()));
        }
        
        let parts: Vec<&str> = definition.split_whitespace().collect();
        if parts.is_empty() {
            return Err(D1RsError::Database("Empty column definition".to_string()));
        }
        
        // Parse column name and type
        let name = parts[0].trim_matches('"').trim_matches('\'').to_string();
        let column_type = if parts.len() > 1 {
            parts[1].to_uppercase()
        } else {
            "TEXT".to_string()
        };
        
        // Parse constraints
        let primary_key = def_upper.contains("PRIMARY KEY");
        let nullable = !def_upper.contains("NOT NULL") && !primary_key;
        let auto_increment = def_upper.contains("AUTOINCREMENT");
        let unique = def_upper.contains("UNIQUE") && !primary_key;
        
        // Parse default value
        let default_value = if let Some(default_start) = def_upper.find("DEFAULT") {
            let after_default = &definition[default_start + 7..].trim();
            if let Some(space_pos) = after_default.find(' ') {
                Some(after_default[..space_pos].trim().to_string())
            } else {
                Some(after_default.trim().to_string())
            }
        } else {
            None
        };
        
        // Apply entity-aware boolean detection
        let final_type = if let Some(boolean_fields) = boolean_fields {
            if boolean_fields.contains(&name) && column_type == "INTEGER" {
                "BOOLEAN".to_string()
            } else {
                column_type
            }
        } else {
            column_type
        };
        
        Ok(ColumnSchema {
            name,
            column_type: final_type,
            nullable,
            default_value,
            primary_key,
            auto_increment,
            unique,
            constraints: Vec::new(),
        })
    }

    /// Parse SQLite foreign keys from CREATE TABLE statement
    fn parse_sqlite_foreign_keys(&self, create_sql: &str) -> Result<Vec<ForeignKeySchema>> {
        let mut foreign_keys = Vec::new();
        
        // Extract the table definition part
        if let Some(start) = create_sql.find('(') {
            if let Some(end) = create_sql.rfind(')') {
                let table_def = &create_sql[start + 1..end];
                
                // Look for FOREIGN KEY constraints in the CREATE TABLE statement
                // SQLite foreign keys can be defined as:
                // 1. FOREIGN KEY (column) REFERENCES table(column)
                // 2. column_name TYPE REFERENCES table(column) 
                
                let mut paren_depth = 0;
                let mut in_quotes = false;
                let mut quote_char = '"';
                let mut current_part = String::new();
                let mut parts = Vec::new();
                
                // Split by commas, handling nested parentheses and quotes
                for ch in table_def.chars() {
                    match ch {
                        '"' | '\'' if !in_quotes => {
                            in_quotes = true;
                            quote_char = ch;
                            current_part.push(ch);
                        }
                        ch if in_quotes && ch == quote_char => {
                            in_quotes = false;
                            current_part.push(ch);
                        }
                        '(' if !in_quotes => {
                            paren_depth += 1;
                            current_part.push(ch);
                        }
                        ')' if !in_quotes => {
                            paren_depth -= 1;
                            current_part.push(ch);
                        }
                        ',' if !in_quotes && paren_depth == 0 => {
                            if !current_part.trim().is_empty() {
                                parts.push(current_part.trim().to_string());
                            }
                            current_part.clear();
                        }
                        _ => {
                            current_part.push(ch);
                        }
                    }
                }
                
                // Handle the last part
                if !current_part.trim().is_empty() {
                    parts.push(current_part.trim().to_string());
                }
                
                // Parse each part for foreign key constraints
                for (index, part) in parts.iter().enumerate() {
                    let part_upper = part.to_uppercase();
                    
                    // Table-level FOREIGN KEY constraint
                    if part_upper.trim().starts_with("FOREIGN KEY") {
                        if let Some(fk) = self.parse_table_level_foreign_key(part)? {
                            foreign_keys.push(fk);
                        }
                    }
                    // Column-level REFERENCES constraint
                    else if part_upper.contains("REFERENCES") {
                        if let Some(fk) = self.parse_column_level_foreign_key(part, index)? {
                            foreign_keys.push(fk);
                        }
                    }
                }
            }
        }
        
        Ok(foreign_keys)
    }
    
    /// Parse table-level foreign key: FOREIGN KEY (column) REFERENCES table(column)
    fn parse_table_level_foreign_key(&self, constraint_def: &str) -> Result<Option<ForeignKeySchema>> {
        let def = constraint_def.trim();
        
        // Extract columns after FOREIGN KEY
        if let Some(fk_start) = def.to_uppercase().find("FOREIGN KEY") {
            let after_fk = &def[fk_start + 11..].trim();
            
            if let Some(paren_start) = after_fk.find('(') {
                if let Some(paren_end) = after_fk.find(')') {
                    let columns_str = &after_fk[paren_start + 1..paren_end];
                    let columns: Vec<String> = columns_str
                        .split(',')
                        .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                        .collect();
                    
                    // Extract REFERENCES part
                    let after_columns = &after_fk[paren_end + 1..].trim();
                    if let Some(ref_start) = after_columns.to_uppercase().find("REFERENCES") {
                        let after_references = &after_columns[ref_start + 10..].trim();
                        
                        // Parse table(columns)
                        if let Some(ref_paren) = after_references.find('(') {
                            let table_name = after_references[..ref_paren].trim().to_string();
                            
                            if let Some(ref_paren_end) = after_references.find(')') {
                                let ref_columns_str = &after_references[ref_paren + 1..ref_paren_end];
                                let ref_columns: Vec<String> = ref_columns_str
                                    .split(',')
                                    .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                                    .collect();
                                
                                // Parse ON DELETE/UPDATE actions
                                let remaining = &after_references[ref_paren_end + 1..];
                                let (on_delete, on_update) = self.parse_foreign_key_actions(remaining);
                                
                                return Ok(Some(ForeignKeySchema {
                                    name: format!("fk_{}_{}", columns.join("_"), table_name),
                                    columns,
                                    referenced_table: table_name,
                                    referenced_columns: ref_columns,
                                    on_delete,
                                    on_update,
                                }));
                            }
                        }
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    /// Parse column-level foreign key: column_name TYPE REFERENCES table(column)
    fn parse_column_level_foreign_key(&self, column_def: &str, index: usize) -> Result<Option<ForeignKeySchema>> {
        let def = column_def.trim();
        let def_upper = def.to_uppercase();
        
        if let Some(ref_start) = def_upper.find("REFERENCES") {
            // Extract column name (first word)
            let parts: Vec<&str> = def.split_whitespace().collect();
            if parts.is_empty() {
                return Ok(None);
            }
            
            let column_name = parts[0].trim_matches('"').trim_matches('\'').to_string();
            
            // Parse REFERENCES part
            let after_references = &def[ref_start + 10..].trim();
            
            if let Some(ref_paren) = after_references.find('(') {
                let table_name = after_references[..ref_paren].trim().to_string();
                
                if let Some(ref_paren_end) = after_references.find(')') {
                    let ref_column = after_references[ref_paren + 1..ref_paren_end]
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_string();
                    
                    // Parse ON DELETE/UPDATE actions
                    let remaining = &after_references[ref_paren_end + 1..];
                    let (on_delete, on_update) = self.parse_foreign_key_actions(remaining);
                    
                    return Ok(Some(ForeignKeySchema {
                        name: format!("fk_{}_{}_{}", column_name, table_name, index),
                        columns: vec![column_name],
                        referenced_table: table_name,
                        referenced_columns: vec![ref_column],
                        on_delete,
                        on_update,
                    }));
                }
            }
        }
        
        Ok(None)
    }
    
    /// Parse ON DELETE and ON UPDATE actions from foreign key definition
    fn parse_foreign_key_actions(&self, remaining: &str) -> (Option<String>, Option<String>) {
        
        // Use simple string splitting approach for more reliable parsing
        let mut on_delete = None;
        let mut on_update = None;
        
        // Split by keywords to extract actions
        if let Some(delete_start) = remaining.to_uppercase().find("ON DELETE") {
            let after_delete = &remaining[delete_start + 9..];
            
            // Find the action part (everything until next "ON" keyword or end)
            let action_end = if let Some(next_on) = after_delete.to_uppercase().find(" ON ") {
                next_on
            } else {
                after_delete.len()
            };
            
            let action = after_delete[..action_end].trim();
            if !action.is_empty() {
                on_delete = Some(action.to_string());
            }
        }
        
        if let Some(update_start) = remaining.to_uppercase().find("ON UPDATE") {
            let after_update = &remaining[update_start + 9..];
            
            // Find the action part (everything until next "ON" keyword or end)
            let action_end = if let Some(next_on) = after_update.to_uppercase().find(" ON ") {
                next_on
            } else {
                after_update.len()
            };
            
            let action = after_update[..action_end].trim();
            if !action.is_empty() {
                on_update = Some(action.to_string());
            }
        }
        
        (on_delete, on_update)
    }

    /// Parse SQLite index from CREATE INDEX SQL statement
    fn parse_sqlite_index_sql(&self, name: &str, create_sql: &str, table_name: &str) -> Result<Option<IndexSchema>> {
        let sql_upper = create_sql.to_uppercase();
        
        // Extract columns from CREATE INDEX statement
        // Format: CREATE [UNIQUE] INDEX name ON table (col1, col2, ...)
        if let Some(on_pos) = sql_upper.find(" ON ") {
            if let Some(paren_start) = create_sql[on_pos..].find('(') {
                if let Some(paren_end) = create_sql[on_pos..].find(')') {
                    let cols_str = &create_sql[on_pos + paren_start + 1..on_pos + paren_end];
                    let columns: Vec<String> = cols_str
                        .split(',')
                        .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                        .collect();
                    
                    let unique = sql_upper.contains("UNIQUE");
                    
                    return Ok(Some(IndexSchema {
                        name: name.to_string(),
                        columns,
                        unique,
                        table_name: Some(table_name.to_string()),
                    }));
                }
            }
        }
        
        Ok(None)
    }
    
    /// Parse PostgreSQL index definition
    #[cfg(feature = "postgres")]
    fn parse_postgres_index_definition(&self, name: &str, definition: &str, table_name: &str) -> Result<Option<IndexSchema>> {
        let def_upper = definition.to_uppercase();
        
        // Extract columns from PostgreSQL index definition
        // Format: CREATE [UNIQUE] INDEX name ON table USING method (col1, col2, ...)
        if let Some(paren_start) = definition.rfind('(') {
            if let Some(paren_end) = definition.rfind(')') {
                let cols_str = &definition[paren_start + 1..paren_end];
                let columns: Vec<String> = cols_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
                
                let unique = def_upper.contains("UNIQUE");
                
                return Ok(Some(IndexSchema {
                    name: name.to_string(),
                    columns,
                    unique,
                    table_name: Some(table_name.to_string()),
                }));
            }
        }
        
        Ok(None)
    }
    
    
    /// Group MySQL index columns into IndexSchema objects
    #[cfg(feature = "mysql")]
    fn group_mysql_index_columns(&self, rows: &[Value], table_name: &str) -> Result<Vec<IndexSchema>> {
        use std::collections::HashMap;
        
        let mut index_map: HashMap<String, (Vec<String>, bool)> = HashMap::new();
        
        for row in rows {
            if let Value::Object(obj) = row {
                if let (
                    Some(Value::String(index_name)),
                    Some(Value::String(column_name)),
                    Some(non_unique_val)
                ) = (
                    obj.get("INDEX_NAME"),
                    obj.get("COLUMN_NAME"),
                    obj.get("NON_UNIQUE")
                ) {
                    let unique = match non_unique_val {
                        Value::Number(n) => n.as_i64() == Some(0),
                        Value::String(s) => s == "0",
                        _ => false,
                    };
                    
                    index_map.entry(index_name.clone())
                        .or_insert_with(|| (Vec::new(), unique))
                        .0
                        .push(column_name.clone());
                }
            }
        }
        
        let mut indexes = Vec::new();
        for (name, (columns, unique)) in index_map {
            indexes.push(IndexSchema {
                name,
                columns,
                unique,
                table_name: Some(table_name.to_string()),
            });
        }
        
        Ok(indexes)
    }
    

    /// Parse PostgreSQL column information from information_schema
    #[cfg(feature = "postgres")]
    fn parse_postgres_column_info(&self, obj: serde_json::Map<String, Value>, boolean_fields: Option<&HashSet<String>>) -> Result<ColumnSchema> {
        let name = obj.get("column_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| D1RsError::Database("Missing PostgreSQL column name".to_string()))?
            .to_string();

        let data_type = obj.get("data_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| D1RsError::Database("Missing PostgreSQL data type".to_string()))?
            .to_string();

        let is_nullable = obj.get("is_nullable")
            .and_then(|v| v.as_str())
            .map(|s| s == "YES")
            .unwrap_or(true);

        let default_value = obj.get("column_default")
            .and_then(|v| if v.is_null() { None } else { v.as_str().map(|s| s.to_string()) });

        // Apply entity-aware boolean detection
        let final_type = if let Some(boolean_fields) = boolean_fields {
            if boolean_fields.contains(&name) && data_type.to_uppercase() == "BOOLEAN" {
                "BOOLEAN".to_string()
            } else {
                data_type.to_uppercase()
            }
        } else {
            data_type.to_uppercase()
        };

        Ok(ColumnSchema {
            name,
            column_type: final_type,
            nullable: is_nullable,
            default_value,
            primary_key: false, // Determined from constraints
            auto_increment: false, // PostgreSQL uses SERIAL types
            unique: false, // Determined from indexes
            constraints: Vec::new(),
        })
    }

    /// Parse MySQL column information from information_schema
    #[cfg(feature = "mysql")]
    fn parse_mysql_column_info(&self, obj: serde_json::Map<String, Value>, boolean_fields: Option<&HashSet<String>>) -> Result<ColumnSchema> {
        let name = obj.get("COLUMN_NAME")
            .and_then(|v| v.as_str())
            .ok_or_else(|| D1RsError::Database("Missing MySQL column name".to_string()))?
            .to_string();

        let data_type = obj.get("DATA_TYPE")
            .and_then(|v| v.as_str())
            .ok_or_else(|| D1RsError::Database("Missing MySQL data type".to_string()))?
            .to_string();

        let is_nullable = obj.get("IS_NULLABLE")
            .and_then(|v| v.as_str())
            .map(|s| s == "YES")
            .unwrap_or(true);

        let default_value = obj.get("COLUMN_DEFAULT")
            .and_then(|v| if v.is_null() { None } else { v.as_str().map(|s| s.to_string()) });

        let column_key = obj.get("COLUMN_KEY")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let extra = obj.get("EXTRA")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Apply entity-aware boolean detection
        let final_type = if let Some(boolean_fields) = boolean_fields {
            if boolean_fields.contains(&name) && (data_type.to_uppercase() == "TINYINT" || data_type.to_uppercase() == "BOOLEAN") {
                "BOOLEAN".to_string()
            } else {
                data_type.to_uppercase()
            }
        } else {
            data_type.to_uppercase()
        };

        Ok(ColumnSchema {
            name,
            column_type: final_type,
            nullable: is_nullable,
            default_value,
            primary_key: column_key == "PRI",
            auto_increment: extra.contains("auto_increment"),
            unique: column_key == "UNI",
            constraints: Vec::new(),
        })
    }


}

/// D1Client adapter for backward compatibility using pure sea-query
/// 
/// This adapter maintains the same interface as the generic introspector
/// but works specifically with D1Client (which is always SQLite).
pub struct D1ClientAdapter<'a> {
    client: &'a D1Client,
}

impl<'a> D1ClientAdapter<'a> {
    /// Introspect the complete database schema
    pub async fn introspect_database(&self) -> Result<DatabaseSchema> {
        let table_names = self.introspect_tables().await?;
        let mut tables = Vec::new();
        
        for table_name in table_names {
            let table_schema = self.introspect_table(&table_name).await?;
            tables.push(table_schema);
        }
        
        Ok(DatabaseSchema {
            tables,
            dialect: DatabaseDialect::SQLite, // D1Client is always SQLite
        })
    }

    /// Get all table names using sea-query builders
    pub async fn introspect_tables(&self) -> Result<Vec<String>> {
        let (sql, params) = Query::select()
            .column(Alias::new("name"))
            .from(Alias::new("sqlite_master"))
            .and_where(Expr::col(Alias::new("type")).eq("table"))
            .and_where(Expr::col(Alias::new("name")).not_like("sqlite_%"))
            .build(SqliteQueryBuilder);

        let sea_params: Vec<Value> = params.into_iter().map(|p| sea_value_to_json(&p)).collect();
        let result = self.client.execute(&sql, &sea_params).await?;

        let mut tables = Vec::new();
        for row in result.rows() {
            if let Value::Object(obj) = row {
                if let Some(Value::String(name)) = obj.get("name") {
                    tables.push(name.clone());
                }
            }
        }
        Ok(tables)
    }

    /// Introspect a specific table
    pub async fn introspect_table(&self, table_name: &str) -> Result<TableSchema> {
        let columns = self.introspect_columns(table_name).await?;
        let indexes = Vec::new(); // Placeholder
        let foreign_keys = Vec::new(); // Placeholder
        let constraints = Vec::new(); // Placeholder
        
        Ok(TableSchema {
            name: table_name.to_string(),
            columns,
            indexes,
            foreign_keys,
            constraints,
        })
    }

    /// Introspect columns for a table using sea-query
    pub async fn introspect_columns(&self, table_name: &str) -> Result<Vec<ColumnSchema>> {
        let (sql, params) = Query::select()
            .column(Alias::new("sql"))
            .from(Alias::new("sqlite_master"))
            .and_where(Expr::col(Alias::new("type")).eq("table"))
            .and_where(Expr::col(Alias::new("name")).eq(table_name))
            .build(SqliteQueryBuilder);

        let sea_params: Vec<Value> = params.into_iter().map(|p| sea_value_to_json(&p)).collect();
        let result = self.client.execute(&sql, &sea_params).await?;

        for row in result.rows() {
            if let Value::Object(obj) = row {
                if let Some(Value::String(create_sql)) = obj.get("sql") {
                    return self.parse_create_table_columns(create_sql, None);
                }
            }
        }
        Ok(Vec::new())
    }

    /// Entity-aware column introspection using sea-query
    pub async fn introspect_columns_with_entity<T: Entity>(&self, table_name: &str) -> Result<Vec<ColumnSchema>> {
        let boolean_fields: HashSet<String> = T::boolean_fields()
            .iter()
            .map(|s| s.to_string())
            .collect();

        let (sql, params) = Query::select()
            .column(Alias::new("sql"))
            .from(Alias::new("sqlite_master"))
            .and_where(Expr::col(Alias::new("type")).eq("table"))
            .and_where(Expr::col(Alias::new("name")).eq(table_name))
            .build(SqliteQueryBuilder);

        let sea_params: Vec<Value> = params.into_iter().map(|p| sea_value_to_json(&p)).collect();
        let result = self.client.execute(&sql, &sea_params).await?;

        for row in result.rows() {
            if let Value::Object(obj) = row {
                if let Some(Value::String(create_sql)) = obj.get("sql") {
                    return self.parse_create_table_columns(create_sql, Some(&boolean_fields));
                }
            }
        }
        Ok(Vec::new())
    }

    /// Parse CREATE TABLE SQL for column definitions (same as main implementation)
    fn parse_create_table_columns(&self, create_sql: &str, boolean_fields: Option<&HashSet<String>>) -> Result<Vec<ColumnSchema>> {
        let mut columns = Vec::new();
        
        if let Some(start) = create_sql.find('(') {
            if let Some(end) = create_sql.rfind(')') {
                let columns_part = &create_sql[start + 1..end];
                
                let mut current_column = String::new();
                let mut paren_depth = 0;
                let mut in_quotes = false;
                let mut quote_char = '"';
                
                for ch in columns_part.chars() {
                    match ch {
                        '"' | '\'' if !in_quotes => {
                            in_quotes = true;
                            quote_char = ch;
                            current_column.push(ch);
                        }
                        ch if in_quotes && ch == quote_char => {
                            in_quotes = false;
                            current_column.push(ch);
                        }
                        '(' if !in_quotes => {
                            paren_depth += 1;
                            current_column.push(ch);
                        }
                        ')' if !in_quotes => {
                            paren_depth -= 1;
                            current_column.push(ch);
                        }
                        ',' if !in_quotes && paren_depth == 0 => {
                            if !current_column.trim().is_empty() {
                                if let Ok(column) = self.parse_column_definition(&current_column.trim(), boolean_fields) {
                                    columns.push(column);
                                }
                            }
                            current_column.clear();
                        }
                        _ => {
                            current_column.push(ch);
                        }
                    }
                }
                
                if !current_column.trim().is_empty() {
                    if let Ok(column) = self.parse_column_definition(&current_column.trim(), boolean_fields) {
                        columns.push(column);
                    }
                }
            }
        }
        
        Ok(columns)
    }

    /// Parse column definition (same as main implementation)
    fn parse_column_definition(&self, definition: &str, boolean_fields: Option<&HashSet<String>>) -> Result<ColumnSchema> {
        let def_upper = definition.to_uppercase();
        if def_upper.trim().starts_with("PRIMARY KEY") ||
           def_upper.trim().starts_with("FOREIGN KEY") ||
           def_upper.trim().starts_with("UNIQUE") ||
           def_upper.trim().starts_with("CHECK") ||
           def_upper.trim().starts_with("CONSTRAINT") {
            return Err(D1RsError::Database("Not a column definition".to_string()));
        }
        
        let parts: Vec<&str> = definition.split_whitespace().collect();
        if parts.is_empty() {
            return Err(D1RsError::Database("Empty column definition".to_string()));
        }
        
        let name = parts[0].trim_matches('"').trim_matches('\'').to_string();
        let column_type = if parts.len() > 1 {
            parts[1].to_uppercase()
        } else {
            "TEXT".to_string()
        };
        
        let primary_key = def_upper.contains("PRIMARY KEY");
        let nullable = !def_upper.contains("NOT NULL") && !primary_key;
        let auto_increment = def_upper.contains("AUTOINCREMENT");
        let unique = def_upper.contains("UNIQUE") && !primary_key;
        
        let default_value = if let Some(default_start) = def_upper.find("DEFAULT") {
            let after_default = &definition[default_start + 7..].trim();
            if let Some(space_pos) = after_default.find(' ') {
                Some(after_default[..space_pos].trim().to_string())
            } else {
                Some(after_default.trim().to_string())
            }
        } else {
            None
        };
        
        let final_type = if let Some(boolean_fields) = boolean_fields {
            if boolean_fields.contains(&name) && column_type == "INTEGER" {
                "BOOLEAN".to_string()
            } else {
                column_type
            }
        } else {
            column_type
        };
        
        Ok(ColumnSchema {
            name,
            column_type: final_type,
            nullable,
            default_value,
            primary_key,
            auto_increment,
            unique,
            constraints: Vec::new(),
        })
    }
}

/// Database schema representation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct DatabaseSchema {
    pub tables: Vec<TableSchema>,
    pub dialect: DatabaseDialect,
}

impl DatabaseSchema {
    /// Get a table by name
    pub fn get_table(&self, name: &str) -> Option<&TableSchema> {
        self.tables.iter().find(|t| t.name == name)
    }
    
    /// Check if a table exists
    pub fn has_table(&self, name: &str) -> bool {
        self.tables.iter().any(|t| t.name == name)
    }
    
    /// Get all table names
    pub fn table_names(&self) -> Vec<&str> {
        self.tables.iter().map(|t| t.name.as_str()).collect()
    }
}

/// Table schema representation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct TableSchema {
    pub name: String,
    pub columns: Vec<ColumnSchema>,
    pub indexes: Vec<IndexSchema>,
    pub foreign_keys: Vec<ForeignKeySchema>,
    pub constraints: Vec<ConstraintSchema>,
}

impl TableSchema {
    /// Get a column by name
    pub fn get_column(&self, name: &str) -> Option<&ColumnSchema> {
        self.columns.iter().find(|c| c.name == name)
    }
    
    /// Check if a column exists
    pub fn has_column(&self, name: &str) -> bool {
        self.columns.iter().any(|c| c.name == name)
    }
    
    /// Get primary key columns
    pub fn primary_key_columns(&self) -> Vec<&ColumnSchema> {
        self.columns.iter().filter(|c| c.primary_key).collect()
    }
}

/// Column schema representation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ColumnSchema {
    pub name: String,
    pub column_type: String,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub primary_key: bool,
    pub auto_increment: bool,
    pub unique: bool,
    pub constraints: Vec<ColumnConstraint>,
}

/// Index schema representation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct IndexSchema {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub table_name: Option<String>,
}

/// Foreign key relationship representation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ForeignKeySchema {
    pub name: String,
    pub columns: Vec<String>,
    pub referenced_table: String,
    pub referenced_columns: Vec<String>,
    pub on_delete: Option<String>,
    pub on_update: Option<String>,
}

/// Table constraint representation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ConstraintSchema {
    pub name: String,
    pub constraint_type: ConstraintType,
    pub columns: Vec<String>,
    pub definition: Option<String>,
}

/// Column-level constraint representation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum ColumnConstraint {
    /// Check constraint with expression
    Check { expression: String },
    /// References constraint (foreign key)
    References { table: String, column: String },
}

/// Types of database constraints
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum ConstraintType {
    PrimaryKey,
    ForeignKey,
    Unique,
    Check,
    NotNull,
    Default,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::D1Client;

    async fn create_test_db() -> D1Client {
        let db = D1Client::new_in_memory().await.unwrap();
        
        // Create a test table using sea-query for consistency
        let sql = r#"
            CREATE TABLE test_users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                email TEXT UNIQUE,
                is_active INTEGER DEFAULT 1
            )
        "#;
        db.execute(sql, &[]).await.unwrap();
        
        db
    }

    #[tokio::test]
    async fn test_database_agnostic_introspection() {
        let db = create_test_db().await;
        let adapter = D1ClientAdapter { client: &db };
        
        // Test basic functionality
        let tables = adapter.introspect_tables().await.unwrap();
        assert!(tables.contains(&"test_users".to_string()));
        
        let columns = adapter.introspect_columns("test_users").await.unwrap();
        assert_eq!(columns.len(), 4);
        
        let id_col = columns.iter().find(|c| c.name == "id").unwrap();
        assert!(id_col.primary_key);
        assert!(id_col.auto_increment);
        
        println!("✅ Database-agnostic introspector works correctly");
    }
}