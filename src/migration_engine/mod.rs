/// Database-agnostic migration engine built on sea-query
/// 
/// This module provides cross-database schema diffing, migration planning,
/// and execution capabilities using sea-query for database-agnostic SQL generation.

pub mod differ;
pub mod ddl_generator;
pub mod executor;
pub mod rollback;
pub mod data_migration;
pub mod verification;
pub mod health;

pub use differ::{
    MigrationOperation, MigrationPlan, SafetyLevel, SchemaDiffer, 
    SchemaError, TableOperation, ColumnOperation, IndexOperation, 
    ConstraintOperation, MigrationResult
};

pub use ddl_generator::{
    DDLGenerator, DDLStatement, DDLOperationType, DDLGenerationResult,
    DDLStatistics, DDLError
};

pub use executor::{
    MigrationExecutor, MigrationExecutionError, MigrationExecutionResult,
    MigrationExecutionConfig, ExecutionStatistics
};

pub use rollback::{
    RollbackGenerator, RollbackPlan, RollbackError, DataLossRisk
};

pub use data_migration::{
    DataMigrator, DataMigrationPlan, DataMigrationOperation, DataMigrationError,
    DataMigrationResult, ColumnTransformation, TransformationType, 
    DataValidationRule, ValidationRuleType, ValidationSeverity,
    SplitStrategy, MergeStrategy, HashAlgorithm, TransformationErrorHandling
};

pub use verification::{
    Phase5Verifier, VerificationReport, CleanupReport, VerificationError
};

pub use health::{
    HealthChecker, SystemHealth, ComponentHealth, HealthStatus, 
    HealthTrend, HealthCheckError
};

// Re-export from introspection for convenience
pub use crate::introspection::{
    UnifiedTableSchema, UnifiedColumnSchema, 
    UnifiedIndexSchema, UnifiedConstraintSchema
};
pub use crate::auto_migration::introspector::DatabaseSchema;