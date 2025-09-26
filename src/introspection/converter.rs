/// Schema Conversion Utilities
/// 
/// This module provides utilities to convert from database-specific schema representations
/// to the unified schema representation. Each database backend uses these converters to
/// normalize their introspection results into a consistent format.
///
/// # Design Principles
/// 
/// - **Database-Specific Knowledge**: Understands the peculiarities of each database
/// - **Type Normalization**: Maps database-specific types to unified types
/// - **Constraint Mapping**: Handles database-specific constraint representations
/// - **Metadata Preservation**: Preserves important database-specific metadata

use super::schema::*;
use crate::dialects::DatabaseDialect;
use crate::auto_migration::introspector::{
    DatabaseSchema, TableSchema, ColumnSchema, IndexSchema, 
    ForeignKeySchema, ConstraintSchema, ConstraintType
};
use std::collections::HashMap;

/// Main converter for database schemas
pub struct SchemaConverter {
    dialect: DatabaseDialect,
}

impl SchemaConverter {
    /// Create a new schema converter for the specified dialect
    pub fn new(dialect: DatabaseDialect) -> Self {
        Self { dialect }
    }

    /// Convert a legacy DatabaseSchema to UnifiedDatabaseSchema
    pub fn convert_database_schema(&self, legacy_schema: DatabaseSchema) -> UnifiedDatabaseSchema {
        let mut unified_schema = UnifiedDatabaseSchema::new(self.dialect);
        
        for table in legacy_schema.tables {
            let unified_table = self.convert_table_schema(table);
            unified_schema.add_table(unified_table);
        }
        
        unified_schema
    }

    /// Convert a legacy TableSchema to UnifiedTableSchema  
    pub fn convert_table_schema(&self, legacy_table: TableSchema) -> UnifiedTableSchema {
        let mut unified_table = UnifiedTableSchema::new(
            legacy_table.name.clone(), 
            UnifiedTableType::Table // Default to table type
        );

        // Convert columns
        for column in legacy_table.columns {
            let unified_column = self.convert_column_schema(column);
            unified_table.add_column(unified_column);
        }

        // Convert indexes
        for index in legacy_table.indexes {
            let unified_index = self.convert_index_schema(index, &legacy_table.name);
            unified_table.add_index(unified_index);
        }

        // Convert foreign keys
        for fk in legacy_table.foreign_keys {
            let unified_fk = self.convert_foreign_key_schema(fk);
            unified_table.add_foreign_key(unified_fk);
        }

        // Convert constraints
        for constraint in legacy_table.constraints {
            let unified_constraint = self.convert_constraint_schema(constraint);
            unified_table.add_constraint(unified_constraint);
        }

        unified_table
    }

    /// Convert a legacy ColumnSchema to UnifiedColumnSchema
    pub fn convert_column_schema(&self, legacy_column: ColumnSchema) -> UnifiedColumnSchema {
        let unified_type = self.normalize_column_type(&legacy_column.column_type);
        
        let mut unified_column = UnifiedColumnSchema::new(
            legacy_column.name,
            unified_type,
            legacy_column.column_type.clone()
        );

        unified_column.nullable = legacy_column.nullable;
        unified_column.default_value = legacy_column.default_value;
        unified_column.primary_key = legacy_column.primary_key;
        unified_column.auto_increment = legacy_column.auto_increment;
        unified_column.unique = legacy_column.unique;

        // Convert column-level constraints
        for constraint in legacy_column.constraints {
            let unified_constraint = self.convert_column_constraint(constraint);
            unified_column.constraints.push(unified_constraint);
        }

        unified_column
    }

    /// Convert a legacy IndexSchema to UnifiedIndexSchema
    pub fn convert_index_schema(&self, legacy_index: IndexSchema, table_name: &str) -> UnifiedIndexSchema {
        UnifiedIndexSchema {
            name: legacy_index.name,
            table_name: legacy_index.table_name.unwrap_or_else(|| table_name.to_string()),
            columns: legacy_index.columns,
            unique: legacy_index.unique,
            primary: false, // Legacy schema doesn't distinguish primary key indexes
            index_type: UnifiedIndexType::BTree, // Default to B-tree
            condition: None,
            comment: None,
        }
    }

    /// Convert a legacy ForeignKeySchema to UnifiedForeignKeySchema
    pub fn convert_foreign_key_schema(&self, legacy_fk: ForeignKeySchema) -> UnifiedForeignKeySchema {
        UnifiedForeignKeySchema {
            name: legacy_fk.name,
            columns: legacy_fk.columns,
            referenced_table: legacy_fk.referenced_table,
            referenced_columns: legacy_fk.referenced_columns,
            on_delete: legacy_fk.on_delete.as_ref().map(|action| self.parse_referential_action(action)),
            on_update: legacy_fk.on_update.as_ref().map(|action| self.parse_referential_action(action)),
            deferrable: false, // Legacy schema doesn't track this
            initially_deferred: false,
        }
    }

    /// Convert a legacy ConstraintSchema to UnifiedConstraintSchema
    pub fn convert_constraint_schema(&self, legacy_constraint: ConstraintSchema) -> UnifiedConstraintSchema {
        let unified_type = match legacy_constraint.constraint_type {
            ConstraintType::PrimaryKey => UnifiedConstraintType::PrimaryKey,
            ConstraintType::Unique => UnifiedConstraintType::Unique,
            ConstraintType::ForeignKey => UnifiedConstraintType::ForeignKey,
            ConstraintType::Check => UnifiedConstraintType::Check,
            ConstraintType::NotNull => UnifiedConstraintType::NotNull,
        };

        UnifiedConstraintSchema {
            name: legacy_constraint.name,
            constraint_type: unified_type,
            definition: legacy_constraint.definition,
            columns: Vec::new(), // Legacy schema doesn't track this explicitly
            deferrable: false,
            initially_deferred: false,
        }
    }

    /// Convert a legacy column constraint
    pub fn convert_column_constraint(&self, legacy_constraint: crate::auto_migration::introspector::ColumnConstraint) -> UnifiedColumnConstraint {
        match legacy_constraint {
            crate::auto_migration::introspector::ColumnConstraint::Check { expression } => {
                UnifiedColumnConstraint::Check { expression }
            },
            crate::auto_migration::introspector::ColumnConstraint::References { table, column } => {
                UnifiedColumnConstraint::References { table, column }
            },
        }
    }

    /// Normalize database-specific column types to unified types
    pub fn normalize_column_type(&self, database_type: &str) -> UnifiedColumnType {
        let upper_type = database_type.to_uppercase();
        
        match self.dialect {
            DatabaseDialect::SQLite => self.normalize_sqlite_type(&upper_type),
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => self.normalize_postgresql_type(&upper_type),
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => self.normalize_mysql_type(&upper_type),
        }
    }

    /// Normalize SQLite column types
    fn normalize_sqlite_type(&self, sqlite_type: &str) -> UnifiedColumnType {
        match sqlite_type {
            t if t.starts_with("TEXT") || t.starts_with("VARCHAR") => UnifiedColumnType::Text,
            t if t.starts_with("INTEGER") || t.starts_with("INT") => UnifiedColumnType::Integer,
            t if t.starts_with("REAL") || t.starts_with("FLOAT") || t.starts_with("DOUBLE") => UnifiedColumnType::Real,
            t if t.starts_with("BLOB") => UnifiedColumnType::Blob,
            t if t.starts_with("BOOLEAN") || t.starts_with("BOOL") => UnifiedColumnType::Boolean,
            t if t.starts_with("DATETIME") || t.starts_with("TIMESTAMP") => UnifiedColumnType::DateTime,
            t if t.starts_with("DATE") => UnifiedColumnType::Date,
            t if t.starts_with("TIME") => UnifiedColumnType::Time,
            _ => UnifiedColumnType::Other(sqlite_type.to_string()),
        }
    }

    /// Normalize PostgreSQL column types
    #[cfg(feature = "postgres")]
    fn normalize_postgresql_type(&self, pg_type: &str) -> UnifiedColumnType {
        match pg_type {
            t if t.starts_with("BOOLEAN") || t.starts_with("BOOL") => UnifiedColumnType::Boolean,
            t if t.starts_with("SMALLINT") || t == "INT2" => UnifiedColumnType::SmallInt,
            t if t.starts_with("INTEGER") || t == "INT4" || t.starts_with("SERIAL") => UnifiedColumnType::Integer,
            t if t.starts_with("BIGINT") || t == "INT8" || t.starts_with("BIGSERIAL") => UnifiedColumnType::BigInt,
            t if t.starts_with("REAL") || t == "FLOAT4" => UnifiedColumnType::Real,
            t if t.starts_with("DOUBLE") || t == "FLOAT8" => UnifiedColumnType::Double,
            t if t.starts_with("NUMERIC") || t.starts_with("DECIMAL") => UnifiedColumnType::Decimal,
            t if t.starts_with("CHAR") && !t.starts_with("CHARACTER") => UnifiedColumnType::Char,
            t if t.starts_with("VARCHAR") || t.starts_with("CHARACTER VARYING") => UnifiedColumnType::VarChar,
            t if t.starts_with("TEXT") => UnifiedColumnType::Text,
            t if t.starts_with("BYTEA") => UnifiedColumnType::Blob,
            t if t.starts_with("JSON") => UnifiedColumnType::Json,
            t if t.starts_with("UUID") => UnifiedColumnType::Uuid,
            "TIMESTAMPTZ" => UnifiedColumnType::Timestamp,
            t if t.starts_with("TIMESTAMP") => UnifiedColumnType::DateTime,
            t if t.starts_with("DATE") => UnifiedColumnType::Date,
            t if t.starts_with("TIME") => UnifiedColumnType::Time,
            _ => UnifiedColumnType::Other(pg_type.to_string()),
        }
    }

    /// Normalize MySQL column types
    #[cfg(feature = "mysql")]
    fn normalize_mysql_type(&self, mysql_type: &str) -> UnifiedColumnType {
        match mysql_type {
            t if t.starts_with("BOOLEAN") || t.starts_with("BOOL") => UnifiedColumnType::Boolean,
            "TINYINT(1)" => UnifiedColumnType::Boolean, // MySQL boolean convention
            t if t.starts_with("TINYINT") => UnifiedColumnType::SmallInt,
            t if t.starts_with("SMALLINT") => UnifiedColumnType::SmallInt,
            t if t.starts_with("MEDIUMINT") || t.starts_with("INT") => UnifiedColumnType::Integer,
            t if t.starts_with("BIGINT") => UnifiedColumnType::BigInt,
            t if t.starts_with("FLOAT") => UnifiedColumnType::Real,
            t if t.starts_with("DOUBLE") => UnifiedColumnType::Double,
            t if t.starts_with("DECIMAL") || t.starts_with("NUMERIC") => UnifiedColumnType::Decimal,
            t if t.starts_with("CHAR") => UnifiedColumnType::Char,
            t if t.starts_with("VARCHAR") => UnifiedColumnType::VarChar,
            t if t.starts_with("TEXT") || t.starts_with("LONGTEXT") || t.starts_with("MEDIUMTEXT") => UnifiedColumnType::Text,
            t if t.starts_with("BLOB") || t.starts_with("LONGBLOB") || t.starts_with("MEDIUMBLOB") => UnifiedColumnType::Blob,
            t if t.starts_with("JSON") => UnifiedColumnType::Json,
            t if t.starts_with("DATE") => UnifiedColumnType::Date,
            t if t.starts_with("TIME") => UnifiedColumnType::Time,
            t if t.starts_with("DATETIME") => UnifiedColumnType::DateTime,
            t if t.starts_with("TIMESTAMP") => UnifiedColumnType::Timestamp,
            _ => UnifiedColumnType::Other(mysql_type.to_string()),
        }
    }

    /// Parse referential action from string
    fn parse_referential_action(&self, action: &str) -> UnifiedReferentialAction {
        match action.to_uppercase().as_str() {
            "RESTRICT" => UnifiedReferentialAction::Restrict,
            "CASCADE" => UnifiedReferentialAction::Cascade,
            "SET NULL" => UnifiedReferentialAction::SetNull,
            "SET DEFAULT" => UnifiedReferentialAction::SetDefault,
            "NO ACTION" => UnifiedReferentialAction::NoAction,
            _ => UnifiedReferentialAction::NoAction, // Default fallback
        }
    }
}

/// Direct conversion functions for new introspectors
impl SchemaConverter {
    /// Convert serde_json::Value rows to UnifiedDatabaseSchema
    /// Used by modern introspectors that work directly with query results
    pub fn convert_from_query_results(
        &self,
        tables: Vec<String>,
        table_results: HashMap<String, TableQueryResults>
    ) -> UnifiedDatabaseSchema {
        let mut unified_schema = UnifiedDatabaseSchema::new(self.dialect);

        for table_name in tables {
            if let Some(results) = table_results.get(&table_name) {
                let unified_table = self.convert_from_table_results(&table_name, results);
                unified_schema.add_table(unified_table);
            }
        }

        unified_schema
    }

    /// Convert query results for a single table to UnifiedTableSchema
    fn convert_from_table_results(&self, table_name: &str, results: &TableQueryResults) -> UnifiedTableSchema {
        let mut unified_table = UnifiedTableSchema::new(
            table_name.to_string(),
            UnifiedTableType::Table
        );

        // Convert columns from query results
        for column_row in &results.columns {
            if let Some(unified_column) = self.convert_column_from_query(column_row) {
                unified_table.add_column(unified_column);
            }
        }

        // Convert indexes from query results
        for index_row in &results.indexes {
            if let Some(unified_index) = self.convert_index_from_query(index_row, table_name) {
                unified_table.add_index(unified_index);
            }
        }

        // Convert foreign keys from query results
        for fk_row in &results.foreign_keys {
            if let Some(unified_fk) = self.convert_foreign_key_from_query(fk_row) {
                unified_table.add_foreign_key(unified_fk);
            }
        }

        // Convert constraints from query results
        for constraint_row in &results.constraints {
            if let Some(unified_constraint) = self.convert_constraint_from_query(constraint_row) {
                unified_table.add_constraint(unified_constraint);
            }
        }

        unified_table
    }

    /// Convert column query result to UnifiedColumnSchema
    fn convert_column_from_query(&self, row: &serde_json::Value) -> Option<UnifiedColumnSchema> {
        let obj = row.as_object()?;
        
        let name = obj.get("column_name")?.as_str()?.to_string();
        let raw_type = obj.get("data_type")?.as_str()?.to_string();
        let unified_type = self.normalize_column_type(&raw_type);
        
        let mut column = UnifiedColumnSchema::new(name, unified_type, raw_type);
        
        // Set nullable
        if let Some(is_nullable) = obj.get("is_nullable").and_then(|v| v.as_str()) {
            column.nullable = is_nullable.to_uppercase() == "YES";
        }
        
        // Set default value
        if let Some(default) = obj.get("column_default").and_then(|v| v.as_str()) {
            column.default_value = Some(default.to_string());
        }
        
        // Set character maximum length
        if let Some(max_length) = obj.get("character_maximum_length").and_then(|v| v.as_i64()) {
            column.character_maximum_length = Some(max_length as i32);
        }
        
        // Set numeric precision and scale
        if let Some(precision) = obj.get("numeric_precision").and_then(|v| v.as_i64()) {
            column.numeric_precision = Some(precision as i32);
        }
        if let Some(scale) = obj.get("numeric_scale").and_then(|v| v.as_i64()) {
            column.numeric_scale = Some(scale as i32);
        }
        
        // Set ordinal position
        if let Some(position) = obj.get("ordinal_position").and_then(|v| v.as_i64()) {
            column.ordinal_position = Some(position as i32);
        }

        Some(column)
    }

    /// Convert index query result to UnifiedIndexSchema
    fn convert_index_from_query(&self, row: &serde_json::Value, table_name: &str) -> Option<UnifiedIndexSchema> {
        let obj = row.as_object()?;
        
        let name = obj.get("index_name")?.as_str()?.to_string();
        let column_name = obj.get("column_name")?.as_str()?.to_string();
        
        // Note: This is simplified - real implementation would need to group columns by index
        Some(UnifiedIndexSchema {
            name,
            table_name: table_name.to_string(),
            columns: vec![column_name],
            unique: obj.get("unique").and_then(|v| v.as_bool()).unwrap_or(false),
            primary: obj.get("primary").and_then(|v| v.as_bool()).unwrap_or(false),
            index_type: UnifiedIndexType::BTree,
            condition: obj.get("condition").and_then(|v| v.as_str()).map(|s| s.to_string()),
            comment: obj.get("comment").and_then(|v| v.as_str()).map(|s| s.to_string()),
        })
    }

    /// Convert foreign key query result to UnifiedForeignKeySchema
    fn convert_foreign_key_from_query(&self, row: &serde_json::Value) -> Option<UnifiedForeignKeySchema> {
        let obj = row.as_object()?;
        
        let name = obj.get("constraint_name")?.as_str()?.to_string();
        let column = obj.get("column_name")?.as_str()?.to_string();
        let referenced_table = obj.get("referenced_table_name")?.as_str()?.to_string();
        let referenced_column = obj.get("referenced_column_name")?.as_str()?.to_string();
        
        // Note: This is simplified - real implementation would need to group columns by FK
        Some(UnifiedForeignKeySchema {
            name,
            columns: vec![column],
            referenced_table,
            referenced_columns: vec![referenced_column],
            on_delete: obj.get("delete_rule")
                .and_then(|v| v.as_str())
                .map(|s| self.parse_referential_action(s)),
            on_update: obj.get("update_rule")
                .and_then(|v| v.as_str())
                .map(|s| self.parse_referential_action(s)),
            deferrable: false,
            initially_deferred: false,
        })
    }

    /// Convert constraint query result to UnifiedConstraintSchema
    fn convert_constraint_from_query(&self, row: &serde_json::Value) -> Option<UnifiedConstraintSchema> {
        let obj = row.as_object()?;
        
        let name = obj.get("constraint_name")?.as_str()?.to_string();
        let constraint_type_str = obj.get("constraint_type")?.as_str()?;
        
        let constraint_type = match constraint_type_str.to_uppercase().as_str() {
            "PRIMARY KEY" => UnifiedConstraintType::PrimaryKey,
            "UNIQUE" => UnifiedConstraintType::Unique,
            "FOREIGN KEY" => UnifiedConstraintType::ForeignKey,
            "CHECK" => UnifiedConstraintType::Check,
            _ => UnifiedConstraintType::Other(constraint_type_str.to_string()),
        };
        
        Some(UnifiedConstraintSchema {
            name,
            constraint_type,
            definition: obj.get("check_clause")
                .or_else(|| obj.get("constraint_definition"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            columns: Vec::new(), // Would need additional query to populate
            deferrable: false,
            initially_deferred: false,
        })
    }
}

/// Query results structure for table introspection
/// Used by modern introspectors to pass structured results to converter
#[derive(Debug, Clone)]
pub struct TableQueryResults {
    pub columns: Vec<serde_json::Value>,
    pub indexes: Vec<serde_json::Value>,
    pub foreign_keys: Vec<serde_json::Value>,
    pub constraints: Vec<serde_json::Value>,
}

impl TableQueryResults {
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
            indexes: Vec::new(),
            foreign_keys: Vec::new(),
            constraints: Vec::new(),
        }
    }
}

/// Type mapping utilities for cross-database compatibility
pub struct TypeMapper;

impl TypeMapper {
    /// Map unified type to database-specific type for DDL generation
    pub fn to_database_type(unified_type: &UnifiedColumnType, dialect: DatabaseDialect) -> String {
        match dialect {
            DatabaseDialect::SQLite => Self::to_sqlite_type(unified_type),
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => Self::to_postgresql_type(unified_type),
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => Self::to_mysql_type(unified_type),
        }
    }

    fn to_sqlite_type(unified_type: &UnifiedColumnType) -> String {
        match unified_type {
            UnifiedColumnType::Boolean => "INTEGER".to_string(), // SQLite stores booleans as integers
            UnifiedColumnType::SmallInt | UnifiedColumnType::Integer | UnifiedColumnType::BigInt => "INTEGER".to_string(),
            UnifiedColumnType::Real | UnifiedColumnType::Double | UnifiedColumnType::Decimal => "REAL".to_string(),
            UnifiedColumnType::Char | UnifiedColumnType::VarChar | UnifiedColumnType::Text => "TEXT".to_string(),
            UnifiedColumnType::Blob => "BLOB".to_string(),
            UnifiedColumnType::Json => "TEXT".to_string(), // SQLite stores JSON as text
            UnifiedColumnType::Date | UnifiedColumnType::Time | UnifiedColumnType::DateTime | UnifiedColumnType::Timestamp => "TEXT".to_string(),
            UnifiedColumnType::Uuid => "TEXT".to_string(),
            UnifiedColumnType::Other(type_name) => type_name.clone(),
        }
    }

    #[cfg(feature = "postgres")]
    fn to_postgresql_type(unified_type: &UnifiedColumnType) -> String {
        match unified_type {
            UnifiedColumnType::Boolean => "BOOLEAN".to_string(),
            UnifiedColumnType::SmallInt => "SMALLINT".to_string(),
            UnifiedColumnType::Integer => "INTEGER".to_string(),
            UnifiedColumnType::BigInt => "BIGINT".to_string(),
            UnifiedColumnType::Real => "REAL".to_string(),
            UnifiedColumnType::Double => "DOUBLE PRECISION".to_string(),
            UnifiedColumnType::Decimal => "NUMERIC".to_string(),
            UnifiedColumnType::Char => "CHAR".to_string(),
            UnifiedColumnType::VarChar => "VARCHAR".to_string(),
            UnifiedColumnType::Text => "TEXT".to_string(),
            UnifiedColumnType::Blob => "BYTEA".to_string(),
            UnifiedColumnType::Json => "JSONB".to_string(),
            UnifiedColumnType::Date => "DATE".to_string(),
            UnifiedColumnType::Time => "TIME".to_string(),
            UnifiedColumnType::DateTime => "TIMESTAMP".to_string(),
            UnifiedColumnType::Timestamp => "TIMESTAMPTZ".to_string(),
            UnifiedColumnType::Uuid => "UUID".to_string(),
            UnifiedColumnType::Other(type_name) => type_name.clone(),
        }
    }

    #[cfg(feature = "mysql")]
    fn to_mysql_type(unified_type: &UnifiedColumnType) -> String {
        match unified_type {
            UnifiedColumnType::Boolean => "TINYINT(1)".to_string(),
            UnifiedColumnType::SmallInt => "SMALLINT".to_string(),
            UnifiedColumnType::Integer => "INT".to_string(),
            UnifiedColumnType::BigInt => "BIGINT".to_string(),
            UnifiedColumnType::Real => "FLOAT".to_string(),
            UnifiedColumnType::Double => "DOUBLE".to_string(),
            UnifiedColumnType::Decimal => "DECIMAL".to_string(),
            UnifiedColumnType::Char => "CHAR".to_string(),
            UnifiedColumnType::VarChar => "VARCHAR".to_string(),
            UnifiedColumnType::Text => "TEXT".to_string(),
            UnifiedColumnType::Blob => "BLOB".to_string(),
            UnifiedColumnType::Json => "JSON".to_string(),
            UnifiedColumnType::Date => "DATE".to_string(),
            UnifiedColumnType::Time => "TIME".to_string(),
            UnifiedColumnType::DateTime => "DATETIME".to_string(),
            UnifiedColumnType::Timestamp => "TIMESTAMP".to_string(),
            UnifiedColumnType::Uuid => "CHAR(36)".to_string(), // MySQL doesn't have native UUID
            UnifiedColumnType::Other(type_name) => type_name.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlite_type_normalization() {
        let converter = SchemaConverter::new(DatabaseDialect::SQLite);

        assert_eq!(converter.normalize_column_type("TEXT"), UnifiedColumnType::Text);
        assert_eq!(converter.normalize_column_type("INTEGER"), UnifiedColumnType::Integer);
        assert_eq!(converter.normalize_column_type("REAL"), UnifiedColumnType::Real);
        assert_eq!(converter.normalize_column_type("BLOB"), UnifiedColumnType::Blob);
        assert_eq!(converter.normalize_column_type("BOOLEAN"), UnifiedColumnType::Boolean);
        assert_eq!(converter.normalize_column_type("DATETIME"), UnifiedColumnType::DateTime);
    }

    #[test]
    #[cfg(feature = "postgres")]
    fn test_postgresql_type_normalization() {
        let converter = SchemaConverter::new(DatabaseDialect::PostgreSQL);

        assert_eq!(converter.normalize_column_type("BOOLEAN"), UnifiedColumnType::Boolean);
        assert_eq!(converter.normalize_column_type("SMALLINT"), UnifiedColumnType::SmallInt);
        assert_eq!(converter.normalize_column_type("INTEGER"), UnifiedColumnType::Integer);
        assert_eq!(converter.normalize_column_type("BIGINT"), UnifiedColumnType::BigInt);
        assert_eq!(converter.normalize_column_type("REAL"), UnifiedColumnType::Real);
        assert_eq!(converter.normalize_column_type("DOUBLE PRECISION"), UnifiedColumnType::Double);
        assert_eq!(converter.normalize_column_type("VARCHAR"), UnifiedColumnType::VarChar);
        assert_eq!(converter.normalize_column_type("TEXT"), UnifiedColumnType::Text);
        assert_eq!(converter.normalize_column_type("BYTEA"), UnifiedColumnType::Blob);
        assert_eq!(converter.normalize_column_type("JSON"), UnifiedColumnType::Json);
        assert_eq!(converter.normalize_column_type("UUID"), UnifiedColumnType::Uuid);
        assert_eq!(converter.normalize_column_type("TIMESTAMPTZ"), UnifiedColumnType::Timestamp);
    }

    #[test]
    #[cfg(feature = "mysql")]
    fn test_mysql_type_normalization() {
        let converter = SchemaConverter::new(DatabaseDialect::MySQL);

        assert_eq!(converter.normalize_column_type("BOOLEAN"), UnifiedColumnType::Boolean);
        assert_eq!(converter.normalize_column_type("TINYINT(1)"), UnifiedColumnType::Boolean);
        assert_eq!(converter.normalize_column_type("SMALLINT"), UnifiedColumnType::SmallInt);
        assert_eq!(converter.normalize_column_type("INT"), UnifiedColumnType::Integer);
        assert_eq!(converter.normalize_column_type("BIGINT"), UnifiedColumnType::BigInt);
        assert_eq!(converter.normalize_column_type("FLOAT"), UnifiedColumnType::Real);
        assert_eq!(converter.normalize_column_type("DOUBLE"), UnifiedColumnType::Double);
        assert_eq!(converter.normalize_column_type("VARCHAR"), UnifiedColumnType::VarChar);
        assert_eq!(converter.normalize_column_type("TEXT"), UnifiedColumnType::Text);
        assert_eq!(converter.normalize_column_type("BLOB"), UnifiedColumnType::Blob);
        assert_eq!(converter.normalize_column_type("JSON"), UnifiedColumnType::Json);
    }

    #[test]
    fn test_referential_action_parsing() {
        let converter = SchemaConverter::new(DatabaseDialect::SQLite);

        assert_eq!(converter.parse_referential_action("CASCADE"), UnifiedReferentialAction::Cascade);
        assert_eq!(converter.parse_referential_action("RESTRICT"), UnifiedReferentialAction::Restrict);
        assert_eq!(converter.parse_referential_action("SET NULL"), UnifiedReferentialAction::SetNull);
        assert_eq!(converter.parse_referential_action("SET DEFAULT"), UnifiedReferentialAction::SetDefault);
        assert_eq!(converter.parse_referential_action("NO ACTION"), UnifiedReferentialAction::NoAction);
        assert_eq!(converter.parse_referential_action("UNKNOWN"), UnifiedReferentialAction::NoAction);
    }

    #[test]
    fn test_type_mapping_sqlite() {
        assert_eq!(TypeMapper::to_database_type(&UnifiedColumnType::Boolean, DatabaseDialect::SQLite), "INTEGER");
        assert_eq!(TypeMapper::to_database_type(&UnifiedColumnType::Integer, DatabaseDialect::SQLite), "INTEGER");
        assert_eq!(TypeMapper::to_database_type(&UnifiedColumnType::Text, DatabaseDialect::SQLite), "TEXT");
        assert_eq!(TypeMapper::to_database_type(&UnifiedColumnType::Blob, DatabaseDialect::SQLite), "BLOB");
        assert_eq!(TypeMapper::to_database_type(&UnifiedColumnType::Json, DatabaseDialect::SQLite), "TEXT");
    }

    #[test]
    #[cfg(feature = "postgres")]
    fn test_type_mapping_postgresql() {
        assert_eq!(TypeMapper::to_database_type(&UnifiedColumnType::Boolean, DatabaseDialect::PostgreSQL), "BOOLEAN");
        assert_eq!(TypeMapper::to_database_type(&UnifiedColumnType::Integer, DatabaseDialect::PostgreSQL), "INTEGER");
        assert_eq!(TypeMapper::to_database_type(&UnifiedColumnType::Text, DatabaseDialect::PostgreSQL), "TEXT");
        assert_eq!(TypeMapper::to_database_type(&UnifiedColumnType::Blob, DatabaseDialect::PostgreSQL), "BYTEA");
        assert_eq!(TypeMapper::to_database_type(&UnifiedColumnType::Json, DatabaseDialect::PostgreSQL), "JSONB");
        assert_eq!(TypeMapper::to_database_type(&UnifiedColumnType::Uuid, DatabaseDialect::PostgreSQL), "UUID");
    }

    #[test]
    #[cfg(feature = "mysql")]
    fn test_type_mapping_mysql() {
        assert_eq!(TypeMapper::to_database_type(&UnifiedColumnType::Boolean, DatabaseDialect::MySQL), "TINYINT(1)");
        assert_eq!(TypeMapper::to_database_type(&UnifiedColumnType::Integer, DatabaseDialect::MySQL), "INT");
        assert_eq!(TypeMapper::to_database_type(&UnifiedColumnType::Text, DatabaseDialect::MySQL), "TEXT");
        assert_eq!(TypeMapper::to_database_type(&UnifiedColumnType::Blob, DatabaseDialect::MySQL), "BLOB");
        assert_eq!(TypeMapper::to_database_type(&UnifiedColumnType::Json, DatabaseDialect::MySQL), "JSON");
        assert_eq!(TypeMapper::to_database_type(&UnifiedColumnType::Uuid, DatabaseDialect::MySQL), "CHAR(36)");
    }

    #[test]
    fn test_unified_column_type_methods() {
        assert!(UnifiedColumnType::Integer.is_numeric());
        assert!(UnifiedColumnType::Real.is_numeric());
        assert!(!UnifiedColumnType::Text.is_numeric());

        assert!(UnifiedColumnType::Text.is_string());
        assert!(UnifiedColumnType::VarChar.is_string());
        assert!(!UnifiedColumnType::Integer.is_string());

        assert!(UnifiedColumnType::DateTime.is_temporal());
        assert!(UnifiedColumnType::Date.is_temporal());
        assert!(!UnifiedColumnType::Text.is_temporal());

        assert!(UnifiedColumnType::Blob.is_binary());
        assert!(!UnifiedColumnType::Text.is_binary());
    }

    #[test]
    fn test_unified_column_storage_size_estimation() {
        let boolean_col = UnifiedColumnSchema::new("test".to_string(), UnifiedColumnType::Boolean, "BOOLEAN".to_string());
        assert_eq!(boolean_col.estimated_storage_size(), Some(1));

        let int_col = UnifiedColumnSchema::new("test".to_string(), UnifiedColumnType::Integer, "INTEGER".to_string());
        assert_eq!(int_col.estimated_storage_size(), Some(4));

        let mut varchar_col = UnifiedColumnSchema::new("test".to_string(), UnifiedColumnType::VarChar, "VARCHAR".to_string());
        varchar_col.character_maximum_length = Some(255);
        assert_eq!(varchar_col.estimated_storage_size(), Some(255));

        let text_col = UnifiedColumnSchema::new("test".to_string(), UnifiedColumnType::Text, "TEXT".to_string());
        assert_eq!(text_col.estimated_storage_size(), None); // Variable size
    }

    #[test]
    fn test_schema_validation() {
        let mut schema = UnifiedDatabaseSchema::new(DatabaseDialect::SQLite);
        
        // Create valid table with primary key
        let mut table = UnifiedTableSchema::new("users".to_string(), UnifiedTableType::Table);
        let mut id_col = UnifiedColumnSchema::new("id".to_string(), UnifiedColumnType::Integer, "INTEGER".to_string());
        id_col.primary_key = true;
        table.add_column(id_col);
        
        schema.add_table(table);
        
        // Should validate successfully
        assert!(schema.validate().is_ok());
        
        // Create invalid table without primary key
        let invalid_table = UnifiedTableSchema::new("invalid".to_string(), UnifiedTableType::Table);
        schema.add_table(invalid_table);
        
        // Should fail validation
        assert!(schema.validate().is_err());
    }
}