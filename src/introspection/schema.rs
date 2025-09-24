/// Unified Schema Representation for Cross-Database Compatibility
/// 
/// This module provides database-agnostic schema structures that can represent
/// schema information from SQLite, PostgreSQL, and MySQL databases in a unified way.
/// All database-specific introspectors convert their results to these unified structures.
///
/// # Design Principles
/// 
/// - **Database Agnostic**: Works identically across SQLite, PostgreSQL, and MySQL
/// - **Comprehensive**: Supports all common schema elements across databases
/// - **Type Safe**: Leverages Rust's type system for compile-time safety
/// - **Normalized**: Consistent representation regardless of source database
/// - **Extensible**: Can be extended to support additional databases

use crate::dialects::DatabaseDialect;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fmt;

/// Complete database schema representation
/// 
/// This structure represents the complete schema of a database, including all tables,
/// their relationships, and metadata. It provides a unified view regardless of the
/// source database type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnifiedDatabaseSchema {
    /// Database dialect this schema originated from
    pub dialect: DatabaseDialect,
    /// Database version information
    pub version: Option<String>,
    /// Current database/schema name
    pub database_name: Option<String>,
    /// All tables in the database
    pub tables: Vec<UnifiedTableSchema>,
    /// Database-level metadata
    pub metadata: HashMap<String, String>,
}

impl UnifiedDatabaseSchema {
    /// Create a new database schema
    pub fn new(dialect: DatabaseDialect) -> Self {
        Self {
            dialect,
            version: None,
            database_name: None,
            tables: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Get a table by name
    pub fn get_table(&self, name: &str) -> Option<&UnifiedTableSchema> {
        self.tables.iter().find(|t| t.name == name)
    }

    /// Get all table names
    pub fn table_names(&self) -> Vec<&str> {
        self.tables.iter().map(|t| t.name.as_str()).collect()
    }

    /// Add a table to the schema
    pub fn add_table(&mut self, table: UnifiedTableSchema) {
        self.tables.push(table);
    }

    /// Get tables by type (regular, view, temporary, etc.)
    pub fn tables_by_type(&self, table_type: &UnifiedTableType) -> Vec<&UnifiedTableSchema> {
        self.tables.iter().filter(|t| &t.table_type == table_type).collect()
    }

    /// Check if schema contains any tables
    pub fn is_empty(&self) -> bool {
        self.tables.is_empty()
    }

    /// Get total column count across all tables
    pub fn total_column_count(&self) -> usize {
        self.tables.iter().map(|t| t.columns.len()).sum()
    }

    /// Get all foreign key relationships in the database
    pub fn all_foreign_keys(&self) -> Vec<(&str, &UnifiedForeignKeySchema)> {
        self.tables
            .iter()
            .flat_map(|table| {
                table.foreign_keys.iter().map(move |fk| (table.name.as_str(), fk))
            })
            .collect()
    }

    /// Validate schema consistency
    pub fn validate(&self) -> Result<(), SchemaValidationError> {
        for table in &self.tables {
            // First validate the table itself (columns, etc.)
            table.validate()?;
            
            // Then validate foreign key references to other tables
            for fk in &table.foreign_keys {
                if self.get_table(&fk.referenced_table).is_none() {
                    return Err(SchemaValidationError::InvalidForeignKeyReference {
                        table: table.name.clone(),
                        foreign_key: fk.name.clone(),
                        referenced_table: fk.referenced_table.clone(),
                    });
                }
            }
        }
        Ok(())
    }
}

/// Unified table schema representation
/// 
/// Represents a table schema in a database-agnostic way, including columns,
/// indexes, constraints, and relationships.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnifiedTableSchema {
    /// Table name
    pub name: String,
    /// Table type (table, view, etc.)
    pub table_type: UnifiedTableType,
    /// Schema/database name (for databases that support schemas)
    pub schema_name: Option<String>,
    /// Table comment/description
    pub comment: Option<String>,
    /// All columns in the table
    pub columns: Vec<UnifiedColumnSchema>,
    /// All indexes on the table
    pub indexes: Vec<UnifiedIndexSchema>,
    /// All foreign key constraints
    pub foreign_keys: Vec<UnifiedForeignKeySchema>,
    /// All table-level constraints
    pub constraints: Vec<UnifiedConstraintSchema>,
    /// Table-level metadata
    pub metadata: HashMap<String, String>,
}

impl UnifiedTableSchema {
    /// Create a new table schema
    pub fn new(name: String, table_type: UnifiedTableType) -> Self {
        Self {
            name,
            table_type,
            schema_name: None,
            comment: None,
            columns: Vec::new(),
            indexes: Vec::new(),
            foreign_keys: Vec::new(),
            constraints: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Get a column by name
    pub fn get_column(&self, name: &str) -> Option<&UnifiedColumnSchema> {
        self.columns.iter().find(|c| c.name == name)
    }

    /// Get all column names
    pub fn column_names(&self) -> Vec<&str> {
        self.columns.iter().map(|c| c.name.as_str()).collect()
    }

    /// Get primary key columns
    pub fn primary_key_columns(&self) -> Vec<&UnifiedColumnSchema> {
        self.columns.iter().filter(|c| c.primary_key).collect()
    }

    /// Get unique columns (excluding primary key)
    pub fn unique_columns(&self) -> Vec<&UnifiedColumnSchema> {
        self.columns.iter().filter(|c| c.unique && !c.primary_key).collect()
    }

    /// Get nullable columns
    pub fn nullable_columns(&self) -> Vec<&UnifiedColumnSchema> {
        self.columns.iter().filter(|c| c.nullable).collect()
    }

    /// Get columns with default values
    pub fn columns_with_defaults(&self) -> Vec<&UnifiedColumnSchema> {
        self.columns.iter().filter(|c| c.default_value.is_some()).collect()
    }

    /// Get foreign key relationships
    pub fn foreign_key_relationships(&self) -> Vec<ForeignKeyRelationship> {
        self.foreign_keys.iter().map(|fk| ForeignKeyRelationship {
            source_table: self.name.clone(),
            source_columns: fk.columns.clone(),
            target_table: fk.referenced_table.clone(),
            target_columns: fk.referenced_columns.clone(),
            on_delete: fk.on_delete.clone(),
            on_update: fk.on_update.clone(),
        }).collect()
    }

    /// Add a column to the table
    pub fn add_column(&mut self, column: UnifiedColumnSchema) {
        self.columns.push(column);
    }

    /// Add an index to the table
    pub fn add_index(&mut self, index: UnifiedIndexSchema) {
        self.indexes.push(index);
    }

    /// Add a foreign key to the table
    pub fn add_foreign_key(&mut self, foreign_key: UnifiedForeignKeySchema) {
        self.foreign_keys.push(foreign_key);
    }

    /// Add a constraint to the table
    pub fn add_constraint(&mut self, constraint: UnifiedConstraintSchema) {
        self.constraints.push(constraint);
    }

    /// Validate table schema consistency
    pub fn validate(&self) -> Result<(), SchemaValidationError> {
        // Validate primary key exists for regular tables
        if self.primary_key_columns().is_empty() && matches!(self.table_type, UnifiedTableType::Table) {
            return Err(SchemaValidationError::NoPrimaryKey {
                table: self.name.clone(),
            });
        }

        // Validate column names are unique
        let mut column_names = std::collections::HashSet::new();
        for column in &self.columns {
            if !column_names.insert(&column.name) {
                return Err(SchemaValidationError::DuplicateColumnName {
                    table: self.name.clone(),
                    column: column.name.clone(),
                });
            }
        }

        // Validate foreign key column references exist
        for fk in &self.foreign_keys {
            for column in &fk.columns {
                if self.get_column(column).is_none() {
                    return Err(SchemaValidationError::InvalidForeignKeyColumn {
                        table: self.name.clone(),
                        foreign_key: fk.name.clone(),
                        column: column.clone(),
                    });
                }
            }
        }

        Ok(())
    }
}

/// Unified column schema representation
/// 
/// Represents a database column in a database-agnostic way with normalized types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnifiedColumnSchema {
    /// Column name
    pub name: String,
    /// Unified column type
    pub column_type: UnifiedColumnType,
    /// Raw database-specific type
    pub raw_type: String,
    /// Whether the column allows NULL values
    pub nullable: bool,
    /// Default value if any
    pub default_value: Option<String>,
    /// Whether this is a primary key column
    pub primary_key: bool,
    /// Whether this column has auto-increment/serial behavior
    pub auto_increment: bool,
    /// Whether this column has a unique constraint
    pub unique: bool,
    /// Column comment/description
    pub comment: Option<String>,
    /// Character maximum length (for string types)
    pub character_maximum_length: Option<i32>,
    /// Numeric precision (for decimal types)
    pub numeric_precision: Option<i32>,
    /// Numeric scale (for decimal types)
    pub numeric_scale: Option<i32>,
    /// Column-level constraints
    pub constraints: Vec<UnifiedColumnConstraint>,
    /// Column position in table (0-based)
    pub ordinal_position: Option<i32>,
}

impl UnifiedColumnSchema {
    /// Create a new column schema
    pub fn new(name: String, column_type: UnifiedColumnType, raw_type: String) -> Self {
        Self {
            name,
            column_type,
            raw_type,
            nullable: true,
            default_value: None,
            primary_key: false,
            auto_increment: false,
            unique: false,
            comment: None,
            character_maximum_length: None,
            numeric_precision: None,
            numeric_scale: None,
            constraints: Vec::new(),
            ordinal_position: None,
        }
    }

    /// Check if column is required (not nullable and no default)
    pub fn is_required(&self) -> bool {
        !self.nullable && self.default_value.is_none()
    }

    /// Check if column can store large data
    pub fn is_large_object(&self) -> bool {
        matches!(self.column_type, UnifiedColumnType::Blob | UnifiedColumnType::Text)
    }

    /// Get effective type size for storage estimation
    pub fn estimated_storage_size(&self) -> Option<usize> {
        match &self.column_type {
            UnifiedColumnType::Boolean => Some(1),
            UnifiedColumnType::SmallInt => Some(2),
            UnifiedColumnType::Integer => Some(4),
            UnifiedColumnType::BigInt => Some(8),
            UnifiedColumnType::Real => Some(4),
            UnifiedColumnType::Double => Some(8),
            UnifiedColumnType::Decimal => Some(16), // Estimate
            UnifiedColumnType::Char | UnifiedColumnType::VarChar => {
                self.character_maximum_length.map(|len| len as usize)
            },
            UnifiedColumnType::Text | UnifiedColumnType::Blob | UnifiedColumnType::Json => None, // Variable
            UnifiedColumnType::Date => Some(4),
            UnifiedColumnType::Time => Some(4),
            UnifiedColumnType::DateTime => Some(8),
            UnifiedColumnType::Timestamp => Some(8),
            UnifiedColumnType::Uuid => Some(16),
            UnifiedColumnType::Other(_) => None,
        }
    }
}

/// Unified column types across databases
/// 
/// These types represent the normalized column types that can be mapped
/// from any of the supported databases.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UnifiedColumnType {
    /// Boolean type (true/false)
    Boolean,
    /// Small integer (-32,768 to 32,767)
    SmallInt,
    /// Regular integer (-2,147,483,648 to 2,147,483,647)
    Integer,
    /// Large integer (-9,223,372,036,854,775,808 to 9,223,372,036,854,775,807)
    BigInt,
    /// Single-precision floating point
    Real,
    /// Double-precision floating point
    Double,
    /// Fixed-precision decimal
    Decimal,
    /// Fixed-length character string
    Char,
    /// Variable-length character string
    VarChar,
    /// Large text object
    Text,
    /// Binary large object
    Blob,
    /// JSON data
    Json,
    /// Date (year, month, day)
    Date,
    /// Time (hour, minute, second)
    Time,
    /// Date and time
    DateTime,
    /// Timestamp with timezone
    Timestamp,
    /// Universally unique identifier
    Uuid,
    /// Other/custom types
    Other(String),
}

impl UnifiedColumnType {
    /// Check if type is numeric
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            UnifiedColumnType::SmallInt
                | UnifiedColumnType::Integer
                | UnifiedColumnType::BigInt
                | UnifiedColumnType::Real
                | UnifiedColumnType::Double
                | UnifiedColumnType::Decimal
        )
    }

    /// Check if type is string-based
    pub fn is_string(&self) -> bool {
        matches!(
            self,
            UnifiedColumnType::Char | UnifiedColumnType::VarChar | UnifiedColumnType::Text
        )
    }

    /// Check if type is temporal (date/time)
    pub fn is_temporal(&self) -> bool {
        matches!(
            self,
            UnifiedColumnType::Date
                | UnifiedColumnType::Time
                | UnifiedColumnType::DateTime
                | UnifiedColumnType::Timestamp
        )
    }

    /// Check if type is binary
    pub fn is_binary(&self) -> bool {
        matches!(self, UnifiedColumnType::Blob)
    }
}

impl fmt::Display for UnifiedColumnType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnifiedColumnType::Boolean => write!(f, "BOOLEAN"),
            UnifiedColumnType::SmallInt => write!(f, "SMALLINT"),
            UnifiedColumnType::Integer => write!(f, "INTEGER"),
            UnifiedColumnType::BigInt => write!(f, "BIGINT"),
            UnifiedColumnType::Real => write!(f, "REAL"),
            UnifiedColumnType::Double => write!(f, "DOUBLE"),
            UnifiedColumnType::Decimal => write!(f, "DECIMAL"),
            UnifiedColumnType::Char => write!(f, "CHAR"),
            UnifiedColumnType::VarChar => write!(f, "VARCHAR"),
            UnifiedColumnType::Text => write!(f, "TEXT"),
            UnifiedColumnType::Blob => write!(f, "BLOB"),
            UnifiedColumnType::Json => write!(f, "JSON"),
            UnifiedColumnType::Date => write!(f, "DATE"),
            UnifiedColumnType::Time => write!(f, "TIME"),
            UnifiedColumnType::DateTime => write!(f, "DATETIME"),
            UnifiedColumnType::Timestamp => write!(f, "TIMESTAMP"),
            UnifiedColumnType::Uuid => write!(f, "UUID"),
            UnifiedColumnType::Other(type_name) => write!(f, "{}", type_name),
        }
    }
}

/// Unified table types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UnifiedTableType {
    /// Regular table
    Table,
    /// Database view
    View,
    /// Temporary table
    Temporary,
    /// System table
    System,
    /// Other/custom table type
    Other(String),
}

/// Unified index schema representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnifiedIndexSchema {
    /// Index name
    pub name: String,
    /// Table this index belongs to
    pub table_name: String,
    /// Columns included in the index (in order)
    pub columns: Vec<String>,
    /// Whether this is a unique index
    pub unique: bool,
    /// Whether this is a primary key index
    pub primary: bool,
    /// Index type (btree, hash, gin, etc.)
    pub index_type: UnifiedIndexType,
    /// Partial index condition (WHERE clause)
    pub condition: Option<String>,
    /// Index comment/description
    pub comment: Option<String>,
}

/// Unified index types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UnifiedIndexType {
    /// B-tree index (most common)
    BTree,
    /// Hash index
    Hash,
    /// GIN (Generalized Inverted Index) - PostgreSQL
    Gin,
    /// GiST (Generalized Search Tree) - PostgreSQL
    Gist,
    /// Full-text index
    FullText,
    /// Spatial index
    Spatial,
    /// Other/database-specific index type
    Other(String),
}

/// Unified foreign key schema representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnifiedForeignKeySchema {
    /// Constraint name
    pub name: String,
    /// Source columns (in this table)
    pub columns: Vec<String>,
    /// Referenced table name
    pub referenced_table: String,
    /// Referenced columns
    pub referenced_columns: Vec<String>,
    /// Action on DELETE
    pub on_delete: Option<UnifiedReferentialAction>,
    /// Action on UPDATE
    pub on_update: Option<UnifiedReferentialAction>,
    /// Whether constraint is deferrable
    pub deferrable: bool,
    /// Whether constraint is initially deferred
    pub initially_deferred: bool,
}

/// Unified referential actions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UnifiedReferentialAction {
    /// Do nothing (raise error)
    Restrict,
    /// Delete/update dependent rows
    Cascade,
    /// Set foreign key to NULL
    SetNull,
    /// Set foreign key to default value
    SetDefault,
    /// Do nothing (allow inconsistency)
    NoAction,
}

impl fmt::Display for UnifiedReferentialAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnifiedReferentialAction::Restrict => write!(f, "RESTRICT"),
            UnifiedReferentialAction::Cascade => write!(f, "CASCADE"),
            UnifiedReferentialAction::SetNull => write!(f, "SET NULL"),
            UnifiedReferentialAction::SetDefault => write!(f, "SET DEFAULT"),
            UnifiedReferentialAction::NoAction => write!(f, "NO ACTION"),
        }
    }
}

/// Unified constraint schema representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnifiedConstraintSchema {
    /// Constraint name
    pub name: String,
    /// Constraint type
    pub constraint_type: UnifiedConstraintType,
    /// Constraint definition/expression
    pub definition: String,
    /// Columns involved in the constraint
    pub columns: Vec<String>,
    /// Whether constraint is deferrable
    pub deferrable: bool,
    /// Whether constraint is initially deferred
    pub initially_deferred: bool,
}

/// Unified constraint types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UnifiedConstraintType {
    /// Primary key constraint
    PrimaryKey,
    /// Unique constraint
    Unique,
    /// Foreign key constraint
    ForeignKey,
    /// Check constraint
    Check,
    /// Not null constraint
    NotNull,
    /// Default constraint
    Default,
    /// Other/database-specific constraint
    Other(String),
}

/// Column-level constraint representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UnifiedColumnConstraint {
    /// Check constraint with expression
    Check { expression: String },
    /// References constraint (foreign key)
    References { table: String, column: String },
    /// Custom constraint
    Custom { name: String, definition: String },
}

/// Foreign key relationship helper structure
#[derive(Debug, Clone, PartialEq)]
pub struct ForeignKeyRelationship {
    pub source_table: String,
    pub source_columns: Vec<String>,
    pub target_table: String,
    pub target_columns: Vec<String>,
    pub on_delete: Option<UnifiedReferentialAction>,
    pub on_update: Option<UnifiedReferentialAction>,
}

/// Schema validation errors
#[derive(Debug, Clone, PartialEq)]
pub enum SchemaValidationError {
    NoPrimaryKey { table: String },
    DuplicateColumnName { table: String, column: String },
    InvalidForeignKeyReference { table: String, foreign_key: String, referenced_table: String },
    InvalidForeignKeyColumn { table: String, foreign_key: String, column: String },
}

impl fmt::Display for SchemaValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SchemaValidationError::NoPrimaryKey { table } => {
                write!(f, "Table '{}' has no primary key", table)
            },
            SchemaValidationError::DuplicateColumnName { table, column } => {
                write!(f, "Duplicate column name '{}' in table '{}'", column, table)
            },
            SchemaValidationError::InvalidForeignKeyReference { table, foreign_key, referenced_table } => {
                write!(f, "Foreign key '{}' in table '{}' references non-existent table '{}'", foreign_key, table, referenced_table)
            },
            SchemaValidationError::InvalidForeignKeyColumn { table, foreign_key, column } => {
                write!(f, "Foreign key '{}' in table '{}' references non-existent column '{}'", foreign_key, table, column)
            },
        }
    }
}

impl std::error::Error for SchemaValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a sample unified database schema for testing
    fn create_sample_schema() -> UnifiedDatabaseSchema {
        let mut schema = UnifiedDatabaseSchema::new(DatabaseDialect::SQLite);
        schema.version = Some("3.40.0".to_string());
        schema.database_name = Some("test_db".to_string());

        // Create users table
        let mut users_table = UnifiedTableSchema::new("users".to_string(), UnifiedTableType::Table);
        users_table.comment = Some("User accounts table".to_string());

        // Add columns to users table
        let mut id_col = UnifiedColumnSchema::new("id".to_string(), UnifiedColumnType::Integer, "INTEGER".to_string());
        id_col.primary_key = true;
        id_col.auto_increment = true;
        id_col.nullable = false;
        users_table.add_column(id_col);

        let mut name_col = UnifiedColumnSchema::new("name".to_string(), UnifiedColumnType::VarChar, "VARCHAR(255)".to_string());
        name_col.nullable = false;
        name_col.character_maximum_length = Some(255);
        users_table.add_column(name_col);

        let mut email_col = UnifiedColumnSchema::new("email".to_string(), UnifiedColumnType::VarChar, "VARCHAR(255)".to_string());
        email_col.nullable = false;
        email_col.unique = true;
        email_col.character_maximum_length = Some(255);
        users_table.add_column(email_col);

        let mut active_col = UnifiedColumnSchema::new("is_active".to_string(), UnifiedColumnType::Boolean, "BOOLEAN".to_string());
        active_col.default_value = Some("true".to_string());
        users_table.add_column(active_col);

        let mut created_col = UnifiedColumnSchema::new("created_at".to_string(), UnifiedColumnType::DateTime, "DATETIME".to_string());
        created_col.default_value = Some("CURRENT_TIMESTAMP".to_string());
        users_table.add_column(created_col);

        // Add index to users table
        let email_index = UnifiedIndexSchema {
            name: "idx_users_email".to_string(),
            table_name: "users".to_string(),
            columns: vec!["email".to_string()],
            unique: true,
            primary: false,
            index_type: UnifiedIndexType::BTree,
            condition: None,
            comment: Some("Unique index on email".to_string()),
        };
        users_table.add_index(email_index);

        // Add constraint to users table
        let check_constraint = UnifiedConstraintSchema {
            name: "check_email_format".to_string(),
            constraint_type: UnifiedConstraintType::Check,
            definition: "email LIKE '%@%'".to_string(),
            columns: vec!["email".to_string()],
            deferrable: false,
            initially_deferred: false,
        };
        users_table.add_constraint(check_constraint);

        schema.add_table(users_table);

        // Create posts table
        let mut posts_table = UnifiedTableSchema::new("posts".to_string(), UnifiedTableType::Table);

        // Add columns to posts table
        let mut post_id_col = UnifiedColumnSchema::new("id".to_string(), UnifiedColumnType::Integer, "INTEGER".to_string());
        post_id_col.primary_key = true;
        post_id_col.auto_increment = true;
        post_id_col.nullable = false;
        posts_table.add_column(post_id_col);

        let mut title_col = UnifiedColumnSchema::new("title".to_string(), UnifiedColumnType::VarChar, "VARCHAR(500)".to_string());
        title_col.nullable = false;
        title_col.character_maximum_length = Some(500);
        posts_table.add_column(title_col);

        let content_col = UnifiedColumnSchema::new("content".to_string(), UnifiedColumnType::Text, "TEXT".to_string());
        posts_table.add_column(content_col);

        let mut user_id_col = UnifiedColumnSchema::new("user_id".to_string(), UnifiedColumnType::Integer, "INTEGER".to_string());
        user_id_col.nullable = false;
        posts_table.add_column(user_id_col);

        // Add foreign key to posts table
        let fk_user = UnifiedForeignKeySchema {
            name: "fk_posts_user_id".to_string(),
            columns: vec!["user_id".to_string()],
            referenced_table: "users".to_string(),
            referenced_columns: vec!["id".to_string()],
            on_delete: Some(UnifiedReferentialAction::Cascade),
            on_update: Some(UnifiedReferentialAction::Restrict),
            deferrable: false,
            initially_deferred: false,
        };
        posts_table.add_foreign_key(fk_user);

        schema.add_table(posts_table);

        schema
    }

    #[test]
    fn test_unified_database_schema_creation() {
        let schema = UnifiedDatabaseSchema::new(DatabaseDialect::SQLite);
        assert_eq!(schema.dialect, DatabaseDialect::SQLite);
        assert!(schema.tables.is_empty());
        assert!(schema.is_empty());
        assert_eq!(schema.total_column_count(), 0);
    }

    #[test]
    fn test_unified_database_schema_operations() {
        let schema = create_sample_schema();

        // Test basic properties
        assert_eq!(schema.dialect, DatabaseDialect::SQLite);
        assert_eq!(schema.version, Some("3.40.0".to_string()));
        assert_eq!(schema.database_name, Some("test_db".to_string()));
        assert_eq!(schema.tables.len(), 2);
        assert!(!schema.is_empty());
        assert_eq!(schema.total_column_count(), 9); // 5 in users + 4 in posts

        // Test table access
        assert!(schema.get_table("users").is_some());
        assert!(schema.get_table("posts").is_some());
        assert!(schema.get_table("nonexistent").is_none());

        // Test table names
        let table_names = schema.table_names();
        assert!(table_names.contains(&"users"));
        assert!(table_names.contains(&"posts"));

        // Test tables by type
        let regular_tables = schema.tables_by_type(&UnifiedTableType::Table);
        assert_eq!(regular_tables.len(), 2);

        // Test foreign key relationships
        let all_fks = schema.all_foreign_keys();
        assert_eq!(all_fks.len(), 1);
        assert_eq!(all_fks[0].0, "posts");
        assert_eq!(all_fks[0].1.referenced_table, "users");
    }

    #[test]
    fn test_unified_table_schema_operations() {
        let schema = create_sample_schema();
        let users_table = schema.get_table("users").unwrap();

        // Test basic properties
        assert_eq!(users_table.name, "users");
        assert_eq!(users_table.table_type, UnifiedTableType::Table);
        assert_eq!(users_table.comment, Some("User accounts table".to_string()));
        assert_eq!(users_table.columns.len(), 5);
        assert_eq!(users_table.indexes.len(), 1);
        assert_eq!(users_table.constraints.len(), 1);

        // Test column access
        assert!(users_table.get_column("id").is_some());
        assert!(users_table.get_column("name").is_some());
        assert!(users_table.get_column("nonexistent").is_none());

        // Test column names
        let column_names = users_table.column_names();
        assert!(column_names.contains(&"id"));
        assert!(column_names.contains(&"name"));
        assert!(column_names.contains(&"email"));

        // Test primary key columns
        let pk_columns = users_table.primary_key_columns();
        assert_eq!(pk_columns.len(), 1);
        assert_eq!(pk_columns[0].name, "id");

        // Test unique columns
        let unique_columns = users_table.unique_columns();
        assert_eq!(unique_columns.len(), 1);
        assert_eq!(unique_columns[0].name, "email");

        // Test nullable columns
        let nullable_columns = users_table.nullable_columns();
        assert_eq!(nullable_columns.len(), 2); // is_active and created_at

        // Test columns with defaults
        let default_columns = users_table.columns_with_defaults();
        assert_eq!(default_columns.len(), 2); // is_active and created_at

        // Test foreign key relationships (posts table)
        let posts_table = schema.get_table("posts").unwrap();
        let relationships = posts_table.foreign_key_relationships();
        assert_eq!(relationships.len(), 1);
        assert_eq!(relationships[0].target_table, "users");
        assert_eq!(relationships[0].on_delete, Some(UnifiedReferentialAction::Cascade));
    }

    #[test]
    fn test_unified_column_schema_properties() {
        let schema = create_sample_schema();
        let users_table = schema.get_table("users").unwrap();

        // Test ID column
        let id_col = users_table.get_column("id").unwrap();
        assert_eq!(id_col.column_type, UnifiedColumnType::Integer);
        assert_eq!(id_col.raw_type, "INTEGER");
        assert!(!id_col.nullable);
        assert!(id_col.primary_key);
        assert!(id_col.auto_increment);
        assert!(id_col.is_required()); // Not nullable and no default

        // Test name column
        let name_col = users_table.get_column("name").unwrap();
        assert_eq!(name_col.column_type, UnifiedColumnType::VarChar);
        assert_eq!(name_col.character_maximum_length, Some(255));
        assert!(name_col.is_required());
        assert_eq!(name_col.estimated_storage_size(), Some(255));

        // Test email column
        let email_col = users_table.get_column("email").unwrap();
        assert!(email_col.unique);
        assert!(!email_col.is_large_object());

        // Test active column
        let active_col = users_table.get_column("is_active").unwrap();
        assert_eq!(active_col.column_type, UnifiedColumnType::Boolean);
        assert!(!active_col.is_required()); // Has default value
        assert_eq!(active_col.estimated_storage_size(), Some(1));

        // Test created_at column
        let created_col = users_table.get_column("created_at").unwrap();
        assert_eq!(created_col.column_type, UnifiedColumnType::DateTime);
        assert!(!created_col.is_required());
    }

    #[test]
    fn test_unified_column_type_methods() {
        // Test numeric types
        assert!(UnifiedColumnType::Integer.is_numeric());
        assert!(UnifiedColumnType::Real.is_numeric());
        assert!(UnifiedColumnType::BigInt.is_numeric());
        assert!(UnifiedColumnType::Decimal.is_numeric());
        assert!(!UnifiedColumnType::Text.is_numeric());
        assert!(!UnifiedColumnType::Boolean.is_numeric());

        // Test string types
        assert!(UnifiedColumnType::Text.is_string());
        assert!(UnifiedColumnType::VarChar.is_string());
        assert!(UnifiedColumnType::Char.is_string());
        assert!(!UnifiedColumnType::Integer.is_string());
        assert!(!UnifiedColumnType::Boolean.is_string());

        // Test temporal types
        assert!(UnifiedColumnType::DateTime.is_temporal());
        assert!(UnifiedColumnType::Date.is_temporal());
        assert!(UnifiedColumnType::Time.is_temporal());
        assert!(UnifiedColumnType::Timestamp.is_temporal());
        assert!(!UnifiedColumnType::Text.is_temporal());
        assert!(!UnifiedColumnType::Integer.is_temporal());

        // Test binary types
        assert!(UnifiedColumnType::Blob.is_binary());
        assert!(!UnifiedColumnType::Text.is_binary());
        assert!(!UnifiedColumnType::Integer.is_binary());
    }

    #[test]
    fn test_unified_column_type_display() {
        assert_eq!(UnifiedColumnType::Boolean.to_string(), "BOOLEAN");
        assert_eq!(UnifiedColumnType::Integer.to_string(), "INTEGER");
        assert_eq!(UnifiedColumnType::VarChar.to_string(), "VARCHAR");
        assert_eq!(UnifiedColumnType::Text.to_string(), "TEXT");
        assert_eq!(UnifiedColumnType::DateTime.to_string(), "DATETIME");
        assert_eq!(UnifiedColumnType::Json.to_string(), "JSON");
        assert_eq!(UnifiedColumnType::Other("CUSTOM".to_string()).to_string(), "CUSTOM");
    }

    #[test]
    fn test_unified_referential_action_display() {
        assert_eq!(UnifiedReferentialAction::Cascade.to_string(), "CASCADE");
        assert_eq!(UnifiedReferentialAction::Restrict.to_string(), "RESTRICT");
        assert_eq!(UnifiedReferentialAction::SetNull.to_string(), "SET NULL");
        assert_eq!(UnifiedReferentialAction::SetDefault.to_string(), "SET DEFAULT");
        assert_eq!(UnifiedReferentialAction::NoAction.to_string(), "NO ACTION");
    }

    #[test]
    fn test_schema_validation_success() {
        let schema = create_sample_schema();
        assert!(schema.validate().is_ok());
    }

    #[test]
    fn test_schema_validation_no_primary_key() {
        let mut schema = UnifiedDatabaseSchema::new(DatabaseDialect::SQLite);
        
        // Create table without primary key
        let mut table = UnifiedTableSchema::new("invalid".to_string(), UnifiedTableType::Table);
        let name_col = UnifiedColumnSchema::new("name".to_string(), UnifiedColumnType::Text, "TEXT".to_string());
        table.add_column(name_col);
        
        schema.add_table(table);
        
        let result = schema.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            SchemaValidationError::NoPrimaryKey { table } => {
                assert_eq!(table, "invalid");
            },
            _ => panic!("Expected NoPrimaryKey error"),
        }
    }

    #[test]
    fn test_schema_validation_duplicate_column() {
        let mut schema = UnifiedDatabaseSchema::new(DatabaseDialect::SQLite);
        
        // Create table with duplicate column names
        let mut table = UnifiedTableSchema::new("invalid".to_string(), UnifiedTableType::Table);
        let mut id_col1 = UnifiedColumnSchema::new("id".to_string(), UnifiedColumnType::Integer, "INTEGER".to_string());
        id_col1.primary_key = true; // Add primary key so we get past the primary key validation
        let id_col2 = UnifiedColumnSchema::new("id".to_string(), UnifiedColumnType::Text, "TEXT".to_string());
        table.add_column(id_col1);
        table.add_column(id_col2);
        
        schema.add_table(table);
        
        let result = schema.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            SchemaValidationError::DuplicateColumnName { table, column } => {
                assert_eq!(table, "invalid");
                assert_eq!(column, "id");
            },
            e => panic!("Expected DuplicateColumnName error, but got: {:?}", e),
        }
    }

    #[test]
    fn test_schema_validation_invalid_foreign_key_reference() {
        let mut schema = UnifiedDatabaseSchema::new(DatabaseDialect::SQLite);
        
        // Create table with foreign key to non-existent table
        let mut table = UnifiedTableSchema::new("posts".to_string(), UnifiedTableType::Table);
        let mut id_col = UnifiedColumnSchema::new("id".to_string(), UnifiedColumnType::Integer, "INTEGER".to_string());
        id_col.primary_key = true;
        table.add_column(id_col);
        
        // Add the user_id column so the foreign key column reference is valid
        let user_id_col = UnifiedColumnSchema::new("user_id".to_string(), UnifiedColumnType::Integer, "INTEGER".to_string());
        table.add_column(user_id_col);
        
        let invalid_fk = UnifiedForeignKeySchema {
            name: "fk_invalid".to_string(),
            columns: vec!["user_id".to_string()],
            referenced_table: "nonexistent_table".to_string(),
            referenced_columns: vec!["id".to_string()],
            on_delete: None,
            on_update: None,
            deferrable: false,
            initially_deferred: false,
        };
        table.add_foreign_key(invalid_fk);
        
        schema.add_table(table);
        
        let result = schema.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            SchemaValidationError::InvalidForeignKeyReference { table, foreign_key, referenced_table } => {
                assert_eq!(table, "posts");
                assert_eq!(foreign_key, "fk_invalid");
                assert_eq!(referenced_table, "nonexistent_table");
            },
            _ => panic!("Expected InvalidForeignKeyReference error"),
        }
    }

    #[test]
    fn test_schema_validation_invalid_foreign_key_column() {
        let mut schema = UnifiedDatabaseSchema::new(DatabaseDialect::SQLite);
        
        // Create table with foreign key referencing non-existent column
        let mut table = UnifiedTableSchema::new("posts".to_string(), UnifiedTableType::Table);
        let mut id_col = UnifiedColumnSchema::new("id".to_string(), UnifiedColumnType::Integer, "INTEGER".to_string());
        id_col.primary_key = true;
        table.add_column(id_col);
        
        let invalid_fk = UnifiedForeignKeySchema {
            name: "fk_invalid".to_string(),
            columns: vec!["nonexistent_column".to_string()],
            referenced_table: "users".to_string(),
            referenced_columns: vec!["id".to_string()],
            on_delete: None,
            on_update: None,
            deferrable: false,
            initially_deferred: false,
        };
        table.add_foreign_key(invalid_fk);
        
        // Add users table
        let mut users_table = UnifiedTableSchema::new("users".to_string(), UnifiedTableType::Table);
        let mut users_id_col = UnifiedColumnSchema::new("id".to_string(), UnifiedColumnType::Integer, "INTEGER".to_string());
        users_id_col.primary_key = true;
        users_table.add_column(users_id_col);
        schema.add_table(users_table);
        
        schema.add_table(table);
        
        let result = schema.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            SchemaValidationError::InvalidForeignKeyColumn { table, foreign_key, column } => {
                assert_eq!(table, "posts");
                assert_eq!(foreign_key, "fk_invalid");
                assert_eq!(column, "nonexistent_column");
            },
            _ => panic!("Expected InvalidForeignKeyColumn error"),
        }
    }

    #[test]
    fn test_schema_validation_error_display() {
        let error = SchemaValidationError::NoPrimaryKey { table: "test".to_string() };
        assert_eq!(error.to_string(), "Table 'test' has no primary key");

        let error = SchemaValidationError::DuplicateColumnName { table: "test".to_string(), column: "id".to_string() };
        assert_eq!(error.to_string(), "Duplicate column name 'id' in table 'test'");

        let error = SchemaValidationError::InvalidForeignKeyReference {
            table: "posts".to_string(),
            foreign_key: "fk_test".to_string(),
            referenced_table: "users".to_string(),
        };
        assert_eq!(error.to_string(), "Foreign key 'fk_test' in table 'posts' references non-existent table 'users'");

        let error = SchemaValidationError::InvalidForeignKeyColumn {
            table: "posts".to_string(),
            foreign_key: "fk_test".to_string(),
            column: "user_id".to_string(),
        };
        assert_eq!(error.to_string(), "Foreign key 'fk_test' in table 'posts' references non-existent column 'user_id'");
    }

    #[test]
    fn test_column_estimated_storage_sizes() {
        let boolean_col = UnifiedColumnSchema::new("test".to_string(), UnifiedColumnType::Boolean, "BOOLEAN".to_string());
        assert_eq!(boolean_col.estimated_storage_size(), Some(1));

        let smallint_col = UnifiedColumnSchema::new("test".to_string(), UnifiedColumnType::SmallInt, "SMALLINT".to_string());
        assert_eq!(smallint_col.estimated_storage_size(), Some(2));

        let int_col = UnifiedColumnSchema::new("test".to_string(), UnifiedColumnType::Integer, "INTEGER".to_string());
        assert_eq!(int_col.estimated_storage_size(), Some(4));

        let bigint_col = UnifiedColumnSchema::new("test".to_string(), UnifiedColumnType::BigInt, "BIGINT".to_string());
        assert_eq!(bigint_col.estimated_storage_size(), Some(8));

        let real_col = UnifiedColumnSchema::new("test".to_string(), UnifiedColumnType::Real, "REAL".to_string());
        assert_eq!(real_col.estimated_storage_size(), Some(4));

        let double_col = UnifiedColumnSchema::new("test".to_string(), UnifiedColumnType::Double, "DOUBLE".to_string());
        assert_eq!(double_col.estimated_storage_size(), Some(8));

        let decimal_col = UnifiedColumnSchema::new("test".to_string(), UnifiedColumnType::Decimal, "DECIMAL".to_string());
        assert_eq!(decimal_col.estimated_storage_size(), Some(16));

        let mut varchar_col = UnifiedColumnSchema::new("test".to_string(), UnifiedColumnType::VarChar, "VARCHAR".to_string());
        varchar_col.character_maximum_length = Some(255);
        assert_eq!(varchar_col.estimated_storage_size(), Some(255));

        let text_col = UnifiedColumnSchema::new("test".to_string(), UnifiedColumnType::Text, "TEXT".to_string());
        assert_eq!(text_col.estimated_storage_size(), None); // Variable size

        let blob_col = UnifiedColumnSchema::new("test".to_string(), UnifiedColumnType::Blob, "BLOB".to_string());
        assert_eq!(blob_col.estimated_storage_size(), None); // Variable size

        let json_col = UnifiedColumnSchema::new("test".to_string(), UnifiedColumnType::Json, "JSON".to_string());
        assert_eq!(json_col.estimated_storage_size(), None); // Variable size

        let date_col = UnifiedColumnSchema::new("test".to_string(), UnifiedColumnType::Date, "DATE".to_string());
        assert_eq!(date_col.estimated_storage_size(), Some(4));

        let time_col = UnifiedColumnSchema::new("test".to_string(), UnifiedColumnType::Time, "TIME".to_string());
        assert_eq!(time_col.estimated_storage_size(), Some(4));

        let datetime_col = UnifiedColumnSchema::new("test".to_string(), UnifiedColumnType::DateTime, "DATETIME".to_string());
        assert_eq!(datetime_col.estimated_storage_size(), Some(8));

        let timestamp_col = UnifiedColumnSchema::new("test".to_string(), UnifiedColumnType::Timestamp, "TIMESTAMP".to_string());
        assert_eq!(timestamp_col.estimated_storage_size(), Some(8));

        let uuid_col = UnifiedColumnSchema::new("test".to_string(), UnifiedColumnType::Uuid, "UUID".to_string());
        assert_eq!(uuid_col.estimated_storage_size(), Some(16));

        let other_col = UnifiedColumnSchema::new("test".to_string(), UnifiedColumnType::Other("CUSTOM".to_string()), "CUSTOM".to_string());
        assert_eq!(other_col.estimated_storage_size(), None);
    }

    #[test]
    fn test_column_property_methods() {
        // Test required column (not nullable, no default)
        let mut required_col = UnifiedColumnSchema::new("test".to_string(), UnifiedColumnType::Text, "TEXT".to_string());
        required_col.nullable = false;
        assert!(required_col.is_required());

        // Test optional column (nullable)
        let mut optional_col = UnifiedColumnSchema::new("test".to_string(), UnifiedColumnType::Text, "TEXT".to_string());
        optional_col.nullable = true;
        assert!(!optional_col.is_required());

        // Test column with default (not required even if not nullable)
        let mut default_col = UnifiedColumnSchema::new("test".to_string(), UnifiedColumnType::Integer, "INTEGER".to_string());
        default_col.nullable = false;
        default_col.default_value = Some("0".to_string());
        assert!(!default_col.is_required());

        // Test large object columns
        let text_col = UnifiedColumnSchema::new("test".to_string(), UnifiedColumnType::Text, "TEXT".to_string());
        assert!(text_col.is_large_object());

        let blob_col = UnifiedColumnSchema::new("test".to_string(), UnifiedColumnType::Blob, "BLOB".to_string());
        assert!(blob_col.is_large_object());

        let varchar_col = UnifiedColumnSchema::new("test".to_string(), UnifiedColumnType::VarChar, "VARCHAR".to_string());
        assert!(!varchar_col.is_large_object());
    }

    #[test]
    fn test_foreign_key_relationship_structure() {
        let relationship = ForeignKeyRelationship {
            source_table: "posts".to_string(),
            source_columns: vec!["user_id".to_string()],
            target_table: "users".to_string(),
            target_columns: vec!["id".to_string()],
            on_delete: Some(UnifiedReferentialAction::Cascade),
            on_update: Some(UnifiedReferentialAction::Restrict),
        };

        assert_eq!(relationship.source_table, "posts");
        assert_eq!(relationship.target_table, "users");
        assert_eq!(relationship.on_delete, Some(UnifiedReferentialAction::Cascade));
        assert_eq!(relationship.on_update, Some(UnifiedReferentialAction::Restrict));
    }

    #[test]
    fn test_metadata_operations() {
        let mut schema = UnifiedDatabaseSchema::new(DatabaseDialect::SQLite);
        schema.metadata.insert("encoding".to_string(), "UTF-8".to_string());
        schema.metadata.insert("page_size".to_string(), "4096".to_string());

        assert_eq!(schema.metadata.get("encoding"), Some(&"UTF-8".to_string()));
        assert_eq!(schema.metadata.get("page_size"), Some(&"4096".to_string()));
        assert_eq!(schema.metadata.get("nonexistent"), None);

        let mut table = UnifiedTableSchema::new("test".to_string(), UnifiedTableType::Table);
        table.metadata.insert("engine".to_string(), "InnoDB".to_string());

        assert_eq!(table.metadata.get("engine"), Some(&"InnoDB".to_string()));
    }

    #[test]
    fn test_complex_schema_operations() {
        let schema = create_sample_schema();

        // Test comprehensive schema operations
        assert_eq!(schema.tables.len(), 2);
        assert_eq!(schema.total_column_count(), 9);

        // Count different column types
        let mut boolean_count = 0;
        let mut integer_count = 0;
        let mut varchar_count = 0;
        let mut text_count = 0;
        let mut datetime_count = 0;

        for table in &schema.tables {
            for column in &table.columns {
                match column.column_type {
                    UnifiedColumnType::Boolean => boolean_count += 1,
                    UnifiedColumnType::Integer => integer_count += 1,
                    UnifiedColumnType::VarChar => varchar_count += 1,
                    UnifiedColumnType::Text => text_count += 1,
                    UnifiedColumnType::DateTime => datetime_count += 1,
                    _ => {}
                }
            }
        }

        assert_eq!(boolean_count, 1); // is_active
        assert_eq!(integer_count, 3); // id columns + user_id
        assert_eq!(varchar_count, 3); // name, email, title
        assert_eq!(text_count, 1); // content
        assert_eq!(datetime_count, 1); // created_at

        // Test foreign key consistency
        let all_fks = schema.all_foreign_keys();
        for (table_name, fk) in &all_fks {
            // Verify source table exists
            assert!(schema.get_table(table_name).is_some());
            // Verify target table exists
            assert!(schema.get_table(&fk.referenced_table).is_some());
        }
    }
}