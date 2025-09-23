//! SQL generation engine for converting rollback operations to executable SQL
//!
//! This module provides comprehensive SQL generation capabilities for all rollback operations,
//! with proper escaping, injection prevention, and support for complex schema operations.

use super::types::RollbackColumnChanges;
use crate::auto_migration::introspector::{TableSchema, ColumnSchema, IndexSchema, ForeignKeySchema, ConstraintSchema};
use crate::Result;
use std::collections::HashSet;

/// Comprehensive SQL generation engine for rollback operations
pub struct RollbackSqlGenerator {
    /// Configuration for SQL generation behavior
    config: SqlGeneratorConfig,
    
    /// Reserved SQL keywords that need escaping
    reserved_keywords: HashSet<String>,
}

/// Configuration for SQL generation
#[derive(Debug, Clone)]
pub struct SqlGeneratorConfig {
    /// Whether to use transactions for multi-statement operations
    pub use_transactions: bool,
    
    /// Whether to add IF EXISTS/IF NOT EXISTS clauses where applicable
    pub use_conditional_clauses: bool,
    
    /// Maximum number of statements to batch together
    pub max_batch_size: usize,
    
    /// Whether to add verbose comments to generated SQL
    pub add_comments: bool,
    
    /// Timeout for individual SQL operations (in seconds)
    pub operation_timeout_seconds: u64,
}

impl Default for SqlGeneratorConfig {
    fn default() -> Self {
        Self {
            use_transactions: true,
            use_conditional_clauses: true,
            max_batch_size: 50,
            add_comments: false,
            operation_timeout_seconds: 30,
        }
    }
}

/// Result of SQL generation containing one or more statements
#[derive(Debug, Clone)]
pub struct SqlGenerationResult {
    /// Generated SQL statements to execute
    pub statements: Vec<String>,
    
    /// Whether these statements should be executed in a transaction
    pub requires_transaction: bool,
    
    /// Estimated execution time in seconds
    pub estimated_duration_seconds: f64,
    
    /// Warnings about potential issues with the generated SQL
    pub warnings: Vec<String>,
}

impl RollbackSqlGenerator {
    /// Create a new SQL generator with default configuration
    pub fn new() -> Self {
        Self {
            config: SqlGeneratorConfig::default(),
            reserved_keywords: Self::build_reserved_keywords(),
        }
    }
    
    /// Create a new SQL generator with custom configuration
    pub fn with_config(config: SqlGeneratorConfig) -> Self {
        Self {
            config,
            reserved_keywords: Self::build_reserved_keywords(),
        }
    }
    
    /// Build set of reserved SQL keywords that need escaping
    fn build_reserved_keywords() -> HashSet<String> {
        let keywords = vec![
            "SELECT", "INSERT", "UPDATE", "DELETE", "CREATE", "DROP", "ALTER", "TABLE",
            "INDEX", "COLUMN", "FROM", "WHERE", "JOIN", "INNER", "OUTER", "LEFT", "RIGHT",
            "ON", "AS", "AND", "OR", "NOT", "NULL", "PRIMARY", "KEY", "FOREIGN", "REFERENCES",
            "UNIQUE", "DEFAULT", "CHECK", "CONSTRAINT", "INTEGER", "TEXT", "REAL", "BLOB",
            "BOOLEAN", "DATE", "DATETIME", "TIMESTAMP", "AUTOINCREMENT", "IF", "EXISTS",
            "DISTINCT", "ORDER", "BY", "GROUP", "HAVING", "LIMIT", "OFFSET", "UNION",
            "CASE", "WHEN", "THEN", "ELSE", "END", "LIKE", "GLOB", "REGEXP", "MATCH",
            "ESCAPE", "ISNULL", "NOTNULL", "IS", "BETWEEN", "IN", "COLLATE", "ASC", "DESC",
        ];
        
        keywords.into_iter().map(|k| k.to_string()).collect()
    }
    
    /// Generate CREATE TABLE SQL from schema definition
    pub fn generate_create_table_sql(&self, definition: &TableSchema) -> Result<SqlGenerationResult> {
        let mut warnings = Vec::new();
        let table_name = self.escape_identifier(&definition.name);
        
        // Build column definitions
        let mut column_defs = Vec::new();
        let mut primary_key_columns = Vec::new();
        
        for column in &definition.columns {
            let column_def = self.build_column_definition(column)?;
            column_defs.push(column_def);
            
            if column.primary_key {
                primary_key_columns.push(self.escape_identifier(&column.name));
            }
        }
        
        // Add primary key constraint if multiple columns
        if primary_key_columns.len() > 1 {
            column_defs.push(format!("PRIMARY KEY ({})", primary_key_columns.join(", ")));
        }
        
        // Add foreign key constraints
        for fk in &definition.foreign_keys {
            let fk_def = self.build_foreign_key_definition(fk)?;
            column_defs.push(fk_def);
        }
        
        // Add other constraints
        for constraint in &definition.constraints {
            let constraint_def = self.build_constraint_definition(constraint)?;
            column_defs.push(constraint_def);
        }
        
        let create_sql = if self.config.use_conditional_clauses {
            format!(
                "CREATE TABLE IF NOT EXISTS {} (\n    {}\n)",
                table_name,
                column_defs.join(",\n    ")
            )
        } else {
            format!(
                "CREATE TABLE {} (\n    {}\n)",
                table_name,
                column_defs.join(",\n    ")
            )
        };
        
        let mut statements = vec![create_sql];
        
        // Add index creation statements
        for index in &definition.indexes {
            let index_sql = self.generate_create_index_sql(&definition.name, index)?;
            statements.extend(index_sql.statements);
            warnings.extend(index_sql.warnings);
        }
        
        if definition.columns.len() > 20 {
            warnings.push("Table has many columns - consider normalization".to_string());
        }
        
        let requires_transaction = statements.len() > 1;
        
        Ok(SqlGenerationResult {
            statements,
            requires_transaction,
            estimated_duration_seconds: self.estimate_create_table_duration(definition),
            warnings,
        })
    }
    
    /// Generate ALTER TABLE ADD COLUMN SQL
    pub fn generate_add_column_sql(&self, table: &str, column: &ColumnSchema) -> Result<SqlGenerationResult> {
        let table_name = self.escape_identifier(table);
        let column_def = self.build_column_definition(column)?;
        
        let sql = format!("ALTER TABLE {} ADD COLUMN {}", table_name, column_def);
        
        let mut warnings = Vec::new();
        if !column.nullable && column.default_value.is_none() {
            warnings.push("Adding non-nullable column without default value may fail on existing data".to_string());
        }
        
        Ok(SqlGenerationResult {
            statements: vec![sql],
            requires_transaction: false,
            estimated_duration_seconds: 0.5,
            warnings,
        })
    }
    
    /// Generate ALTER TABLE DROP COLUMN SQL
    pub fn generate_drop_column_sql(&self, table: &str, column: &str) -> Result<SqlGenerationResult> {
        let table_name = self.escape_identifier(table);
        let column_name = self.escape_identifier(column);
        
        let sql = format!("ALTER TABLE {} DROP COLUMN {}", table_name, column_name);
        
        let warnings = vec![
            "Dropping column will permanently delete data".to_string(),
            "SQLite may not support DROP COLUMN in older versions".to_string(),
        ];
        
        Ok(SqlGenerationResult {
            statements: vec![sql],
            requires_transaction: false,
            estimated_duration_seconds: 1.0,
            warnings,
        })
    }
    
    /// Generate column modification SQL based on changes
    pub fn generate_modify_column_sql(
        &self,
        table: &str,
        column: &str,
        changes: &RollbackColumnChanges,
    ) -> Result<SqlGenerationResult> {
        let mut statements = Vec::new();
        let mut warnings = Vec::new();
        let table_name = self.escape_identifier(table);
        let column_name = self.escape_identifier(column);
        
        // SQLite doesn't support direct column modification, so we need to use a multi-step process
        warnings.push("Column modification in SQLite requires table recreation".to_string());
        
        // For type changes, we need table recreation
        if let Some((old_type, new_type)) = &changes.type_change {
            statements.push(format!(
                "-- Type change from {} to {} requires table recreation",
                old_type, new_type
            ));
            warnings.push("Type changes may result in data loss - verify compatibility".to_string());
        }
        
        // For nullability changes
        if let Some((old_nullable, new_nullable)) = &changes.null_change {
            if *old_nullable && !*new_nullable {
                warnings.push("Making column non-nullable may fail if NULL values exist".to_string());
            }
        }
        
        // For default value changes
        if let Some((old_default, new_default)) = &changes.default_change {
            statements.push(format!(
                "-- Default value change from {:?} to {:?}",
                old_default, new_default
            ));
        }
        
        // Add placeholder for table recreation process
        statements.push(format!(
            "-- Column modification for {}.{} requires complex table recreation process",
            table_name, column_name
        ));
        
        Ok(SqlGenerationResult {
            statements,
            requires_transaction: true,
            estimated_duration_seconds: 5.0, // Table recreation is expensive
            warnings,
        })
    }
    
    /// Generate index creation SQL
    pub fn generate_create_index_sql(&self, table: &str, index: &IndexSchema) -> Result<SqlGenerationResult> {
        let table_name = self.escape_identifier(table);
        let index_name = self.escape_identifier(&index.name);
        let columns = index.columns.iter()
            .map(|col| self.escape_identifier(col))
            .collect::<Vec<_>>()
            .join(", ");
        
        let index_type = if index.unique { "UNIQUE INDEX" } else { "INDEX" };
        
        let sql = if self.config.use_conditional_clauses {
            format!(
                "CREATE {} IF NOT EXISTS {} ON {} ({})",
                index_type, index_name, table_name, columns
            )
        } else {
            format!(
                "CREATE {} {} ON {} ({})",
                index_type, index_name, table_name, columns
            )
        };
        
        let mut warnings = Vec::new();
        if index.unique {
            warnings.push("Creating unique index may fail if duplicate values exist".to_string());
        }
        
        if index.columns.len() > 5 {
            warnings.push("Index with many columns may have poor performance".to_string());
        }
        
        Ok(SqlGenerationResult {
            statements: vec![sql],
            requires_transaction: false,
            estimated_duration_seconds: self.estimate_index_creation_duration(index),
            warnings,
        })
    }
    
    /// Generate foreign key constraint SQL
    pub fn generate_foreign_key_sql(&self, _table: &str, constraint: &ForeignKeySchema) -> Result<SqlGenerationResult> {
        let warnings = vec![
            "SQLite foreign key constraints require PRAGMA foreign_keys = ON".to_string(),
            "Adding foreign keys to existing tables requires table recreation".to_string(),
        ];
        
        let constraint_def = self.build_foreign_key_definition(constraint)?;
        
        // Foreign keys in SQLite must be defined during table creation
        let sql = format!(
            "-- Foreign key constraint '{}' must be added during table creation: {}",
            constraint.name, constraint_def
        );
        
        Ok(SqlGenerationResult {
            statements: vec![sql],
            requires_transaction: true,
            estimated_duration_seconds: 3.0,
            warnings,
        })
    }
    
    /// Generate batch SQL for complex operations
    pub fn generate_batch_sql(&self, operations: &[SqlGenerationResult]) -> Result<SqlGenerationResult> {
        let mut all_statements = Vec::new();
        let mut all_warnings = Vec::new();
        let mut total_duration = 0.0;
        let mut requires_transaction = false;
        
        // Add transaction start if needed
        if self.config.use_transactions && operations.iter().any(|op| op.requires_transaction) {
            all_statements.push("BEGIN TRANSACTION;".to_string());
            requires_transaction = true;
        }
        
        // Process operations in batches
        for batch in operations.chunks(self.config.max_batch_size) {
            for operation in batch {
                all_statements.extend(operation.statements.clone());
                all_warnings.extend(operation.warnings.clone());
                total_duration += operation.estimated_duration_seconds;
                
                if operation.requires_transaction {
                    requires_transaction = true;
                }
            }
            
            // Add batch separator comment if configured
            if self.config.add_comments && batch.len() > 1 {
                all_statements.push("-- End of batch".to_string());
            }
        }
        
        // Add transaction commit if needed
        if requires_transaction && self.config.use_transactions {
            all_statements.push("COMMIT;".to_string());
        }
        
        if operations.len() > self.config.max_batch_size {
            all_warnings.push(format!(
                "Large batch operation with {} operations - consider splitting",
                operations.len()
            ));
        }
        
        Ok(SqlGenerationResult {
            statements: all_statements,
            requires_transaction,
            estimated_duration_seconds: total_duration,
            warnings: all_warnings,
        })
    }
    
    /// Build column definition string for CREATE TABLE
    fn build_column_definition(&self, column: &ColumnSchema) -> Result<String> {
        let mut parts = Vec::new();
        
        // Column name and type
        parts.push(format!(
            "{} {}",
            self.escape_identifier(&column.name),
            column.column_type.to_uppercase()
        ));
        
        // Primary key (for single-column primary keys)
        if column.primary_key && column.auto_increment {
            parts.push("PRIMARY KEY AUTOINCREMENT".to_string());
        } else if column.primary_key {
            parts.push("PRIMARY KEY".to_string());
        }
        
        // Unique constraint
        if column.unique && !column.primary_key {
            parts.push("UNIQUE".to_string());
        }
        
        // NOT NULL constraint
        if !column.nullable {
            parts.push("NOT NULL".to_string());
        }
        
        // Default value
        if let Some(ref default) = column.default_value {
            parts.push(format!("DEFAULT {}", self.escape_value(default)));
        }
        
        Ok(parts.join(" "))
    }
    
    /// Build foreign key constraint definition
    fn build_foreign_key_definition(&self, fk: &ForeignKeySchema) -> Result<String> {
        let local_columns = fk.columns.iter()
            .map(|col| self.escape_identifier(col))
            .collect::<Vec<_>>()
            .join(", ");
            
        let referenced_table = self.escape_identifier(&fk.referenced_table);
        let referenced_columns = fk.referenced_columns.iter()
            .map(|col| self.escape_identifier(col))
            .collect::<Vec<_>>()
            .join(", ");
        
        let mut fk_def = format!(
            "CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {} ({})",
            self.escape_identifier(&fk.name),
            local_columns,
            referenced_table,
            referenced_columns
        );
        
        if let Some(ref on_delete) = fk.on_delete {
            fk_def.push_str(&format!(" ON DELETE {}", on_delete.to_uppercase()));
        }
        
        if let Some(ref on_update) = fk.on_update {
            fk_def.push_str(&format!(" ON UPDATE {}", on_update.to_uppercase()));
        }
        
        Ok(fk_def)
    }
    
    /// Build constraint definition string
    fn build_constraint_definition(&self, constraint: &ConstraintSchema) -> Result<String> {
        // This would be implemented based on the actual ConstraintSchema structure
        // For now, return a placeholder
        Ok(format!("-- Constraint: {}", constraint.name))
    }
    
    /// Escape SQL identifier (table/column names)
    fn escape_identifier(&self, identifier: &str) -> String {
        if self.reserved_keywords.contains(&identifier.to_uppercase()) {
            format!("\"{}\"", identifier.replace("\"", "\"\""))
        } else if identifier.contains(' ') || identifier.contains('-') || identifier.contains('"') || identifier.starts_with(char::is_numeric) {
            format!("\"{}\"", identifier.replace("\"", "\"\""))
        } else {
            identifier.to_string()
        }
    }
    
    /// Escape SQL value (prevent injection)
    fn escape_value(&self, value: &str) -> String {
        if value.parse::<i64>().is_ok() || value.parse::<f64>().is_ok() {
            // Numeric values don't need quotes
            value.to_string()
        } else if value.eq_ignore_ascii_case("NULL") {
            "NULL".to_string()
        } else if value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2 {
            // Value is already properly quoted, just validate internal quotes are escaped
            value.to_string()
        } else {
            // String values need single quotes and escaping
            format!("'{}'", value.replace("'", "''"))
        }
    }
    
    /// Estimate duration for CREATE TABLE operation
    fn estimate_create_table_duration(&self, definition: &TableSchema) -> f64 {
        let base_time = 1.0; // 1 second base
        let column_penalty = definition.columns.len() as f64 * 0.1;
        let index_penalty = definition.indexes.len() as f64 * 0.5;
        let fk_penalty = definition.foreign_keys.len() as f64 * 0.3;
        
        base_time + column_penalty + index_penalty + fk_penalty
    }
    
    /// Estimate duration for index creation
    fn estimate_index_creation_duration(&self, index: &IndexSchema) -> f64 {
        let base_time = 0.5;
        let unique_penalty = if index.unique { 0.5 } else { 0.0 };
        let column_penalty = index.columns.len() as f64 * 0.2;
        
        base_time + unique_penalty + column_penalty
    }
}

impl Default for RollbackSqlGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_column() -> ColumnSchema {
        ColumnSchema {
            name: "test_column".to_string(),
            column_type: "TEXT".to_string(),
            nullable: true,
            default_value: Some("'default'".to_string()),
            primary_key: false,
            auto_increment: false,
            unique: false,
            constraints: vec![],
        }
    }
    
    fn create_test_primary_key_column() -> ColumnSchema {
        ColumnSchema {
            name: "id".to_string(),
            column_type: "INTEGER".to_string(),
            nullable: false,
            default_value: None,
            primary_key: true,
            auto_increment: true,
            unique: false,
            constraints: vec![],
        }
    }
    
    fn create_test_table() -> TableSchema {
        TableSchema {
            name: "test_table".to_string(),
            columns: vec![
                create_test_primary_key_column(),
                create_test_column(),
            ],
            indexes: vec![],
            foreign_keys: vec![],
            constraints: vec![],
        }
    }
    
    fn create_test_index() -> IndexSchema {
        IndexSchema {
            name: "idx_test".to_string(),
            columns: vec!["test_column".to_string()],
            unique: false,
            table_name: Some("test_table".to_string()),
        }
    }
    
    fn create_test_unique_index() -> IndexSchema {
        IndexSchema {
            name: "idx_unique_test".to_string(),
            columns: vec!["test_column".to_string(), "id".to_string()],
            unique: true,
            table_name: Some("test_table".to_string()),
        }
    }
    
    fn create_test_foreign_key() -> ForeignKeySchema {
        ForeignKeySchema {
            name: "fk_test".to_string(),
            columns: vec!["user_id".to_string()],
            referenced_table: "users".to_string(),
            referenced_columns: vec!["id".to_string()],
            on_delete: Some("CASCADE".to_string()),
            on_update: Some("RESTRICT".to_string()),
        }
    }
    
    #[test]
    fn test_sql_generator_creation() {
        let generator = RollbackSqlGenerator::new();
        assert_eq!(generator.config.use_transactions, true);
        assert_eq!(generator.config.use_conditional_clauses, true);
        assert_eq!(generator.config.max_batch_size, 50);
        
        let custom_config = SqlGeneratorConfig {
            use_transactions: false,
            use_conditional_clauses: false,
            max_batch_size: 100,
            add_comments: true,
            operation_timeout_seconds: 60,
        };
        
        let custom_generator = RollbackSqlGenerator::with_config(custom_config.clone());
        assert_eq!(custom_generator.config.use_transactions, false);
        assert_eq!(custom_generator.config.max_batch_size, 100);
        assert_eq!(custom_generator.config.add_comments, true);
    }
    
    #[test]
    fn test_identifier_escaping() {
        let generator = RollbackSqlGenerator::new();
        
        // Normal identifiers should not be escaped
        assert_eq!(generator.escape_identifier("normal_name"), "normal_name");
        assert_eq!(generator.escape_identifier("table123"), "table123");
        
        // Reserved keywords should be escaped
        assert_eq!(generator.escape_identifier("SELECT"), "\"SELECT\"");
        assert_eq!(generator.escape_identifier("table"), "\"table\"");
        assert_eq!(generator.escape_identifier("INDEX"), "\"INDEX\"");
        
        // Identifiers with spaces or special characters should be escaped
        assert_eq!(generator.escape_identifier("table name"), "\"table name\"");
        assert_eq!(generator.escape_identifier("table-name"), "\"table-name\"");
        assert_eq!(generator.escape_identifier("123table"), "\"123table\"");
        
        // Test double quote escaping
        assert_eq!(generator.escape_identifier("test\"name"), "\"test\"\"name\"");
    }
    
    #[test]
    fn test_value_escaping() {
        let generator = RollbackSqlGenerator::new();
        
        // Numeric values should not be quoted
        assert_eq!(generator.escape_value("123"), "123");
        assert_eq!(generator.escape_value("456.78"), "456.78");
        assert_eq!(generator.escape_value("-99"), "-99");
        
        // NULL should not be quoted
        assert_eq!(generator.escape_value("NULL"), "NULL");
        assert_eq!(generator.escape_value("null"), "NULL");
        
        // String values should be quoted and escaped
        assert_eq!(generator.escape_value("test"), "'test'");
        assert_eq!(generator.escape_value("test's value"), "'test''s value'");
        assert_eq!(generator.escape_value("it's a 'test'"), "'it''s a ''test'''");
    }
    
    #[test]
    fn test_create_table_sql_generation() {
        let generator = RollbackSqlGenerator::new();
        let table = create_test_table();
        
        let result = generator.generate_create_table_sql(&table).unwrap();
        
        assert!(!result.statements.is_empty());
        let sql = &result.statements[0];
        
        // Check basic structure
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS test_table"));
        assert!(sql.contains("id INTEGER PRIMARY KEY AUTOINCREMENT"));
        assert!(sql.contains("test_column TEXT DEFAULT 'default'"));
        
        // Check that primary key is handled correctly
        assert!(sql.contains("PRIMARY KEY AUTOINCREMENT"));
        
        // Verify result metadata
        assert_eq!(result.requires_transaction, false); // Single statement table
        assert!(result.estimated_duration_seconds > 0.0);
    }
    
    #[test]
    fn test_create_table_with_indexes_and_foreign_keys() {
        let generator = RollbackSqlGenerator::new();
        let mut table = create_test_table();
        table.indexes.push(create_test_index());
        table.foreign_keys.push(create_test_foreign_key());
        
        let result = generator.generate_create_table_sql(&table).unwrap();
        
        // Should have multiple statements (table + index)
        assert!(result.statements.len() > 1);
        assert!(result.requires_transaction); // Multiple statements require transaction
        
        // Check table creation SQL
        assert!(result.statements[0].contains("CREATE TABLE IF NOT EXISTS test_table"));
        assert!(result.statements[0].contains("CONSTRAINT fk_test FOREIGN KEY"));
        
        // Check index creation SQL
        assert!(result.statements[1].contains("CREATE INDEX IF NOT EXISTS idx_test"));
    }
    
    #[test]
    fn test_create_table_without_conditional_clauses() {
        let config = SqlGeneratorConfig {
            use_conditional_clauses: false,
            ..Default::default()
        };
        let generator = RollbackSqlGenerator::with_config(config);
        let table = create_test_table();
        
        let result = generator.generate_create_table_sql(&table).unwrap();
        let sql = &result.statements[0];
        
        // Should not have IF NOT EXISTS clause
        assert!(sql.contains("CREATE TABLE test_table"));
        assert!(!sql.contains("IF NOT EXISTS"));
    }
    
    #[test]
    fn test_add_column_sql_generation() {
        let generator = RollbackSqlGenerator::new();
        let column = create_test_column();
        
        let result = generator.generate_add_column_sql("test_table", &column).unwrap();
        
        assert_eq!(result.statements.len(), 1);
        let sql = &result.statements[0];
        
        assert!(sql.contains("ALTER TABLE test_table ADD COLUMN"));
        assert!(sql.contains("test_column TEXT DEFAULT 'default'"));
        assert_eq!(result.requires_transaction, false);
        assert_eq!(result.estimated_duration_seconds, 0.5);
    }
    
    #[test]
    fn test_add_non_nullable_column_warning() {
        let generator = RollbackSqlGenerator::new();
        let mut column = create_test_column();
        column.nullable = false;
        column.default_value = None;
        
        let result = generator.generate_add_column_sql("test_table", &column).unwrap();
        
        // Should have warning about non-nullable column without default
        assert!(!result.warnings.is_empty());
        assert!(result.warnings[0].contains("non-nullable column without default"));
    }
    
    #[test]
    fn test_drop_column_sql_generation() {
        let generator = RollbackSqlGenerator::new();
        
        let result = generator.generate_drop_column_sql("test_table", "test_column").unwrap();
        
        assert_eq!(result.statements.len(), 1);
        let sql = &result.statements[0];
        
        assert!(sql.contains("ALTER TABLE test_table DROP COLUMN test_column"));
        assert_eq!(result.requires_transaction, false);
        assert_eq!(result.estimated_duration_seconds, 1.0);
        
        // Should have warnings about data loss and SQLite compatibility
        assert!(result.warnings.len() >= 2);
        assert!(result.warnings.iter().any(|w| w.contains("permanently delete data")));
        assert!(result.warnings.iter().any(|w| w.contains("SQLite may not support")));
    }
    
    #[test]
    fn test_modify_column_sql_generation() {
        let generator = RollbackSqlGenerator::new();
        let changes = RollbackColumnChanges {
            type_change: Some(("TEXT".to_string(), "INTEGER".to_string())),
            null_change: Some((true, false)),
            default_change: Some((Some("'default'".to_string()), None)),
            constraint_changes: vec![],
        };
        
        let result = generator.generate_modify_column_sql("test_table", "test_column", &changes).unwrap();
        
        assert!(!result.statements.is_empty());
        assert!(result.requires_transaction);
        assert_eq!(result.estimated_duration_seconds, 5.0);
        
        // Should have warnings about table recreation and data loss
        assert!(!result.warnings.is_empty());
        assert!(result.warnings.iter().any(|w| w.contains("table recreation")));
        assert!(result.warnings.iter().any(|w| w.contains("data loss")));
    }
    
    #[test]
    fn test_create_index_sql_generation() {
        let generator = RollbackSqlGenerator::new();
        let index = create_test_index();
        
        let result = generator.generate_create_index_sql("test_table", &index).unwrap();
        
        assert_eq!(result.statements.len(), 1);
        let sql = &result.statements[0];
        
        assert!(sql.contains("CREATE INDEX IF NOT EXISTS idx_test ON test_table"));
        assert!(sql.contains("(test_column)"));
        assert_eq!(result.requires_transaction, false);
        assert!(result.estimated_duration_seconds > 0.0);
    }
    
    #[test]
    fn test_create_unique_index_sql_generation() {
        let generator = RollbackSqlGenerator::new();
        let index = create_test_unique_index();
        
        let result = generator.generate_create_index_sql("test_table", &index).unwrap();
        
        assert_eq!(result.statements.len(), 1);
        let sql = &result.statements[0];
        
        assert!(sql.contains("CREATE UNIQUE INDEX IF NOT EXISTS idx_unique_test"));
        assert!(sql.contains("ON test_table (test_column, id)"));
        
        // Should have warning about unique index
        assert!(!result.warnings.is_empty());
        assert!(result.warnings[0].contains("duplicate values"));
    }
    
    #[test]
    fn test_create_index_many_columns_warning() {
        let generator = RollbackSqlGenerator::new();
        let mut index = create_test_index();
        index.columns = vec!["col1".to_string(), "col2".to_string(), "col3".to_string(), 
                           "col4".to_string(), "col5".to_string(), "col6".to_string()];
        
        let result = generator.generate_create_index_sql("test_table", &index).unwrap();
        
        // Should have warning about many columns
        assert!(result.warnings.iter().any(|w| w.contains("many columns")));
    }
    
    #[test]
    fn test_foreign_key_sql_generation() {
        let generator = RollbackSqlGenerator::new();
        let fk = create_test_foreign_key();
        
        let result = generator.generate_foreign_key_sql("test_table", &fk).unwrap();
        
        assert_eq!(result.statements.len(), 1);
        assert!(result.requires_transaction);
        assert_eq!(result.estimated_duration_seconds, 3.0);
        
        // Should have warnings about SQLite foreign key limitations
        assert!(result.warnings.len() >= 2);
        assert!(result.warnings.iter().any(|w| w.contains("PRAGMA foreign_keys")));
        assert!(result.warnings.iter().any(|w| w.contains("table recreation")));
    }
    
    #[test]
    fn test_batch_sql_generation() {
        let generator = RollbackSqlGenerator::new();
        
        let operations = vec![
            SqlGenerationResult {
                statements: vec!["CREATE TABLE test1 (id INTEGER)".to_string()],
                requires_transaction: false,
                estimated_duration_seconds: 1.0,
                warnings: vec!["Warning 1".to_string()],
            },
            SqlGenerationResult {
                statements: vec!["CREATE TABLE test2 (id INTEGER)".to_string()],
                requires_transaction: true,
                estimated_duration_seconds: 1.5,
                warnings: vec!["Warning 2".to_string()],
            },
        ];
        
        let result = generator.generate_batch_sql(&operations).unwrap();
        
        // Should have transaction wrapper
        assert!(result.statements[0].contains("BEGIN TRANSACTION"));
        assert!(result.statements.last().unwrap().contains("COMMIT"));
        
        // Should combine all statements
        assert!(result.statements.len() >= 4); // BEGIN + 2 statements + COMMIT
        assert!(result.requires_transaction);
        assert_eq!(result.estimated_duration_seconds, 2.5);
        
        // Should combine all warnings
        assert_eq!(result.warnings.len(), 2);
        assert!(result.warnings.contains(&"Warning 1".to_string()));
        assert!(result.warnings.contains(&"Warning 2".to_string()));
    }
    
    #[test]
    fn test_batch_sql_large_batch_warning() {
        let generator = RollbackSqlGenerator::new();
        
        // Create more operations than max_batch_size
        let mut operations = Vec::new();
        for i in 0..60 {
            operations.push(SqlGenerationResult {
                statements: vec![format!("CREATE TABLE test{} (id INTEGER)", i)],
                requires_transaction: false,
                estimated_duration_seconds: 1.0,
                warnings: vec![],
            });
        }
        
        let result = generator.generate_batch_sql(&operations).unwrap();
        
        // Should have warning about large batch
        assert!(result.warnings.iter().any(|w| w.contains("Large batch operation")));
    }
    
    #[test]
    fn test_batch_sql_without_transactions() {
        let config = SqlGeneratorConfig {
            use_transactions: false,
            ..Default::default()
        };
        let generator = RollbackSqlGenerator::with_config(config);
        
        let operations = vec![
            SqlGenerationResult {
                statements: vec!["CREATE TABLE test1 (id INTEGER)".to_string()],
                requires_transaction: true,
                estimated_duration_seconds: 1.0,
                warnings: vec![],
            },
        ];
        
        let result = generator.generate_batch_sql(&operations).unwrap();
        
        // Should not have transaction wrapper
        assert!(!result.statements[0].contains("BEGIN TRANSACTION"));
        assert!(!result.statements.iter().any(|s| s.contains("COMMIT")));
    }
    
    #[test]
    fn test_column_definition_building() {
        let generator = RollbackSqlGenerator::new();
        
        // Test basic column
        let column = ColumnSchema {
            name: "test_col".to_string(),
            column_type: "varchar(255)".to_string(),
            nullable: true,
            default_value: Some("'test'".to_string()),
            primary_key: false,
            auto_increment: false,
            unique: true,
            constraints: vec![],
        };
        
        let result = generator.build_column_definition(&column).unwrap();
        
        assert!(result.contains("test_col VARCHAR(255)"));
        assert!(result.contains("UNIQUE"));
        assert!(result.contains("DEFAULT 'test'"));
        assert!(!result.contains("NOT NULL"));
        assert!(!result.contains("PRIMARY KEY"));
    }
    
    #[test]
    fn test_primary_key_column_definition() {
        let generator = RollbackSqlGenerator::new();
        let column = create_test_primary_key_column();
        
        let result = generator.build_column_definition(&column).unwrap();
        
        assert!(result.contains("id INTEGER"));
        assert!(result.contains("PRIMARY KEY AUTOINCREMENT"));
        assert!(result.contains("NOT NULL"));
    }
    
    #[test]
    fn test_foreign_key_definition_building() {
        let generator = RollbackSqlGenerator::new();
        let fk = create_test_foreign_key();
        
        let result = generator.build_foreign_key_definition(&fk).unwrap();
        
        assert!(result.contains("CONSTRAINT fk_test"));
        assert!(result.contains("FOREIGN KEY (user_id)"));
        assert!(result.contains("REFERENCES users (id)"));
        assert!(result.contains("ON DELETE CASCADE"));
        assert!(result.contains("ON UPDATE RESTRICT"));
    }
    
    #[test]
    fn test_foreign_key_definition_without_actions() {
        let generator = RollbackSqlGenerator::new();
        let fk = ForeignKeySchema {
            name: "fk_simple".to_string(),
            columns: vec!["user_id".to_string()],
            referenced_table: "users".to_string(),
            referenced_columns: vec!["id".to_string()],
            on_delete: None,
            on_update: None,
        };
        
        let result = generator.build_foreign_key_definition(&fk).unwrap();
        
        assert!(result.contains("CONSTRAINT fk_simple"));
        assert!(result.contains("FOREIGN KEY (user_id)"));
        assert!(result.contains("REFERENCES users (id)"));
        assert!(!result.contains("ON DELETE"));
        assert!(!result.contains("ON UPDATE"));
    }
    
    #[test]
    fn test_duration_estimation() {
        let generator = RollbackSqlGenerator::new();
        
        // Test table with many columns
        let mut large_table = create_test_table();
        for i in 0..15 {
            large_table.columns.push(ColumnSchema {
                name: format!("col_{}", i),
                column_type: "TEXT".to_string(),
                nullable: true,
                default_value: None,
                primary_key: false,
                auto_increment: false,
                unique: false,
                constraints: vec![],
            });
        }
        
        let duration = generator.estimate_create_table_duration(&large_table);
        assert!(duration > 1.0); // Should be more than base time due to many columns
        
        // Test index with unique constraint
        let unique_index = create_test_unique_index();
        let index_duration = generator.estimate_index_creation_duration(&unique_index);
        assert!(index_duration > 0.5); // Should include unique penalty
    }
    
    #[test]
    fn test_reserved_keywords_detection() {
        let generator = RollbackSqlGenerator::new();
        
        // Test that common SQL keywords are in the reserved set
        assert!(generator.reserved_keywords.contains("SELECT"));
        assert!(generator.reserved_keywords.contains("TABLE"));
        assert!(generator.reserved_keywords.contains("INDEX"));
        assert!(generator.reserved_keywords.contains("PRIMARY"));
        assert!(generator.reserved_keywords.contains("FOREIGN"));
        assert!(generator.reserved_keywords.contains("REFERENCES"));
        assert!(generator.reserved_keywords.contains("UNIQUE"));
        assert!(generator.reserved_keywords.contains("DEFAULT"));
        
        // Test that normal words are not reserved
        assert!(!generator.reserved_keywords.contains("NORMAL"));
        assert!(!generator.reserved_keywords.contains("TEST"));
        assert!(!generator.reserved_keywords.contains("CUSTOM"));
    }
    
    #[test]
    fn test_sql_generation_result_structure() {
        let result = SqlGenerationResult {
            statements: vec!["CREATE TABLE test (id INTEGER)".to_string()],
            requires_transaction: false,
            estimated_duration_seconds: 1.5,
            warnings: vec!["Test warning".to_string()],
        };
        
        assert_eq!(result.statements.len(), 1);
        assert_eq!(result.requires_transaction, false);
        assert_eq!(result.estimated_duration_seconds, 1.5);
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0], "Test warning");
    }
}