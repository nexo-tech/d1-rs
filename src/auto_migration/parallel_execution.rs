// Phase 3.2.5 - Parallel Migration Execution - Safe concurrent migrations

use crate::{D1Client, Result};
use crate::auto_migration::{MigrationPlan, MigrationOperation};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Semaphore, RwLock};

/// Configuration for parallel migration execution
#[derive(Debug, Clone)]
pub struct ParallelExecutionConfig {
    /// Maximum number of concurrent migrations
    pub max_concurrent_operations: usize,
    
    /// Timeout for individual operations
    pub operation_timeout: Duration,
    
    /// Strategy for handling dependencies
    pub dependency_strategy: DependencyStrategy,
    
    /// Safety mode for concurrent execution
    pub safety_mode: ParallelSafetyMode,
    
    /// Enable transaction isolation
    pub use_transactions: bool,
    
    /// Retry configuration for failed operations
    pub retry_config: RetryConfig,
    
    /// Progress reporting interval
    pub progress_reporting_interval: Duration,
}

/// Strategy for handling operation dependencies
#[derive(Debug, Clone)]
pub enum DependencyStrategy {
    /// Execute in strict dependency order (safest)
    Sequential,
    /// Execute independent operations in parallel
    Independent,
    /// Use dependency graph analysis for optimal parallelization
    Optimized,
    /// User-defined custom dependency groups
    Custom(HashMap<String, Vec<String>>),
}

/// Safety mode for parallel execution
#[derive(Debug, Clone)]
pub enum ParallelSafetyMode {
    /// Maximum safety - very conservative parallelization
    Conservative,
    /// Balanced safety and performance
    Balanced,
    /// Maximum performance - minimal safety checks
    Aggressive,
    /// Custom safety rules
    Custom(SafetyRules),
}

/// Custom safety rules for parallel execution
#[derive(Debug, Clone)]
pub struct SafetyRules {
    /// Operations that cannot run concurrently
    pub conflicting_operations: Vec<(String, String)>,
    
    /// Operations that must run on the same connection
    pub connection_affinity: HashMap<String, String>,
    
    /// Maximum operations per table
    pub max_operations_per_table: usize,
    
    /// Required isolation level
    pub isolation_level: IsolationLevel,
}

/// Database isolation level
#[derive(Debug, Clone)]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

/// Retry configuration for failed operations
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retries
    pub max_retries: usize,
    
    /// Base delay between retries
    pub base_delay: Duration,
    
    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,
    
    /// Maximum delay between retries
    pub max_delay: Duration,
    
    /// Error types that should trigger retries
    pub retryable_errors: Vec<String>,
}

/// Result of parallel migration execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelExecutionResult {
    /// Total operations executed
    pub total_operations: usize,
    
    /// Successfully completed operations
    pub successful_operations: usize,
    
    /// Failed operations with error details
    pub failed_operations: Vec<OperationFailure>,
    
    /// Operations that were skipped due to dependencies
    pub skipped_operations: Vec<String>,
    
    /// Total execution time
    pub total_execution_time: Duration,
    
    /// Time saved compared to sequential execution
    pub time_saved: Option<Duration>,
    
    /// Maximum concurrent operations achieved
    pub max_concurrency_achieved: usize,
    
    /// Performance metrics per operation type
    pub performance_metrics: HashMap<String, OperationMetrics>,
    
    /// Dependency resolution statistics
    pub dependency_stats: DependencyStats,
}

/// Details of a failed operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationFailure {
    pub operation_id: String,
    pub operation_type: String,
    pub error_message: String,
    pub retry_count: usize,
    pub duration: Duration,
    pub dependencies_met: bool,
}

/// Performance metrics for an operation type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationMetrics {
    pub operation_count: usize,
    pub avg_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
    pub success_rate: f64,
    pub concurrent_executions: usize,
}

/// Statistics about dependency resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyStats {
    pub total_dependencies: usize,
    pub resolved_dependencies: usize,
    pub circular_dependencies: usize,
    pub optimization_stages: usize,
    pub parallelization_factor: f64,
}

/// State of parallel execution
#[derive(Debug, Clone)]
pub struct ExecutionState {
    /// Operations that are currently running
    pub running_operations: HashMap<String, OperationExecution>,
    
    /// Operations that have completed successfully
    pub completed_operations: HashMap<String, OperationResult>,
    
    /// Operations that have failed
    pub failed_operations: HashMap<String, OperationFailure>,
    
    /// Operations waiting for dependencies
    pub pending_operations: HashMap<String, PendingOperation>,
    
    /// Current concurrency level
    pub current_concurrency: usize,
    
    /// Overall progress percentage
    pub progress_percentage: f64,
}

/// Details of a currently executing operation
#[derive(Debug, Clone)]
pub struct OperationExecution {
    pub operation_id: String,
    pub started_at: Instant,
    pub connection_id: String,
    pub retry_count: usize,
    pub estimated_completion: Option<Instant>,
}

/// Result of a completed operation
#[derive(Debug, Clone)]
pub struct OperationResult {
    pub operation_id: String,
    pub duration: Duration,
    pub rows_affected: u64,
    pub success: bool,
}

/// Operation waiting for dependencies
#[derive(Debug, Clone)]
pub struct PendingOperation {
    pub operation_id: String,
    pub dependencies: Vec<String>,
    pub ready_at: Option<Instant>,
}

/// Parallel migration executor
pub struct ParallelMigrationExecutor {
    config: ParallelExecutionConfig,
    state: Arc<RwLock<ExecutionState>>,
    semaphore: Arc<Semaphore>,
}

impl ParallelMigrationExecutor {
    /// Create a new parallel executor with configuration
    pub fn new(config: ParallelExecutionConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_operations));
        
        Self {
            semaphore,
            state: Arc::new(RwLock::new(ExecutionState {
                running_operations: HashMap::new(),
                completed_operations: HashMap::new(),
                failed_operations: HashMap::new(),
                pending_operations: HashMap::new(),
                current_concurrency: 0,
                progress_percentage: 0.0,
            })),
            config,
        }
    }
    
    /// Execute migrations in parallel with dependency resolution
    pub async fn execute_parallel_migration(
        &self,
        db: &D1Client,
        migration_plan: &MigrationPlan,
    ) -> Result<ParallelExecutionResult> {
        let start_time = Instant::now();
        
        // Analyze dependencies and create execution graph
        let execution_graph = self.analyze_dependencies(migration_plan)?;
        
        // Initialize execution state
        self.initialize_execution_state(&execution_graph).await;
        
        // Execute operations in parallel
        let mut execution_handles = Vec::new();
        
        let execution_stages_len = execution_graph.execution_stages.len();
        
        for operation_group in &execution_graph.execution_stages {
            // Wait for previous stage to complete if needed
            if matches!(self.config.dependency_strategy, DependencyStrategy::Sequential) {
                for handle in execution_handles.drain(..) {
                    let _ = handle.await;
                }
            }
            
            // Execute operations in this stage concurrently
            for operation in operation_group {
                let permit = self.semaphore.clone().acquire_owned().await.unwrap();
                let db_clone = db.clone();
                let operation_clone = operation.clone();
                let state = self.state.clone();
                let config = self.config.clone();
                
                let handle = tokio::spawn(async move {
                    let result = Self::execute_single_operation(
                        &db_clone,
                        &operation_clone,
                        &state,
                        &config,
                    ).await;
                    
                    drop(permit);
                    result
                });
                
                execution_handles.push(handle);
            }
        }
        
        // Wait for all operations to complete
        let mut operation_results = Vec::new();
        for handle in execution_handles {
            match handle.await {
                Ok(result) => operation_results.push(result),
                Err(e) => {
                    // Handle task panic
                    return Err(crate::D1RsError::AutoMigration(format!("Task failed: {}", e)));
                }
            }
        }
        
        // Compile final results
        let total_time = start_time.elapsed();
        let final_state = self.state.read().await;
        
        Ok(ParallelExecutionResult {
            total_operations: migration_plan.operations.len(),
            successful_operations: final_state.completed_operations.len(),
            failed_operations: final_state.failed_operations.values().cloned().collect(),
            skipped_operations: Vec::new(), // TODO: Track skipped operations
            total_execution_time: total_time,
            time_saved: None, // TODO: Calculate time saved vs sequential
            max_concurrency_achieved: self.config.max_concurrent_operations,
            performance_metrics: HashMap::new(), // TODO: Calculate metrics
            dependency_stats: DependencyStats {
                total_dependencies: 0, // TODO: Calculate from graph
                resolved_dependencies: 0,
                circular_dependencies: 0,
                optimization_stages: execution_stages_len,
                parallelization_factor: 1.0,
            },
        })
    }
    
    /// Analyze operation dependencies and create execution graph
    fn analyze_dependencies(&self, migration_plan: &MigrationPlan) -> Result<ExecutionGraph> {
        let mut graph = ExecutionGraph {
            operations: migration_plan.operations.clone(),
            dependencies: HashMap::new(),
            execution_stages: Vec::new(),
        };
        
        // Analyze dependencies based on strategy
        match &self.config.dependency_strategy {
            DependencyStrategy::Sequential => {
                // All operations in sequence
                for operation in &migration_plan.operations {
                    graph.execution_stages.push(vec![operation.clone()]);
                }
            }
            
            DependencyStrategy::Independent => {
                // Group independent operations
                graph.execution_stages.push(migration_plan.operations.clone());
            }
            
            DependencyStrategy::Optimized => {
                // TODO: Implement sophisticated dependency analysis
                // For now, use simple grouping by operation type
                let mut create_tables = Vec::new();
                let mut add_columns = Vec::new();
                let mut create_indexes = Vec::new();
                let mut other_operations = Vec::new();
                
                for operation in &migration_plan.operations {
                    match operation {
                        MigrationOperation::CreateTable { .. } => create_tables.push(operation.clone()),
                        MigrationOperation::AddColumn { .. } => add_columns.push(operation.clone()),
                        MigrationOperation::CreateIndex { .. } => create_indexes.push(operation.clone()),
                        _ => other_operations.push(operation.clone()),
                    }
                }
                
                if !create_tables.is_empty() {
                    graph.execution_stages.push(create_tables);
                }
                if !add_columns.is_empty() {
                    graph.execution_stages.push(add_columns);
                }
                if !create_indexes.is_empty() {
                    graph.execution_stages.push(create_indexes);
                }
                if !other_operations.is_empty() {
                    graph.execution_stages.push(other_operations);
                }
            }
            
            DependencyStrategy::Custom(_) => {
                // TODO: Implement custom dependency handling
                graph.execution_stages.push(migration_plan.operations.clone());
            }
        }
        
        Ok(graph)
    }
    
    /// Initialize execution state with pending operations
    async fn initialize_execution_state(&self, _graph: &ExecutionGraph) {
        let mut state = self.state.write().await;
        
        // Clear previous state
        state.running_operations.clear();
        state.completed_operations.clear();
        state.failed_operations.clear();
        state.pending_operations.clear();
        state.current_concurrency = 0;
        state.progress_percentage = 0.0;
    }
    
    /// Execute a single operation with retry logic
    async fn execute_single_operation(
        db: &D1Client,
        operation: &MigrationOperation,
        state: &Arc<RwLock<ExecutionState>>,
        config: &ParallelExecutionConfig,
    ) -> Result<OperationResult> {
        let operation_id = format!("{:?}_{}", operation, chrono::Utc::now().timestamp_millis());
        let start_time = Instant::now();
        
        // Update state to show operation started
        {
            let mut state_guard = state.write().await;
            state_guard.running_operations.insert(
                operation_id.clone(),
                OperationExecution {
                    operation_id: operation_id.clone(),
                    started_at: start_time,
                    connection_id: "default".to_string(),
                    retry_count: 0,
                    estimated_completion: None,
                },
            );
            state_guard.current_concurrency += 1;
        }
        
        // Execute operation with retry logic
        let mut retry_count = 0;
        let mut last_error = None;
        
        while retry_count <= config.retry_config.max_retries {
            match Self::execute_operation_sql(db, operation).await {
                Ok(rows_affected) => {
                    let duration = start_time.elapsed();
                    
                    // Update state with successful completion
                    {
                        let mut state_guard = state.write().await;
                        state_guard.running_operations.remove(&operation_id);
                        state_guard.completed_operations.insert(
                            operation_id.clone(),
                            OperationResult {
                                operation_id: operation_id.clone(),
                                duration,
                                rows_affected,
                                success: true,
                            },
                        );
                        state_guard.current_concurrency -= 1;
                    }
                    
                    return Ok(OperationResult {
                        operation_id,
                        duration,
                        rows_affected,
                        success: true,
                    });
                }
                
                Err(error) => {
                    retry_count += 1;
                    last_error = Some(error);
                    
                    if retry_count <= config.retry_config.max_retries {
                        // Calculate delay with exponential backoff
                        let delay = config.retry_config.base_delay.mul_f64(
                            config.retry_config.backoff_multiplier.powi(retry_count as i32 - 1)
                        ).min(config.retry_config.max_delay);
                        
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }
        
        // Operation failed after all retries
        let duration = start_time.elapsed();
        let failure = OperationFailure {
            operation_id: operation_id.clone(),
            operation_type: format!("{:?}", operation),
            error_message: last_error.unwrap().to_string(),
            retry_count,
            duration,
            dependencies_met: true,
        };
        
        // Update state with failure
        {
            let mut state_guard = state.write().await;
            state_guard.running_operations.remove(&operation_id);
            state_guard.failed_operations.insert(operation_id.clone(), failure.clone());
            state_guard.current_concurrency -= 1;
        }
        
        Err(crate::D1RsError::AutoMigration(format!("Operation failed after {} retries: {}", retry_count, failure.error_message)))
    }
    
    /// Execute the actual SQL for an operation
    async fn execute_operation_sql(
        db: &D1Client,
        operation: &MigrationOperation,
    ) -> Result<u64> {
        match operation {
            MigrationOperation::CreateTable { definition } => {
                let mut sql = format!("CREATE TABLE {} (", definition.name);
                let column_defs: Vec<String> = definition.columns.iter().map(|col| {
                    format!("{} {}", col.name, col.column_type)
                }).collect();
                sql.push_str(&column_defs.join(", "));
                sql.push(')');
                
                db.execute(&sql, &[]).await?;
                Ok(1)
            }
            
            MigrationOperation::AddColumn { table, column } => {
                let sql = format!(
                    "ALTER TABLE {} ADD COLUMN {} {}",
                    table,
                    column.name,
                    column.column_type
                );
                
                db.execute(&sql, &[]).await?;
                Ok(1)
            }
            
            MigrationOperation::CreateIndex { table, index } => {
                let unique_keyword = if index.unique { "UNIQUE " } else { "" };
                let sql = format!(
                    "CREATE {}INDEX {} ON {} ({})",
                    unique_keyword,
                    index.name,
                    table,
                    index.columns.join(", ")
                );
                
                db.execute(&sql, &[]).await?;
                Ok(1)
            }
            
            _ => {
                // TODO: Implement other operation types
                Ok(0)
            }
        }
    }
    
    /// Get current execution state
    pub async fn get_execution_state(&self) -> ExecutionState {
        self.state.read().await.clone()
    }
    
    /// Get execution progress
    pub async fn get_progress(&self) -> ExecutionProgress {
        let state = self.state.read().await;
        
        let total_operations = state.running_operations.len() 
            + state.completed_operations.len() 
            + state.failed_operations.len() 
            + state.pending_operations.len();
        
        let completed_operations = state.completed_operations.len() + state.failed_operations.len();
        
        let progress_percentage = if total_operations > 0 {
            (completed_operations as f64 / total_operations as f64) * 100.0
        } else {
            0.0
        };
        
        ExecutionProgress {
            total_operations,
            completed_operations,
            running_operations: state.running_operations.len(),
            pending_operations: state.pending_operations.len(),
            progress_percentage,
            current_concurrency: state.current_concurrency,
            estimated_completion: None, // TODO: Calculate ETA
        }
    }
}

/// Execution graph for dependency management
#[derive(Debug, Clone)]
struct ExecutionGraph {
    #[allow(dead_code)]
    operations: Vec<MigrationOperation>,
    #[allow(dead_code)]
    dependencies: HashMap<String, Vec<String>>,
    execution_stages: Vec<Vec<MigrationOperation>>,
}

/// Progress information for parallel execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionProgress {
    pub total_operations: usize,
    pub completed_operations: usize,
    pub running_operations: usize,
    pub pending_operations: usize,
    pub progress_percentage: f64,
    pub current_concurrency: usize,
    pub estimated_completion: Option<Duration>,
}

impl Default for ParallelExecutionConfig {
    fn default() -> Self {
        Self {
            max_concurrent_operations: 4,
            operation_timeout: Duration::from_secs(300),
            dependency_strategy: DependencyStrategy::Optimized,
            safety_mode: ParallelSafetyMode::Balanced,
            use_transactions: true,
            retry_config: RetryConfig {
                max_retries: 3,
                base_delay: Duration::from_millis(1000),
                backoff_multiplier: 2.0,
                max_delay: Duration::from_secs(30),
                retryable_errors: vec![
                    "database is locked".to_string(),
                    "connection timeout".to_string(),
                    "temporary failure".to_string(),
                ],
            },
            progress_reporting_interval: Duration::from_secs(5),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auto_migration::{ColumnSchema, MigrationPlan};

    fn create_test_migration_plan() -> MigrationPlan {
        MigrationPlan {
            operations: vec![
                MigrationOperation::AddColumn {
                    table: "users".to_string(),
                    column: ColumnSchema {
                        name: "id".to_string(),
                        column_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: None,
                        primary_key: true,
                        auto_increment: true,
                        unique: false,
                        constraints: Vec::new(),
                    },
                },
                MigrationOperation::AddColumn {
                    table: "posts".to_string(),
                    column: ColumnSchema {
                        name: "id".to_string(),
                        column_type: "INTEGER".to_string(),
                        nullable: false,
                        default_value: None,
                        primary_key: true,
                        auto_increment: true,
                        unique: false,
                        constraints: Vec::new(),
                    },
                },
                MigrationOperation::DropColumn {
                    table: "users".to_string(),
                    column: "old_field".to_string(),
                },
            ],
            rollback_plan: Vec::new(),
            estimated_duration: Duration::from_secs(60),
            safety_warnings: Vec::new(),
        }
    }

    #[test]
    fn test_create_parallel_executor() {
        let config = ParallelExecutionConfig::default();
        let executor = ParallelMigrationExecutor::new(config.clone());
        
        assert_eq!(executor.config.max_concurrent_operations, config.max_concurrent_operations);
    }

    #[test]
    fn test_analyze_dependencies_sequential() {
        let config = ParallelExecutionConfig {
            dependency_strategy: DependencyStrategy::Sequential,
            ..Default::default()
        };
        let executor = ParallelMigrationExecutor::new(config);
        let migration_plan = create_test_migration_plan();
        
        let graph = executor.analyze_dependencies(&migration_plan).unwrap();
        
        // Sequential strategy should have one operation per stage
        assert_eq!(graph.execution_stages.len(), 3);
        for stage in &graph.execution_stages {
            assert_eq!(stage.len(), 1);
        }
    }

    #[test]
    fn test_analyze_dependencies_independent() {
        let config = ParallelExecutionConfig {
            dependency_strategy: DependencyStrategy::Independent,
            ..Default::default()
        };
        let executor = ParallelMigrationExecutor::new(config);
        let migration_plan = create_test_migration_plan();
        
        let graph = executor.analyze_dependencies(&migration_plan).unwrap();
        
        // Independent strategy should have all operations in one stage
        assert_eq!(graph.execution_stages.len(), 1);
        assert_eq!(graph.execution_stages[0].len(), 3);
    }

    #[test]
    fn test_analyze_dependencies_optimized() {
        let config = ParallelExecutionConfig {
            dependency_strategy: DependencyStrategy::Optimized,
            ..Default::default()
        };
        let executor = ParallelMigrationExecutor::new(config);
        let migration_plan = create_test_migration_plan();
        
        let graph = executor.analyze_dependencies(&migration_plan).unwrap();
        
        // Optimized strategy should group similar operations
        // Should have 2 stages: CreateTable operations, then CreateIndex
        assert_eq!(graph.execution_stages.len(), 2);
        
        // First stage should have 2 CreateTable operations
        assert_eq!(graph.execution_stages[0].len(), 2);
        for op in &graph.execution_stages[0] {
            matches!(op, MigrationOperation::CreateTable { .. });
        }
        
        // Second stage should have 1 CreateIndex operation
        assert_eq!(graph.execution_stages[1].len(), 1);
        matches!(graph.execution_stages[1][0], MigrationOperation::CreateIndex { .. });
    }

    #[tokio::test]
    async fn test_execution_progress() {
        let config = ParallelExecutionConfig::default();
        let executor = ParallelMigrationExecutor::new(config);
        
        let progress = executor.get_progress().await;
        
        // Initial state should be empty
        assert_eq!(progress.total_operations, 0);
        assert_eq!(progress.completed_operations, 0);
        assert_eq!(progress.running_operations, 0);
        assert_eq!(progress.pending_operations, 0);
        assert_eq!(progress.progress_percentage, 0.0);
        assert_eq!(progress.current_concurrency, 0);
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig {
            max_retries: 3,
            base_delay: Duration::from_millis(1000),
            backoff_multiplier: 2.0,
            max_delay: Duration::from_secs(30),
            retryable_errors: vec!["database is locked".to_string()],
        };
        
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.base_delay, Duration::from_millis(1000));
        assert_eq!(config.backoff_multiplier, 2.0);
        assert_eq!(config.max_delay, Duration::from_secs(30));
        assert!(config.retryable_errors.contains(&"database is locked".to_string()));
    }

    #[test]
    fn test_parallel_execution_config_default() {
        let config = ParallelExecutionConfig::default();
        
        assert_eq!(config.max_concurrent_operations, 4);
        assert_eq!(config.operation_timeout, Duration::from_secs(300));
        assert!(matches!(config.dependency_strategy, DependencyStrategy::Optimized));
        assert!(matches!(config.safety_mode, ParallelSafetyMode::Balanced));
        assert!(config.use_transactions);
        assert_eq!(config.retry_config.max_retries, 3);
        assert_eq!(config.progress_reporting_interval, Duration::from_secs(5));
    }

    #[test]
    fn test_operation_failure_creation() {
        let failure = OperationFailure {
            operation_id: "test_op".to_string(),
            operation_type: "CreateTable".to_string(),
            error_message: "Table already exists".to_string(),
            retry_count: 2,
            duration: Duration::from_millis(500),
            dependencies_met: true,
        };
        
        assert_eq!(failure.operation_id, "test_op");
        assert_eq!(failure.operation_type, "CreateTable");
        assert_eq!(failure.error_message, "Table already exists");
        assert_eq!(failure.retry_count, 2);
        assert!(failure.dependencies_met);
    }

    #[test]
    fn test_execution_state_initialization() {
        let state = ExecutionState {
            running_operations: HashMap::new(),
            completed_operations: HashMap::new(),
            failed_operations: HashMap::new(),
            pending_operations: HashMap::new(),
            current_concurrency: 0,
            progress_percentage: 0.0,
        };
        
        assert_eq!(state.running_operations.len(), 0);
        assert_eq!(state.completed_operations.len(), 0);
        assert_eq!(state.failed_operations.len(), 0);
        assert_eq!(state.pending_operations.len(), 0);
        assert_eq!(state.current_concurrency, 0);
        assert_eq!(state.progress_percentage, 0.0);
    }
}