/// Database-Agnostic DDL Generation Engine
/// 
/// This module provides comprehensive DDL generation capabilities for migration operations
/// across SQLite, PostgreSQL, and MySQL databases using sea-query for type-safe SQL generation.

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use thiserror::Error;

use crate::dialects::DatabaseDialect;
use crate::migration_engine::{
    MigrationOperation, MigrationPlan, TableOperation, ColumnOperation,
    IndexOperation, ConstraintOperation
};
use crate::introspection::{
    UnifiedTableSchema, UnifiedColumnSchema, UnifiedIndexSchema, UnifiedConstraintSchema,
    UnifiedColumnType
};
use sea_query::{
    Alias, ColumnDef, Table, Index, SqliteQueryBuilder, ColumnType, StringLen, IntoIden
};

#[cfg(feature = "postgres")]
use sea_query::PostgresQueryBuilder;

#[cfg(feature = "mysql")]
use sea_query::MysqlQueryBuilder;

/// Comprehensive error types for DDL generation operations
#[derive(Debug, Error, Clone, PartialEq)]
pub enum DDLError {
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),
    
    #[error("Invalid column type conversion: {0}")]
    InvalidColumnType(String),
    
    #[error("Schema validation error: {0}")]
    SchemaValidation(String),
    
    #[error("SQL generation failed: {0}")]
    SqlGeneration(String),
}

/// Generated DDL statement with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DDLStatement {
    /// The generated SQL statement
    pub sql: String,
    /// Type of operation (CREATE TABLE, DROP INDEX, etc.)
    pub operation_type: DDLOperationType,
    /// Database dialect this statement was generated for
    pub dialect: DatabaseDialect,
    /// Estimated execution time in milliseconds
    pub estimated_duration_ms: u64,
    /// Whether this operation is reversible
    pub is_reversible: bool,
    /// Tables affected by this statement
    pub affected_tables: Vec<String>,
}

/// Types of DDL operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DDLOperationType {
    CreateTable,
    DropTable,
    AlterTable,
    CreateIndex,
    DropIndex,
    AddConstraint,
    DropConstraint,
}

impl DDLOperationType {
    /// Get display name for the operation type
    pub fn display_name(&self) -> &'static str {
        match self {
            DDLOperationType::CreateTable => "CREATE TABLE",
            DDLOperationType::DropTable => "DROP TABLE",
            DDLOperationType::AlterTable => "ALTER TABLE",
            DDLOperationType::CreateIndex => "CREATE INDEX",
            DDLOperationType::DropIndex => "DROP INDEX",
            DDLOperationType::AddConstraint => "ADD CONSTRAINT",
            DDLOperationType::DropConstraint => "DROP CONSTRAINT",
        }
    }
}

/// Complete DDL generation result for a migration plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DDLGenerationResult {
    /// All generated DDL statements in execution order
    pub statements: Vec<DDLStatement>,
    /// Target database dialect
    pub dialect: DatabaseDialect,
    /// Total estimated execution time
    pub total_estimated_duration_ms: u64,
    /// Number of reversible operations
    pub reversible_operations: usize,
    /// Number of irreversible operations (data loss risk)
    pub irreversible_operations: usize,
    /// Comprehensive execution statistics
    pub statistics: DDLStatistics,
}

/// Statistical information about DDL generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DDLStatistics {
    /// Total number of statements generated
    pub total_statements: usize,
    /// Breakdown by operation type
    pub operations_by_type: HashMap<DDLOperationType, usize>,
    /// Breakdown by affected table
    pub operations_by_table: HashMap<String, usize>,
    /// Number of operations per database dialect feature
    pub dialect_specific_operations: usize,
    /// Cross-database compatible operations
    pub cross_database_operations: usize,
}

/// Database-agnostic DDL generator engine
pub struct DDLGenerator {
    /// Target database dialect
    dialect: DatabaseDialect,
}

impl DDLGenerator {
    /// Create a new DDL generator for the specified database dialect
    pub fn new(dialect: DatabaseDialect) -> Self {
        Self {
            dialect,
        }
    }

    /// Generate complete DDL for a migration plan
    pub fn generate_ddl(&self, plan: &MigrationPlan) -> Result<DDLGenerationResult, DDLError> {
        let mut statements = Vec::new();
        let mut total_duration = 0u64;
        let mut reversible_count = 0usize;
        let mut irreversible_count = 0usize;

        // Process operations in dependency order
        for operation in &plan.operations {
            let mut operation_statements = self.generate_operation_ddl(operation)?;
            
            for statement in &mut operation_statements {
                total_duration += statement.estimated_duration_ms;
                if statement.is_reversible {
                    reversible_count += 1;
                } else {
                    irreversible_count += 1;
                }
            }
            
            statements.extend(operation_statements);
        }

        // Generate comprehensive statistics
        let statistics = self.generate_statistics(&statements);

        Ok(DDLGenerationResult {
            statements,
            dialect: self.dialect,
            total_estimated_duration_ms: total_duration,
            reversible_operations: reversible_count,
            irreversible_operations: irreversible_count,
            statistics,
        })
    }

    /// Generate DDL statements for a single migration operation
    pub fn generate_operation_ddl(&self, operation: &MigrationOperation) -> Result<Vec<DDLStatement>, DDLError> {
        match operation {
            MigrationOperation::Table(table_op) => self.generate_table_ddl(table_op),
            MigrationOperation::Column(column_op) => self.generate_column_ddl(column_op),
            MigrationOperation::Index(index_op) => self.generate_index_ddl(index_op),
            MigrationOperation::Constraint(constraint_op) => self.generate_constraint_ddl(constraint_op),
        }
    }

    /// Generate DDL for table operations
    fn generate_table_ddl(&self, operation: &TableOperation) -> Result<Vec<DDLStatement>, DDLError> {
        let mut statements = Vec::new();
        
        match operation {
            TableOperation::CreateTable { schema, .. } => {
                let sql = self.generate_create_table_sql(schema)?;
                statements.push(DDLStatement {
                    sql,
                    operation_type: DDLOperationType::CreateTable,
                    dialect: self.dialect,
                    estimated_duration_ms: self.estimate_create_table_duration(schema),
                    is_reversible: true,
                    affected_tables: vec![schema.name.clone()],
                });
            },
            TableOperation::DropTable { table_name } => {
                let sql = self.generate_drop_table_sql(table_name)?;
                statements.push(DDLStatement {
                    sql,
                    operation_type: DDLOperationType::DropTable,
                    dialect: self.dialect,
                    estimated_duration_ms: 100, // Quick operation
                    is_reversible: false, // Data loss
                    affected_tables: vec![table_name.clone()],
                });
            },
            TableOperation::RenameTable { old_name, new_name } => {
                let sql = self.generate_rename_table_sql(old_name, new_name)?;
                statements.push(DDLStatement {
                    sql,
                    operation_type: DDLOperationType::AlterTable,
                    dialect: self.dialect,
                    estimated_duration_ms: 50,
                    is_reversible: true,
                    affected_tables: vec![old_name.clone(), new_name.clone()],
                });
            },
        }
        
        Ok(statements)
    }

    /// Generate DDL for column operations
    fn generate_column_ddl(&self, operation: &ColumnOperation) -> Result<Vec<DDLStatement>, DDLError> {
        let mut statements = Vec::new();
        
        match operation {
            ColumnOperation::AddColumn { table_name, column } => {
                let sql = self.generate_add_column_sql(table_name, column)?;
                statements.push(DDLStatement {
                    sql,
                    operation_type: DDLOperationType::AlterTable,
                    dialect: self.dialect,
                    estimated_duration_ms: 200,
                    is_reversible: true,
                    affected_tables: vec![table_name.clone()],
                });
            },
            ColumnOperation::DropColumn { table_name, column_name } => {
                let sql = self.generate_drop_column_sql(table_name, column_name)?;
                statements.push(DDLStatement {
                    sql,
                    operation_type: DDLOperationType::AlterTable,
                    dialect: self.dialect,
                    estimated_duration_ms: 300,
                    is_reversible: false, // Data loss
                    affected_tables: vec![table_name.clone()],
                });
            },
            ColumnOperation::ModifyColumn { table_name, old_column, new_column } => {
                let sql = self.generate_modify_column_sql(table_name, old_column, new_column)?;
                statements.push(DDLStatement {
                    sql,
                    operation_type: DDLOperationType::AlterTable,
                    dialect: self.dialect,
                    estimated_duration_ms: 400,
                    is_reversible: false, // Potential data loss
                    affected_tables: vec![table_name.clone()],
                });
            },
            ColumnOperation::RenameColumn { table_name, old_name, new_name } => {
                let sql = self.generate_rename_column_sql(table_name, old_name, new_name)?;
                statements.push(DDLStatement {
                    sql,
                    operation_type: DDLOperationType::AlterTable,
                    dialect: self.dialect,
                    estimated_duration_ms: 150,
                    is_reversible: true,
                    affected_tables: vec![table_name.clone()],
                });
            },
        }
        
        Ok(statements)
    }

    /// Generate DDL for index operations
    fn generate_index_ddl(&self, operation: &IndexOperation) -> Result<Vec<DDLStatement>, DDLError> {
        let mut statements = Vec::new();
        
        match operation {
            IndexOperation::CreateIndex { index, .. } => {
                let sql = self.generate_create_index_sql(index)?;
                statements.push(DDLStatement {
                    sql,
                    operation_type: DDLOperationType::CreateIndex,
                    dialect: self.dialect,
                    estimated_duration_ms: self.estimate_index_creation_duration(index),
                    is_reversible: true,
                    affected_tables: vec![index.table_name.clone()],
                });
            },
            IndexOperation::DropIndex { table_name, index_name } => {
                let sql = self.generate_drop_index_sql(table_name, index_name)?;
                statements.push(DDLStatement {
                    sql,
                    operation_type: DDLOperationType::DropIndex,
                    dialect: self.dialect,
                    estimated_duration_ms: 100,
                    is_reversible: true,
                    affected_tables: vec![table_name.clone()],
                });
            },
            IndexOperation::ModifyIndex { .. } => {
                // Simplified handling - would need more complex logic for modifying indexes
                statements.push(DDLStatement {
                    sql: "-- ModifyIndex not implemented".to_string(),
                    operation_type: DDLOperationType::AlterTable,
                    dialect: self.dialect,
                    estimated_duration_ms: 200,
                    is_reversible: true,
                    affected_tables: vec!["unknown".to_string()],
                });
            },
        }
        
        Ok(statements)
    }

    /// Generate DDL for constraint operations
    fn generate_constraint_ddl(&self, operation: &ConstraintOperation) -> Result<Vec<DDLStatement>, DDLError> {
        let mut statements = Vec::new();
        
        match operation {
            ConstraintOperation::AddConstraint { table_name, constraint } => {
                let sql = self.generate_add_constraint_sql(table_name, constraint)?;
                statements.push(DDLStatement {
                    sql,
                    operation_type: DDLOperationType::AddConstraint,
                    dialect: self.dialect,
                    estimated_duration_ms: 250,
                    is_reversible: true,
                    affected_tables: vec![table_name.clone()],
                });
            },
            ConstraintOperation::DropConstraint { table_name, constraint_name } => {
                let sql = self.generate_drop_constraint_sql(table_name, constraint_name)?;
                statements.push(DDLStatement {
                    sql,
                    operation_type: DDLOperationType::DropConstraint,
                    dialect: self.dialect,
                    estimated_duration_ms: 150,
                    is_reversible: true,
                    affected_tables: vec![table_name.clone()],
                });
            },
            ConstraintOperation::ModifyConstraint { .. } => {
                // Simplified handling - would need more complex logic for modifying constraints
                statements.push(DDLStatement {
                    sql: "-- ModifyConstraint not implemented".to_string(),
                    operation_type: DDLOperationType::AlterTable,
                    dialect: self.dialect,
                    estimated_duration_ms: 200,
                    is_reversible: true,
                    affected_tables: vec!["unknown".to_string()],
                });
            },
        }
        
        Ok(statements)
    }

    /// Generate CREATE TABLE SQL statement
    fn generate_create_table_sql(&self, schema: &UnifiedTableSchema) -> Result<String, DDLError> {
        let mut table = Table::create()
            .table(Alias::new(&schema.name))
            .if_not_exists()
            .to_owned();

        // Add columns
        for column in &schema.columns {
            let mut column_def = self.build_column_definition(column)?;
            table.col(&mut column_def);
        }

        // Generate SQL based on dialect
        let sql = match self.dialect {
            DatabaseDialect::SQLite => table.to_string(SqliteQueryBuilder),
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => table.to_string(PostgresQueryBuilder),
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => table.to_string(MysqlQueryBuilder),
        };

        Ok(sql)
    }

    /// Generate DROP TABLE SQL statement
    fn generate_drop_table_sql(&self, table_name: &str) -> Result<String, DDLError> {
        let table = Table::drop()
            .table(Alias::new(table_name))
            .if_exists()
            .to_owned();

        let sql = match self.dialect {
            DatabaseDialect::SQLite => table.to_string(SqliteQueryBuilder),
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => table.to_string(PostgresQueryBuilder),
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => table.to_string(MysqlQueryBuilder),
        };

        Ok(sql)
    }

    /// Generate RENAME TABLE SQL statement
    fn generate_rename_table_sql(&self, old_name: &str, new_name: &str) -> Result<String, DDLError> {
        let sql = match self.dialect {
            DatabaseDialect::SQLite => {
                format!("ALTER TABLE {} RENAME TO {}", old_name, new_name)
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                format!("ALTER TABLE {} RENAME TO {}", old_name, new_name)
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                format!("RENAME TABLE {} TO {}", old_name, new_name)
            }
        };
        
        Ok(sql)
    }

    /// Generate ADD COLUMN SQL statement
    fn generate_add_column_sql(&self, table_name: &str, column: &UnifiedColumnSchema) -> Result<String, DDLError> {
        let column_def = self.build_column_definition(column)?;
        
        let alter_table = Table::alter()
            .table(Alias::new(table_name))
            .add_column(column_def)
            .to_owned();

        let sql = match self.dialect {
            DatabaseDialect::SQLite => alter_table.to_string(SqliteQueryBuilder),
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => alter_table.to_string(PostgresQueryBuilder),
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => alter_table.to_string(MysqlQueryBuilder),
        };

        Ok(sql)
    }

    /// Generate DROP COLUMN SQL statement
    fn generate_drop_column_sql(&self, _table_name: &str, _column_name: &str) -> Result<String, DDLError> {
        match self.dialect {
            DatabaseDialect::SQLite => {
                Err(DDLError::UnsupportedOperation(
                    "SQLite does not support DROP COLUMN operation. Consider recreating the table.".to_string()
                ))
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                Ok(format!("ALTER TABLE {} DROP COLUMN {}", _table_name, _column_name))
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                Ok(format!("ALTER TABLE {} DROP COLUMN {}", _table_name, _column_name))
            }
        }
    }

    /// Generate MODIFY COLUMN SQL statement
    fn generate_modify_column_sql(&self, _table_name: &str, _old_column: &UnifiedColumnSchema, new_column: &UnifiedColumnSchema) -> Result<String, DDLError> {
        let _column_def = self.build_column_definition(new_column)?;
        
        match self.dialect {
            DatabaseDialect::SQLite => {
                Err(DDLError::UnsupportedOperation(
                    "SQLite does not support MODIFY COLUMN operation. Consider recreating the table.".to_string()
                ))
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                let column_type = self.convert_unified_type_to_sea_query(&new_column.column_type)?;
                let column_def = ColumnDef::new_with_type(Alias::new(&new_column.name), column_type);
                let alter_table = Table::alter()
                    .table(Alias::new(_table_name))
                    .modify_column(column_def.clone())
                    .to_owned();
                Ok(alter_table.to_string(PostgresQueryBuilder))
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                Ok(format!("ALTER TABLE {} MODIFY COLUMN {}", _table_name, new_column.name))
            }
        }
    }

    /// Generate RENAME COLUMN SQL statement
    fn generate_rename_column_sql(&self, _table_name: &str, _old_name: &str, _new_name: &str) -> Result<String, DDLError> {
        match self.dialect {
            DatabaseDialect::SQLite => {
                Err(DDLError::UnsupportedOperation(
                    "SQLite does not support RENAME COLUMN operation. Consider recreating the table.".to_string()
                ))
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                Ok(format!("ALTER TABLE {} RENAME COLUMN {} TO {}", _table_name, _old_name, _new_name))
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                Ok(format!("ALTER TABLE {} CHANGE {} {}", _table_name, _old_name, _new_name))
            }
        }
    }

    /// Generate CREATE INDEX SQL statement
    fn generate_create_index_sql(&self, index: &UnifiedIndexSchema) -> Result<String, DDLError> {
        let mut create_index = Index::create()
            .name(&index.name)
            .table(Alias::new(&index.table_name))
            .to_owned();

        // Add columns to index
        for column_name in &index.columns {
            create_index.col(Alias::new(column_name));
        }

        let sql = match self.dialect {
            DatabaseDialect::SQLite => create_index.to_string(SqliteQueryBuilder),
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => create_index.to_string(PostgresQueryBuilder),
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => create_index.to_string(MysqlQueryBuilder),
        };

        Ok(sql)
    }

    /// Generate DROP INDEX SQL statement
    fn generate_drop_index_sql(&self, _table_name: &str, index_name: &str) -> Result<String, DDLError> {
        let drop_index = Index::drop()
            .name(index_name)
            .to_owned();

        let sql = match self.dialect {
            DatabaseDialect::SQLite => drop_index.to_string(SqliteQueryBuilder),
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => drop_index.to_string(PostgresQueryBuilder),
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => drop_index.to_string(MysqlQueryBuilder),
        };

        Ok(sql)
    }

    /// Generate ADD CONSTRAINT SQL statement
    fn generate_add_constraint_sql(&self, table_name: &str, _constraint: &UnifiedConstraintSchema) -> Result<String, DDLError> {
        // Simplified constraint handling - full implementation would handle FK, PK, etc.
        let sql = match self.dialect {
            DatabaseDialect::SQLite => {
                format!("-- SQLite constraint handling requires table recreation for {}", table_name)
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                format!("ALTER TABLE {} ADD CONSTRAINT {} CHECK (1=1)", table_name, _constraint.name)
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                format!("ALTER TABLE {} ADD CONSTRAINT {} CHECK (1=1)", table_name, _constraint.name)
            }
        };

        Ok(sql)
    }

    /// Generate DROP CONSTRAINT SQL statement
    fn generate_drop_constraint_sql(&self, table_name: &str, _constraint_name: &str) -> Result<String, DDLError> {
        let sql = match self.dialect {
            DatabaseDialect::SQLite => {
                format!("-- SQLite constraint removal requires table recreation for {}", table_name)
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                format!("ALTER TABLE {} DROP CONSTRAINT {}", table_name, _constraint_name)
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                format!("ALTER TABLE {} DROP CONSTRAINT {}", table_name, _constraint_name)
            }
        };

        Ok(sql)
    }

    /// Build a sea-query column definition from unified schema
    fn build_column_definition(&self, column: &UnifiedColumnSchema) -> Result<ColumnDef, DDLError> {
        let column_type = self.convert_unified_type_to_sea_query(&column.column_type)?;
        
        let mut col = ColumnDef::new_with_type(Alias::new(&column.name), column_type);
        
        if !column.nullable {
            col.not_null();
        }

        Ok(col)
    }

    /// Convert UnifiedColumnType to sea-query ColumnType
    fn convert_unified_type_to_sea_query(&self, unified_type: &UnifiedColumnType) -> Result<ColumnType, DDLError> {
        match unified_type {
            UnifiedColumnType::Boolean => Ok(ColumnType::Boolean),
            UnifiedColumnType::SmallInt => Ok(ColumnType::SmallInteger),
            UnifiedColumnType::Integer => Ok(ColumnType::Integer),
            UnifiedColumnType::BigInt => Ok(ColumnType::BigInteger),
            UnifiedColumnType::Real => Ok(ColumnType::Float),
            UnifiedColumnType::Double => Ok(ColumnType::Double),
            UnifiedColumnType::Decimal => Ok(ColumnType::Decimal(None)),
            UnifiedColumnType::Char => Ok(ColumnType::Char(None)),
            UnifiedColumnType::VarChar => Ok(ColumnType::String(StringLen::None)),
            UnifiedColumnType::Text => Ok(ColumnType::Text),
            UnifiedColumnType::Blob => Ok(ColumnType::Binary(1024)),
            UnifiedColumnType::Json => Ok(ColumnType::Json),
            UnifiedColumnType::Date => Ok(ColumnType::Date),
            UnifiedColumnType::Time => Ok(ColumnType::Time),
            UnifiedColumnType::DateTime => Ok(ColumnType::DateTime),
            UnifiedColumnType::Timestamp => Ok(ColumnType::Timestamp),
            UnifiedColumnType::Uuid => Ok(ColumnType::Uuid),
            UnifiedColumnType::Other(custom_type) => {
                Ok(ColumnType::Custom(Alias::new(custom_type).into_iden()))
            },
        }
    }


    /// Estimate CREATE TABLE execution duration
    fn estimate_create_table_duration(&self, schema: &UnifiedTableSchema) -> u64 {
        // Base time + per-column overhead
        100 + (schema.columns.len() as u64 * 20)
    }

    /// Estimate index creation duration
    fn estimate_index_creation_duration(&self, _index: &UnifiedIndexSchema) -> u64 {
        // Depends on table size, but we'll use a conservative estimate
        500
    }

    /// Generate comprehensive statistics
    fn generate_statistics(&self, statements: &[DDLStatement]) -> DDLStatistics {
        let mut operations_by_type = HashMap::new();
        let mut operations_by_table = HashMap::new();
        let mut dialect_specific_operations = 0;

        for statement in statements {
            *operations_by_type.entry(statement.operation_type).or_insert(0) += 1;
            
            for table in &statement.affected_tables {
                *operations_by_table.entry(table.clone()).or_insert(0) += 1;
            }

            // Count dialect-specific operations (simplified heuristic)
            if statement.sql.contains("CONSTRAINT") || statement.sql.contains("INDEX") {
                dialect_specific_operations += 1;
            }
        }

        DDLStatistics {
            total_statements: statements.len(),
            operations_by_type,
            operations_by_table,
            dialect_specific_operations,
            cross_database_operations: statements.len() - dialect_specific_operations,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::introspection::{UnifiedColumnType, UnifiedTableType, UnifiedIndexType};

    #[test]
    fn test_ddl_generator_creation() {
        let generator = DDLGenerator::new(DatabaseDialect::SQLite);
        assert_eq!(generator.dialect, DatabaseDialect::SQLite);
    }

    #[test]
    fn test_column_type_conversion() {
        let generator = DDLGenerator::new(DatabaseDialect::SQLite);
        
        let result = generator.convert_unified_type_to_sea_query(&UnifiedColumnType::Integer);
        assert!(result.is_ok());
        
        let result = generator.convert_unified_type_to_sea_query(&UnifiedColumnType::VarChar);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_table_sql_generation() {
        let generator = DDLGenerator::new(DatabaseDialect::SQLite);
        
        let schema = UnifiedTableSchema {
            name: "test_table".to_string(),
            table_type: UnifiedTableType::Table,
            schema_name: None,
            comment: None,
            columns: vec![
                UnifiedColumnSchema {
                    name: "id".to_string(),
                    column_type: UnifiedColumnType::Integer,
                    raw_type: "INTEGER".to_string(),
                    nullable: false,
                    default_value: None,
                    primary_key: true,
                    auto_increment: false,
                    unique: false,
                    comment: None,
                    character_maximum_length: None,
                    numeric_precision: None,
                    numeric_scale: None,
                    constraints: vec![],
                    ordinal_position: Some(1),
                },
                UnifiedColumnSchema {
                    name: "name".to_string(),
                    column_type: UnifiedColumnType::VarChar,
                    raw_type: "VARCHAR".to_string(),
                    nullable: true,
                    default_value: None,
                    primary_key: false,
                    auto_increment: false,
                    unique: false,
                    comment: None,
                    character_maximum_length: None,
                    numeric_precision: None,
                    numeric_scale: None,
                    constraints: vec![],
                    ordinal_position: Some(2),
                },
            ],
            indexes: vec![],
            foreign_keys: vec![],
            constraints: vec![],
            metadata: std::collections::HashMap::new(),
        };

        let result = generator.generate_create_table_sql(&schema);
        assert!(result.is_ok());
        
        let sql = result.unwrap();
        assert!(sql.contains("CREATE TABLE"));
        assert!(sql.contains("test_table"));
        assert!(sql.contains("id"));
        assert!(sql.contains("name"));
    }

    #[test]
    fn test_drop_column_sqlite_restriction() {
        let generator = DDLGenerator::new(DatabaseDialect::SQLite);
        let result = generator.generate_drop_column_sql("test_table", "test_column");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DDLError::UnsupportedOperation(_)));
    }

    #[test]
    fn test_index_sql_generation() {
        let generator = DDLGenerator::new(DatabaseDialect::SQLite);
        
        let index_schema = UnifiedIndexSchema {
            name: "idx_test".to_string(),
            table_name: "test_table".to_string(),
            columns: vec!["column1".to_string(), "column2".to_string()],
            unique: false,
            primary: false,
            index_type: UnifiedIndexType::BTree,
            condition: None,
            comment: None,
        };

        let result = generator.generate_create_index_sql(&index_schema);
        assert!(result.is_ok());
        
        let sql = result.unwrap();
        assert!(sql.contains("CREATE INDEX"));
        assert!(sql.contains("idx_test"));
        assert!(sql.contains("test_table"));
    }

    #[test]
    fn test_ddl_operation_type_display() {
        assert_eq!(DDLOperationType::CreateTable.display_name(), "CREATE TABLE");
        assert_eq!(DDLOperationType::DropTable.display_name(), "DROP TABLE");
        assert_eq!(DDLOperationType::CreateIndex.display_name(), "CREATE INDEX");
    }


    #[test]
    fn test_duration_estimation() {
        let generator = DDLGenerator::new(DatabaseDialect::SQLite);
        
        let schema = UnifiedTableSchema {
            name: "test".to_string(),
            table_type: UnifiedTableType::Table,
            schema_name: None,
            comment: None,
            columns: vec![
                UnifiedColumnSchema {
                    name: "id".to_string(),
                    column_type: UnifiedColumnType::Integer,
                    raw_type: "INTEGER".to_string(),
                    nullable: false,
                    default_value: None,
                    primary_key: false,
                    auto_increment: false,
                    unique: false,
                    comment: None,
                    character_maximum_length: None,
                    numeric_precision: None,
                    numeric_scale: None,
                    constraints: vec![],
                    ordinal_position: Some(1),
                },
            ],
            indexes: vec![],
            foreign_keys: vec![],
            constraints: vec![],
            metadata: std::collections::HashMap::new(),
        };

        let duration = generator.estimate_create_table_duration(&schema);
        assert_eq!(duration, 120); // 100 base + 20 per column
    }

    #[test]
    fn test_statistics_generation() {
        let generator = DDLGenerator::new(DatabaseDialect::SQLite);
        
        let statements = vec![
            DDLStatement {
                sql: "CREATE TABLE test".to_string(),
                operation_type: DDLOperationType::CreateTable,
                dialect: DatabaseDialect::SQLite,
                estimated_duration_ms: 100,
                is_reversible: true,
                affected_tables: vec!["test".to_string()],
            },
            DDLStatement {
                sql: "CREATE INDEX idx_test".to_string(),
                operation_type: DDLOperationType::CreateIndex,
                dialect: DatabaseDialect::SQLite,
                estimated_duration_ms: 200,
                is_reversible: true,
                affected_tables: vec!["test".to_string()],
            },
        ];

        let stats = generator.generate_statistics(&statements);
        assert_eq!(stats.total_statements, 2);
        assert_eq!(stats.operations_by_type.get(&DDLOperationType::CreateTable), Some(&1));
        assert_eq!(stats.operations_by_type.get(&DDLOperationType::CreateIndex), Some(&1));
        assert_eq!(stats.operations_by_table.get("test"), Some(&2));
    }
}