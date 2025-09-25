/// Migration Plan Execution Engine
/// 
/// This module provides comprehensive migration execution capabilities with transaction safety,
/// pre-execution validation, progress tracking, and automatic rollback functionality.

use crate::migration_engine::{MigrationPlan, DDLGenerator, DDLGenerationResult, DDLError};
use crate::dialects::DatabaseDialect;
use crate::backends::DatabaseBackend;
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use thiserror::Error;

/// Comprehensive error types for migration execution operations
#[derive(Debug, Error)]
pub enum MigrationExecutionError {
    #[error("DDL generation failed: {0}")]
    DdlGeneration(#[from] DDLError),
    
    #[error("Database execution failed: {0}")]
    DatabaseExecution(String),
    
    #[error("Transaction rollback failed: {0}")]
    RollbackFailed(String),
    
    #[error("Pre-execution validation failed: {0}")]
    ValidationFailed(String),

    #[error("Migration plan is empty")]
    EmptyPlan,

    #[error("Timeout during execution: {0}")]
    Timeout(String),
}

/// Result of migration execution with comprehensive statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationExecutionResult {
    /// Unique identifier for this migration execution
    pub plan_id: String,
    /// Whether the migration completed successfully
    pub success: bool,
    /// Number of DDL statements successfully executed
    pub executed_statements: usize,
    /// Total number of DDL statements in the plan
    pub total_statements: usize,
    /// Total time taken for the entire execution
    pub execution_duration: Duration,
    /// List of errors encountered during execution
    pub errors: Vec<String>,
    /// Whether a rollback was performed due to errors
    pub rollback_performed: bool,
    /// Warnings generated during validation or execution
    pub warnings: Vec<String>,
    /// Detailed statistics about the execution
    pub statistics: ExecutionStatistics,
}

/// Detailed execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStatistics {
    /// Time spent on pre-execution validation
    pub validation_duration: Duration,
    /// Time spent on DDL generation
    pub generation_duration: Duration,
    /// Time spent on actual database execution
    pub database_duration: Duration,
    /// Average time per statement
    pub avg_statement_duration: Duration,
    /// Number of retried operations
    pub retry_count: usize,
}

/// Configuration for migration execution behavior
#[derive(Debug, Clone)]
pub struct MigrationExecutionConfig {
    /// Whether to perform safety validation before execution
    pub validate_safety: bool,
    /// Whether to use database transactions for rollback safety
    pub use_transactions: bool,
    /// Maximum execution time before timeout
    pub timeout: Option<Duration>,
    /// Number of retry attempts for failed statements
    pub max_retries: usize,
    /// Whether to continue execution after non-critical failures
    pub continue_on_error: bool,
}

impl Default for MigrationExecutionConfig {
    fn default() -> Self {
        Self {
            validate_safety: true,
            use_transactions: true,
            timeout: Some(Duration::from_secs(300)), // 5 minutes default
            max_retries: 1,
            continue_on_error: false,
        }
    }
}

/// Migration execution engine with comprehensive safety and monitoring
pub struct MigrationExecutor {
    ddl_generator: DDLGenerator,
    dialect: DatabaseDialect,
    config: MigrationExecutionConfig,
}

impl MigrationExecutor {
    /// Create a new migration executor for the specified database dialect
    pub fn new(dialect: DatabaseDialect) -> Self {
        Self {
            ddl_generator: DDLGenerator::new(dialect),
            dialect,
            config: MigrationExecutionConfig::default(),
        }
    }

    /// Create a migration executor with custom configuration
    pub fn with_config(dialect: DatabaseDialect, config: MigrationExecutionConfig) -> Self {
        Self {
            ddl_generator: DDLGenerator::new(dialect),
            dialect,
            config,
        }
    }

    /// Get the current configuration
    pub fn config(&self) -> &MigrationExecutionConfig {
        &self.config
    }

    /// Update the configuration
    pub fn set_config(&mut self, config: MigrationExecutionConfig) {
        self.config = config;
    }

    /// Execute a migration plan with comprehensive safety checks and monitoring
    pub async fn execute_migration<B: DatabaseBackend>(
        &self,
        plan: &MigrationPlan,
        backend: &B,
    ) -> Result<MigrationExecutionResult, MigrationExecutionError> {
        let start_time = Instant::now();
        let plan_id = self.generate_plan_id();
        
        // Check for empty plan
        if plan.operations.is_empty() {
            return Err(MigrationExecutionError::EmptyPlan);
        }

        let mut warnings = Vec::new();
        let mut statistics = ExecutionStatistics {
            validation_duration: Duration::ZERO,
            generation_duration: Duration::ZERO,
            database_duration: Duration::ZERO,
            avg_statement_duration: Duration::ZERO,
            retry_count: 0,
        };

        // Pre-execution validation with timing
        let validation_start = Instant::now();
        if self.config.validate_safety {
            self.validate_migration_safety(plan, backend, &mut warnings).await?;
        }
        statistics.validation_duration = validation_start.elapsed();

        // Generate DDL statements with timing
        let generation_start = Instant::now();
        let ddl_result = self.ddl_generator.generate_ddl(plan)?;
        statistics.generation_duration = generation_start.elapsed();

        // Execute with transaction safety and monitoring
        let execution_start = Instant::now();
        let execution_result = self.execute_with_transaction_safety(
            ddl_result, 
            backend, 
            &plan_id, 
            start_time,
            &mut statistics
        ).await;
        statistics.database_duration = execution_start.elapsed();

        // Calculate average statement duration
        if execution_result.as_ref().map_or(true, |r| r.executed_statements > 0) {
            let executed_count = execution_result.as_ref().map_or(0, |r| r.executed_statements);
            if executed_count > 0 {
                statistics.avg_statement_duration = statistics.database_duration / executed_count as u32;
            }
        }

        // Merge warnings and statistics into final result
        match execution_result {
            Ok(mut result) => {
                result.warnings.extend(warnings);
                result.statistics = statistics;
                Ok(result)
            }
            Err(e) => Err(e)
        }
    }

    /// Generate a unique plan ID for tracking
    fn generate_plan_id(&self) -> String {
        use std::time::SystemTime;
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();
        format!("migration_{}_{}", timestamp, rand::random::<u32>())
    }

    /// Validate migration safety with comprehensive checks
    async fn validate_migration_safety<B: DatabaseBackend>(
        &self,
        plan: &MigrationPlan,
        _backend: &B,
        warnings: &mut Vec<String>,
    ) -> Result<(), MigrationExecutionError> {
        let mut destructive_operations = Vec::new();
        let mut high_risk_operations = Vec::new();

        // Analyze each operation for safety level
        for operation in &plan.operations {
            let safety_level = operation.safety_level();
            
            if safety_level.is_destructive() {
                destructive_operations.push(format!("{:?}", operation));
            } else if matches!(safety_level, crate::migration_engine::SafetyLevel::HighRisk) {
                high_risk_operations.push(format!("{:?}", operation));
            }
        }

        // Handle destructive operations
        if !destructive_operations.is_empty() {
            return Err(MigrationExecutionError::ValidationFailed(
                format!("Destructive operations detected: {}", destructive_operations.join(", "))
            ));
        }

        // Add warnings for high-risk operations
        if !high_risk_operations.is_empty() {
            warnings.push(format!("High-risk operations detected: {}", high_risk_operations.join(", ")));
        }

        // Validate database-specific constraints
        self.validate_database_specific_constraints(&plan.operations, warnings)?;

        Ok(())
    }

    /// Validate database-specific constraints and limitations
    fn validate_database_specific_constraints(
        &self,
        operations: &[crate::migration_engine::MigrationOperation],
        warnings: &mut Vec<String>,
    ) -> Result<(), MigrationExecutionError> {
        use crate::migration_engine::{MigrationOperation, TableOperation, ColumnOperation};

        for operation in operations {
            match (&self.dialect, operation) {
                // SQLite-specific validations
                (DatabaseDialect::SQLite, MigrationOperation::Table(TableOperation::RenameTable { .. })) => {
                    warnings.push("SQLite table renaming may not be supported in all versions".to_string());
                }
                (DatabaseDialect::SQLite, MigrationOperation::Column(ColumnOperation::ModifyColumn { .. })) => {
                    warnings.push("SQLite column modifications require table recreation".to_string());
                }
                (DatabaseDialect::SQLite, MigrationOperation::Column(ColumnOperation::DropColumn { .. })) => {
                    warnings.push("SQLite column dropping requires table recreation".to_string());
                }
                
                // PostgreSQL-specific validations
                #[cfg(feature = "postgres")]
                (DatabaseDialect::PostgreSQL, _) => {
                    // PostgreSQL generally supports most operations
                }
                
                // MySQL-specific validations  
                #[cfg(feature = "mysql")]
                (DatabaseDialect::MySQL, _) => {
                    // MySQL has specific constraints that could be validated here
                }
                
                _ => {}
            }
        }

        Ok(())
    }

    /// Execute DDL statements with transaction safety and comprehensive error handling
    async fn execute_with_transaction_safety<B: DatabaseBackend>(
        &self,
        ddl_result: DDLGenerationResult,
        backend: &B,
        plan_id: &str,
        start_time: Instant,
        statistics: &mut ExecutionStatistics,
    ) -> Result<MigrationExecutionResult, MigrationExecutionError> {
        let total_statements = ddl_result.statements.len();
        let mut executed_statements = 0;
        let mut errors = Vec::new();
        let mut rollback_performed = false;

        // Check timeout before starting
        if let Some(timeout) = self.config.timeout {
            if start_time.elapsed() > timeout {
                return Err(MigrationExecutionError::Timeout(
                    "Timeout reached before statement execution".to_string()
                ));
            }
        }

        // Begin transaction if configured
        if self.config.use_transactions {
            if let Err(e) = backend.execute_query("BEGIN TRANSACTION", &[]).await {
                return Err(MigrationExecutionError::DatabaseExecution(
                    format!("Failed to begin transaction: {:?}", e)
                ));
            }
        }

        // Execute each DDL statement with retry logic
        for (index, statement) in ddl_result.statements.iter().enumerate() {
            // Check timeout
            if let Some(timeout) = self.config.timeout {
                if start_time.elapsed() > timeout {
                    errors.push(format!("Timeout reached at statement {}", index + 1));
                    break;
                }
            }

            let mut retry_count = 0;
            let mut statement_success = false;

            // Retry logic for individual statements
            while retry_count <= self.config.max_retries && !statement_success {
                match backend.execute_query(&statement.sql, &[]).await {
                    Ok(_) => {
                        executed_statements += 1;
                        statement_success = true;
                    }
                    Err(e) => {
                        let error_msg = format!("Statement {} failed (attempt {}): {}: {:?}", 
                            index + 1, retry_count + 1, statement.sql, e);
                        
                        if retry_count < self.config.max_retries {
                            statistics.retry_count += 1;
                            retry_count += 1;
                        } else {
                            errors.push(error_msg);
                            
                            if !self.config.continue_on_error {
                                break;
                            }
                        }
                    }
                }
            }

            // Break if statement failed and we don't continue on error
            if !statement_success && !self.config.continue_on_error {
                break;
            }
        }

        // Handle transaction rollback or commit
        if self.config.use_transactions {
            let has_errors = !errors.is_empty();
            
            if has_errors {
                // Rollback on error
                match backend.execute_query("ROLLBACK", &[]).await {
                    Ok(_) => {
                        rollback_performed = true;
                    }
                    Err(rollback_error) => {
                        return Err(MigrationExecutionError::RollbackFailed(
                            format!("Rollback failed: {:?}", rollback_error)
                        ));
                    }
                }
            } else {
                // Commit on success
                if let Err(commit_error) = backend.execute_query("COMMIT", &[]).await {
                    return Err(MigrationExecutionError::DatabaseExecution(
                        format!("Commit failed: {:?}", commit_error)
                    ));
                }
            }
        }

        Ok(MigrationExecutionResult {
            plan_id: plan_id.to_string(),
            success: errors.is_empty() && executed_statements == total_statements,
            executed_statements,
            total_statements,
            execution_duration: start_time.elapsed(),
            errors,
            rollback_performed,
            warnings: Vec::new(), // Warnings are added by caller
            statistics: ExecutionStatistics {
                validation_duration: Duration::ZERO, // Set by caller
                generation_duration: Duration::ZERO, // Set by caller
                database_duration: Duration::ZERO,   // Set by caller
                avg_statement_duration: Duration::ZERO, // Set by caller
                retry_count: statistics.retry_count,
            },
        })
    }

    /// Estimate execution time based on migration plan complexity
    pub fn estimate_execution_time(&self, plan: &MigrationPlan) -> Duration {
        let base_overhead = Duration::from_millis(100);
        let per_operation_time = Duration::from_millis(50);
        let per_statement_time = Duration::from_millis(10);
        
        // Generate DDL to get accurate statement count
        if let Ok(ddl_result) = self.ddl_generator.generate_ddl(plan) {
            base_overhead 
                + per_operation_time * plan.operations.len() as u32
                + per_statement_time * ddl_result.statements.len() as u32
        } else {
            base_overhead + per_operation_time * plan.operations.len() as u32
        }
    }

    /// Check if a migration plan is safe to execute
    pub fn is_safe_to_execute(&self, plan: &MigrationPlan) -> bool {
        for operation in &plan.operations {
            if operation.safety_level().is_destructive() {
                return false;
            }
        }
        true
    }

    /// Get the database dialect this executor is configured for
    pub fn dialect(&self) -> DatabaseDialect {
        self.dialect
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration_engine::{MigrationOperation, TableOperation, SafetyLevel};
    use crate::introspection::{UnifiedTableSchema, UnifiedTableType};
    use crate::backends::{BackendError, QueryResult};
    use crate::dialects::DatabaseDialect;
    use serde_json::Value;
    use std::collections::{HashMap, HashSet};

    // Mock implementation for testing
    #[derive(Clone)]
    struct MockBackend {
        pub dialect: DatabaseDialect,
        pub should_fail: bool,
        pub should_fail_transaction: bool,
        pub transaction_begun: std::sync::Arc<std::sync::Mutex<bool>>,
        pub statements_executed: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    }

    impl MockBackend {
        fn new(dialect: DatabaseDialect) -> Self {
            Self {
                dialect,
                should_fail: false,
                should_fail_transaction: false,
                transaction_begun: std::sync::Arc::new(std::sync::Mutex::new(false)),
                statements_executed: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            }
        }

        fn with_failure(mut self) -> Self {
            self.should_fail = true;
            self
        }

        fn with_transaction_failure(mut self) -> Self {
            self.should_fail_transaction = true;
            self
        }
    }

    #[derive(Debug)]
    struct MockQueryResult {
        rows: Vec<Value>,
    }

    impl QueryResult for MockQueryResult {
        type Error = BackendError;

        fn rows(&self) -> &[Value] {
            &self.rows
        }

        fn into_rows(self) -> Vec<Value> {
            self.rows
        }

        fn into_entities<T>(self) -> Result<Vec<T>, BackendError>
        where
            T: serde::de::DeserializeOwned + crate::Entity,
        {
            Ok(Vec::new())
        }

        fn into_entity<T>(self) -> Result<Option<T>, BackendError>
        where
            T: serde::de::DeserializeOwned + crate::Entity,
        {
            Ok(None)
        }

        fn into_simple_entities<T>(self) -> Result<Vec<T>, BackendError>
        where
            T: serde::de::DeserializeOwned,
        {
            Ok(Vec::new())
        }

        fn into_simple_entity<T>(self) -> Result<Option<T>, BackendError>
        where
            T: serde::de::DeserializeOwned,
        {
            Ok(None)
        }
    }

    #[async_trait::async_trait]
    impl DatabaseBackend for MockBackend {
        type QueryResult = MockQueryResult;
        type Error = BackendError;

        async fn execute_query(
            &self,
            sql: &str,
            _params: &[Value],
        ) -> Result<Self::QueryResult, Self::Error> {
            // Track executed statements
            self.statements_executed
                .lock()
                .unwrap()
                .push(sql.to_string());

            // Handle transaction commands
            if sql == "BEGIN TRANSACTION" {
                if self.should_fail_transaction {
                    return Err(BackendError::Database("Transaction begin failed".to_string()));
                }
                *self.transaction_begun.lock().unwrap() = true;
                return Ok(MockQueryResult { rows: vec![] });
            }

            if sql == "COMMIT" {
                if self.should_fail_transaction {
                    return Err(BackendError::Database("Transaction commit failed".to_string()));
                }
                *self.transaction_begun.lock().unwrap() = false;
                return Ok(MockQueryResult { rows: vec![] });
            }

            if sql == "ROLLBACK" {
                *self.transaction_begun.lock().unwrap() = false;
                return Ok(MockQueryResult { rows: vec![] });
            }

            // Simulate failure for DDL statements if configured
            if self.should_fail {
                return Err(BackendError::Query("Mock query failure".to_string()));
            }

            Ok(MockQueryResult { rows: vec![] })
        }

        async fn execute_schema(&self, sql: &str) -> Result<(), Self::Error> {
            if self.should_fail {
                Err(BackendError::Database("Mock schema failure".to_string()))
            } else {
                self.statements_executed
                    .lock()
                    .unwrap()
                    .push(sql.to_string());
                Ok(())
            }
        }

        fn dialect(&self) -> DatabaseDialect {
            self.dialect
        }

        fn connection_info(&self) -> String {
            format!("mock://{:?}", self.dialect)
        }

        async fn ping(&self) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    fn create_test_migration_plan() -> MigrationPlan {
        MigrationPlan {
            operations: vec![
                MigrationOperation::Table(TableOperation::CreateTable {
                    table_name: "test_table".to_string(),
                    schema: UnifiedTableSchema {
                        name: "test_table".to_string(),
                        table_type: UnifiedTableType::Table,
                        schema_name: None,
                        comment: None,
                        columns: vec![],
                        indexes: vec![],
                        foreign_keys: vec![],
                        constraints: vec![],
                        metadata: HashMap::new(),
                    },
                })
            ],
            dialect: DatabaseDialect::SQLite,
            safety_level: SafetyLevel::Safe,
            estimated_duration_seconds: 1,
            affected_tables: HashSet::from_iter(vec!["test_table".to_string()]),
            warnings: vec![],
        }
    }

    fn create_destructive_migration_plan() -> MigrationPlan {
        MigrationPlan {
            operations: vec![
                MigrationOperation::Table(TableOperation::DropTable {
                    table_name: "old_table".to_string(),
                })
            ],
            dialect: DatabaseDialect::SQLite,
            safety_level: SafetyLevel::Destructive,
            estimated_duration_seconds: 1,
            affected_tables: HashSet::from_iter(vec!["old_table".to_string()]),
            warnings: vec![],
        }
    }

    #[test]
    fn test_migration_executor_creation() {
        let executor = MigrationExecutor::new(DatabaseDialect::SQLite);
        assert_eq!(executor.dialect(), DatabaseDialect::SQLite);
        assert!(executor.config().validate_safety);
        assert!(executor.config().use_transactions);
    }

    #[test]
    fn test_migration_executor_with_config() {
        let config = MigrationExecutionConfig {
            validate_safety: false,
            use_transactions: false,
            timeout: Some(Duration::from_secs(60)),
            max_retries: 3,
            continue_on_error: true,
        };

        let executor = MigrationExecutor::with_config(DatabaseDialect::SQLite, config);
        assert_eq!(executor.dialect(), DatabaseDialect::SQLite);
        assert!(!executor.config().validate_safety);
        assert!(!executor.config().use_transactions);
        assert_eq!(executor.config().max_retries, 3);
        assert!(executor.config().continue_on_error);
    }

    #[test]
    fn test_is_safe_to_execute() {
        let executor = MigrationExecutor::new(DatabaseDialect::SQLite);
        
        let safe_plan = create_test_migration_plan();
        assert!(executor.is_safe_to_execute(&safe_plan));
        
        let destructive_plan = create_destructive_migration_plan();
        assert!(!executor.is_safe_to_execute(&destructive_plan));
    }

    #[test]
    fn test_estimate_execution_time() {
        let executor = MigrationExecutor::new(DatabaseDialect::SQLite);
        let plan = create_test_migration_plan();
        
        let estimate = executor.estimate_execution_time(&plan);
        assert!(estimate > Duration::ZERO);
        assert!(estimate < Duration::from_secs(5)); // Should be reasonable
    }

    #[tokio::test]
    async fn test_successful_migration_execution() {
        let executor = MigrationExecutor::new(DatabaseDialect::SQLite);
        let backend = MockBackend::new(DatabaseDialect::SQLite);
        let plan = create_test_migration_plan();

        let result = executor.execute_migration(&plan, &backend).await;
        assert!(result.is_ok());
        
        let execution_result = result.unwrap();
        assert!(execution_result.success);
        assert_eq!(execution_result.executed_statements, 1);
        assert_eq!(execution_result.total_statements, 1);
        assert!(!execution_result.rollback_performed);
        assert!(execution_result.errors.is_empty());

        // Verify transaction was used
        let statements = backend.statements_executed.lock().unwrap();
        assert!(statements.contains(&"BEGIN TRANSACTION".to_string()));
        assert!(statements.contains(&"COMMIT".to_string()));
    }

    #[tokio::test]
    async fn test_empty_plan_handling() {
        let executor = MigrationExecutor::new(DatabaseDialect::SQLite);
        let backend = MockBackend::new(DatabaseDialect::SQLite);
        let empty_plan = MigrationPlan {
            operations: vec![],
            dialect: DatabaseDialect::SQLite,
            safety_level: SafetyLevel::Safe,
            estimated_duration_seconds: 0,
            affected_tables: HashSet::new(),
            warnings: vec![],
        };

        let result = executor.execute_migration(&empty_plan, &backend).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MigrationExecutionError::EmptyPlan));
    }

    #[tokio::test]
    async fn test_destructive_operation_validation() {
        let executor = MigrationExecutor::new(DatabaseDialect::SQLite);
        let backend = MockBackend::new(DatabaseDialect::SQLite);
        let destructive_plan = create_destructive_migration_plan();

        let result = executor.execute_migration(&destructive_plan, &backend).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MigrationExecutionError::ValidationFailed(_)));
    }

    #[tokio::test]
    async fn test_migration_with_validation_disabled() {
        let config = MigrationExecutionConfig {
            validate_safety: false,
            ..Default::default()
        };
        let executor = MigrationExecutor::with_config(DatabaseDialect::SQLite, config);
        let backend = MockBackend::new(DatabaseDialect::SQLite);
        let destructive_plan = create_destructive_migration_plan();

        // Should succeed because validation is disabled
        let result = executor.execute_migration(&destructive_plan, &backend).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rollback_on_failure() {
        let executor = MigrationExecutor::new(DatabaseDialect::SQLite);
        let backend = MockBackend::new(DatabaseDialect::SQLite).with_failure();
        let plan = create_test_migration_plan();

        let result = executor.execute_migration(&plan, &backend).await;
        assert!(result.is_ok()); // Should succeed in creating result even with errors
        
        let execution_result = result.unwrap();
        assert!(!execution_result.success);
        assert_eq!(execution_result.executed_statements, 0);
        assert!(execution_result.rollback_performed);
        assert!(!execution_result.errors.is_empty());

        // Verify rollback was executed
        let statements = backend.statements_executed.lock().unwrap();
        assert!(statements.contains(&"ROLLBACK".to_string()));
    }

    #[tokio::test]
    async fn test_transaction_begin_failure() {
        let executor = MigrationExecutor::new(DatabaseDialect::SQLite);
        let backend = MockBackend::new(DatabaseDialect::SQLite).with_transaction_failure();
        let plan = create_test_migration_plan();

        let result = executor.execute_migration(&plan, &backend).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MigrationExecutionError::DatabaseExecution(_)));
    }

    #[tokio::test]
    async fn test_execution_without_transactions() {
        let config = MigrationExecutionConfig {
            use_transactions: false,
            ..Default::default()
        };
        let executor = MigrationExecutor::with_config(DatabaseDialect::SQLite, config);
        let backend = MockBackend::new(DatabaseDialect::SQLite);
        let plan = create_test_migration_plan();

        let result = executor.execute_migration(&plan, &backend).await;
        assert!(result.is_ok());
        
        let execution_result = result.unwrap();
        assert!(execution_result.success);
        assert!(!execution_result.rollback_performed);

        // Verify no transaction commands were executed
        let statements = backend.statements_executed.lock().unwrap();
        assert!(!statements.contains(&"BEGIN TRANSACTION".to_string()));
        assert!(!statements.contains(&"COMMIT".to_string()));
    }

    #[tokio::test]
    async fn test_execution_statistics() {
        let executor = MigrationExecutor::new(DatabaseDialect::SQLite);
        let backend = MockBackend::new(DatabaseDialect::SQLite);
        let plan = create_test_migration_plan();

        let result = executor.execute_migration(&plan, &backend).await;
        assert!(result.is_ok());
        
        let execution_result = result.unwrap();
        let stats = &execution_result.statistics;
        
        assert!(stats.validation_duration >= Duration::ZERO);
        assert!(stats.generation_duration >= Duration::ZERO);
        assert!(stats.database_duration >= Duration::ZERO);
        assert!(stats.avg_statement_duration >= Duration::ZERO);
        assert_eq!(stats.retry_count, 0);
    }

    #[tokio::test] 
    async fn test_retry_logic() {
        let config = MigrationExecutionConfig {
            max_retries: 2,
            ..Default::default()
        };
        let executor = MigrationExecutor::with_config(DatabaseDialect::SQLite, config);
        let backend = MockBackend::new(DatabaseDialect::SQLite).with_failure();
        let plan = create_test_migration_plan();

        let result = executor.execute_migration(&plan, &backend).await;
        assert!(result.is_ok());
        
        let execution_result = result.unwrap();
        assert!(!execution_result.success);
        assert!(execution_result.statistics.retry_count > 0);
    }

    #[tokio::test]
    async fn test_database_specific_warnings() {
        let executor = MigrationExecutor::new(DatabaseDialect::SQLite);
        let backend = MockBackend::new(DatabaseDialect::SQLite);
        
        // Create a plan with SQLite-specific operations that should generate warnings
        let plan = MigrationPlan {
            operations: vec![
                MigrationOperation::Table(TableOperation::RenameTable {
                    old_name: "old_table".to_string(),
                    new_name: "new_table".to_string(),
                })
            ],
            dialect: DatabaseDialect::SQLite,
            safety_level: SafetyLevel::ModerateRisk,
            estimated_duration_seconds: 1,
            affected_tables: HashSet::from_iter(vec!["old_table".to_string(), "new_table".to_string()]),
            warnings: vec![],
        };

        let result = executor.execute_migration(&plan, &backend).await;
        assert!(result.is_ok());
        
        let execution_result = result.unwrap();
        assert!(!execution_result.warnings.is_empty());
        assert!(execution_result.warnings[0].contains("SQLite table renaming"));
    }

    #[tokio::test]
    async fn test_migration_execution_result_serialization() {
        let result = MigrationExecutionResult {
            plan_id: "test_123".to_string(),
            success: true,
            executed_statements: 5,
            total_statements: 5,
            execution_duration: Duration::from_millis(150),
            errors: vec![],
            rollback_performed: false,
            warnings: vec!["Test warning".to_string()],
            statistics: ExecutionStatistics {
                validation_duration: Duration::from_millis(10),
                generation_duration: Duration::from_millis(20),
                database_duration: Duration::from_millis(120),
                avg_statement_duration: Duration::from_millis(24),
                retry_count: 0,
            },
        };

        // Test serialization
        let json = serde_json::to_string(&result);
        assert!(json.is_ok());

        // Test deserialization
        let deserialized = serde_json::from_str::<MigrationExecutionResult>(&json.unwrap());
        assert!(deserialized.is_ok());
        
        let deserialized = deserialized.unwrap();
        assert_eq!(deserialized.plan_id, "test_123");
        assert_eq!(deserialized.success, true);
        assert_eq!(deserialized.executed_statements, 5);
    }
}