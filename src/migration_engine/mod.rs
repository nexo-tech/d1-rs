/// Database-agnostic migration engine built on sea-query
/// 
/// This module provides cross-database schema diffing, migration planning,
/// and execution capabilities using sea-query for database-agnostic SQL generation.

pub mod differ;
pub mod ddl_generator;

pub use differ::{
    MigrationOperation, MigrationPlan, SafetyLevel, SchemaDiffer, 
    SchemaError, TableOperation, ColumnOperation, IndexOperation, 
    ConstraintOperation, MigrationResult
};

pub use ddl_generator::{
    DDLGenerator, DDLStatement, DDLOperationType, DDLGenerationResult,
    DDLStatistics, DDLError
};

// Re-export from introspection for convenience
pub use crate::introspection::{
    UnifiedTableSchema, UnifiedColumnSchema, 
    UnifiedIndexSchema, UnifiedConstraintSchema
};
pub use crate::auto_migration::introspector::DatabaseSchema;