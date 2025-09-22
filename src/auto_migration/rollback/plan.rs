//! Rollback plan structures and execution framework
//!
//! This module provides comprehensive rollback plan management, including execution validation,
//! metadata tracking, and serialization support. The rollback plan system ensures safe and
//! reliable migration reversals with enterprise-grade safety guarantees.

use super::types::{RollbackOperation, RollbackRiskLevel, DataPreservationRequirement, RollbackConfig};
use std::time::{Duration, SystemTime};
use std::fmt;

/// Comprehensive rollback execution plan containing all operations, validations, and metadata
/// 
/// A rollback plan represents a complete strategy for reversing a migration, including
/// the sequence of operations to execute, safety validations to perform, and metadata
/// for tracking and auditing purposes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct RollbackPlan {
    /// Unique identifier for this rollback plan
    pub plan_id: String,
    
    /// Ordered sequence of rollback operations to execute
    pub operations: Vec<RollbackOperation>,
    
    /// Data preservation requirements for safe rollback execution
    pub data_preservation_requirements: Vec<DataPreservationRequirement>,
    
    /// Pre-execution safety checks that must pass before rollback
    pub pre_execution_checks: Vec<PreExecutionCheck>,
    
    /// Post-execution validations to verify rollback success
    pub post_execution_validations: Vec<PostExecutionValidation>,
    
    /// Estimated total duration for rollback execution
    pub estimated_duration: Duration,
    
    /// ID of the original migration being rolled back
    pub original_migration_id: Option<String>,
    
    /// Timestamp when this plan was created
    pub created_at: SystemTime,
    
    /// Overall risk assessment for this rollback plan
    pub risk_assessment: RollbackRiskLevel,
    
    /// Configuration used to generate this plan
    pub config: RollbackConfig,
    
    /// Human-readable description of the rollback plan
    pub description: String,
    
    /// Version of the rollback system that created this plan
    pub rollback_system_version: String,
    
    /// Additional metadata for tracking and auditing
    pub metadata: RollbackPlanMetadata,
}

/// Metadata associated with a rollback plan for tracking and auditing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct RollbackPlanMetadata {
    /// Environment where this plan was created (dev, staging, prod)
    pub environment: String,
    
    /// User or system that created this plan
    pub created_by: Option<String>,
    
    /// Tags for categorization and filtering
    pub tags: Vec<String>,
    
    /// Number of database tables affected by this rollback
    pub affected_tables: usize,
    
    /// Estimated number of rows that will be affected
    pub estimated_affected_rows: Option<usize>,
    
    /// Whether this rollback requires manual intervention
    pub requires_manual_intervention: bool,
    
    /// Priority level for execution scheduling
    pub execution_priority: ExecutionPriority,
}

/// Priority levels for rollback plan execution scheduling
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExecutionPriority {
    /// Low priority - can be scheduled during maintenance windows
    Low = 1,
    
    /// Normal priority - standard rollback execution
    Normal = 2,
    
    /// High priority - should be executed soon
    High = 3,
    
    /// Critical priority - requires immediate execution
    Critical = 4,
}

/// Pre-execution safety check that must pass before rollback execution
///
/// Pre-execution checks validate the current database state and ensure
/// that conditions are suitable for safe rollback execution.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PreExecutionCheck {
    /// Unique identifier for this check
    pub check_id: String,
    
    /// Type of check being performed
    pub check_type: CheckType,
    
    /// Human-readable description of what this check validates
    pub description: String,
    
    /// Optional SQL query to execute for validation
    pub sql_query: Option<String>,
    
    /// Expected result description for validation
    pub expected_result: String,
    
    /// Whether this check is required or optional
    pub is_required: bool,
    
    /// Timeout for check execution
    pub timeout: Duration,
    
    /// What to do if this check fails
    pub failure_action: CheckFailureAction,
}

/// Types of pre-execution checks that can be performed
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum CheckType {
    /// Verify table exists and has expected structure
    TableExists,
    
    /// Check that required columns exist with correct types
    ColumnExists,
    
    /// Validate data integrity constraints
    DataIntegrity,
    
    /// Check foreign key constraint status
    ForeignKeyConstraints,
    
    /// Verify index existence and structure
    IndexExists,
    
    /// Check database lock status
    LockStatus,
    
    /// Validate backup availability
    BackupAvailability,
    
    /// Check disk space requirements
    DiskSpace,
    
    /// Verify user permissions
    Permissions,
    
    /// Custom validation check
    Custom,
}

/// Actions to take when a pre-execution check fails
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum CheckFailureAction {
    /// Stop rollback execution immediately
    Stop,
    
    /// Log warning but continue execution
    Warn,
    
    /// Attempt automatic remediation
    AutoRemediate,
    
    /// Request manual intervention
    ManualIntervention,
}

/// Post-execution validation to verify rollback success
///
/// Post-execution validations ensure that the rollback operation completed
/// successfully and that the database is in the expected state.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PostExecutionValidation {
    /// Unique identifier for this validation
    pub validation_id: String,
    
    /// Type of validation being performed
    pub validation_type: ValidationType,
    
    /// Human-readable description of what this validation checks
    pub description: String,
    
    /// Optional SQL query to execute for validation
    pub sql_query: Option<String>,
    
    /// Expected result description for validation success
    pub expected_result: String,
    
    /// Whether this validation is required for rollback success
    pub is_required: bool,
    
    /// Timeout for validation execution
    pub timeout: Duration,
    
    /// What to do if this validation fails
    pub failure_action: ValidationFailureAction,
}

/// Types of post-execution validations that can be performed
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum ValidationType {
    /// Verify schema matches expected state
    SchemaValidation,
    
    /// Check data integrity after rollback
    DataIntegrity,
    
    /// Validate constraint enforcement
    ConstraintValidation,
    
    /// Check index functionality
    IndexValidation,
    
    /// Verify foreign key relationships
    ForeignKeyValidation,
    
    /// Validate data restoration completeness
    DataRestoration,
    
    /// Check application connectivity
    ApplicationConnectivity,
    
    /// Performance validation
    PerformanceCheck,
    
    /// Custom validation logic
    Custom,
}

/// Actions to take when a post-execution validation fails
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum ValidationFailureAction {
    /// Mark rollback as failed
    MarkFailed,
    
    /// Log warning but mark as successful
    WarnButContinue,
    
    /// Attempt automatic repair
    AutoRepair,
    
    /// Request manual verification
    ManualVerification,
    
    /// Initiate emergency rollback of the rollback
    EmergencyRevert,
}

/// Result of plan validation indicating readiness for execution
#[derive(Debug, Clone, PartialEq)]
pub struct PlanValidationResult {
    /// Whether the plan is valid and ready for execution
    pub is_valid: bool,
    
    /// Overall risk level after validation
    pub validated_risk_level: RollbackRiskLevel,
    
    /// Issues found during validation
    pub validation_issues: Vec<PlanValidationIssue>,
    
    /// Warnings that don't prevent execution
    pub warnings: Vec<String>,
    
    /// Estimated execution time after validation
    pub estimated_execution_time: Duration,
    
    /// Whether manual approval is required
    pub requires_manual_approval: bool,
}

/// Issue found during plan validation
#[derive(Debug, Clone, PartialEq)]
pub struct PlanValidationIssue {
    /// Severity level of this issue
    pub severity: IssueSeverity,
    
    /// Component that has the issue
    pub component: String,
    
    /// Description of the issue
    pub description: String,
    
    /// Suggested resolution
    pub suggested_resolution: Option<String>,
    
    /// Whether this issue blocks execution
    pub is_blocking: bool,
}

/// Severity levels for validation issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IssueSeverity {
    /// Informational - no action needed
    Info,
    
    /// Warning - should be addressed but not blocking
    Warning,
    
    /// Error - prevents execution
    Error,
    
    /// Critical - serious safety concern
    Critical,
}

impl RollbackPlan {
    /// Create a new rollback plan with comprehensive initialization
    pub fn new(
        plan_id: String,
        operations: Vec<RollbackOperation>,
        original_migration_id: Option<String>,
        config: RollbackConfig,
        description: String,
    ) -> Self {
        let risk_assessment = Self::calculate_overall_risk(&operations);
        let estimated_duration = Self::estimate_execution_duration(&operations, &config);
        let affected_tables = Self::count_affected_tables(&operations);
        
        Self {
            plan_id,
            operations,
            data_preservation_requirements: Vec::new(),
            pre_execution_checks: Vec::new(),
            post_execution_validations: Vec::new(),
            estimated_duration,
            original_migration_id,
            created_at: SystemTime::now(),
            risk_assessment,
            config,
            description,
            rollback_system_version: env!("CARGO_PKG_VERSION").to_string(),
            metadata: RollbackPlanMetadata {
                environment: "default".to_string(),
                created_by: None,
                tags: Vec::new(),
                affected_tables,
                estimated_affected_rows: None,
                requires_manual_intervention: risk_assessment >= RollbackRiskLevel::High,
                execution_priority: ExecutionPriority::Normal,
            },
        }
    }
    
    /// Add a data preservation requirement to the plan
    pub fn add_data_preservation_requirement(&mut self, requirement: DataPreservationRequirement) {
        self.data_preservation_requirements.push(requirement);
        self.update_risk_assessment();
    }
    
    /// Add a pre-execution check to the plan
    pub fn add_pre_execution_check(&mut self, check: PreExecutionCheck) {
        self.pre_execution_checks.push(check);
    }
    
    /// Add a post-execution validation to the plan
    pub fn add_post_execution_validation(&mut self, validation: PostExecutionValidation) {
        self.post_execution_validations.push(validation);
    }
    
    /// Validate the entire plan for execution readiness
    pub fn validate(&self) -> PlanValidationResult {
        let mut validation_issues = Vec::new();
        let mut warnings = Vec::new();
        
        // Validate operations
        self.validate_operations(&mut validation_issues, &mut warnings);
        
        // Validate data preservation requirements
        self.validate_data_preservation(&mut validation_issues, &mut warnings);
        
        // Validate checks and validations
        self.validate_checks_and_validations(&mut validation_issues, &mut warnings);
        
        // Check for blocking issues
        let is_valid = !validation_issues.iter().any(|issue| issue.is_blocking);
        
        // Calculate validated risk level
        let validated_risk_level = if validation_issues.iter().any(|issue| issue.severity == IssueSeverity::Critical) {
            RollbackRiskLevel::Critical
        } else if validation_issues.iter().any(|issue| issue.severity == IssueSeverity::Error) {
            RollbackRiskLevel::High
        } else {
            self.risk_assessment
        };
        
        PlanValidationResult {
            is_valid,
            validated_risk_level,
            validation_issues,
            warnings,
            estimated_execution_time: self.estimated_duration,
            requires_manual_approval: validated_risk_level >= RollbackRiskLevel::High,
        }
    }
    
    /// Get the total number of operations in this plan
    pub fn operation_count(&self) -> usize {
        self.operations.len()
    }
    
    /// Check if this plan has any destructive operations
    pub fn has_destructive_operations(&self) -> bool {
        self.operations.iter().any(|op| op.is_destructive())
    }
    
    /// Get all tables affected by this rollback plan
    pub fn affected_tables(&self) -> Vec<String> {
        self.operations.iter()
            .map(|op| op.affected_table().to_string())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }
    
    /// Serialize the plan to JSON for persistence
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
    
    /// Deserialize a plan from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
    
    /// Calculate the overall risk level for a set of operations
    fn calculate_overall_risk(operations: &[RollbackOperation]) -> RollbackRiskLevel {
        if operations.is_empty() {
            return RollbackRiskLevel::Low;
        }
        
        let risks: Vec<RollbackRiskLevel> = operations.iter()
            .map(|op| op.assess_risk_level())
            .collect();
        
        RollbackRiskLevel::max_level(risks)
    }
    
    /// Estimate the total execution duration for the operations
    fn estimate_execution_duration(operations: &[RollbackOperation], config: &RollbackConfig) -> Duration {
        let base_duration = operations.len() as u64 * 30; // 30 seconds per operation baseline
        let config_multiplier = if config.use_transactions { 1.2 } else { 1.0 };
        let safety_multiplier = if config.create_backup_tables { 1.5 } else { 1.0 };
        
        Duration::from_secs((base_duration as f64 * config_multiplier * safety_multiplier) as u64)
    }
    
    /// Count the number of unique tables affected by operations
    fn count_affected_tables(operations: &[RollbackOperation]) -> usize {
        operations.iter()
            .map(|op| op.affected_table())
            .collect::<std::collections::HashSet<_>>()
            .len()
    }
    
    /// Update the risk assessment based on current plan state
    fn update_risk_assessment(&mut self) {
        self.risk_assessment = Self::calculate_overall_risk(&self.operations);
        
        // Update metadata based on risk assessment
        self.metadata.requires_manual_intervention = self.risk_assessment >= RollbackRiskLevel::High;
        self.metadata.execution_priority = match self.risk_assessment {
            RollbackRiskLevel::Low => ExecutionPriority::Low,
            RollbackRiskLevel::Medium => ExecutionPriority::Normal,
            RollbackRiskLevel::High => ExecutionPriority::High,
            RollbackRiskLevel::Critical => ExecutionPriority::Critical,
        };
    }
    
    /// Validate operations for consistency and safety
    fn validate_operations(&self, issues: &mut Vec<PlanValidationIssue>, warnings: &mut Vec<String>) {
        if self.operations.is_empty() {
            issues.push(PlanValidationIssue {
                severity: IssueSeverity::Error,
                component: "operations".to_string(),
                description: "Rollback plan contains no operations".to_string(),
                suggested_resolution: Some("Add at least one rollback operation".to_string()),
                is_blocking: true,
            });
        }
        
        // Check for operation dependencies
        let table_operations = self.group_operations_by_table();
        for (table, ops) in table_operations {
            if ops.len() > 3 {
                warnings.push(format!("Table '{}' has {} operations - consider consolidating", table, ops.len()));
            }
        }
    }
    
    /// Validate data preservation requirements
    fn validate_data_preservation(&self, issues: &mut Vec<PlanValidationIssue>, _warnings: &mut Vec<String>) {
        let destructive_tables: std::collections::HashSet<String> = self.operations.iter()
            .filter(|op| op.is_destructive())
            .map(|op| op.affected_table().to_string())
            .collect();
        
        let preserved_tables: std::collections::HashSet<String> = self.data_preservation_requirements.iter()
            .map(|req| req.table.clone())
            .collect();
        
        for table in &destructive_tables {
            if !preserved_tables.contains(table) {
                issues.push(PlanValidationIssue {
                    severity: IssueSeverity::Warning,
                    component: "data_preservation".to_string(),
                    description: format!("Destructive operation on table '{}' without data preservation", table),
                    suggested_resolution: Some("Add data preservation requirement for this table".to_string()),
                    is_blocking: false,
                });
            }
        }
    }
    
    /// Validate checks and validations for completeness
    fn validate_checks_and_validations(&self, issues: &mut Vec<PlanValidationIssue>, warnings: &mut Vec<String>) {
        if self.risk_assessment >= RollbackRiskLevel::High && self.pre_execution_checks.is_empty() {
            issues.push(PlanValidationIssue {
                severity: IssueSeverity::Warning,
                component: "pre_execution_checks".to_string(),
                description: "High-risk rollback plan without pre-execution checks".to_string(),
                suggested_resolution: Some("Add pre-execution safety checks".to_string()),
                is_blocking: false,
            });
        }
        
        if self.has_destructive_operations() && self.post_execution_validations.is_empty() {
            warnings.push("Destructive operations without post-execution validation".to_string());
        }
    }
    
    /// Group operations by affected table for analysis
    fn group_operations_by_table(&self) -> std::collections::HashMap<String, Vec<&RollbackOperation>> {
        let mut table_operations: std::collections::HashMap<String, Vec<&RollbackOperation>> = std::collections::HashMap::new();
        
        for operation in &self.operations {
            table_operations.entry(operation.affected_table().to_string())
                .or_default()
                .push(operation);
        }
        
        table_operations
    }
}

impl PreExecutionCheck {
    /// Create a new pre-execution check with required parameters
    pub fn new(
        check_id: String,
        check_type: CheckType,
        description: String,
        is_required: bool,
    ) -> Self {
        Self {
            check_id,
            check_type,
            description,
            sql_query: None,
            expected_result: "Check passed".to_string(),
            is_required,
            timeout: Duration::from_secs(30),
            failure_action: if is_required { CheckFailureAction::Stop } else { CheckFailureAction::Warn },
        }
    }
    
    /// Set SQL query for this check
    pub fn with_sql_query(mut self, query: String, expected_result: String) -> Self {
        self.sql_query = Some(query);
        self.expected_result = expected_result;
        self
    }
    
    /// Set timeout for this check
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
    
    /// Set failure action for this check
    pub fn with_failure_action(mut self, action: CheckFailureAction) -> Self {
        self.failure_action = action;
        self
    }
}

impl PostExecutionValidation {
    /// Create a new post-execution validation with required parameters
    pub fn new(
        validation_id: String,
        validation_type: ValidationType,
        description: String,
        is_required: bool,
    ) -> Self {
        Self {
            validation_id,
            validation_type,
            description,
            sql_query: None,
            expected_result: "Validation passed".to_string(),
            is_required,
            timeout: Duration::from_secs(60),
            failure_action: if is_required { ValidationFailureAction::MarkFailed } else { ValidationFailureAction::WarnButContinue },
        }
    }
    
    /// Set SQL query for this validation
    pub fn with_sql_query(mut self, query: String, expected_result: String) -> Self {
        self.sql_query = Some(query);
        self.expected_result = expected_result;
        self
    }
    
    /// Set timeout for this validation
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
    
    /// Set failure action for this validation
    pub fn with_failure_action(mut self, action: ValidationFailureAction) -> Self {
        self.failure_action = action;
        self
    }
}

// Display implementations for human-readable output
impl fmt::Display for RollbackPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RollbackPlan {} ({} operations, {} risk)", 
               self.plan_id, self.operations.len(), self.risk_assessment)
    }
}

impl fmt::Display for ExecutionPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutionPriority::Low => write!(f, "Low"),
            ExecutionPriority::Normal => write!(f, "Normal"),
            ExecutionPriority::High => write!(f, "High"),
            ExecutionPriority::Critical => write!(f, "Critical"),
        }
    }
}

impl fmt::Display for CheckType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckType::TableExists => write!(f, "Table Exists"),
            CheckType::ColumnExists => write!(f, "Column Exists"),
            CheckType::DataIntegrity => write!(f, "Data Integrity"),
            CheckType::ForeignKeyConstraints => write!(f, "Foreign Key Constraints"),
            CheckType::IndexExists => write!(f, "Index Exists"),
            CheckType::LockStatus => write!(f, "Lock Status"),
            CheckType::BackupAvailability => write!(f, "Backup Availability"),
            CheckType::DiskSpace => write!(f, "Disk Space"),
            CheckType::Permissions => write!(f, "Permissions"),
            CheckType::Custom => write!(f, "Custom"),
        }
    }
}

impl fmt::Display for ValidationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationType::SchemaValidation => write!(f, "Schema Validation"),
            ValidationType::DataIntegrity => write!(f, "Data Integrity"),
            ValidationType::ConstraintValidation => write!(f, "Constraint Validation"),
            ValidationType::IndexValidation => write!(f, "Index Validation"),
            ValidationType::ForeignKeyValidation => write!(f, "Foreign Key Validation"),
            ValidationType::DataRestoration => write!(f, "Data Restoration"),
            ValidationType::ApplicationConnectivity => write!(f, "Application Connectivity"),
            ValidationType::PerformanceCheck => write!(f, "Performance Check"),
            ValidationType::Custom => write!(f, "Custom"),
        }
    }
}

impl fmt::Display for IssueSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IssueSeverity::Info => write!(f, "Info"),
            IssueSeverity::Warning => write!(f, "Warning"),
            IssueSeverity::Error => write!(f, "Error"),
            IssueSeverity::Critical => write!(f, "Critical"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auto_migration::rollback::types::*;
    use crate::auto_migration::ColumnSchema;
    
    #[test]
    fn test_rollback_plan_creation() {
        let operations = vec![
            RollbackOperation::DropTable {
                name: "temp_table".to_string(),
                preserve_data: true,
                backup_table_name: Some("temp_table_backup".to_string()),
            }
        ];
        
        let config = RollbackConfig::default();
        let plan = RollbackPlan::new(
            "test_plan_001".to_string(),
            operations,
            Some("migration_123".to_string()),
            config,
            "Test rollback plan".to_string(),
        );
        
        assert_eq!(plan.plan_id, "test_plan_001");
        assert_eq!(plan.operations.len(), 1);
        assert_eq!(plan.original_migration_id, Some("migration_123".to_string()));
        assert_eq!(plan.description, "Test rollback plan");
        assert!(plan.estimated_duration > Duration::from_secs(0));
    }
    
    #[test]
    fn test_plan_validation_empty_operations() {
        let plan = RollbackPlan::new(
            "empty_plan".to_string(),
            vec![],
            None,
            RollbackConfig::default(),
            "Empty plan".to_string(),
        );
        
        let validation_result = plan.validate();
        assert!(!validation_result.is_valid);
        assert!(!validation_result.validation_issues.is_empty());
        assert!(validation_result.validation_issues.iter().any(|issue| issue.is_blocking));
    }
    
    #[test]
    fn test_pre_execution_check_creation() {
        let check = PreExecutionCheck::new(
            "check_001".to_string(),
            CheckType::TableExists,
            "Verify table exists before rollback".to_string(),
            true,
        )
        .with_sql_query(
            "SELECT COUNT(*) FROM sqlite_master WHERE name = 'users'".to_string(),
            "1".to_string(),
        )
        .with_timeout(Duration::from_secs(10));
        
        assert_eq!(check.check_id, "check_001");
        assert_eq!(check.check_type, CheckType::TableExists);
        assert!(check.is_required);
        assert!(check.sql_query.is_some());
        assert_eq!(check.timeout, Duration::from_secs(10));
    }
    
    #[test]
    fn test_post_execution_validation_creation() {
        let validation = PostExecutionValidation::new(
            "validation_001".to_string(),
            ValidationType::SchemaValidation,
            "Verify schema after rollback".to_string(),
            true,
        )
        .with_sql_query(
            "PRAGMA table_info(users)".to_string(),
            "Expected schema structure".to_string(),
        )
        .with_failure_action(ValidationFailureAction::MarkFailed);
        
        assert_eq!(validation.validation_id, "validation_001");
        assert_eq!(validation.validation_type, ValidationType::SchemaValidation);
        assert!(validation.is_required);
        assert_eq!(validation.failure_action, ValidationFailureAction::MarkFailed);
    }
    
    #[test]
    fn test_plan_serialization() {
        let operations = vec![
            RollbackOperation::DropColumn {
                table: "users".to_string(),
                column: "temp_column".to_string(),
                preserve_data: false,
            }
        ];
        
        let plan = RollbackPlan::new(
            "serialization_test".to_string(),
            operations,
            None,
            RollbackConfig::default(),
            "Serialization test plan".to_string(),
        );
        
        // Test serialization
        let json = plan.to_json().expect("Serialization should succeed");
        assert!(!json.is_empty());
        assert!(json.contains("serialization_test"));
        
        // Test deserialization
        let deserialized_plan = RollbackPlan::from_json(&json).expect("Deserialization should succeed");
        assert_eq!(deserialized_plan.plan_id, plan.plan_id);
        assert_eq!(deserialized_plan.operations.len(), plan.operations.len());
    }
    
    #[test]
    fn test_plan_risk_assessment() {
        let low_risk_operations = vec![
            RollbackOperation::RenameTable {
                old_name: "temp_name".to_string(),
                new_name: "final_name".to_string(),
            }
        ];
        
        let high_risk_operations = vec![
            RollbackOperation::DropTable {
                name: "critical_table".to_string(),
                preserve_data: false,
                backup_table_name: None,
            }
        ];
        
        let low_risk_plan = RollbackPlan::new(
            "low_risk".to_string(),
            low_risk_operations,
            None,
            RollbackConfig::default(),
            "Low risk plan".to_string(),
        );
        
        let high_risk_plan = RollbackPlan::new(
            "high_risk".to_string(),
            high_risk_operations,
            None,
            RollbackConfig::default(),
            "High risk plan".to_string(),
        );
        
        assert_eq!(low_risk_plan.risk_assessment, RollbackRiskLevel::Low);
        assert_eq!(high_risk_plan.risk_assessment, RollbackRiskLevel::Critical);
        assert!(!low_risk_plan.metadata.requires_manual_intervention);
        assert!(high_risk_plan.metadata.requires_manual_intervention);
    }
    
    #[test]
    fn test_plan_affected_tables() {
        let operations = vec![
            RollbackOperation::DropColumn {
                table: "users".to_string(),
                column: "temp_col".to_string(),
                preserve_data: true,
            },
            RollbackOperation::AddColumn {
                table: "posts".to_string(),
                column: ColumnSchema {
                    name: "new_col".to_string(),
                    column_type: "TEXT".to_string(),
                    nullable: true,
                    primary_key: false,
                    unique: false,
                    auto_increment: false,
                    default_value: None,
                    constraints: vec![],
                },
                restore_data: false,
            },
            RollbackOperation::DropIndex {
                name: "idx_users_email".to_string(),
                table: "users".to_string(),
            },
        ];
        
        let plan = RollbackPlan::new(
            "multi_table".to_string(),
            operations,
            None,
            RollbackConfig::default(),
            "Multi-table plan".to_string(),
        );
        
        let affected_tables = plan.affected_tables();
        assert_eq!(affected_tables.len(), 2);
        assert!(affected_tables.contains(&"users".to_string()));
        assert!(affected_tables.contains(&"posts".to_string()));
        assert_eq!(plan.metadata.affected_tables, 2);
    }
    
    #[test]
    fn test_execution_priority_ordering() {
        assert!(ExecutionPriority::Critical > ExecutionPriority::High);
        assert!(ExecutionPriority::High > ExecutionPriority::Normal);
        assert!(ExecutionPriority::Normal > ExecutionPriority::Low);
    }
    
    #[test]
    fn test_issue_severity_ordering() {
        assert!(IssueSeverity::Critical > IssueSeverity::Error);
        assert!(IssueSeverity::Error > IssueSeverity::Warning);
        assert!(IssueSeverity::Warning > IssueSeverity::Info);
    }
    
    #[test]
    fn test_display_implementations() {
        let plan = RollbackPlan::new(
            "display_test".to_string(),
            vec![],
            None,
            RollbackConfig::default(),
            "Display test".to_string(),
        );
        
        let display_str = format!("{}", plan);
        assert!(display_str.contains("display_test"));
        assert!(display_str.contains("0 operations"));
        
        assert_eq!(format!("{}", ExecutionPriority::Critical), "Critical");
        assert_eq!(format!("{}", CheckType::TableExists), "Table Exists");
        assert_eq!(format!("{}", ValidationType::SchemaValidation), "Schema Validation");
        assert_eq!(format!("{}", IssueSeverity::Error), "Error");
    }
}