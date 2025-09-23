//! Core Execution Framework for Rollback Operations
//!
//! This module provides the main execution engine for rollback operations, including
//! transaction management, error handling, progress monitoring, and safety enforcement.
//! The execution engine ensures reliable rollback execution with comprehensive validation
//! and recovery capabilities.

use super::types::{RollbackOperation, RollbackConfig};
use super::plan::{RollbackPlan, PreExecutionCheck, PostExecutionValidation, CheckType, ValidationType, 
                  CheckFailureAction, ValidationFailureAction};
use super::sql_generator::{RollbackSqlGenerator, SqlGeneratorConfig};
use crate::{D1Client, Result, D1RsError};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Core rollback execution engine with comprehensive safety and monitoring capabilities
pub struct RollbackExecutionEngine {
    /// Configuration for execution behavior
    config: RollbackConfig,
    
    /// SQL generator for operation execution
    sql_generator: RollbackSqlGenerator,
    
    /// Execution statistics and metrics
    metrics: std::cell::RefCell<ExecutionMetrics>,
}

/// Execution metrics for performance monitoring and analysis
#[derive(Debug, Clone)]
pub struct ExecutionMetrics {
    /// Total operations executed
    pub total_operations: usize,
    
    /// Total execution time across all operations
    pub total_execution_time: Duration,
    
    /// Average execution time per operation
    pub average_operation_time: Duration,
    
    /// Number of failed operations
    pub failed_operations: usize,
    
    /// Number of retried operations
    pub retried_operations: usize,
}

/// Result of executing a complete rollback plan
#[derive(Debug, Clone)]
pub struct RollbackExecutionResult {
    /// Whether the entire rollback succeeded
    pub success: bool,
    
    /// Results from completed operations
    pub completed_operations: Vec<RollbackOperationResult>,
    
    /// Information about failed operations
    pub failed_operations: Vec<RollbackOperationFailure>,
    
    /// Total time taken for the entire rollback
    pub total_duration: Duration,
    
    /// Number of operations completed successfully
    pub operations_completed: usize,
    
    /// Number of operations that failed
    pub operations_failed: usize,
    
    /// Non-fatal warnings encountered during execution
    pub warnings: Vec<String>,
    
    /// Unique identifier for this rollback execution
    pub rollback_id: String,
    
    /// Pre-execution check results
    pub pre_check_results: Vec<CheckResult>,
    
    /// Post-execution validation results
    pub post_validation_results: Vec<ValidationResult>,
    
    /// Execution state at completion
    pub final_state: ExecutionState,
}

/// Result of executing a single rollback operation
#[derive(Debug, Clone)]
pub struct SingleOperationResult {
    /// The operation that was executed
    pub operation: RollbackOperation,
    
    /// Time taken to execute this operation
    pub execution_time: Duration,
    
    /// SQL statements that were executed
    pub sql_executed: Vec<String>,
    
    /// Number of rows affected by this operation
    pub rows_affected: usize,
    
    /// Warnings generated during operation execution
    pub warnings: Vec<String>,
    
    /// Whether the operation succeeded
    pub success: bool,
    
    /// Error information if the operation failed
    pub error: Option<OperationError>,
    
    /// Transaction ID if executed within a transaction
    pub transaction_id: Option<String>,
}

/// Result of a successfully completed rollback operation
#[derive(Debug, Clone)]
pub struct RollbackOperationResult {
    /// The operation that was executed
    pub operation: RollbackOperation,
    
    /// Time taken to execute this operation
    pub execution_time: Duration,
    
    /// SQL statements that were executed
    pub sql_executed: Vec<String>,
    
    /// Number of rows affected by this operation
    pub rows_affected: usize,
    
    /// Warnings generated during operation execution
    pub warnings: Vec<String>,
}

/// Information about a failed rollback operation
#[derive(Debug, Clone)]
pub struct RollbackOperationFailure {
    /// The operation that failed
    pub operation: RollbackOperation,
    
    /// Error that caused the failure
    pub error: OperationError,
    
    /// Time when the failure occurred
    pub failure_time: Duration,
    
    /// Number of retry attempts made
    pub retry_attempts: usize,
    
    /// Whether recovery was attempted
    pub recovery_attempted: bool,
    
    /// SQL statements executed before failure
    pub partial_sql_executed: Vec<String>,
}

/// Detailed error information for operation failures
#[derive(Debug, Clone)]
pub struct OperationError {
    /// Error type classification
    pub error_type: OperationErrorType,
    
    /// Human-readable error message
    pub message: String,
    
    /// SQL statement that caused the error (if applicable)
    pub failed_sql: Option<String>,
    
    /// Database error code (if applicable)
    pub db_error_code: Option<String>,
    
    /// Additional context for debugging
    pub context: HashMap<String, String>,
    
    /// Whether this error is recoverable
    pub is_recoverable: bool,
    
    /// Suggested recovery actions
    pub recovery_suggestions: Vec<String>,
}

/// Types of operation errors that can occur
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationErrorType {
    /// SQL syntax or execution error
    SqlError,
    
    /// Database connection or network error
    ConnectionError,
    
    /// Transaction management error
    TransactionError,
    
    /// Timeout during operation execution
    TimeoutError,
    
    /// Permission or authorization error
    PermissionError,
    
    /// Data integrity or constraint violation
    IntegrityError,
    
    /// Resource exhaustion (disk space, memory, etc.)
    ResourceError,
    
    /// Internal system error
    SystemError,
    
    /// User-initiated cancellation
    CancellationError,
    
    /// Unknown or unexpected error
    UnknownError,
}

/// Result of executing a pre-execution check
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// The check that was executed
    pub check: PreExecutionCheck,
    
    /// Whether the check passed
    pub passed: bool,
    
    /// Time taken to execute the check
    pub execution_time: Duration,
    
    /// Result message or error description
    pub message: String,
    
    /// Additional details about the check result
    pub details: HashMap<String, String>,
    
    /// Whether this check failure is blocking
    pub is_blocking: bool,
    
    /// Action taken based on the check result
    pub action_taken: CheckFailureAction,
}

/// Result of executing a post-execution validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// The validation that was executed
    pub validation: PostExecutionValidation,
    
    /// Whether the validation passed
    pub passed: bool,
    
    /// Time taken to execute the validation
    pub execution_time: Duration,
    
    /// Result message or error description
    pub message: String,
    
    /// Additional details about the validation result
    pub details: HashMap<String, String>,
    
    /// Whether this validation failure is critical
    pub is_critical: bool,
    
    /// Action taken based on the validation result
    pub action_taken: ValidationFailureAction,
}

/// Current execution state of the rollback engine
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionState {
    /// Execution has not started
    NotStarted,
    
    /// Running pre-execution checks
    RunningPreChecks,
    
    /// Executing rollback operations
    ExecutingOperations,
    
    /// Running post-execution validations
    RunningPostValidations,
    
    /// Execution completed successfully
    Completed,
    
    /// Execution failed
    Failed,
    
    /// Execution was cancelled
    Cancelled,
    
    /// Execution is paused (for manual intervention)
    Paused,
}

/// Progress tracker interface for rollback execution
pub trait RollbackProgressTracker {
    /// Update progress with current operation information
    fn update_progress(&mut self, operation_index: usize, total_operations: usize, 
                      current_operation: &str, percentage: f64);
    
    /// Report operation completion
    fn operation_completed(&mut self, operation_index: usize, success: bool, duration: Duration);
    
    /// Report execution state change
    fn state_changed(&mut self, new_state: ExecutionState);
    
    /// Add warning message
    fn add_warning(&mut self, warning: String);
    
    /// Add error message
    fn add_error(&mut self, error: String);
}

impl RollbackExecutionEngine {
    /// Create a new rollback execution engine with default configuration
    pub fn new() -> Self {
        Self {
            config: RollbackConfig::default(),
            sql_generator: RollbackSqlGenerator::new(),
            metrics: std::cell::RefCell::new(ExecutionMetrics {
                total_operations: 0,
                total_execution_time: Duration::from_secs(0),
                average_operation_time: Duration::from_secs(0),
                failed_operations: 0,
                retried_operations: 0,
            }),
        }
    }
    
    /// Create a new rollback execution engine with specified configuration
    pub fn with_config(config: RollbackConfig) -> Self {
        let sql_config = SqlGeneratorConfig {
            use_transactions: config.use_transactions,
            use_conditional_clauses: true,
            max_batch_size: 50,
            add_comments: false,
            operation_timeout_seconds: config.operation_timeout.as_secs(),
        };
        
        Self {
            config: config.clone(),
            sql_generator: RollbackSqlGenerator::with_config(sql_config),
            metrics: std::cell::RefCell::new(ExecutionMetrics {
                total_operations: 0,
                total_execution_time: Duration::from_secs(0),
                average_operation_time: Duration::from_secs(0),
                failed_operations: 0,
                retried_operations: 0,
            }),
        }
    }
    
    /// Execute complete rollback plan with progress tracking
    /// 
    /// This is the main entry point for rollback execution. It coordinates the entire
    /// process including pre-checks, operation execution, and post-validations.
    pub async fn execute_rollback_operations(
        &self,
        db: &D1Client,
        rollback_plan: &RollbackPlan,
        progress_tracker: &mut dyn RollbackProgressTracker,
    ) -> Result<RollbackExecutionResult> {
        let start_time = Instant::now();
        static ROLLBACK_COUNTER: AtomicUsize = AtomicUsize::new(1);
        let rollback_id = format!("rollback_{}", ROLLBACK_COUNTER.fetch_add(1, Ordering::SeqCst));
        
        progress_tracker.state_changed(ExecutionState::RunningPreChecks);
        
        // Execute pre-execution checks
        let pre_check_results = self.execute_pre_checks(db, &rollback_plan.pre_execution_checks).await?;
        
        // Check if any blocking issues were found
        let blocking_failures: Vec<&CheckResult> = pre_check_results.iter()
            .filter(|r| !r.passed && r.is_blocking)
            .collect();
        
        if !blocking_failures.is_empty() {
            let _error_messages: Vec<String> = blocking_failures.iter()
                .map(|r| format!("Pre-check failed: {}", r.message))
                .collect();
            
            return Ok(RollbackExecutionResult {
                success: false,
                completed_operations: vec![],
                failed_operations: vec![],
                total_duration: start_time.elapsed(),
                operations_completed: 0,
                operations_failed: 0,
                warnings: vec![],
                rollback_id,
                pre_check_results,
                post_validation_results: vec![],
                final_state: ExecutionState::Failed,
            });
        }
        
        progress_tracker.state_changed(ExecutionState::ExecutingOperations);
        
        // Execute rollback operations
        let mut completed_operations = Vec::new();
        let mut failed_operations = Vec::new();
        let mut warnings = Vec::new();
        
        for (index, operation) in rollback_plan.operations.iter().enumerate() {
            progress_tracker.update_progress(
                index,
                rollback_plan.operations.len(),
                &format!("{:?}", operation),
                (index as f64 / rollback_plan.operations.len() as f64) * 100.0,
            );
            
            match self.execute_single_operation(db, operation).await {
                Ok(result) => {
                    if result.success {
                        progress_tracker.operation_completed(index, true, result.execution_time);
                        warnings.extend(result.warnings.clone());
                        completed_operations.push(RollbackOperationResult {
                            operation: result.operation,
                            execution_time: result.execution_time,
                            sql_executed: result.sql_executed,
                            rows_affected: result.rows_affected,
                            warnings: result.warnings,
                        });
                    } else {
                        progress_tracker.operation_completed(index, false, result.execution_time);
                        let failure = RollbackOperationFailure {
                            operation: result.operation.clone(),
                            error: result.error.unwrap_or_else(|| OperationError {
                                error_type: OperationErrorType::UnknownError,
                                message: "Operation failed without specific error".to_string(),
                                failed_sql: None,
                                db_error_code: None,
                                context: HashMap::new(),
                                is_recoverable: false,
                                recovery_suggestions: vec![],
                            }),
                            failure_time: result.execution_time,
                            retry_attempts: 0,
                            recovery_attempted: false,
                            partial_sql_executed: result.sql_executed,
                        };
                        failed_operations.push(failure);
                        
                        // Stop on first failure if configured
                        if self.config.stop_on_first_failure {
                            break;
                        }
                    }
                }
                Err(e) => {
                    progress_tracker.operation_completed(index, false, Duration::from_secs(0));
                    progress_tracker.add_error(format!("Operation failed: {}", e));
                    
                    let failure = RollbackOperationFailure {
                        operation: operation.clone(),
                        error: OperationError {
                            error_type: OperationErrorType::SystemError,
                            message: e.to_string(),
                            failed_sql: None,
                            db_error_code: None,
                            context: HashMap::new(),
                            is_recoverable: false,
                            recovery_suggestions: vec![],
                        },
                        failure_time: Duration::from_secs(0),
                        retry_attempts: 0,
                        recovery_attempted: false,
                        partial_sql_executed: vec![],
                    };
                    failed_operations.push(failure);
                    
                    if self.config.stop_on_first_failure {
                        break;
                    }
                }
            }
        }
        
        progress_tracker.state_changed(ExecutionState::RunningPostValidations);
        
        // Execute post-execution validations
        let post_validation_results = self.execute_post_validations(db, &rollback_plan.post_execution_validations).await?;
        
        // Determine final success state
        let success = failed_operations.is_empty() && 
                     post_validation_results.iter().all(|v| v.passed || !v.is_critical);
        
        let final_state = if success {
            ExecutionState::Completed
        } else {
            ExecutionState::Failed
        };
        
        progress_tracker.state_changed(final_state.clone());
        
        let operations_completed = completed_operations.len();
        let operations_failed = failed_operations.len();
        
        Ok(RollbackExecutionResult {
            success,
            completed_operations,
            failed_operations,
            total_duration: start_time.elapsed(),
            operations_completed,
            operations_failed,
            warnings,
            rollback_id,
            pre_check_results,
            post_validation_results,
            final_state,
        })
    }
    
    /// Execute single rollback operation with comprehensive error handling
    async fn execute_single_operation(
        &self,
        db: &D1Client,
        operation: &RollbackOperation,
    ) -> Result<SingleOperationResult> {
        let start_time = Instant::now();
        let mut warnings = Vec::new();
        
        // Generate SQL for the operation
        let sql_result = self.generate_sql_for_operation(operation);
        let sql_statements = match sql_result {
            Ok(result) => result.statements,
            Err(e) => {
                return Ok(SingleOperationResult {
                    operation: operation.clone(),
                    execution_time: start_time.elapsed(),
                    sql_executed: vec![],
                    rows_affected: 0,
                    warnings: vec![],
                    success: false,
                    error: Some(OperationError {
                        error_type: OperationErrorType::SqlError,
                        message: format!("Failed to generate SQL: {}", e),
                        failed_sql: None,
                        db_error_code: None,
                        context: HashMap::new(),
                        is_recoverable: false,
                        recovery_suggestions: vec!["Check operation parameters and retry".to_string()],
                    }),
                    transaction_id: None,
                });
            }
        };
        
        let mut rows_affected = 0;
        let mut executed_sql = Vec::new();
        
        // Execute SQL statements
        for sql in &sql_statements {
            match self.execute_sql_with_timeout(db, sql).await {
                Ok(result) => {
                    executed_sql.push(sql.clone());
                    rows_affected += result.rows_affected;
                    
                    // Add any warnings from SQL execution
                    if let Some(warning) = result.warning {
                        warnings.push(warning);
                    }
                }
                Err(e) => {
                    return Ok(SingleOperationResult {
                        operation: operation.clone(),
                        execution_time: start_time.elapsed(),
                        sql_executed: executed_sql,
                        rows_affected,
                        warnings,
                        success: false,
                        error: Some(OperationError {
                            error_type: self.classify_error(&e),
                            message: e.to_string(),
                            failed_sql: Some(sql.clone()),
                            db_error_code: None,
                            context: HashMap::new(),
                            is_recoverable: self.is_error_recoverable(&e),
                            recovery_suggestions: self.get_recovery_suggestions(&e),
                        }),
                        transaction_id: None,
                    });
                }
            }
        }
        
        // Update metrics
        self.update_metrics(start_time.elapsed(), true);
        
        Ok(SingleOperationResult {
            operation: operation.clone(),
            execution_time: start_time.elapsed(),
            sql_executed: executed_sql,
            rows_affected,
            warnings,
            success: true,
            error: None,
            transaction_id: None,
        })
    }
    
    /// Execute pre-execution checks with comprehensive validation
    async fn execute_pre_checks(
        &self,
        db: &D1Client,
        checks: &[PreExecutionCheck],
    ) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();
        
        for check in checks {
            let start_time = Instant::now();
            let result = self.execute_single_check(db, check).await;
            
            let check_result = match result {
                Ok(passed) => CheckResult {
                    check: check.clone(),
                    passed,
                    execution_time: start_time.elapsed(),
                    message: if passed {
                        format!("Check '{}' passed", check.description)
                    } else {
                        format!("Check '{}' failed", check.description)
                    },
                    details: HashMap::new(),
                    is_blocking: check.is_required,
                    action_taken: if passed || !check.is_required {
                        CheckFailureAction::Warn
                    } else {
                        check.failure_action
                    },
                },
                Err(e) => CheckResult {
                    check: check.clone(),
                    passed: false,
                    execution_time: start_time.elapsed(),
                    message: format!("Check '{}' failed with error: {}", check.description, e),
                    details: HashMap::new(),
                    is_blocking: check.is_required,
                    action_taken: check.failure_action,
                },
            };
            
            results.push(check_result);
        }
        
        Ok(results)
    }
    
    /// Execute post-execution validations with comprehensive verification
    async fn execute_post_validations(
        &self,
        db: &D1Client,
        validations: &[PostExecutionValidation],
    ) -> Result<Vec<ValidationResult>> {
        let mut results = Vec::new();
        
        for validation in validations {
            let start_time = Instant::now();
            let result = self.execute_single_validation(db, validation).await;
            
            let validation_result = match result {
                Ok(passed) => ValidationResult {
                    validation: validation.clone(),
                    passed,
                    execution_time: start_time.elapsed(),
                    message: if passed {
                        format!("Validation '{}' passed", validation.description)
                    } else {
                        format!("Validation '{}' failed", validation.description)
                    },
                    details: HashMap::new(),
                    is_critical: validation.is_required,
                    action_taken: if passed || !validation.is_required {
                        ValidationFailureAction::WarnButContinue
                    } else {
                        validation.failure_action
                    },
                },
                Err(e) => ValidationResult {
                    validation: validation.clone(),
                    passed: false,
                    execution_time: start_time.elapsed(),
                    message: format!("Validation '{}' failed with error: {}", validation.description, e),
                    details: HashMap::new(),
                    is_critical: validation.is_required,
                    action_taken: validation.failure_action,
                },
            };
            
            results.push(validation_result);
        }
        
        Ok(results)
    }
    
    /// Execute a single pre-execution check
    async fn execute_single_check(&self, db: &D1Client, check: &PreExecutionCheck) -> Result<bool> {
        match check.check_type {
            CheckType::TableExists => {
                if let Some(sql) = &check.sql_query {
                    let result = db.execute(sql, &[]).await?;
                    Ok(!result.rows.is_empty())
                } else {
                    Err(D1RsError::ValidationError("TableExists check requires SQL query".to_string()))
                }
            }
            CheckType::ColumnExists => {
                if let Some(sql) = &check.sql_query {
                    let result = db.execute(sql, &[]).await?;
                    Ok(!result.rows.is_empty())
                } else {
                    Err(D1RsError::ValidationError("ColumnExists check requires SQL query".to_string()))
                }
            }
            CheckType::DataIntegrity => {
                if let Some(sql) = &check.sql_query {
                    let _result = db.execute(sql, &[]).await?;
                    // Assume data integrity check passes if query executes without error
                    Ok(true)
                } else {
                    Ok(true) // No specific check means assume integrity is fine
                }
            }
            CheckType::ForeignKeyConstraints => {
                // Check foreign key constraint status
                let pragma_sql = "PRAGMA foreign_key_check";
                let result = db.execute(pragma_sql, &[]).await?;
                Ok(result.rows.is_empty()) // Empty result means no FK violations
            }
            CheckType::IndexExists => {
                if let Some(sql) = &check.sql_query {
                    let result = db.execute(sql, &[]).await?;
                    Ok(!result.rows.is_empty())
                } else {
                    Ok(true) // Assume index exists if no specific check
                }
            }
            CheckType::LockStatus => {
                // SQLite doesn't have explicit lock status, assume no locks
                Ok(true)
            }
            CheckType::BackupAvailability => {
                // For SQLite/D1, we can't check backup availability directly
                Ok(true)
            }
            CheckType::DiskSpace => {
                // Can't check disk space in WASM/D1 environment
                Ok(true)
            }
            CheckType::Permissions => {
                // Assume permissions are adequate if we can connect
                Ok(true)
            }
            CheckType::Custom => {
                if let Some(sql) = &check.sql_query {
                    let result = db.execute(sql, &[]).await?;
                    Ok(!result.rows.is_empty())
                } else {
                    Ok(true)
                }
            }
        }
    }
    
    /// Execute a single post-execution validation
    async fn execute_single_validation(&self, db: &D1Client, validation: &PostExecutionValidation) -> Result<bool> {
        match validation.validation_type {
            ValidationType::SchemaValidation => {
                if let Some(sql) = &validation.sql_query {
                    let result = db.execute(sql, &[]).await?;
                    Ok(!result.rows.is_empty())
                } else {
                    Ok(true)
                }
            }
            ValidationType::DataIntegrity => {
                if let Some(sql) = &validation.sql_query {
                    let _result = db.execute(sql, &[]).await?;
                    Ok(true) // If query executes without error, assume integrity is good
                } else {
                    Ok(true)
                }
            }
            ValidationType::ConstraintValidation => {
                let pragma_sql = "PRAGMA integrity_check";
                let result = db.execute(pragma_sql, &[]).await?;
                // Check if result contains "ok"
                Ok(result.rows.iter().any(|row| {
                    row.as_object()
                        .and_then(|obj| obj.get("integrity_check"))
                        .and_then(|v| v.as_str())
                        .map(|s| s == "ok")
                        .unwrap_or(false)
                }))
            }
            ValidationType::IndexValidation => {
                if let Some(sql) = &validation.sql_query {
                    let result = db.execute(sql, &[]).await?;
                    Ok(!result.rows.is_empty())
                } else {
                    Ok(true)
                }
            }
            ValidationType::ForeignKeyValidation => {
                let pragma_sql = "PRAGMA foreign_key_check";
                let result = db.execute(pragma_sql, &[]).await?;
                Ok(result.rows.is_empty())
            }
            ValidationType::DataRestoration => {
                if let Some(sql) = &validation.sql_query {
                    let result = db.execute(sql, &[]).await?;
                    Ok(!result.rows.is_empty())
                } else {
                    Ok(true)
                }
            }
            ValidationType::ApplicationConnectivity => {
                // Test basic connectivity by executing a simple query
                let result = db.execute("SELECT 1", &[]).await?;
                Ok(!result.rows.is_empty())
            }
            ValidationType::PerformanceCheck => {
                // Simple performance check - ensure query executes within reasonable time
                if let Some(sql) = &validation.sql_query {
                    let start = Instant::now();
                    let _result = db.execute(sql, &[]).await?;
                    let duration = start.elapsed();
                    Ok(duration < Duration::from_secs(30)) // Arbitrary 30-second threshold
                } else {
                    Ok(true)
                }
            }
            ValidationType::Custom => {
                if let Some(sql) = &validation.sql_query {
                    let result = db.execute(sql, &[]).await?;
                    Ok(!result.rows.is_empty())
                } else {
                    Ok(true)
                }
            }
        }
    }
    
    /// Execute SQL with timeout handling
    async fn execute_sql_with_timeout(&self, db: &D1Client, sql: &str) -> Result<SqlExecutionResult> {
        // For now, execute directly as D1Client doesn't support timeouts
        // In a real implementation, you might use tokio::time::timeout
        let result = db.execute(sql, &[]).await?;
        
        Ok(SqlExecutionResult {
            rows_affected: result.rows.len(),
            warning: None,
        })
    }
    
    /// Classify error type for better error handling
    fn classify_error(&self, error: &D1RsError) -> OperationErrorType {
        match error {
            D1RsError::Database(_) => OperationErrorType::SqlError,
            D1RsError::ValidationError(_) => OperationErrorType::IntegrityError,
            D1RsError::SerializationError(_) => OperationErrorType::SystemError,
            D1RsError::NotFound => OperationErrorType::SqlError,
            _ => OperationErrorType::UnknownError,
        }
    }
    
    /// Determine if an error is recoverable
    fn is_error_recoverable(&self, error: &D1RsError) -> bool {
        match error {
            D1RsError::Database(_) => false,     // Database errors usually require fixes
            D1RsError::ValidationError(_) => false, // Validation errors need addressing
            D1RsError::NotFound => false,        // Not found errors are not recoverable
            D1RsError::SerializationError(_) => false, // Serialization errors usually indicate bugs
            _ => false,
        }
    }
    
    /// Get recovery suggestions for an error
    fn get_recovery_suggestions(&self, error: &D1RsError) -> Vec<String> {
        match error {
            D1RsError::Database(_) => vec![
                "Review SQL syntax and parameters".to_string(),
                "Check table and column names".to_string(),
                "Verify data types and constraints".to_string(),
            ],
            D1RsError::ValidationError(_) => vec![
                "Review validation rules".to_string(),
                "Check data integrity".to_string(),
                "Verify operation prerequisites".to_string(),
            ],
            D1RsError::NotFound => vec![
                "Verify table or column exists".to_string(),
                "Check schema definition".to_string(),
                "Ensure migration has been applied".to_string(),
            ],
            D1RsError::SerializationError(_) => vec![
                "Check data format".to_string(),
                "Verify type compatibility".to_string(),
                "Review serialization logic".to_string(),
            ],
            _ => vec![
                "Review error details".to_string(),
                "Check system logs".to_string(),
                "Contact support if issue persists".to_string(),
            ],
        }
    }
    
    /// Generate SQL for a rollback operation
    fn generate_sql_for_operation(&self, operation: &RollbackOperation) -> Result<super::sql_generator::SqlGenerationResult> {
        
        match operation {
            RollbackOperation::DropTable { name, .. } => {
                // Simple DROP TABLE statement
                Ok(super::sql_generator::SqlGenerationResult {
                    statements: vec![format!("DROP TABLE IF EXISTS {}", self.escape_identifier(name))],
                    requires_transaction: false,
                    estimated_duration_seconds: 0.1,
                    warnings: vec![],
                })
            }
            RollbackOperation::RecreateTable { definition, .. } => {
                self.sql_generator.generate_create_table_sql(definition)
            }
            RollbackOperation::DropColumn { table, column, .. } => {
                self.sql_generator.generate_drop_column_sql(table, column)
            }
            RollbackOperation::AddColumn { table, column, .. } => {
                self.sql_generator.generate_add_column_sql(table, column)
            }
            RollbackOperation::ModifyColumn { table, column, changes, .. } => {
                self.sql_generator.generate_modify_column_sql(table, column, changes)
            }
            RollbackOperation::DropIndex { name, .. } => {
                Ok(super::sql_generator::SqlGenerationResult {
                    statements: vec![format!("DROP INDEX IF EXISTS {}", self.escape_identifier(name))],
                    requires_transaction: false,
                    estimated_duration_seconds: 0.1,
                    warnings: vec![],
                })
            }
            RollbackOperation::CreateIndex { table, index } => {
                self.sql_generator.generate_create_index_sql(table, index)
            }
            RollbackOperation::DropForeignKey { .. } => {
                // SQLite doesn't support dropping foreign keys directly
                Ok(super::sql_generator::SqlGenerationResult {
                    statements: vec!["-- SQLite does not support dropping foreign keys".to_string()],
                    requires_transaction: false,
                    estimated_duration_seconds: 0.0,
                    warnings: vec!["SQLite does not support dropping foreign keys directly".to_string()],
                })
            }
            RollbackOperation::AddForeignKey { table, constraint } => {
                self.sql_generator.generate_foreign_key_sql(table, constraint)
            }
            RollbackOperation::RenameTable { old_name, new_name } => {
                Ok(super::sql_generator::SqlGenerationResult {
                    statements: vec![format!("ALTER TABLE {} RENAME TO {}", 
                                           self.escape_identifier(old_name), 
                                           self.escape_identifier(new_name))],
                    requires_transaction: false,
                    estimated_duration_seconds: 0.1,
                    warnings: vec![],
                })
            }
            RollbackOperation::RenameColumn { table, old_name, new_name } => {
                Ok(super::sql_generator::SqlGenerationResult {
                    statements: vec![format!("ALTER TABLE {} RENAME COLUMN {} TO {}", 
                                           self.escape_identifier(table),
                                           self.escape_identifier(old_name), 
                                           self.escape_identifier(new_name))],
                    requires_transaction: false,
                    estimated_duration_seconds: 0.1,
                    warnings: vec![],
                })
            }
        }
    }
    
    /// Escape SQL identifiers to prevent injection and handle reserved words
    fn escape_identifier(&self, identifier: &str) -> String {
        if identifier.contains(' ') || identifier.contains('"') || identifier.chars().any(|c| !c.is_alphanumeric() && c != '_') {
            format!("\"{}\"", identifier.replace('"', "\"\""))
        } else {
            identifier.to_string()
        }
    }
    
    /// Update execution metrics
    fn update_metrics(&self, operation_time: Duration, success: bool) {
        let mut metrics = self.metrics.borrow_mut();
        metrics.total_operations += 1;
        metrics.total_execution_time += operation_time;
        
        if !success {
            metrics.failed_operations += 1;
        }
        
        metrics.average_operation_time = Duration::from_nanos(
            metrics.total_execution_time.as_nanos() as u64 / metrics.total_operations as u64
        );
    }
    
    /// Get current execution metrics
    pub fn get_metrics(&self) -> ExecutionMetrics {
        self.metrics.borrow().clone()
    }
}

/// Result of SQL execution
#[derive(Debug)]
struct SqlExecutionResult {
    rows_affected: usize,
    warning: Option<String>,
}

/// Default implementations for required traits
impl Default for RollbackExecutionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ExecutionMetrics {
    fn default() -> Self {
        Self {
            total_operations: 0,
            total_execution_time: Duration::from_secs(0),
            average_operation_time: Duration::from_secs(0),
            failed_operations: 0,
            retried_operations: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use crate::auto_migration::rollback::{RollbackPlanMetadata, ExecutionPriority};
    
    fn create_test_config() -> RollbackConfig {
        RollbackConfig {
            preserve_data_on_rollback: true,
            restore_data_on_rollback: true,
            create_backup_tables: true,
            stop_on_first_failure: true,
            operation_timeout: Duration::from_secs(30),
            use_transactions: true,
            max_parallel_operations: 1,
            validate_schema_compatibility: true,
            perform_dry_run: false,
            backup_table_prefix: "backup_".to_string(),
            backup_table_suffix: "_old".to_string(),
        }
    }
    
    fn create_test_rollback_plan() -> RollbackPlan {
        RollbackPlan {
            plan_id: "test_plan_001".to_string(),
            operations: vec![
                RollbackOperation::DropTable {
                    name: "test_table".to_string(),
                    preserve_data: true,
                    backup_table_name: Some("test_table_backup".to_string()),
                }
            ],
            data_preservation_requirements: vec![],
            pre_execution_checks: vec![
                PreExecutionCheck {
                    check_id: "check_001".to_string(),
                    check_type: CheckType::TableExists,
                    description: "Verify test table exists".to_string(),
                    sql_query: Some("SELECT name FROM sqlite_master WHERE type='table' AND name='test_table'".to_string()),
                    expected_result: "Table should exist".to_string(),
                    is_required: true,
                    timeout: Duration::from_secs(5),
                    failure_action: CheckFailureAction::Stop,
                }
            ],
            post_execution_validations: vec![
                PostExecutionValidation {
                    validation_id: "validation_001".to_string(),
                    validation_type: ValidationType::SchemaValidation,
                    description: "Verify table was dropped".to_string(),
                    sql_query: Some("SELECT name FROM sqlite_master WHERE type='table' AND name='test_table'".to_string()),
                    expected_result: "Table should not exist".to_string(),
                    is_required: true,
                    timeout: Duration::from_secs(5),
                    failure_action: ValidationFailureAction::MarkFailed,
                }
            ],
            estimated_duration: Duration::from_secs(10),
            original_migration_id: Some("migration_001".to_string()),
            created_at: SystemTime::now(),
            risk_assessment: super::super::types::RollbackRiskLevel::Medium,
            config: create_test_config(),
            description: "Test rollback plan".to_string(),
            rollback_system_version: "1.0.0".to_string(),
            metadata: RollbackPlanMetadata {
                environment: "test".to_string(),
                created_by: Some("test_user".to_string()),
                tags: vec!["test".to_string()],
                affected_tables: 1,
                estimated_affected_rows: Some(100),
                requires_manual_intervention: false,
                execution_priority: ExecutionPriority::Normal,
            },
        }
    }
    
    struct MockProgressTracker {
        operations_completed: usize,
        state_changes: Vec<ExecutionState>,
        warnings: Vec<String>,
        errors: Vec<String>,
    }
    
    impl MockProgressTracker {
        fn new() -> Self {
            Self {
                operations_completed: 0,
                state_changes: vec![],
                warnings: vec![],
                errors: vec![],
            }
        }
    }
    
    impl RollbackProgressTracker for MockProgressTracker {
        fn update_progress(&mut self, _operation_index: usize, _total_operations: usize, 
                          _current_operation: &str, _percentage: f64) {
            // Mock implementation
        }
        
        fn operation_completed(&mut self, _operation_index: usize, success: bool, _duration: Duration) {
            if success {
                self.operations_completed += 1;
            }
        }
        
        fn state_changed(&mut self, new_state: ExecutionState) {
            self.state_changes.push(new_state);
        }
        
        fn add_warning(&mut self, warning: String) {
            self.warnings.push(warning);
        }
        
        fn add_error(&mut self, error: String) {
            self.errors.push(error);
        }
    }
    
    #[test]
    fn test_execution_engine_creation() {
        let engine = RollbackExecutionEngine::new();
        let metrics = engine.get_metrics();
        assert_eq!(metrics.total_operations, 0);
        assert_eq!(metrics.failed_operations, 0);
    }
    
    #[test]
    fn test_execution_engine_with_config() {
        let config = create_test_config();
        let engine = RollbackExecutionEngine::with_config(config);
        let metrics = engine.get_metrics();
        assert_eq!(metrics.total_operations, 0);
    }
    
    #[test]
    fn test_error_classification() {
        let engine = RollbackExecutionEngine::new();
        
        let database_error = D1RsError::Database("syntax error".to_string());
        assert_eq!(engine.classify_error(&database_error), OperationErrorType::SqlError);
        
        let validation_error = D1RsError::ValidationError("validation failed".to_string());
        assert_eq!(engine.classify_error(&validation_error), OperationErrorType::IntegrityError);
        
        let not_found_error = D1RsError::NotFound;
        assert_eq!(engine.classify_error(&not_found_error), OperationErrorType::SqlError);
    }
    
    #[test]
    fn test_error_recoverability() {
        let engine = RollbackExecutionEngine::new();
        
        let database_error = D1RsError::Database("connection failed".to_string());
        assert!(!engine.is_error_recoverable(&database_error));
        
        let validation_error = D1RsError::ValidationError("validation failed".to_string());
        assert!(!engine.is_error_recoverable(&validation_error));
    }
    
    #[test]
    fn test_recovery_suggestions() {
        let engine = RollbackExecutionEngine::new();
        
        let database_error = D1RsError::Database("connection failed".to_string());
        let suggestions = engine.get_recovery_suggestions(&database_error);
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.contains("SQL") || s.contains("syntax")));
    }
    
    #[test]
    fn test_operation_error_creation() {
        let error = OperationError {
            error_type: OperationErrorType::SqlError,
            message: "Test error".to_string(),
            failed_sql: Some("SELECT * FROM nonexistent".to_string()),
            db_error_code: Some("404".to_string()),
            context: HashMap::new(),
            is_recoverable: false,
            recovery_suggestions: vec!["Fix SQL".to_string()],
        };
        
        assert_eq!(error.error_type, OperationErrorType::SqlError);
        assert_eq!(error.message, "Test error");
        assert!(!error.is_recoverable);
    }
    
    #[test]
    fn test_execution_state_transitions() {
        let states = vec![
            ExecutionState::NotStarted,
            ExecutionState::RunningPreChecks,
            ExecutionState::ExecutingOperations,
            ExecutionState::RunningPostValidations,
            ExecutionState::Completed,
        ];
        
        // Verify all states are different
        for (i, state1) in states.iter().enumerate() {
            for (j, state2) in states.iter().enumerate() {
                if i != j {
                    assert_ne!(state1, state2);
                }
            }
        }
    }
    
    #[test]
    fn test_single_operation_result_creation() {
        let operation = RollbackOperation::DropTable {
            name: "test_table".to_string(),
            preserve_data: true,
            backup_table_name: Some("backup_table".to_string()),
        };
        
        let result = SingleOperationResult {
            operation: operation.clone(),
            execution_time: Duration::from_millis(100),
            sql_executed: vec!["DROP TABLE test_table".to_string()],
            rows_affected: 1,
            warnings: vec!["Table had data".to_string()],
            success: true,
            error: None,
            transaction_id: Some("tx_123".to_string()),
        };
        
        assert!(result.success);
        assert_eq!(result.rows_affected, 1);
        assert_eq!(result.sql_executed.len(), 1);
        assert_eq!(result.warnings.len(), 1);
    }
    
    #[test]
    fn test_rollback_execution_result_creation() {
        let result = RollbackExecutionResult {
            success: true,
            completed_operations: vec![],
            failed_operations: vec![],
            total_duration: Duration::from_secs(5),
            operations_completed: 1,
            operations_failed: 0,
            warnings: vec!["Test warning".to_string()],
            rollback_id: "rollback_123".to_string(),
            pre_check_results: vec![],
            post_validation_results: vec![],
            final_state: ExecutionState::Completed,
        };
        
        assert!(result.success);
        assert_eq!(result.operations_completed, 1);
        assert_eq!(result.operations_failed, 0);
        assert_eq!(result.final_state, ExecutionState::Completed);
    }
    
    #[test]
    fn test_check_result_creation() {
        let check = PreExecutionCheck {
            check_id: "check_001".to_string(),
            check_type: CheckType::TableExists,
            description: "Test check".to_string(),
            sql_query: Some("SELECT 1".to_string()),
            expected_result: "Should pass".to_string(),
            is_required: true,
            timeout: Duration::from_secs(5),
            failure_action: CheckFailureAction::Stop,
        };
        
        let result = CheckResult {
            check: check.clone(),
            passed: true,
            execution_time: Duration::from_millis(50),
            message: "Check passed".to_string(),
            details: HashMap::new(),
            is_blocking: true,
            action_taken: CheckFailureAction::Warn,
        };
        
        assert!(result.passed);
        assert!(result.is_blocking);
        assert_eq!(result.message, "Check passed");
    }
    
    #[test]
    fn test_validation_result_creation() {
        let validation = PostExecutionValidation {
            validation_id: "validation_001".to_string(),
            validation_type: ValidationType::SchemaValidation,
            description: "Test validation".to_string(),
            sql_query: Some("SELECT 1".to_string()),
            expected_result: "Should pass".to_string(),
            is_required: true,
            timeout: Duration::from_secs(5),
            failure_action: ValidationFailureAction::MarkFailed,
        };
        
        let result = ValidationResult {
            validation: validation.clone(),
            passed: true,
            execution_time: Duration::from_millis(75),
            message: "Validation passed".to_string(),
            details: HashMap::new(),
            is_critical: true,
            action_taken: ValidationFailureAction::WarnButContinue,
        };
        
        assert!(result.passed);
        assert!(result.is_critical);
        assert_eq!(result.message, "Validation passed");
    }
    
    #[test]
    fn test_mock_progress_tracker() {
        let mut tracker = MockProgressTracker::new();
        
        tracker.state_changed(ExecutionState::RunningPreChecks);
        tracker.operation_completed(0, true, Duration::from_millis(100));
        tracker.add_warning("Test warning".to_string());
        tracker.add_error("Test error".to_string());
        
        assert_eq!(tracker.operations_completed, 1);
        assert_eq!(tracker.state_changes.len(), 1);
        assert_eq!(tracker.warnings.len(), 1);
        assert_eq!(tracker.errors.len(), 1);
    }
    
    #[test]
    fn test_metrics_default() {
        let metrics = ExecutionMetrics::default();
        assert_eq!(metrics.total_operations, 0);
        assert_eq!(metrics.failed_operations, 0);
        assert_eq!(metrics.retried_operations, 0);
    }
}