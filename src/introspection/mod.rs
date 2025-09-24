use crate::backends::DatabaseBackend;
use crate::dialects::DatabaseDialect;
use crate::auto_migration::introspector::{
    DatabaseSchema, TableSchema, ColumnSchema, IndexSchema, ForeignKeySchema, ConstraintSchema
};
use crate::D1RsError;
use async_trait::async_trait;
use std::fmt;

/// Database-agnostic trait for schema introspection
/// 
/// This trait provides a unified interface for inspecting database schemas across different
/// database backends (SQLite, PostgreSQL, MySQL). Each backend implements this trait using
/// database-specific mechanisms while providing a consistent API.
/// 
/// The trait replaces raw SQL queries with sea-query builders and provides the foundation
/// for database-agnostic schema operations in the ORM.
/// 
/// # Design Principles
/// 
/// - **Database Agnostic**: Works across SQLite, PostgreSQL, and MySQL
/// - **Sea-Query Only**: All queries use sea-query builders, no raw SQL
/// - **Type Safe**: Leverages Rust's type system for compile-time safety
/// - **Entity Aware**: Supports Entity::boolean_fields() for accurate type detection
/// - **Performance Optimized**: Efficient queries with minimal database round-trips
/// 
/// # Implementation Strategy
/// 
/// Different databases use different introspection mechanisms:
/// - **SQLite**: Uses PRAGMA functions (pragma_table_info, pragma_index_list, etc.)
/// - **PostgreSQL**: Uses information_schema views and pg_* system tables
/// - **MySQL**: Uses information_schema views and SHOW commands
/// 
/// Each implementation uses sea-query builders to generate the appropriate SQL for
/// the target database dialect.
#[async_trait]
pub trait SchemaIntrospector<B: DatabaseBackend> {
    /// Error type for schema introspection operations
    /// 
    /// Each implementation can define its own error type that implements
    /// the required traits for proper error handling.
    type Error: std::error::Error + Send + Sync + 'static + fmt::Debug + fmt::Display;

    /// Get the database dialect this introspector works with
    /// 
    /// This allows for dialect-specific optimizations and SQL generation.
    fn dialect(&self) -> DatabaseDialect;

    /// List all table names in the database
    /// 
    /// Returns a list of user-defined table names, excluding system tables.
    /// The implementation should filter out database-specific system tables
    /// while preserving application tables including migration tables.
    /// 
    /// # Examples
    /// ```
    /// let tables = introspector.list_tables().await?;
    /// assert!(tables.contains(&"users".to_string()));
    /// assert!(tables.contains(&"posts".to_string()));
    /// ```
    async fn list_tables(&self) -> std::result::Result<Vec<String>, Self::Error>;

    /// Get complete schema information for a specific table
    /// 
    /// Returns detailed schema information including columns, indexes, foreign keys,
    /// and table-level constraints. This is the primary method for getting complete
    /// table structure information.
    /// 
    /// # Arguments
    /// * `table_name` - Name of the table to introspect
    /// 
    /// # Examples
    /// ```
    /// let schema = introspector.describe_table("users").await?;
    /// assert_eq!(schema.name, "users");
    /// assert!(!schema.columns.is_empty());
    /// ```
    async fn describe_table(&self, table_name: &str) -> std::result::Result<TableSchema, Self::Error>;

    /// List all columns for a specific table
    /// 
    /// Returns detailed column information including types, constraints, defaults,
    /// and metadata. The implementation should provide accurate type information
    /// and handle database-specific type variations.
    /// 
    /// # Arguments
    /// * `table_name` - Name of the table to introspect
    /// 
    /// # Examples
    /// ```
    /// let columns = introspector.list_columns("users").await?;
    /// let id_col = columns.iter().find(|c| c.name == "id").unwrap();
    /// assert!(id_col.primary_key);
    /// ```
    async fn list_columns(&self, table_name: &str) -> std::result::Result<Vec<ColumnSchema>, Self::Error>;

    /// List all indexes for a specific table
    /// 
    /// Returns information about all indexes defined on the table, including
    /// unique constraints, composite indexes, and partial indexes where supported.
    /// 
    /// # Arguments
    /// * `table_name` - Name of the table to introspect
    /// 
    /// # Examples
    /// ```
    /// let indexes = introspector.list_indexes("users").await?;
    /// let email_idx = indexes.iter().find(|i| i.columns.contains(&"email".to_string())).unwrap();
    /// assert!(email_idx.unique);
    /// ```
    async fn list_indexes(&self, table_name: &str) -> std::result::Result<Vec<IndexSchema>, Self::Error>;

    /// List all foreign key constraints for a specific table
    /// 
    /// Returns information about foreign key relationships, including referenced
    /// tables, columns, and referential actions (CASCADE, RESTRICT, etc.).
    /// 
    /// # Arguments
    /// * `table_name` - Name of the table to introspect
    /// 
    /// # Examples
    /// ```
    /// let foreign_keys = introspector.list_foreign_keys("posts").await?;
    /// let user_fk = foreign_keys.iter().find(|fk| fk.referenced_table == "users").unwrap();
    /// assert_eq!(fk.columns, vec!["user_id"]);
    /// ```
    async fn list_foreign_keys(&self, table_name: &str) -> std::result::Result<Vec<ForeignKeySchema>, Self::Error>;

    /// List table-level constraints for a specific table
    /// 
    /// Returns information about table-level constraints such as CHECK constraints,
    /// composite UNIQUE constraints, and other table-wide constraints not covered
    /// by column-level introspection.
    /// 
    /// # Arguments
    /// * `table_name` - Name of the table to introspect
    /// 
    /// # Examples
    /// ```
    /// let constraints = introspector.list_constraints("users").await?;
    /// let check_age = constraints.iter().find(|c| c.name == "check_age_positive").unwrap();
    /// assert_eq!(c.constraint_type, ConstraintType::Check);
    /// ```
    async fn list_constraints(&self, table_name: &str) -> std::result::Result<Vec<ConstraintSchema>, Self::Error>;

    /// Get complete database schema including all tables
    /// 
    /// This is a convenience method that introspects the entire database and
    /// returns a complete DatabaseSchema structure. It internally calls the
    /// other methods to build up the full schema representation.
    /// 
    /// # Performance Note
    /// This method can be expensive for large databases as it introspects all
    /// tables. Consider using the individual methods for specific tables when
    /// performance is critical.
    /// 
    /// # Examples
    /// ```
    /// let db_schema = introspector.introspect_database().await?;
    /// assert!(!db_schema.tables.is_empty());
    /// let users_table = db_schema.tables.iter().find(|t| t.name == "users").unwrap();
    /// assert!(!users_table.columns.is_empty());
    /// ```
    async fn introspect_database(&self) -> std::result::Result<DatabaseSchema, Self::Error> {
        let table_names = self.list_tables().await?;
        let mut tables = Vec::with_capacity(table_names.len());
        
        for table_name in table_names {
            let table_schema = self.describe_table(&table_name).await?;
            tables.push(table_schema);
        }
        
        Ok(DatabaseSchema { tables })
    }

    /// Entity-aware column introspection with boolean field detection
    /// 
    /// This method enhances standard column introspection by using Entity metadata
    /// to provide more accurate type information, particularly for boolean fields
    /// which may be stored as integers in some databases (e.g., SQLite).
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
    /// // active field is correctly identified as boolean even if stored as INTEGER
    /// ```
    async fn list_columns_for_entity<T>(&self, table_name: &str) -> std::result::Result<Vec<ColumnSchema>, Self::Error>
    where
        T: crate::Entity + Send + Sync,
    {
        let mut columns = self.list_columns(table_name).await?;
        
        // Apply Entity-aware boolean field detection
        let boolean_fields = T::boolean_fields();
        for column in &mut columns {
            if boolean_fields.contains(&column.name.as_str()) {
                // Override column type for boolean fields
                // This handles SQLite's INTEGER storage for booleans
                column.column_type = "BOOLEAN".to_string();
            }
        }
        
        Ok(columns)
    }

    /// Validate table existence
    /// 
    /// Efficient check for table existence without full introspection.
    /// Useful for validation and error handling.
    /// 
    /// # Arguments
    /// * `table_name` - Name of the table to check
    /// 
    /// # Examples
    /// ```
    /// let exists = introspector.table_exists("users").await?;
    /// assert!(exists);
    /// ```
    async fn table_exists(&self, table_name: &str) -> std::result::Result<bool, Self::Error> {
        let tables = self.list_tables().await?;
        Ok(tables.contains(&table_name.to_string()))
    }

    /// Check if a specific column exists in a table
    /// 
    /// Efficient check for column existence without full table introspection.
    /// 
    /// # Arguments
    /// * `table_name` - Name of the table
    /// * `column_name` - Name of the column to check
    /// 
    /// # Examples
    /// ```
    /// let exists = introspector.column_exists("users", "email").await?;
    /// assert!(exists);
    /// ```
    async fn column_exists(&self, table_name: &str, column_name: &str) -> std::result::Result<bool, Self::Error> {
        let columns = self.list_columns(table_name).await?;
        Ok(columns.iter().any(|c| c.name == column_name))
    }

    /// Get database-specific metadata
    /// 
    /// Returns database-specific information such as version, capabilities,
    /// and configuration that may be useful for schema operations.
    /// 
    /// # Examples
    /// ```
    /// let metadata = introspector.get_database_metadata().await?;
    /// let version = metadata.get("version").unwrap();
    /// ```
    async fn get_database_metadata(&self) -> std::result::Result<std::collections::HashMap<String, String>, Self::Error>;
}

/// Error type for generic schema introspection operations
/// 
/// This error type is used for operations that don't depend on a specific
/// backend implementation.
#[derive(Debug, Clone)]
pub struct IntrospectionError {
    pub message: String,
    pub kind: IntrospectionErrorKind,
}

/// Categories of introspection errors
#[derive(Debug, Clone, PartialEq)]
pub enum IntrospectionErrorKind {
    /// Database connection or query execution failed
    DatabaseError,
    /// Table or schema element not found
    NotFound,
    /// Invalid schema structure or data
    InvalidSchema,
    /// Unsupported operation for this database
    Unsupported,
    /// Configuration or setup error
    Configuration,
}

impl fmt::Display for IntrospectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl fmt::Display for IntrospectionErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntrospectionErrorKind::DatabaseError => write!(f, "Database Error"),
            IntrospectionErrorKind::NotFound => write!(f, "Not Found"),
            IntrospectionErrorKind::InvalidSchema => write!(f, "Invalid Schema"),
            IntrospectionErrorKind::Unsupported => write!(f, "Unsupported"),
            IntrospectionErrorKind::Configuration => write!(f, "Configuration Error"),
        }
    }
}

impl std::error::Error for IntrospectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl From<D1RsError> for IntrospectionError {
    fn from(err: D1RsError) -> Self {
        IntrospectionError {
            message: err.to_string(),
            kind: IntrospectionErrorKind::DatabaseError,
        }
    }
}

impl From<serde_json::Error> for IntrospectionError {
    fn from(err: serde_json::Error) -> Self {
        IntrospectionError {
            message: format!("JSON parsing error: {}", err),
            kind: IntrospectionErrorKind::InvalidSchema,
        }
    }
}

/// SQLite schema introspector implementation
pub mod sqlite;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "mysql")]
pub mod mysql;

/// Utility functions for schema introspection
pub mod utils {
    use super::*;
    
    /// Normalize column type names across databases
    /// 
    /// Different databases use different type names for similar concepts.
    /// This function normalizes them to a common set of type names.
    /// 
    /// # Arguments
    /// * `database_type` - Raw type name from database
    /// * `dialect` - Database dialect for context
    /// 
    /// # Examples
    /// ```
    /// let normalized = normalize_column_type("VARCHAR(255)", DatabaseDialect::PostgreSQL);
    /// assert_eq!(normalized, "TEXT");
    /// ```
    pub fn normalize_column_type(database_type: &str, dialect: DatabaseDialect) -> String {
        let upper_type = database_type.to_uppercase();
        
        match dialect {
            DatabaseDialect::SQLite => {
                if upper_type.starts_with("VARCHAR") || upper_type.starts_with("TEXT") {
                    "TEXT".to_string()
                } else if upper_type.starts_with("INTEGER") || upper_type.starts_with("INT") {
                    "INTEGER".to_string()
                } else if upper_type.starts_with("REAL") || upper_type.starts_with("FLOAT") || upper_type.starts_with("DOUBLE") {
                    "REAL".to_string()
                } else if upper_type.starts_with("BLOB") {
                    "BLOB".to_string()
                } else if upper_type.starts_with("BOOLEAN") || upper_type.starts_with("BOOL") {
                    "BOOLEAN".to_string()
                } else {
                    database_type.to_string()
                }
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                if upper_type.starts_with("VARCHAR") || upper_type.starts_with("TEXT") || upper_type.starts_with("CHAR") {
                    "TEXT".to_string()
                } else if upper_type.starts_with("INTEGER") || upper_type.starts_with("INT") || upper_type.starts_with("SERIAL") {
                    "INTEGER".to_string()
                } else if upper_type.starts_with("REAL") || upper_type.starts_with("FLOAT") || upper_type.starts_with("DOUBLE") || upper_type.starts_with("NUMERIC") {
                    "REAL".to_string()
                } else if upper_type.starts_with("BYTEA") {
                    "BLOB".to_string()
                } else if upper_type.starts_with("BOOLEAN") || upper_type.starts_with("BOOL") {
                    "BOOLEAN".to_string()
                } else if upper_type.starts_with("TIMESTAMP") || upper_type.starts_with("DATE") || upper_type.starts_with("TIME") {
                    "DATETIME".to_string()
                } else {
                    database_type.to_string()
                }
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                if upper_type.starts_with("VARCHAR") || upper_type.starts_with("TEXT") || upper_type.starts_with("CHAR") {
                    "TEXT".to_string()
                } else if upper_type.starts_with("INT") || upper_type.starts_with("BIGINT") || upper_type.starts_with("SMALLINT") || upper_type.starts_with("TINYINT") {
                    "INTEGER".to_string()
                } else if upper_type.starts_with("FLOAT") || upper_type.starts_with("DOUBLE") || upper_type.starts_with("DECIMAL") {
                    "REAL".to_string()
                } else if upper_type.starts_with("BLOB") || upper_type.starts_with("BINARY") {
                    "BLOB".to_string()
                } else if upper_type.starts_with("BOOLEAN") || upper_type.starts_with("BOOL") || upper_type == "TINYINT(1)" {
                    "BOOLEAN".to_string()
                } else if upper_type.starts_with("TIMESTAMP") || upper_type.starts_with("DATETIME") || upper_type.starts_with("DATE") || upper_type.starts_with("TIME") {
                    "DATETIME".to_string()
                } else {
                    database_type.to_string()
                }
            },
        }
    }

    /// Extract table name from a fully qualified name
    /// 
    /// Handles schema-qualified table names by extracting just the table name.
    /// 
    /// # Examples
    /// ```
    /// let table_name = extract_table_name("public.users");
    /// assert_eq!(table_name, "users");
    /// ```
    pub fn extract_table_name(qualified_name: &str) -> &str {
        qualified_name.split('.').last().unwrap_or(qualified_name)
    }

    /// Check if a table name represents a system table
    /// 
    /// Different databases have different system table naming conventions.
    /// This function identifies system tables that should be filtered out
    /// during introspection.
    /// 
    /// # Arguments
    /// * `table_name` - Name of the table to check
    /// * `dialect` - Database dialect for context
    /// 
    /// # Examples
    /// ```
    /// let is_system = is_system_table("sqlite_master", DatabaseDialect::SQLite);
    /// assert!(is_system);
    /// ```
    pub fn is_system_table(table_name: &str, dialect: DatabaseDialect) -> bool {
        match dialect {
            DatabaseDialect::SQLite => {
                table_name.starts_with("sqlite_") || table_name.starts_with("_")
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                table_name.starts_with("pg_") || 
                table_name.starts_with("information_schema") ||
                table_name == "sql_features" ||
                table_name == "sql_implementation_info" ||
                table_name == "sql_parts" ||
                table_name == "sql_sizing"
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                table_name.starts_with("INFORMATION_SCHEMA") ||
                table_name.starts_with("performance_schema") ||
                table_name.starts_with("sys") ||
                table_name.starts_with("mysql")
            },
        }
    }

    /// Parse constraint definition from SQL
    /// 
    /// Extracts constraint information from SQL CREATE statements.
    /// This is used for table-level constraint introspection.
    /// 
    /// # Arguments
    /// * `sql` - SQL CREATE TABLE statement
    /// * `constraint_name` - Name of the constraint to find
    /// 
    /// # Examples
    /// ```
    /// let constraint = parse_constraint_from_sql(
    ///     "CREATE TABLE users (id INTEGER, CHECK (age > 0))",
    ///     "check_age"
    /// );
    /// ```
    pub fn parse_constraint_from_sql(sql: &str, constraint_name: &str) -> Option<String> {
        // Simple constraint parsing - this could be enhanced with a proper SQL parser
        if sql.to_uppercase().contains(&constraint_name.to_uppercase()) {
            // Extract the constraint definition
            // This is a simplified implementation - real implementation would use a SQL parser
            Some(format!("CONSTRAINT {} extracted from SQL", constraint_name))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialects::DatabaseDialect;
    use serde_json::Value;

    #[test]
    fn test_normalize_column_type_sqlite() {
        assert_eq!(utils::normalize_column_type("VARCHAR(255)", DatabaseDialect::SQLite), "TEXT");
        assert_eq!(utils::normalize_column_type("INTEGER", DatabaseDialect::SQLite), "INTEGER");
        assert_eq!(utils::normalize_column_type("REAL", DatabaseDialect::SQLite), "REAL");
        assert_eq!(utils::normalize_column_type("BLOB", DatabaseDialect::SQLite), "BLOB");
        assert_eq!(utils::normalize_column_type("BOOLEAN", DatabaseDialect::SQLite), "BOOLEAN");
    }

    #[test]
    #[cfg(feature = "postgres")]
    fn test_normalize_column_type_postgresql() {
        assert_eq!(utils::normalize_column_type("VARCHAR(255)", DatabaseDialect::PostgreSQL), "TEXT");
        assert_eq!(utils::normalize_column_type("INTEGER", DatabaseDialect::PostgreSQL), "INTEGER");
        assert_eq!(utils::normalize_column_type("SERIAL", DatabaseDialect::PostgreSQL), "INTEGER");
        assert_eq!(utils::normalize_column_type("REAL", DatabaseDialect::PostgreSQL), "REAL");
        assert_eq!(utils::normalize_column_type("BYTEA", DatabaseDialect::PostgreSQL), "BLOB");
        assert_eq!(utils::normalize_column_type("BOOLEAN", DatabaseDialect::PostgreSQL), "BOOLEAN");
        assert_eq!(utils::normalize_column_type("TIMESTAMP", DatabaseDialect::PostgreSQL), "DATETIME");
    }

    #[test]
    #[cfg(feature = "mysql")]
    fn test_normalize_column_type_mysql() {
        assert_eq!(utils::normalize_column_type("VARCHAR(255)", DatabaseDialect::MySQL), "TEXT");
        assert_eq!(utils::normalize_column_type("INT", DatabaseDialect::MySQL), "INTEGER");
        assert_eq!(utils::normalize_column_type("BIGINT", DatabaseDialect::MySQL), "INTEGER");
        assert_eq!(utils::normalize_column_type("FLOAT", DatabaseDialect::MySQL), "REAL");
        assert_eq!(utils::normalize_column_type("BLOB", DatabaseDialect::MySQL), "BLOB");
        assert_eq!(utils::normalize_column_type("TINYINT(1)", DatabaseDialect::MySQL), "BOOLEAN");
        assert_eq!(utils::normalize_column_type("DATETIME", DatabaseDialect::MySQL), "DATETIME");
    }

    #[test]
    fn test_extract_table_name() {
        assert_eq!(utils::extract_table_name("users"), "users");
        assert_eq!(utils::extract_table_name("public.users"), "users");
        assert_eq!(utils::extract_table_name("schema.public.users"), "users");
    }

    #[test]
    fn test_is_system_table_sqlite() {
        assert!(utils::is_system_table("sqlite_master", DatabaseDialect::SQLite));
        assert!(utils::is_system_table("sqlite_sequence", DatabaseDialect::SQLite));
        assert!(utils::is_system_table("_internal", DatabaseDialect::SQLite));
        assert!(!utils::is_system_table("users", DatabaseDialect::SQLite));
        assert!(!utils::is_system_table("migrations", DatabaseDialect::SQLite));
    }

    #[test]
    #[cfg(feature = "postgres")]
    fn test_is_system_table_postgresql() {
        assert!(utils::is_system_table("pg_class", DatabaseDialect::PostgreSQL));
        assert!(utils::is_system_table("information_schema", DatabaseDialect::PostgreSQL));
        assert!(utils::is_system_table("sql_features", DatabaseDialect::PostgreSQL));
        assert!(!utils::is_system_table("users", DatabaseDialect::PostgreSQL));
        assert!(!utils::is_system_table("migrations", DatabaseDialect::PostgreSQL));
    }

    #[test]
    #[cfg(feature = "mysql")]
    fn test_is_system_table_mysql() {
        assert!(utils::is_system_table("INFORMATION_SCHEMA", DatabaseDialect::MySQL));
        assert!(utils::is_system_table("performance_schema", DatabaseDialect::MySQL));
        assert!(utils::is_system_table("sys", DatabaseDialect::MySQL));
        assert!(utils::is_system_table("mysql", DatabaseDialect::MySQL));
        assert!(!utils::is_system_table("users", DatabaseDialect::MySQL));
        assert!(!utils::is_system_table("migrations", DatabaseDialect::MySQL));
    }

    #[test]
    fn test_introspection_error_display() {
        let error = IntrospectionError {
            message: "Table not found".to_string(),
            kind: IntrospectionErrorKind::NotFound,
        };
        assert_eq!(error.to_string(), "Not Found: Table not found");
    }

    #[test]
    fn test_introspection_error_kind_display() {
        assert_eq!(IntrospectionErrorKind::DatabaseError.to_string(), "Database Error");
        assert_eq!(IntrospectionErrorKind::NotFound.to_string(), "Not Found");
        assert_eq!(IntrospectionErrorKind::InvalidSchema.to_string(), "Invalid Schema");
        assert_eq!(IntrospectionErrorKind::Unsupported.to_string(), "Unsupported");
        assert_eq!(IntrospectionErrorKind::Configuration.to_string(), "Configuration Error");
    }

    #[test]
    fn test_introspection_error_from_d1rs_error() {
        let d1_error = D1RsError::Database("Connection failed".to_string());
        let intro_error: IntrospectionError = d1_error.into();
        assert_eq!(intro_error.kind, IntrospectionErrorKind::DatabaseError);
        assert!(intro_error.message.contains("Connection failed"));
    }

    #[test]
    fn test_introspection_error_from_json_error() {
        let json_error = serde_json::from_str::<Value>("invalid json").unwrap_err();
        let intro_error: IntrospectionError = json_error.into();
        assert_eq!(intro_error.kind, IntrospectionErrorKind::InvalidSchema);
        assert!(intro_error.message.contains("JSON parsing error"));
    }

    #[test]
    fn test_parse_constraint_from_sql() {
        let sql = "CREATE TABLE users (id INTEGER, CHECK (age > 0))";
        let result = utils::parse_constraint_from_sql(sql, "age");
        assert!(result.is_some());
        assert!(result.unwrap().contains("age"));
    }

    #[test]
    fn test_parse_constraint_from_sql_not_found() {
        let sql = "CREATE TABLE users (id INTEGER, name TEXT)";
        let result = utils::parse_constraint_from_sql(sql, "non_existent");
        assert!(result.is_none());
    }

    // Integration tests with mock implementations would go here
    // These would test the trait methods with actual database backends
    // but since this is just the trait definition, we focus on utility functions
}