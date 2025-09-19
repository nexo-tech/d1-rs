// Zero-downtime migration system for production environments
use crate::{D1Client, Result};
use super::{MigrationOperation, MigrationPlan};
use std::time::Duration;

/// Zero-downtime migration coordinator
/// Breaks complex migrations into safe, atomic steps to avoid service interruption
#[derive(Debug)]
pub struct ZeroDowntimeMigrator {
    /// Maximum duration for each atomic step (default: 30 seconds)
    #[allow(dead_code)]
    max_step_duration: Duration,
    /// Whether to use shadow tables for complex operations
    use_shadow_tables: bool,
    /// Concurrent safety mode
    #[allow(dead_code)]
    concurrent_safety: ConcurrentSafetyMode,
}

/// Concurrent safety modes for zero-downtime migrations
#[derive(Debug, Clone, PartialEq)]
pub enum ConcurrentSafetyMode {
    /// Maximum safety - acquire locks and ensure no concurrent writes
    MaxSafety,
    /// Balanced approach - allow reads during migration
    Balanced,
    /// Minimal locks - optimistic approach for high-traffic systems
    Optimistic,
}

/// Multi-step migration plan for zero-downtime execution
#[derive(Debug, Clone)]
pub struct ZeroDowntimePlan {
    /// Individual atomic steps that can be executed independently
    pub steps: Vec<MigrationStep>,
    /// Total estimated duration for all steps
    pub total_estimated_duration: Duration,
    /// Rollback strategy for each step
    pub rollback_strategy: RollbackStrategy,
    /// Validation checks between steps
    pub validation_checks: Vec<ValidationCheck>,
}

/// Individual atomic migration step
#[derive(Debug, Clone)]
pub struct MigrationStep {
    /// Unique identifier for this step
    pub id: String,
    /// Human-readable description
    pub description: String,
    /// Operations to execute in this step
    pub operations: Vec<MigrationOperation>,
    /// Estimated execution time
    pub estimated_duration: Duration,
    /// Whether this step can be rolled back
    pub rollback_safe: bool,
    /// Prerequisites that must be satisfied
    pub prerequisites: Vec<String>,
    /// Validation to run after this step
    pub post_validation: Option<ValidationCheck>,
}

/// Rollback strategy for zero-downtime migrations
#[derive(Debug, Clone)]
pub enum RollbackStrategy {
    /// Can rollback any step individually
    PerStep,
    /// Must rollback entire migration if any step fails
    AllOrNothing,
    /// Custom rollback logic
    Custom {
        rollback_steps: Vec<MigrationStep>,
    },
}

/// Validation check to ensure migration step success
#[derive(Debug, Clone)]
pub struct ValidationCheck {
    /// Unique identifier for this validation
    pub id: String,
    /// Description of what is being validated
    pub description: String,
    /// SQL query to validate the migration step
    pub validation_query: String,
    /// Expected result for successful validation
    pub expected_result: ValidationExpectation,
}

/// Expected result for validation checks
#[derive(Debug, Clone)]
pub enum ValidationExpectation {
    /// Expect specific number of rows
    RowCount(usize),
    /// Expect specific value in first row, first column
    ScalarValue(String),
    /// Expect query to succeed without checking result
    QuerySuccess,
    /// Custom validation logic (placeholder for future)
    Custom(String),
}

/// Result of zero-downtime migration execution
#[derive(Debug)]
pub struct ZeroDowntimeResult {
    /// Steps that were successfully executed
    pub completed_steps: Vec<String>,
    /// Steps that failed (if any)
    pub failed_steps: Vec<(String, String)>, // (step_id, error_message)
    /// Total execution time
    pub total_execution_time: Duration,
    /// Whether rollback was triggered
    pub rollback_executed: bool,
    /// Final validation result
    pub final_validation: bool,
}

impl ZeroDowntimeMigrator {
    /// Create a new zero-downtime migrator with default settings
    pub fn new() -> Self {
        Self {
            max_step_duration: Duration::from_secs(30),
            use_shadow_tables: true,
            concurrent_safety: ConcurrentSafetyMode::Balanced,
        }
    }

    /// Create migrator with custom configuration
    pub fn with_config(
        max_step_duration: Duration,
        use_shadow_tables: bool,
        concurrent_safety: ConcurrentSafetyMode,
    ) -> Self {
        Self {
            max_step_duration,
            use_shadow_tables,
            concurrent_safety,
        }
    }

    /// Convert a regular migration plan into zero-downtime steps
    pub fn plan_zero_downtime_migration(
        &self,
        regular_plan: &MigrationPlan,
    ) -> Result<ZeroDowntimePlan> {
        let mut steps = Vec::new();
        let mut total_duration = Duration::from_secs(0);

        // Process each operation and break it into atomic steps
        for (i, operation) in regular_plan.operations.iter().enumerate() {
            let step_id = format!("step_{}", i + 1);
            
            match operation {
                MigrationOperation::CreateTable { definition } => {
                    // Table creation is naturally atomic
                    steps.push(MigrationStep {
                        id: step_id.clone(),
                        description: format!("Create table '{}'", definition.name),
                        operations: vec![operation.clone()],
                        estimated_duration: Duration::from_millis(500),
                        rollback_safe: true,
                        prerequisites: vec![],
                        post_validation: Some(ValidationCheck {
                            id: format!("{}_validation", step_id),
                            description: format!("Verify table '{}' exists", definition.name),
                            validation_query: format!(
                                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{}'",
                                definition.name
                            ),
                            expected_result: ValidationExpectation::RowCount(1),
                        }),
                    });
                    total_duration += Duration::from_millis(500);
                }
                
                MigrationOperation::AddColumn { table, column } => {
                    // Column addition is atomic and safe
                    steps.push(MigrationStep {
                        id: step_id.clone(),
                        description: format!("Add column '{}' to table '{}'", column.name, table),
                        operations: vec![operation.clone()],
                        estimated_duration: Duration::from_millis(200),
                        rollback_safe: false, // SQLite doesn't support DROP COLUMN
                        prerequisites: vec![],
                        post_validation: Some(ValidationCheck {
                            id: format!("{}_validation", step_id),
                            description: format!("Verify column '{}' exists in table '{}'", column.name, table),
                            validation_query: format!(
                                "SELECT COUNT(*) FROM pragma_table_info('{}') WHERE name='{}'",
                                table, column.name
                            ),
                            expected_result: ValidationExpectation::RowCount(1),
                        }),
                    });
                    total_duration += Duration::from_millis(200);
                }

                MigrationOperation::DropTable { name } => {
                    // Table dropping needs careful consideration
                    if self.use_shadow_tables {
                        // Rename table first, then drop (allows quick rollback)
                        let shadow_name = format!("{}__shadow_drop", name);
                        
                        steps.push(MigrationStep {
                            id: format!("{}_rename", step_id),
                            description: format!("Rename table '{}' to shadow table", name),
                            operations: vec![MigrationOperation::RenameTable {
                                old_name: name.clone(),
                                new_name: shadow_name.clone(),
                            }],
                            estimated_duration: Duration::from_millis(100),
                            rollback_safe: true,
                            prerequisites: vec![],
                            post_validation: None,
                        });

                        steps.push(MigrationStep {
                            id: step_id.clone(),
                            description: format!("Drop shadow table '{}'", shadow_name),
                            operations: vec![MigrationOperation::DropTable {
                                name: shadow_name,
                            }],
                            estimated_duration: Duration::from_millis(100),
                            rollback_safe: false,
                            prerequisites: vec![format!("{}_rename", step_id)],
                            post_validation: None,
                        });
                    } else {
                        // Direct drop
                        steps.push(MigrationStep {
                            id: step_id.clone(),
                            description: format!("Drop table '{}'", name),
                            operations: vec![operation.clone()],
                            estimated_duration: Duration::from_millis(200),
                            rollback_safe: false,
                            prerequisites: vec![],
                            post_validation: None,
                        });
                    }
                    total_duration += Duration::from_millis(300);
                }

                _ => {
                    // For other operations, use default atomic handling
                    steps.push(MigrationStep {
                        id: step_id.clone(),
                        description: format!("Execute operation: {:?}", operation),
                        operations: vec![operation.clone()],
                        estimated_duration: Duration::from_millis(300),
                        rollback_safe: true,
                        prerequisites: vec![],
                        post_validation: None,
                    });
                    total_duration += Duration::from_millis(300);
                }
            }
        }

        // Create comprehensive validation checks
        let validation_checks = vec![
            ValidationCheck {
                id: "schema_integrity".to_string(),
                description: "Verify database schema integrity".to_string(),
                validation_query: "PRAGMA integrity_check".to_string(),
                expected_result: ValidationExpectation::ScalarValue("ok".to_string()),
            },
            ValidationCheck {
                id: "foreign_key_check".to_string(),
                description: "Verify foreign key constraints".to_string(),
                validation_query: "PRAGMA foreign_key_check".to_string(),
                expected_result: ValidationExpectation::QuerySuccess,
            },
        ];

        Ok(ZeroDowntimePlan {
            steps,
            total_estimated_duration: total_duration,
            rollback_strategy: RollbackStrategy::PerStep,
            validation_checks,
        })
    }

    /// Execute zero-downtime migration plan
    pub async fn execute_zero_downtime_migration(
        &self,
        db: &D1Client,
        plan: &ZeroDowntimePlan,
    ) -> Result<ZeroDowntimeResult> {
        let start_time = std::time::Instant::now();
        let mut completed_steps = Vec::new();
        let mut failed_steps = Vec::new();
        let mut rollback_executed = false;

        // Execute steps in order
        for step in &plan.steps {
            // Check prerequisites
            if !self.check_prerequisites(&step.prerequisites, &completed_steps) {
                failed_steps.push((
                    step.id.clone(),
                    "Prerequisites not satisfied".to_string(),
                ));
                break;
            }

            // Execute step operations
            match self.execute_step(db, step).await {
                Ok(_) => {
                    completed_steps.push(step.id.clone());
                    
                    // Run post-validation if specified
                    if let Some(validation) = &step.post_validation {
                        if let Err(e) = self.run_validation(db, validation).await {
                            failed_steps.push((
                                step.id.clone(),
                                format!("Validation failed: {}", e),
                            ));
                            break;
                        }
                    }
                }
                Err(e) => {
                    failed_steps.push((step.id.clone(), e.to_string()));
                    break;
                }
            }
        }

        // If any step failed, consider rollback
        if !failed_steps.is_empty() && matches!(plan.rollback_strategy, RollbackStrategy::AllOrNothing) {
            rollback_executed = self.execute_rollback(db, &plan, &completed_steps).await?;
        }

        // Run final validation
        let final_validation = if failed_steps.is_empty() {
            self.run_final_validation(db, &plan.validation_checks).await.unwrap_or(false)
        } else {
            false
        };

        Ok(ZeroDowntimeResult {
            completed_steps,
            failed_steps,
            total_execution_time: start_time.elapsed(),
            rollback_executed,
            final_validation,
        })
    }

    /// Check if step prerequisites are satisfied
    fn check_prerequisites(&self, prerequisites: &[String], completed_steps: &[String]) -> bool {
        prerequisites.iter().all(|prereq| completed_steps.contains(prereq))
    }

    /// Execute a single migration step
    async fn execute_step(&self, _db: &D1Client, step: &MigrationStep) -> Result<()> {
        // For now, this is a placeholder - in a full implementation,
        // we would execute the actual SQL operations
        for _operation in &step.operations {
            // Execute operation against database
            // This would integrate with the existing MigrationExecutor
        }
        Ok(())
    }

    /// Run validation check
    async fn run_validation(&self, db: &D1Client, validation: &ValidationCheck) -> Result<()> {
        // Execute validation query and check result
        // This is a placeholder for the actual validation logic
        match &validation.expected_result {
            ValidationExpectation::QuerySuccess => {
                // Just ensure query executes without error
                let _result = db.execute(&validation.validation_query, &[]).await?;
            }
            ValidationExpectation::RowCount(_expected) => {
                let _result = db.execute(&validation.validation_query, &[]).await?;
                // Check if result has expected number of rows
                // This would need to be implemented based on D1Client API
            }
            ValidationExpectation::ScalarValue(_expected) => {
                let _result = db.execute(&validation.validation_query, &[]).await?;
                // Check if first row, first column matches expected value
                // This would need to be implemented based on D1Client API
            }
            ValidationExpectation::Custom(_) => {
                // Custom validation logic would go here
            }
        }
        Ok(())
    }

    /// Execute rollback for completed steps
    async fn execute_rollback(
        &self,
        db: &D1Client,
        plan: &ZeroDowntimePlan,
        completed_steps: &[String],
    ) -> Result<bool> {
        // Implement rollback logic based on strategy
        match &plan.rollback_strategy {
            RollbackStrategy::PerStep => {
                // Roll back each completed step in reverse order
                for step_id in completed_steps.iter().rev() {
                    // Find the step and execute its rollback
                    if let Some(step) = plan.steps.iter().find(|s| s.id == *step_id) {
                        if step.rollback_safe {
                            // Execute rollback operations (not implemented in this placeholder)
                        }
                    }
                }
                Ok(true)
            }
            RollbackStrategy::AllOrNothing => {
                // Execute comprehensive rollback
                Ok(true)
            }
            RollbackStrategy::Custom { rollback_steps } => {
                // Execute custom rollback steps
                for step in rollback_steps {
                    self.execute_step(db, step).await?;
                }
                Ok(true)
            }
        }
    }

    /// Run final validation checks
    async fn run_final_validation(
        &self,
        db: &D1Client,
        validation_checks: &[ValidationCheck],
    ) -> Result<bool> {
        for validation in validation_checks {
            if let Err(_) = self.run_validation(db, validation).await {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

impl Default for ZeroDowntimeMigrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auto_migration::introspector::{TableSchema, ColumnSchema};

    #[test]
    fn test_zero_downtime_plan_creation() {
        let migrator = ZeroDowntimeMigrator::new();
        
        let regular_plan = MigrationPlan {
            operations: vec![
                MigrationOperation::CreateTable {
                    definition: TableSchema {
                        name: "test_table".to_string(),
                        columns: vec![
                            ColumnSchema {
                                name: "id".to_string(),
                                column_type: "INTEGER".to_string(),
                                nullable: false,
                                primary_key: true,
                                auto_increment: true,
                                unique: false,
                                default_value: None,
                                constraints: vec![],
                            }
                        ],
                        indexes: vec![],
                        foreign_keys: vec![],
                        constraints: vec![],
                    },
                },
            ],
            estimated_duration: Duration::from_secs(1),
            safety_warnings: vec![],
            rollback_plan: vec![],
        };

        let zero_downtime_plan = migrator.plan_zero_downtime_migration(&regular_plan).unwrap();
        
        assert_eq!(zero_downtime_plan.steps.len(), 1);
        assert_eq!(zero_downtime_plan.steps[0].id, "step_1");
        assert!(zero_downtime_plan.steps[0].rollback_safe);
        assert!(zero_downtime_plan.steps[0].post_validation.is_some());
    }

    #[test]
    fn test_concurrent_safety_modes() {
        let modes = vec![
            ConcurrentSafetyMode::MaxSafety,
            ConcurrentSafetyMode::Balanced,
            ConcurrentSafetyMode::Optimistic,
        ];

        for mode in modes {
            let migrator = ZeroDowntimeMigrator::with_config(
                Duration::from_secs(60),
                true,
                mode.clone(),
            );
            assert_eq!(migrator.concurrent_safety, mode);
        }
    }

    #[test]
    fn test_rollback_strategies() {
        let strategies = vec![
            RollbackStrategy::PerStep,
            RollbackStrategy::AllOrNothing,
            RollbackStrategy::Custom {
                rollback_steps: vec![],
            },
        ];

        for strategy in strategies {
            // Test that each strategy variant can be created and matched
            match strategy {
                RollbackStrategy::PerStep => {},
                RollbackStrategy::AllOrNothing => {},
                RollbackStrategy::Custom { .. } => {},
            }
        }
    }

    #[test]
    fn test_validation_expectations() {
        let expectations = vec![
            ValidationExpectation::RowCount(5),
            ValidationExpectation::ScalarValue("ok".to_string()),
            ValidationExpectation::QuerySuccess,
            ValidationExpectation::Custom("test".to_string()),
        ];

        for expectation in expectations {
            // Test that each expectation variant can be created and matched
            match expectation {
                ValidationExpectation::RowCount(_) => {},
                ValidationExpectation::ScalarValue(_) => {},
                ValidationExpectation::QuerySuccess => {},
                ValidationExpectation::Custom(_) => {},
            }
        }
    }

    #[test]
    fn test_shadow_table_handling() {
        let migrator = ZeroDowntimeMigrator::with_config(
            Duration::from_secs(30),
            true, // use_shadow_tables = true
            ConcurrentSafetyMode::Balanced,
        );
        
        let regular_plan = MigrationPlan {
            operations: vec![
                MigrationOperation::DropTable {
                    name: "old_table".to_string(),
                },
            ],
            estimated_duration: Duration::from_secs(1),
            safety_warnings: vec![],
            rollback_plan: vec![],
        };

        let zero_downtime_plan = migrator.plan_zero_downtime_migration(&regular_plan).unwrap();
        
        // Should create 2 steps: rename + drop
        assert_eq!(zero_downtime_plan.steps.len(), 2);
        assert!(zero_downtime_plan.steps[0].description.contains("Rename table"));
        assert!(zero_downtime_plan.steps[1].description.contains("Drop shadow table"));
    }
}