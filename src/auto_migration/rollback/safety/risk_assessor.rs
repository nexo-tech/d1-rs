//! Risk Assessment Engine for rollback safety validation
//!
//! This module provides comprehensive risk assessment capabilities for rollback operations,
//! including data loss analysis, dependency validation, schema compatibility checks,
//! and performance impact estimation.

use super::super::types::{RollbackOperation, RollbackConfig, RollbackRiskLevel, DataLossRisk, RollbackColumnChanges, RollbackRiskSeverity};
use super::super::plan::RollbackPlan;
use crate::auto_migration::{DatabaseSchema, TableSchema, ColumnSchema, ForeignKeySchema};
use crate::Result;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

/// Comprehensive rollback safety assessment engine
pub struct RollbackRiskAssessor {
    /// Configuration for risk assessment behavior (currently unused but reserved for future features)
    #[allow(dead_code)]
    config: RollbackConfig,
    
    /// Cache for schema lookups during validation
    schema_cache: std::cell::RefCell<HashMap<String, CachedValidationResult>>,
}

/// Cached validation results for performance optimization (reserved for future caching implementation)
#[derive(Debug, Clone)]
#[allow(dead_code)]
enum CachedValidationResult {
    Table { exists: bool, schema: Option<TableSchema> },
    Column { exists: bool, compatible: bool },
    Index { exists: bool, recreatable: bool },
    ForeignKey { valid: bool, dependencies: Vec<String> },
}

/// Comprehensive validation result for entire rollback plan
#[derive(Debug, Clone)]
pub struct RollbackValidationResult {
    /// Overall risk level for the rollback
    pub overall_risk: RollbackRiskLevel,
    
    /// Individual operation validation results
    pub operation_results: Vec<OperationValidationResult>,
    
    /// Dependency validation results
    pub dependency_result: DependencyValidationResult,
    
    /// Schema compatibility assessment
    pub schema_compatibility: SchemaCompatibilityResult,
    
    /// Performance impact analysis
    pub performance_impact: PerformanceImpactResult,
    
    /// Blocking issues that prevent rollback execution
    pub blocking_issues: Vec<RollbackBlockingIssue>,
    
    /// All validation issues found
    pub validation_issues: Vec<RollbackValidationIssue>,
    
    /// Estimated total execution time
    pub estimated_duration: Duration,
    
    /// Recommended mitigation actions
    pub mitigation_suggestions: Vec<String>,
}

/// Validation result for individual rollback operation
#[derive(Debug, Clone)]
pub struct OperationValidationResult {
    /// Index of operation in rollback plan
    pub operation_index: usize,
    
    /// Type of rollback operation
    pub operation_type: String,
    
    /// Individual risk level for this operation
    pub risk_level: RollbackRiskLevel,
    
    /// Data loss risk assessment
    pub data_loss_risk: DataLossRisk,
    
    /// Whether operation has blocking issues
    pub has_blocking_issues: bool,
    
    /// Issues found for this operation
    pub issues: Vec<RollbackValidationIssue>,
    
    /// Estimated execution time for this operation
    pub estimated_duration: Duration,
}

/// Dependency validation results
#[derive(Debug, Clone)]
pub struct DependencyValidationResult {
    /// Whether all dependencies are satisfied
    pub dependencies_satisfied: bool,
    
    /// Circular dependencies detected
    pub circular_dependencies: Vec<CircularDependency>,
    
    /// Missing dependencies
    pub missing_dependencies: Vec<MissingDependency>,
    
    /// Dependency conflict warnings
    pub conflicts: Vec<DependencyConflict>,
}

/// Schema compatibility validation results
#[derive(Debug, Clone)]
pub struct SchemaCompatibilityResult {
    /// Whether all required schema elements exist
    pub schema_compatible: bool,
    
    /// Missing schema elements that need to be recreated
    pub missing_elements: Vec<MissingSchemaElement>,
    
    /// Incompatible type conversions
    pub type_incompatibilities: Vec<TypeIncompatibility>,
    
    /// Index recreation issues
    pub index_issues: Vec<IndexCompatibilityIssue>,
}

/// Performance impact analysis results
#[derive(Debug, Clone)]
pub struct PerformanceImpactResult {
    /// Total estimated execution time
    pub total_duration: Duration,
    
    /// Expected downtime during rollback
    pub estimated_downtime: Duration,
    
    /// Tables that will be locked during rollback
    pub locked_tables: Vec<String>,
    
    /// Storage requirements for backups
    pub backup_storage_mb: Option<f64>,
    
    /// Performance warnings
    pub performance_warnings: Vec<String>,
}

/// Individual validation issue
#[derive(Debug, Clone)]
pub struct RollbackValidationIssue {
    /// Type of operation causing the issue
    pub operation_type: String,
    
    /// Detailed description of the issue
    pub description: String,
    
    /// Severity level of the issue
    pub severity: RollbackRiskSeverity,
    
    /// Suggested mitigation action
    pub mitigation: String,
    
    /// Table affected by this issue
    pub affected_table: Option<String>,
    
    /// Column affected by this issue (if applicable)
    pub affected_column: Option<String>,
}

/// Blocking issue that prevents rollback execution
#[derive(Debug, Clone)]
pub struct RollbackBlockingIssue {
    /// Index of operation causing the blocking issue
    pub operation_index: usize,
    
    /// Detailed description of blocking condition
    pub description: String,
    
    /// Required resolution steps
    pub resolution: String,
    
    /// Whether this is resolvable automatically
    pub auto_resolvable: bool,
}


/// Circular dependency detection result
#[derive(Debug, Clone)]
pub struct CircularDependency {
    /// Operations involved in the circular dependency
    pub operation_indices: Vec<usize>,
    
    /// Description of the dependency cycle
    pub cycle_description: String,
}

/// Missing dependency information
#[derive(Debug, Clone)]
pub struct MissingDependency {
    /// Operation that has missing dependency
    pub operation_index: usize,
    
    /// Required dependency that is missing
    pub required_dependency: String,
    
    /// Suggested resolution
    pub resolution: String,
}

/// Dependency conflict information
#[derive(Debug, Clone)]
pub struct DependencyConflict {
    /// Operations in conflict
    pub conflicting_operations: Vec<usize>,
    
    /// Description of the conflict
    pub conflict_description: String,
    
    /// Suggested resolution strategy
    pub resolution_strategy: String,
}

/// Missing schema element information
#[derive(Debug, Clone)]
pub struct MissingSchemaElement {
    /// Type of schema element (table, column, index, etc.)
    pub element_type: String,
    
    /// Name of missing element
    pub element_name: String,
    
    /// Whether this can be recreated from rollback operations
    pub can_recreate: bool,
    
    /// Recreation strategy
    pub recreation_strategy: Option<String>,
}

/// Type incompatibility information
#[derive(Debug, Clone)]
pub struct TypeIncompatibility {
    /// Table containing the incompatible types
    pub table: String,
    
    /// Column with type incompatibility
    pub column: String,
    
    /// Source type in rollback
    pub source_type: String,
    
    /// Target type in current schema
    pub target_type: String,
    
    /// Whether conversion is lossy
    pub is_lossy: bool,
    
    /// Conversion strategy
    pub conversion_strategy: Option<String>,
}

/// Index compatibility issue
#[derive(Debug, Clone)]
pub struct IndexCompatibilityIssue {
    /// Table containing the index
    pub table: String,
    
    /// Index name
    pub index_name: String,
    
    /// Description of the compatibility issue
    pub issue_description: String,
    
    /// Whether index can be recreated
    pub can_recreate: bool,
}

impl RollbackRiskAssessor {
    /// Create a new rollback risk assessor
    pub fn new() -> Self {
        Self {
            config: RollbackConfig::default(),
            schema_cache: std::cell::RefCell::new(HashMap::new()),
        }
    }
    
    /// Create a rollback risk assessor with custom configuration
    pub fn with_config(config: RollbackConfig) -> Self {
        Self {
            config,
            schema_cache: std::cell::RefCell::new(HashMap::new()),
        }
    }
    
    /// Perform comprehensive safety analysis of rollback plan
    pub async fn analyze_rollback_safety(
        &self,
        rollback_plan: &RollbackPlan,
        current_schema: &DatabaseSchema,
    ) -> Result<RollbackValidationResult> {
        // Clear cache for fresh analysis
        self.schema_cache.borrow_mut().clear();
        
        // Validate individual operations
        let mut operation_results = Vec::new();
        let mut all_validation_issues = Vec::new();
        let mut blocking_issues = Vec::new();
        
        for (index, operation) in rollback_plan.operations.iter().enumerate() {
            let result = self.validate_operation_safety(operation, current_schema, index).await?;
            
            // Collect blocking issues
            if result.has_blocking_issues {
                for issue in &result.issues {
                    if issue.severity == RollbackRiskSeverity::Blocking {
                        blocking_issues.push(RollbackBlockingIssue {
                            operation_index: index,
                            description: issue.description.clone(),
                            resolution: issue.mitigation.clone(),
                            auto_resolvable: self.is_auto_resolvable(&issue.operation_type),
                        });
                    }
                }
            }
            
            all_validation_issues.extend(result.issues.clone());
            operation_results.push(result);
        }
        
        // Validate dependencies
        let dependency_result = self.validate_operation_dependencies(&rollback_plan.operations).await?;
        
        // Check schema compatibility
        let schema_compatibility = self.validate_schema_compatibility(&rollback_plan.operations, current_schema).await?;
        
        // Analyze performance impact
        let performance_impact = self.analyze_performance_impact(&rollback_plan.operations, current_schema).await?;
        
        // Calculate overall risk level
        let overall_risk = self.calculate_risk_level(&all_validation_issues, &blocking_issues);
        
        // Generate mitigation suggestions
        let mitigation_suggestions = self.generate_mitigation_suggestions(&all_validation_issues, &blocking_issues);
        
        // Calculate total estimated duration
        let estimated_duration = operation_results.iter()
            .map(|r| r.estimated_duration)
            .fold(Duration::ZERO, |acc, dur| acc + dur);
        
        Ok(RollbackValidationResult {
            overall_risk,
            operation_results,
            dependency_result,
            schema_compatibility,
            performance_impact,
            blocking_issues,
            validation_issues: all_validation_issues,
            estimated_duration,
            mitigation_suggestions,
        })
    }
    
    /// Validate individual rollback operation safety
    async fn validate_operation_safety(
        &self,
        operation: &RollbackOperation,
        schema: &DatabaseSchema,
        operation_index: usize,
    ) -> Result<OperationValidationResult> {
        let operation_type = operation.operation_type().to_string();
        let mut issues = Vec::new();
        
        // Assess data loss risk for this operation
        let data_loss_risk = self.assess_operation_data_loss_risk(operation);
        
        // Perform operation-specific validation
        match operation {
            RollbackOperation::RecreateTable { definition, restore_data, .. } => {
                self.validate_recreate_table_safety(&definition.name, definition, *restore_data, schema, &mut issues);
            }
            RollbackOperation::DropTable { name, .. } => {
                self.validate_drop_table_safety(name, schema, &mut issues);
            }
            RollbackOperation::AddColumn { table, column, restore_data, .. } => {
                self.validate_add_column_safety(table, column, *restore_data, schema, &mut issues);
            }
            RollbackOperation::DropColumn { table, column, .. } => {
                self.validate_drop_column_safety(table, column, schema, &mut issues);
            }
            RollbackOperation::ModifyColumn { table, column, changes, preserve_data, .. } => {
                self.validate_modify_column_safety(table, column, changes, *preserve_data, schema, &mut issues);
            }
            RollbackOperation::CreateIndex { table, index, .. } => {
                self.validate_create_index_safety(table, index, schema, &mut issues);
            }
            RollbackOperation::DropIndex { table, name, .. } => {
                self.validate_drop_index_safety(table, name, schema, &mut issues);
            }
            RollbackOperation::AddForeignKey { table, constraint, .. } => {
                self.validate_add_foreign_key_safety(table, constraint, schema, &mut issues);
            }
            RollbackOperation::DropForeignKey { table, constraint_name, .. } => {
                self.validate_drop_foreign_key_safety(table, constraint_name, schema, &mut issues);
            }
            RollbackOperation::RenameTable { old_name, new_name, .. } => {
                self.validate_rename_table_safety(old_name, new_name, schema, &mut issues);
            }
            RollbackOperation::RenameColumn { table, old_name, new_name, .. } => {
                self.validate_rename_column_safety(table, old_name, new_name, schema, &mut issues);
            }
        }
        
        // Check for blocking issues
        let has_blocking_issues = issues.iter().any(|issue| issue.severity == RollbackRiskSeverity::Blocking);
        
        // Calculate individual risk level
        let risk_level = if has_blocking_issues {
            RollbackRiskLevel::Critical
        } else {
            self.calculate_operation_risk_level(&issues, data_loss_risk)
        };
        
        // Estimate execution duration
        let estimated_duration = self.estimate_operation_duration(operation);
        
        Ok(OperationValidationResult {
            operation_index,
            operation_type,
            risk_level,
            data_loss_risk,
            has_blocking_issues,
            issues,
            estimated_duration,
        })
    }
    
    /// Validate operation dependencies and detect conflicts
    async fn validate_operation_dependencies(
        &self,
        operations: &[RollbackOperation],
    ) -> Result<DependencyValidationResult> {
        let mut circular_dependencies = Vec::new();
        let mut missing_dependencies = Vec::new();
        let mut conflicts = Vec::new();
        
        // Build dependency graph
        let dependency_graph = self.build_dependency_graph(operations);
        
        // Detect circular dependencies using DFS
        circular_dependencies.extend(self.detect_circular_dependencies(&dependency_graph));
        
        // Validate all dependencies are satisfied
        for (operation_index, dependencies) in &dependency_graph {
            for required_dependency in dependencies {
                if !self.dependency_exists(operations, *required_dependency) {
                    missing_dependencies.push(MissingDependency {
                        operation_index: *operation_index,
                        required_dependency: format!("Operation {}", required_dependency),
                        resolution: format!("Ensure operation {} is included in rollback plan", required_dependency),
                    });
                }
            }
        }
        
        // Detect conflicting operations
        conflicts.extend(self.detect_dependency_conflicts(operations));
        
        let dependencies_satisfied = circular_dependencies.is_empty() && missing_dependencies.is_empty() && conflicts.is_empty();
        
        Ok(DependencyValidationResult {
            dependencies_satisfied,
            circular_dependencies,
            missing_dependencies,
            conflicts,
        })
    }
    
    /// Validate schema compatibility for rollback operations
    async fn validate_schema_compatibility(
        &self,
        operations: &[RollbackOperation],
        current_schema: &DatabaseSchema,
    ) -> Result<SchemaCompatibilityResult> {
        let mut missing_elements = Vec::new();
        let mut type_incompatibilities = Vec::new();
        let mut index_issues = Vec::new();
        
        for operation in operations {
            match operation {
                RollbackOperation::RecreateTable { definition, .. } => {
                    let name = &definition.name;
                    // Check if we have enough information to recreate the table
                    if definition.columns.is_empty() {
                        missing_elements.push(MissingSchemaElement {
                            element_type: "table_definition".to_string(),
                            element_name: name.clone(),
                            can_recreate: false,
                            recreation_strategy: Some("Requires complete table schema definition".to_string()),
                        });
                    }
                    
                    // Validate column type compatibility
                    for column in &definition.columns {
                        if let Some(incompatibility) = self.check_column_type_compatibility(name, column, current_schema) {
                            type_incompatibilities.push(incompatibility);
                        }
                    }
                    
                    // Check index recreation capability
                    for index in &definition.indexes {
                        if let Some(issue) = self.check_index_recreation_capability(name, index) {
                            index_issues.push(issue);
                        }
                    }
                }
                
                RollbackOperation::AddColumn { table, .. } => {
                    // Check if table exists
                    if !self.table_exists(table, current_schema) {
                        missing_elements.push(MissingSchemaElement {
                            element_type: "table".to_string(),
                            element_name: table.clone(),
                            can_recreate: false,
                            recreation_strategy: Some("Table must exist before adding columns".to_string()),
                        });
                    }
                }
                
                RollbackOperation::ModifyColumn { table, column, changes, .. } => {
                    // Check type conversion compatibility
                    if let Some((from_type, to_type)) = &changes.type_change {
                        if self.is_type_conversion_incompatible(from_type, to_type) {
                            type_incompatibilities.push(TypeIncompatibility {
                                table: table.clone(),
                                column: column.clone(),
                                source_type: from_type.clone(),
                                target_type: to_type.clone(),
                                is_lossy: self.is_lossy_conversion(from_type, to_type),
                                conversion_strategy: self.suggest_conversion_strategy(from_type, to_type),
                            });
                        }
                    }
                }
                
                _ => {
                    // Other operations validated separately
                }
            }
        }
        
        let schema_compatible = missing_elements.is_empty() && 
                               type_incompatibilities.iter().all(|i| !i.is_lossy) && 
                               index_issues.is_empty();
        
        Ok(SchemaCompatibilityResult {
            schema_compatible,
            missing_elements,
            type_incompatibilities,
            index_issues,
        })
    }
    
    /// Analyze performance impact of rollback operations
    async fn analyze_performance_impact(
        &self,
        operations: &[RollbackOperation],
        current_schema: &DatabaseSchema,
    ) -> Result<PerformanceImpactResult> {
        let mut locked_tables = HashSet::new();
        let mut performance_warnings = Vec::new();
        let mut total_duration = Duration::ZERO;
        let mut backup_storage_mb = 0.0;
        
        for operation in operations {
            // Calculate duration for each operation
            let operation_duration = self.estimate_operation_duration(operation);
            total_duration += operation_duration;
            
            // Identify tables that will be locked
            let affected_table = operation.affected_table();
            locked_tables.insert(affected_table);
            
            // Estimate backup storage requirements
            if operation.is_destructive() {
                if let Some(table_size) = self.estimate_table_size(affected_table, current_schema) {
                    backup_storage_mb += table_size;
                }
            }
            
            // Generate performance warnings
            match operation {
                RollbackOperation::RecreateTable { .. } => {
                    performance_warnings.push(format!(
                        "Table recreation for '{}' will cause extended downtime", 
                        affected_table
                    ));
                }
                RollbackOperation::ModifyColumn { changes, .. } => {
                    if changes.type_change.is_some() {
                        performance_warnings.push(format!(
                            "Column type modification on '{}' may require table rebuild", 
                            affected_table
                        ));
                    }
                }
                RollbackOperation::CreateIndex { index, .. } => {
                    if index.unique {
                        performance_warnings.push(format!(
                            "Creating unique index '{}' may be slow on large tables", 
                            index.name
                        ));
                    }
                }
                _ => {}
            }
        }
        
        // Calculate estimated downtime (operations that require exclusive locks)
        let estimated_downtime = operations.iter()
            .filter(|op| self.requires_exclusive_lock(op))
            .map(|op| self.estimate_operation_duration(op))
            .fold(Duration::ZERO, |acc, dur| acc + dur);
        
        // Add warnings for high-impact scenarios
        if total_duration > Duration::from_secs(300) { // 5 minutes
            performance_warnings.push("Rollback operation will take more than 5 minutes".to_string());
        }
        
        if locked_tables.len() > 5 {
            performance_warnings.push(format!("Rollback will lock {} tables simultaneously", locked_tables.len()));
        }
        
        if backup_storage_mb > 1000.0 { // 1GB
            performance_warnings.push(format!("Backup storage requirement: {:.1} MB", backup_storage_mb));
        }
        
        Ok(PerformanceImpactResult {
            total_duration,
            estimated_downtime,
            locked_tables: locked_tables.into_iter().map(|s| s.to_string()).collect(),
            backup_storage_mb: Some(backup_storage_mb),
            performance_warnings,
        })
    }
    
    // Helper methods for validation
    
    fn assess_operation_data_loss_risk(&self, operation: &RollbackOperation) -> DataLossRisk {
        match operation {
            RollbackOperation::DropTable { .. } => DataLossRisk::High,
            RollbackOperation::DropColumn { .. } => DataLossRisk::High,
            RollbackOperation::ModifyColumn { changes, preserve_data, .. } => {
                if changes.is_lossy() && !*preserve_data {
                    DataLossRisk::High
                } else if changes.is_lossy() {
                    DataLossRisk::Medium
                } else {
                    DataLossRisk::Low
                }
            }
            RollbackOperation::RecreateTable { restore_data, .. } => {
                if *restore_data {
                    DataLossRisk::Medium
                } else {
                    DataLossRisk::High
                }
            }
            _ => DataLossRisk::Low,
        }
    }
    
    fn calculate_operation_risk_level(&self, issues: &[RollbackValidationIssue], data_loss_risk: DataLossRisk) -> RollbackRiskLevel {
        let max_issue_severity = issues.iter()
            .map(|issue| &issue.severity)
            .max()
            .unwrap_or(&RollbackRiskSeverity::Info);
        
        match (max_issue_severity, data_loss_risk) {
            (RollbackRiskSeverity::Blocking, _) => RollbackRiskLevel::Critical,
            (RollbackRiskSeverity::Critical, _) => RollbackRiskLevel::Critical,
            (RollbackRiskSeverity::High, DataLossRisk::High) => RollbackRiskLevel::High,
            (RollbackRiskSeverity::High, _) => RollbackRiskLevel::Medium,
            (_, DataLossRisk::High) => RollbackRiskLevel::High,
            (_, DataLossRisk::Medium) => RollbackRiskLevel::Medium,
            _ => RollbackRiskLevel::Low,
        }
    }
    
    fn calculate_risk_level(&self, issues: &[RollbackValidationIssue], blocking_issues: &[RollbackBlockingIssue]) -> RollbackRiskLevel {
        if !blocking_issues.is_empty() {
            return RollbackRiskLevel::Critical;
        }
        
        let critical_issues = issues.iter().any(|i| i.severity == RollbackRiskSeverity::Critical);
        if critical_issues {
            return RollbackRiskLevel::Critical;
        }
        
        let high_issues = issues.iter().filter(|i| i.severity == RollbackRiskSeverity::High).count();
        if high_issues > 0 {
            return RollbackRiskLevel::High;
        }
        
        let warning_issues = issues.iter().filter(|i| i.severity == RollbackRiskSeverity::Warning).count();
        if warning_issues > 3 {
            return RollbackRiskLevel::Medium;
        } else if warning_issues > 0 {
            return RollbackRiskLevel::Low;
        }
        
        RollbackRiskLevel::Low
    }
    
    fn estimate_operation_duration(&self, operation: &RollbackOperation) -> Duration {
        match operation {
            RollbackOperation::RecreateTable { .. } => Duration::from_secs(30),
            RollbackOperation::DropTable { .. } => Duration::from_secs(2),
            RollbackOperation::AddColumn { .. } => Duration::from_secs(5),
            RollbackOperation::DropColumn { .. } => Duration::from_secs(10),
            RollbackOperation::ModifyColumn { changes, .. } => {
                if changes.type_change.is_some() {
                    Duration::from_secs(15)
                } else {
                    Duration::from_secs(3)
                }
            }
            RollbackOperation::CreateIndex { index, .. } => {
                if index.unique {
                    Duration::from_secs(20)
                } else {
                    Duration::from_secs(10)
                }
            }
            RollbackOperation::DropIndex { .. } => Duration::from_secs(1),
            RollbackOperation::AddForeignKey { .. } => Duration::from_secs(8),
            RollbackOperation::DropForeignKey { .. } => Duration::from_secs(2),
            RollbackOperation::RenameTable { .. } => Duration::from_secs(1),
            RollbackOperation::RenameColumn { .. } => Duration::from_secs(3),
        }
    }
    
    fn requires_exclusive_lock(&self, operation: &RollbackOperation) -> bool {
        matches!(
            operation,
            RollbackOperation::RecreateTable { .. } |
            RollbackOperation::DropTable { .. } |
            RollbackOperation::ModifyColumn { .. }
        )
    }
    
    fn is_auto_resolvable(&self, operation_type: &str) -> bool {
        matches!(operation_type, "rename_table" | "rename_column")
    }
    
    // Validation helper methods that would be implemented for each operation type
    // These are placeholder implementations for the comprehensive functionality
    
    fn validate_recreate_table_safety(&self, name: &str, definition: &TableSchema, restore_data: bool, schema: &DatabaseSchema, issues: &mut Vec<RollbackValidationIssue>) {
        // Check if table exists in current schema
        if !self.table_exists(name, schema) {
            issues.push(RollbackValidationIssue {
                operation_type: "recreate_table".to_string(),
                description: format!("Table '{}' does not exist in current schema for recreation", name),
                severity: RollbackRiskSeverity::Critical,
                mitigation: "Ensure table exists before attempting recreation".to_string(),
                affected_table: Some(name.to_string()),
                affected_column: None,
            });
        }
        
        // Check if definition has required columns
        if definition.columns.is_empty() {
            issues.push(RollbackValidationIssue {
                operation_type: "recreate_table".to_string(),
                description: format!("Table '{}' definition is missing column specifications", name),
                severity: RollbackRiskSeverity::Blocking,
                mitigation: "Provide complete table schema with all column definitions".to_string(),
                affected_table: Some(name.to_string()),
                affected_column: None,
            });
        }
        
        // Validate column type compatibility
        for column in &definition.columns {
            if let Some(current_table) = schema.tables.iter().find(|t| t.name == name) {
                if let Some(current_column) = current_table.columns.iter().find(|c| c.name == column.name) {
                    if self.is_type_conversion_incompatible(&current_column.column_type, &column.column_type) {
                        issues.push(RollbackValidationIssue {
                            operation_type: "recreate_table".to_string(),
                            description: format!(
                                "Column '{}' type conversion from '{}' to '{}' is incompatible",
                                column.name, current_column.column_type, column.column_type
                            ),
                            severity: RollbackRiskSeverity::High,
                            mitigation: "Review data compatibility and consider type conversion strategy".to_string(),
                            affected_table: Some(name.to_string()),
                            affected_column: Some(column.name.clone()),
                        });
                    }
                }
            }
        }
        
        // Assess data preservation risk
        if !restore_data {
            issues.push(RollbackValidationIssue {
                operation_type: "recreate_table".to_string(),
                description: format!("Table '{}' recreation will not restore data - data loss will occur", name),
                severity: RollbackRiskSeverity::Critical,
                mitigation: "Enable data restoration or manually backup table data before recreation".to_string(),
                affected_table: Some(name.to_string()),
                affected_column: None,
            });
        }
        
        // Check for dependent foreign key constraints
        for table in &schema.tables {
            for fk in &table.foreign_keys {
                if fk.referenced_table == name {
                    issues.push(RollbackValidationIssue {
                        operation_type: "recreate_table".to_string(),
                        description: format!(
                            "Table '{}' has foreign key dependency from table '{}'",
                            name, table.name
                        ),
                        severity: RollbackRiskSeverity::High,
                        mitigation: "Drop dependent foreign keys before recreating table".to_string(),
                        affected_table: Some(name.to_string()),
                        affected_column: None,
                    });
                }
            }
        }
    }
    
    fn validate_drop_table_safety(&self, name: &str, schema: &DatabaseSchema, issues: &mut Vec<RollbackValidationIssue>) {
        // Check if table exists for dropping
        if !self.table_exists(name, schema) {
            issues.push(RollbackValidationIssue {
                operation_type: "drop_table".to_string(),
                description: format!("Cannot drop table '{}' - table does not exist", name),
                severity: RollbackRiskSeverity::Blocking,
                mitigation: "Verify table name and ensure table exists before attempting to drop".to_string(),
                affected_table: Some(name.to_string()),
                affected_column: None,
            });
            return;
        }
        
        // Check for foreign key dependencies (other tables referencing this table)
        for table in &schema.tables {
            for fk in &table.foreign_keys {
                if fk.referenced_table == name {
                    issues.push(RollbackValidationIssue {
                        operation_type: "drop_table".to_string(),
                        description: format!(
                            "Cannot drop table '{}' - referenced by foreign key in table '{}'",
                            name, table.name
                        ),
                        severity: RollbackRiskSeverity::Blocking,
                        mitigation: "Drop dependent foreign key constraints before dropping table".to_string(),
                        affected_table: Some(name.to_string()),
                        affected_column: None,
                    });
                }
            }
        }
        
        // Check if table contains data (high data loss risk)
        let table_schema = schema.tables.iter().find(|t| t.name == name);
        if let Some(_table) = table_schema {
            issues.push(RollbackValidationIssue {
                operation_type: "drop_table".to_string(),
                description: format!("Dropping table '{}' will permanently delete all data", name),
                severity: RollbackRiskSeverity::Critical,
                mitigation: "Backup table data before dropping or use preserve_data option".to_string(),
                affected_table: Some(name.to_string()),
                affected_column: None,
            });
        }
        
        // Check for indexes that will be lost
        if let Some(table) = table_schema {
            if !table.indexes.is_empty() {
                issues.push(RollbackValidationIssue {
                    operation_type: "drop_table".to_string(),
                    description: format!(
                        "Dropping table '{}' will also drop {} index(es)",
                        name, table.indexes.len()
                    ),
                    severity: RollbackRiskSeverity::Warning,
                    mitigation: "Document index definitions for potential recreation".to_string(),
                    affected_table: Some(name.to_string()),
                    affected_column: None,
                });
            }
        }
        
        // Always warn about irreversible operation
        issues.push(RollbackValidationIssue {
            operation_type: "drop_table".to_string(),
            description: format!("Table drop operation for '{}' is irreversible without data backup", name),
            severity: RollbackRiskSeverity::High,
            mitigation: "Ensure complete table backup exists before proceeding".to_string(),
            affected_table: Some(name.to_string()),
            affected_column: None,
        });
    }
    
    fn validate_add_column_safety(&self, table: &str, column: &ColumnSchema, restore_data: bool, schema: &DatabaseSchema, issues: &mut Vec<RollbackValidationIssue>) {
        // Check if target table exists
        if !self.table_exists(table, schema) {
            issues.push(RollbackValidationIssue {
                operation_type: "add_column".to_string(),
                description: format!("Cannot add column to table '{}' - table does not exist", table),
                severity: RollbackRiskSeverity::Blocking,
                mitigation: "Create table before adding column or verify table name".to_string(),
                affected_table: Some(table.to_string()),
                affected_column: Some(column.name.clone()),
            });
            return;
        }
        
        // Check if column already exists
        if let Some(existing_table) = schema.tables.iter().find(|t| t.name == table) {
            if existing_table.columns.iter().any(|c| c.name == column.name) {
                issues.push(RollbackValidationIssue {
                    operation_type: "add_column".to_string(),
                    description: format!("Column '{}' already exists in table '{}'", column.name, table),
                    severity: RollbackRiskSeverity::Blocking,
                    mitigation: "Use a different column name or modify existing column instead".to_string(),
                    affected_table: Some(table.to_string()),
                    affected_column: Some(column.name.clone()),
                });
            }
        }
        
        // Validate column constraints for existing data
        if !column.nullable && column.default_value.is_none() {
            issues.push(RollbackValidationIssue {
                operation_type: "add_column".to_string(),
                description: format!(
                    "Adding non-nullable column '{}' without default value to table with existing data",
                    column.name
                ),
                severity: RollbackRiskSeverity::High,
                mitigation: "Add default value or make column nullable for existing rows".to_string(),
                affected_table: Some(table.to_string()),
                affected_column: Some(column.name.clone()),
            });
        }
        
        // Check for unique constraint conflicts
        if column.unique {
            issues.push(RollbackValidationIssue {
                operation_type: "add_column".to_string(),
                description: format!(
                    "Adding unique column '{}' to table with existing data may cause constraint violations",
                    column.name
                ),
                severity: RollbackRiskSeverity::High,
                mitigation: "Ensure uniqueness of default values or populate column before adding constraint".to_string(),
                affected_table: Some(table.to_string()),
                affected_column: Some(column.name.clone()),
            });
        }
        
        // Assess data restoration requirements
        if restore_data && column.default_value.is_none() {
            issues.push(RollbackValidationIssue {
                operation_type: "add_column".to_string(),
                description: format!(
                    "Data restoration requested for column '{}' but no default value provided",
                    column.name
                ),
                severity: RollbackRiskSeverity::Warning,
                mitigation: "Provide default value or prepare data restoration strategy".to_string(),
                affected_table: Some(table.to_string()),
                affected_column: Some(column.name.clone()),
            });
        }
        
        // Check for foreign key constraint dependencies
        for constraint in &column.constraints {
            if let crate::auto_migration::introspector::ColumnConstraint::References { table: ref_table, .. } = constraint {
                // Validate referenced table exists
                if !self.table_exists(ref_table, schema) {
                    issues.push(RollbackValidationIssue {
                        operation_type: "add_column".to_string(),
                        description: format!(
                            "Foreign key column '{}' references non-existent table '{}'",
                            column.name, ref_table
                        ),
                        severity: RollbackRiskSeverity::Blocking,
                        mitigation: "Create referenced table before adding foreign key column".to_string(),
                        affected_table: Some(table.to_string()),
                        affected_column: Some(column.name.clone()),
                    });
                }
            }
        }
    }
    
    fn validate_drop_column_safety(&self, table: &str, column: &str, schema: &DatabaseSchema, issues: &mut Vec<RollbackValidationIssue>) {
        // Check if table exists
        if !self.table_exists(table, schema) {
            issues.push(RollbackValidationIssue {
                operation_type: "drop_column".to_string(),
                description: format!("Cannot drop column from table '{}' - table does not exist", table),
                severity: RollbackRiskSeverity::Blocking,
                mitigation: "Verify table name and ensure table exists".to_string(),
                affected_table: Some(table.to_string()),
                affected_column: Some(column.to_string()),
            });
            return;
        }
        
        let table_schema = schema.tables.iter().find(|t| t.name == table).unwrap();
        
        // Check if column exists
        let column_schema = table_schema.columns.iter().find(|c| c.name == column);
        if column_schema.is_none() {
            issues.push(RollbackValidationIssue {
                operation_type: "drop_column".to_string(),
                description: format!("Column '{}' does not exist in table '{}'", column, table),
                severity: RollbackRiskSeverity::Blocking,
                mitigation: "Verify column name and ensure column exists before dropping".to_string(),
                affected_table: Some(table.to_string()),
                affected_column: Some(column.to_string()),
            });
            return;
        }
        
        let column_schema = column_schema.unwrap();
        
        // Always warn about data loss
        issues.push(RollbackValidationIssue {
            operation_type: "drop_column".to_string(),
            description: format!("Dropping column '{}' will permanently delete all data in that column", column),
            severity: RollbackRiskSeverity::Critical,
            mitigation: "Backup column data before dropping or use preserve_data option".to_string(),
            affected_table: Some(table.to_string()),
            affected_column: Some(column.to_string()),
        });
        
        // Check if column is part of primary key
        if column_schema.primary_key {
            issues.push(RollbackValidationIssue {
                operation_type: "drop_column".to_string(),
                description: format!("Cannot drop primary key column '{}'", column),
                severity: RollbackRiskSeverity::Blocking,
                mitigation: "Remove primary key constraint before dropping column".to_string(),
                affected_table: Some(table.to_string()),
                affected_column: Some(column.to_string()),
            });
        }
        
        // Check for foreign key dependencies (this column referencing other tables)
        for constraint in &column_schema.constraints {
            if let crate::auto_migration::introspector::ColumnConstraint::References { .. } = constraint {
                issues.push(RollbackValidationIssue {
                    operation_type: "drop_column".to_string(),
                    description: format!("Column '{}' has foreign key constraint", column),
                    severity: RollbackRiskSeverity::High,
                    mitigation: "Drop foreign key constraint before dropping column".to_string(),
                    affected_table: Some(table.to_string()),
                    affected_column: Some(column.to_string()),
                });
            }
        }
        
        // Check if other tables reference this column via foreign keys
        for other_table in &schema.tables {
            for fk in &other_table.foreign_keys {
                if fk.referenced_table == table && fk.referenced_columns.contains(&column.to_string()) {
                    issues.push(RollbackValidationIssue {
                        operation_type: "drop_column".to_string(),
                        description: format!(
                            "Column '{}' is referenced by foreign key in table '{}'",
                            column, other_table.name
                        ),
                        severity: RollbackRiskSeverity::Blocking,
                        mitigation: "Drop referencing foreign key constraints before dropping column".to_string(),
                        affected_table: Some(table.to_string()),
                        affected_column: Some(column.to_string()),
                    });
                }
            }
        }
        
        // Check if column is part of unique constraints
        if column_schema.unique {
            issues.push(RollbackValidationIssue {
                operation_type: "drop_column".to_string(),
                description: format!("Column '{}' has unique constraint", column),
                severity: RollbackRiskSeverity::Warning,
                mitigation: "Consider impact of removing unique constraint".to_string(),
                affected_table: Some(table.to_string()),
                affected_column: Some(column.to_string()),
            });
        }
        
        // Check if column is used in indexes
        for index in &table_schema.indexes {
            if index.columns.contains(&column.to_string()) {
                issues.push(RollbackValidationIssue {
                    operation_type: "drop_column".to_string(),
                    description: format!("Column '{}' is part of index '{}'", column, index.name),
                    severity: RollbackRiskSeverity::High,
                    mitigation: "Drop or modify indexes before dropping column".to_string(),
                    affected_table: Some(table.to_string()),
                    affected_column: Some(column.to_string()),
                });
            }
        }
    }
    
    fn validate_modify_column_safety(&self, table: &str, column: &str, changes: &RollbackColumnChanges, preserve_data: bool, schema: &DatabaseSchema, issues: &mut Vec<RollbackValidationIssue>) {
        // Check if table exists
        if !self.table_exists(table, schema) {
            issues.push(RollbackValidationIssue {
                operation_type: "modify_column".to_string(),
                description: format!("Cannot modify column in table '{}' - table does not exist", table),
                severity: RollbackRiskSeverity::Blocking,
                mitigation: "Verify table name and ensure table exists".to_string(),
                affected_table: Some(table.to_string()),
                affected_column: Some(column.to_string()),
            });
            return;
        }
        
        let table_schema = schema.tables.iter().find(|t| t.name == table).unwrap();
        
        // Check if column exists
        let column_schema = table_schema.columns.iter().find(|c| c.name == column);
        if column_schema.is_none() {
            issues.push(RollbackValidationIssue {
                operation_type: "modify_column".to_string(),
                description: format!("Column '{}' does not exist in table '{}'", column, table),
                severity: RollbackRiskSeverity::Blocking,
                mitigation: "Verify column name and ensure column exists before modifying".to_string(),
                affected_table: Some(table.to_string()),
                affected_column: Some(column.to_string()),
            });
            return;
        }
        
        // Validate type changes
        if let Some((from_type, to_type)) = &changes.type_change {
            if self.is_type_conversion_incompatible(from_type, to_type) {
                issues.push(RollbackValidationIssue {
                    operation_type: "modify_column".to_string(),
                    description: format!(
                        "Type conversion from '{}' to '{}' is incompatible for column '{}'",
                        from_type, to_type, column
                    ),
                    severity: RollbackRiskSeverity::Blocking,
                    mitigation: "Use compatible type conversion or manually migrate data".to_string(),
                    affected_table: Some(table.to_string()),
                    affected_column: Some(column.to_string()),
                });
            } else if self.is_lossy_conversion(from_type, to_type) {
                let severity = if preserve_data { RollbackRiskSeverity::High } else { RollbackRiskSeverity::Critical };
                issues.push(RollbackValidationIssue {
                    operation_type: "modify_column".to_string(),
                    description: format!(
                        "Type conversion from '{}' to '{}' may cause data loss for column '{}'",
                        from_type, to_type, column
                    ),
                    severity,
                    mitigation: "Enable data preservation or ensure conversion safety".to_string(),
                    affected_table: Some(table.to_string()),
                    affected_column: Some(column.to_string()),
                });
            }
        }
        
        // Validate nullable changes
        if let Some((old_nullable, new_nullable)) = changes.null_change {
            // Changing from nullable to non-nullable
            if old_nullable && !new_nullable {
                issues.push(RollbackValidationIssue {
                    operation_type: "modify_column".to_string(),
                    description: format!(
                        "Changing column '{}' from nullable to non-nullable may fail if NULL values exist",
                        column
                    ),
                    severity: RollbackRiskSeverity::High,
                    mitigation: "Ensure no NULL values exist or provide default value".to_string(),
                    affected_table: Some(table.to_string()),
                    affected_column: Some(column.to_string()),
                });
            }
        }
        
        // Validate default value changes
        if let Some((old_default, new_default)) = &changes.default_change {
            match (old_default, new_default) {
                (Some(_), Some(_)) => {
                    // Changing default value - generally safe
                    issues.push(RollbackValidationIssue {
                        operation_type: "modify_column".to_string(),
                        description: format!("Default value change for column '{}'", column),
                        severity: RollbackRiskSeverity::Info,
                        mitigation: "Verify new default value compatibility with existing data".to_string(),
                        affected_table: Some(table.to_string()),
                        affected_column: Some(column.to_string()),
                    });
                }
                (Some(_), None) => {
                    // Removing default value
                    issues.push(RollbackValidationIssue {
                        operation_type: "modify_column".to_string(),
                        description: format!("Removing default value from column '{}'", column),
                        severity: RollbackRiskSeverity::Warning,
                        mitigation: "Ensure application handles NULL values appropriately".to_string(),
                        affected_table: Some(table.to_string()),
                        affected_column: Some(column.to_string()),
                    });
                }
                (None, Some(_)) => {
                    // Adding default value
                    issues.push(RollbackValidationIssue {
                        operation_type: "modify_column".to_string(),
                        description: format!("Adding default value to column '{}'", column),
                        severity: RollbackRiskSeverity::Info,
                        mitigation: "New default value will apply to new rows only".to_string(),
                        affected_table: Some(table.to_string()),
                        affected_column: Some(column.to_string()),
                    });
                }
                (None, None) => {
                    // No change to default value
                }
            }
        }
        
        
        // Check if column is involved in relationships when making major changes
        if changes.type_change.is_some() {
            let current_column = column_schema.unwrap();
            
            // Check for foreign key constraints
            for constraint in &current_column.constraints {
                if let crate::auto_migration::introspector::ColumnConstraint::References { .. } = constraint {
                    issues.push(RollbackValidationIssue {
                        operation_type: "modify_column".to_string(),
                        description: format!("Modifying type of column '{}' with foreign key constraint", column),
                        severity: RollbackRiskSeverity::High,
                        mitigation: "Ensure new type is compatible with referenced column".to_string(),
                        affected_table: Some(table.to_string()),
                        affected_column: Some(column.to_string()),
                    });
                    break; // Only need to warn once
                }
            }
            
            // Check if other tables reference this column
            for other_table in &schema.tables {
                for fk in &other_table.foreign_keys {
                    if fk.referenced_table == table && fk.referenced_columns.contains(&column.to_string()) {
                        issues.push(RollbackValidationIssue {
                            operation_type: "modify_column".to_string(),
                            description: format!(
                                "Column '{}' type modification affects foreign key in table '{}'",
                                column, other_table.name
                            ),
                            severity: RollbackRiskSeverity::High,
                            mitigation: "Ensure type compatibility with referencing foreign keys".to_string(),
                            affected_table: Some(table.to_string()),
                            affected_column: Some(column.to_string()),
                        });
                    }
                }
            }
        }
        
        // Assess overall modification risk
        if changes.is_complex_change() {
            issues.push(RollbackValidationIssue {
                operation_type: "modify_column".to_string(),
                description: format!("Complex modification planned for column '{}'", column),
                severity: RollbackRiskSeverity::Warning,
                mitigation: "Consider breaking into smaller, safer modifications".to_string(),
                affected_table: Some(table.to_string()),
                affected_column: Some(column.to_string()),
            });
        }
    }
    
    fn validate_create_index_safety(&self, table: &str, index: &crate::auto_migration::introspector::IndexSchema, schema: &DatabaseSchema, issues: &mut Vec<RollbackValidationIssue>) {
        // Check if table exists
        if !self.table_exists(table, schema) {
            issues.push(RollbackValidationIssue {
                operation_type: "create_index".to_string(),
                description: format!("Cannot create index on table '{}' - table does not exist", table),
                severity: RollbackRiskSeverity::Blocking,
                mitigation: "Create table before adding index".to_string(),
                affected_table: Some(table.to_string()),
                affected_column: None,
            });
            return;
        }
        
        let table_schema = schema.tables.iter().find(|t| t.name == table).unwrap();
        
        // Check if index already exists
        if table_schema.indexes.iter().any(|idx| idx.name == index.name) {
            issues.push(RollbackValidationIssue {
                operation_type: "create_index".to_string(),
                description: format!("Index '{}' already exists on table '{}'", index.name, table),
                severity: RollbackRiskSeverity::Blocking,
                mitigation: "Use a different index name or drop existing index first".to_string(),
                affected_table: Some(table.to_string()),
                affected_column: None,
            });
        }
        
        // Validate that all index columns exist in the table
        for index_col_name in &index.columns {
            if !table_schema.columns.iter().any(|col| col.name == *index_col_name) {
                issues.push(RollbackValidationIssue {
                    operation_type: "create_index".to_string(),
                    description: format!(
                        "Index column '{}' does not exist in table '{}'",
                        index_col_name, table
                    ),
                    severity: RollbackRiskSeverity::Blocking,
                    mitigation: "Ensure all index columns exist in the table".to_string(),
                    affected_table: Some(table.to_string()),
                    affected_column: Some(index_col_name.clone()),
                });
            }
        }
        
        // Check for unique index constraints on existing data
        if index.unique {
            issues.push(RollbackValidationIssue {
                operation_type: "create_index".to_string(),
                description: format!(
                    "Creating unique index '{}' on table with existing data may fail if duplicates exist",
                    index.name
                ),
                severity: RollbackRiskSeverity::High,
                mitigation: "Ensure data uniqueness before creating unique index".to_string(),
                affected_table: Some(table.to_string()),
                affected_column: None,
            });
        }
        
        // Performance warning for large tables
        issues.push(RollbackValidationIssue {
            operation_type: "create_index".to_string(),
            description: format!("Creating index '{}' may take time on large tables", index.name),
            severity: RollbackRiskSeverity::Info,
            mitigation: "Consider maintenance window for index creation on large tables".to_string(),
            affected_table: Some(table.to_string()),
            affected_column: None,
        });
        
        // Check for redundant indexes
        for existing_index in &table_schema.indexes {
            if existing_index.columns.len() == index.columns.len() {
                let columns_match = existing_index.columns.iter()
                    .zip(index.columns.iter())
                    .all(|(e, n)| e == n);
                
                if columns_match {
                    issues.push(RollbackValidationIssue {
                        operation_type: "create_index".to_string(),
                        description: format!(
                            "New index '{}' is redundant with existing index '{}'",
                            index.name, existing_index.name
                        ),
                        severity: RollbackRiskSeverity::Warning,
                        mitigation: "Consider if both indexes are necessary".to_string(),
                        affected_table: Some(table.to_string()),
                        affected_column: None,
                    });
                }
            }
        }
    }
    
    fn validate_drop_index_safety(&self, table: &str, name: &str, schema: &DatabaseSchema, issues: &mut Vec<RollbackValidationIssue>) {
        // Check if table exists
        if !self.table_exists(table, schema) {
            issues.push(RollbackValidationIssue {
                operation_type: "drop_index".to_string(),
                description: format!("Cannot drop index from table '{}' - table does not exist", table),
                severity: RollbackRiskSeverity::Blocking,
                mitigation: "Verify table name and ensure table exists".to_string(),
                affected_table: Some(table.to_string()),
                affected_column: None,
            });
            return;
        }
        
        let table_schema = schema.tables.iter().find(|t| t.name == table).unwrap();
        
        // Check if index exists
        if !table_schema.indexes.iter().any(|idx| idx.name == name) {
            issues.push(RollbackValidationIssue {
                operation_type: "drop_index".to_string(),
                description: format!("Index '{}' does not exist on table '{}'", name, table),
                severity: RollbackRiskSeverity::Blocking,
                mitigation: "Verify index name and ensure index exists before dropping".to_string(),
                affected_table: Some(table.to_string()),
                affected_column: None,
            });
            return;
        }
        
        // Find the index to get more details
        let index = table_schema.indexes.iter().find(|idx| idx.name == name).unwrap();
        
        // Warn about performance impact if unique index
        if index.unique {
            issues.push(RollbackValidationIssue {
                operation_type: "drop_index".to_string(),
                description: format!("Dropping unique index '{}' will remove uniqueness constraint", name),
                severity: RollbackRiskSeverity::High,
                mitigation: "Ensure application-level uniqueness validation or recreate constraint".to_string(),
                affected_table: Some(table.to_string()),
                affected_column: None,
            });
        }
        
        // Performance warning - queries may slow down
        issues.push(RollbackValidationIssue {
            operation_type: "drop_index".to_string(),
            description: format!("Dropping index '{}' may affect query performance", name),
            severity: RollbackRiskSeverity::Warning,
            mitigation: "Monitor query performance after index removal".to_string(),
            affected_table: Some(table.to_string()),
            affected_column: None,
        });
    }
    
    fn validate_add_foreign_key_safety(&self, table: &str, constraint: &ForeignKeySchema, schema: &DatabaseSchema, issues: &mut Vec<RollbackValidationIssue>) {
        // Check if source table exists
        if !self.table_exists(table, schema) {
            issues.push(RollbackValidationIssue {
                operation_type: "add_foreign_key".to_string(),
                description: format!("Cannot add foreign key to table '{}' - table does not exist", table),
                severity: RollbackRiskSeverity::Blocking,
                mitigation: "Create table before adding foreign key constraint".to_string(),
                affected_table: Some(table.to_string()),
                affected_column: None,
            });
            return;
        }
        
        // Check if referenced table exists
        if !self.table_exists(&constraint.referenced_table, schema) {
            issues.push(RollbackValidationIssue {
                operation_type: "add_foreign_key".to_string(),
                description: format!("Referenced table '{}' does not exist", constraint.referenced_table),
                severity: RollbackRiskSeverity::Blocking,
                mitigation: "Create referenced table before adding foreign key constraint".to_string(),
                affected_table: Some(table.to_string()),
                affected_column: None,
            });
            return;
        }
        
        let source_table = schema.tables.iter().find(|t| t.name == table).unwrap();
        let target_table = schema.tables.iter().find(|t| t.name == constraint.referenced_table).unwrap();
        
        // Check if source columns exist
        for col_name in &constraint.columns {
            if !source_table.columns.iter().any(|c| c.name == *col_name) {
                issues.push(RollbackValidationIssue {
                    operation_type: "add_foreign_key".to_string(),
                    description: format!("Source column '{}' does not exist in table '{}'", col_name, table),
                    severity: RollbackRiskSeverity::Blocking,
                    mitigation: "Create source column before adding foreign key constraint".to_string(),
                    affected_table: Some(table.to_string()),
                    affected_column: Some(col_name.clone()),
                });
            }
        }
        
        // Check if referenced columns exist
        for col_name in &constraint.referenced_columns {
            if !target_table.columns.iter().any(|c| c.name == *col_name) {
                issues.push(RollbackValidationIssue {
                    operation_type: "add_foreign_key".to_string(),
                    description: format!("Referenced column '{}' does not exist in table '{}'", col_name, constraint.referenced_table),
                    severity: RollbackRiskSeverity::Blocking,
                    mitigation: "Create referenced column before adding foreign key constraint".to_string(),
                    affected_table: Some(table.to_string()),
                    affected_column: Some(col_name.clone()),
                });
            }
        }
        
        // Check type compatibility between source and referenced columns
        for (source_col_name, ref_col_name) in constraint.columns.iter().zip(constraint.referenced_columns.iter()) {
            if let (Some(source_col), Some(target_col)) = (
                source_table.columns.iter().find(|c| c.name == *source_col_name),
                target_table.columns.iter().find(|c| c.name == *ref_col_name)
            ) {
                if source_col.column_type != target_col.column_type {
                    issues.push(RollbackValidationIssue {
                        operation_type: "add_foreign_key".to_string(),
                        description: format!(
                            "Column type mismatch: '{}' ({}) vs '{}' ({})",
                            source_col_name, source_col.column_type,
                            ref_col_name, target_col.column_type
                        ),
                        severity: RollbackRiskSeverity::Blocking,
                        mitigation: "Ensure column types match before adding foreign key constraint".to_string(),
                        affected_table: Some(table.to_string()),
                        affected_column: Some(source_col_name.clone()),
                    });
                }
            }
        }
        
        // Warn about data validation
        issues.push(RollbackValidationIssue {
            operation_type: "add_foreign_key".to_string(),
            description: "Adding foreign key constraint may fail if referential integrity is violated".to_string(),
            severity: RollbackRiskSeverity::High,
            mitigation: "Validate data integrity before adding foreign key constraint".to_string(),
            affected_table: Some(table.to_string()),
            affected_column: constraint.columns.first().cloned(),
        });
    }
    
    fn validate_drop_foreign_key_safety(&self, table: &str, constraint_name: &str, schema: &DatabaseSchema, issues: &mut Vec<RollbackValidationIssue>) {
        // Check if table exists
        if !self.table_exists(table, schema) {
            issues.push(RollbackValidationIssue {
                operation_type: "drop_foreign_key".to_string(),
                description: format!("Cannot drop foreign key from table '{}' - table does not exist", table),
                severity: RollbackRiskSeverity::Blocking,
                mitigation: "Verify table name and ensure table exists".to_string(),
                affected_table: Some(table.to_string()),
                affected_column: None,
            });
            return;
        }
        
        let table_schema = schema.tables.iter().find(|t| t.name == table).unwrap();
        
        // Check if foreign key constraint exists
        if !table_schema.foreign_keys.iter().any(|fk| fk.name == constraint_name) {
            issues.push(RollbackValidationIssue {
                operation_type: "drop_foreign_key".to_string(),
                description: format!("Foreign key constraint '{}' does not exist on table '{}'", constraint_name, table),
                severity: RollbackRiskSeverity::Blocking,
                mitigation: "Verify constraint name and ensure foreign key exists before dropping".to_string(),
                affected_table: Some(table.to_string()),
                affected_column: None,
            });
            return;
        }
        
        // Warn about referential integrity
        issues.push(RollbackValidationIssue {
            operation_type: "drop_foreign_key".to_string(),
            description: format!("Dropping foreign key constraint '{}' removes referential integrity protection", constraint_name),
            severity: RollbackRiskSeverity::Warning,
            mitigation: "Ensure application-level data integrity validation".to_string(),
            affected_table: Some(table.to_string()),
            affected_column: None,
        });
    }
    
    fn validate_rename_table_safety(&self, old_name: &str, new_name: &str, schema: &DatabaseSchema, issues: &mut Vec<RollbackValidationIssue>) {
        // Check if source table exists
        if !self.table_exists(old_name, schema) {
            issues.push(RollbackValidationIssue {
                operation_type: "rename_table".to_string(),
                description: format!("Cannot rename table '{}' - table does not exist", old_name),
                severity: RollbackRiskSeverity::Blocking,
                mitigation: "Verify table name and ensure table exists before renaming".to_string(),
                affected_table: Some(old_name.to_string()),
                affected_column: None,
            });
            return;
        }
        
        // Check if target name already exists
        if self.table_exists(new_name, schema) {
            issues.push(RollbackValidationIssue {
                operation_type: "rename_table".to_string(),
                description: format!("Cannot rename to '{}' - table name already exists", new_name),
                severity: RollbackRiskSeverity::Blocking,
                mitigation: "Choose a different name or drop existing table first".to_string(),
                affected_table: Some(old_name.to_string()),
                affected_column: None,
            });
        }
        
        // Check for foreign key references that will be affected
        for table in &schema.tables {
            for fk in &table.foreign_keys {
                if fk.referenced_table == old_name {
                    issues.push(RollbackValidationIssue {
                        operation_type: "rename_table".to_string(),
                        description: format!(
                            "Table '{}' is referenced by foreign key in table '{}'",
                            old_name, table.name
                        ),
                        severity: RollbackRiskSeverity::High,
                        mitigation: "Update foreign key references to use new table name".to_string(),
                        affected_table: Some(old_name.to_string()),
                        affected_column: None,
                    });
                }
            }
        }
        
        // Generally safe operation with proper referential updates
        issues.push(RollbackValidationIssue {
            operation_type: "rename_table".to_string(),
            description: format!("Renaming table from '{}' to '{}'", old_name, new_name),
            severity: RollbackRiskSeverity::Info,
            mitigation: "Update application code to use new table name".to_string(),
            affected_table: Some(old_name.to_string()),
            affected_column: None,
        });
    }
    
    fn validate_rename_column_safety(&self, table: &str, old_name: &str, new_name: &str, schema: &DatabaseSchema, issues: &mut Vec<RollbackValidationIssue>) {
        // Check if table exists
        if !self.table_exists(table, schema) {
            issues.push(RollbackValidationIssue {
                operation_type: "rename_column".to_string(),
                description: format!("Cannot rename column in table '{}' - table does not exist", table),
                severity: RollbackRiskSeverity::Blocking,
                mitigation: "Verify table name and ensure table exists".to_string(),
                affected_table: Some(table.to_string()),
                affected_column: Some(old_name.to_string()),
            });
            return;
        }
        
        let table_schema = schema.tables.iter().find(|t| t.name == table).unwrap();
        
        // Check if source column exists
        if !table_schema.columns.iter().any(|c| c.name == old_name) {
            issues.push(RollbackValidationIssue {
                operation_type: "rename_column".to_string(),
                description: format!("Column '{}' does not exist in table '{}'", old_name, table),
                severity: RollbackRiskSeverity::Blocking,
                mitigation: "Verify column name and ensure column exists before renaming".to_string(),
                affected_table: Some(table.to_string()),
                affected_column: Some(old_name.to_string()),
            });
            return;
        }
        
        // Check if target column name already exists
        if table_schema.columns.iter().any(|c| c.name == new_name) {
            issues.push(RollbackValidationIssue {
                operation_type: "rename_column".to_string(),
                description: format!("Column '{}' already exists in table '{}'", new_name, table),
                severity: RollbackRiskSeverity::Blocking,
                mitigation: "Choose a different column name or drop existing column first".to_string(),
                affected_table: Some(table.to_string()),
                affected_column: Some(old_name.to_string()),
            });
        }
        
        // Check if column is referenced by foreign keys in other tables
        for other_table in &schema.tables {
            for fk in &other_table.foreign_keys {
                if fk.referenced_table == table && fk.referenced_columns.contains(&old_name.to_string()) {
                    issues.push(RollbackValidationIssue {
                        operation_type: "rename_column".to_string(),
                        description: format!(
                            "Column '{}' is referenced by foreign key in table '{}'",
                            old_name, other_table.name
                        ),
                        severity: RollbackRiskSeverity::High,
                        mitigation: "Update foreign key references to use new column name".to_string(),
                        affected_table: Some(table.to_string()),
                        affected_column: Some(old_name.to_string()),
                    });
                }
            }
        }
        
        // Check if column is used in indexes
        for index in &table_schema.indexes {
            if index.columns.contains(&old_name.to_string()) {
                issues.push(RollbackValidationIssue {
                    operation_type: "rename_column".to_string(),
                    description: format!("Column '{}' is used in index '{}'", old_name, index.name),
                    severity: RollbackRiskSeverity::Warning,
                    mitigation: "Index will be updated to use new column name".to_string(),
                    affected_table: Some(table.to_string()),
                    affected_column: Some(old_name.to_string()),
                });
            }
        }
        
        // Generally safe operation
        issues.push(RollbackValidationIssue {
            operation_type: "rename_column".to_string(),
            description: format!("Renaming column from '{}' to '{}' in table '{}'", old_name, new_name, table),
            severity: RollbackRiskSeverity::Info,
            mitigation: "Update application code to use new column name".to_string(),
            affected_table: Some(table.to_string()),
            affected_column: Some(old_name.to_string()),
        });
    }
    
    // Dependency analysis helper methods
    
    fn build_dependency_graph(&self, operations: &[RollbackOperation]) -> HashMap<usize, Vec<usize>> {
        let mut graph = HashMap::new();
        
        for (i, operation) in operations.iter().enumerate() {
            let mut dependencies = Vec::new();
            
            match operation {
                RollbackOperation::RecreateTable { .. } => {
                    // Must happen after any operations that drop foreign keys referencing this table
                    for (j, other_op) in operations.iter().enumerate() {
                        if i != j {
                            match other_op {
                                RollbackOperation::DropForeignKey { .. } => {
                                    // Check if this FK references the table we're recreating
                                    // This is a simplified check - in practice would need more detailed analysis
                                    dependencies.push(j);
                                }
                                _ => {}
                            }
                        }
                    }
                }
                
                RollbackOperation::AddForeignKey { constraint, .. } => {
                    // Must happen after referenced table is created/recreated
                    for (j, other_op) in operations.iter().enumerate() {
                        if i != j {
                            match other_op {
                                RollbackOperation::RecreateTable { definition, .. } => {
                                    if &definition.name == &constraint.referenced_table {
                                        dependencies.push(j);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                
                RollbackOperation::AddColumn { table, column, .. } => {
                    // Must happen after table is created/recreated
                    for (j, other_op) in operations.iter().enumerate() {
                        if i != j {
                            match other_op {
                                RollbackOperation::RecreateTable { definition, .. } => {
                                    if &definition.name == table {
                                        dependencies.push(j);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    
                    // If column has FK constraint, must happen after referenced table exists
                    for constraint in &column.constraints {
                        if let crate::auto_migration::introspector::ColumnConstraint::References { table: ref_table, .. } = constraint {
                            for (j, other_op) in operations.iter().enumerate() {
                                if i != j {
                                    match other_op {
                                        RollbackOperation::RecreateTable { definition, .. } => {
                                            if &definition.name == ref_table {
                                                dependencies.push(j);
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
                
                RollbackOperation::CreateIndex { table, .. } => {
                    // Must happen after table and columns are created
                    for (j, other_op) in operations.iter().enumerate() {
                        if i != j {
                            match other_op {
                                RollbackOperation::RecreateTable { definition, .. } => {
                                    if &definition.name == table {
                                        dependencies.push(j);
                                    }
                                }
                                RollbackOperation::AddColumn { table: col_table, .. } => {
                                    if col_table == table {
                                        dependencies.push(j);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                
                _ => {
                    // Other operations have minimal dependencies
                }
            }
            
            if !dependencies.is_empty() {
                graph.insert(i, dependencies);
            }
        }
        
        graph
    }
    
    fn detect_circular_dependencies(&self, graph: &HashMap<usize, Vec<usize>>) -> Vec<CircularDependency> {
        let mut circular_deps = Vec::new();
        let mut visited = HashSet::new();
        let mut recursion_stack = HashSet::new();
        let mut path = Vec::new();
        
        for &node in graph.keys() {
            if !visited.contains(&node) {
                if let Some(cycle) = self.dfs_cycle_detection(node, graph, &mut visited, &mut recursion_stack, &mut path) {
                    circular_deps.push(CircularDependency {
                        operation_indices: cycle.clone(),
                        cycle_description: format!(
                            "Circular dependency detected: operations {} form a dependency cycle",
                            cycle.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(" -> ")
                        ),
                    });
                }
            }
        }
        
        circular_deps
    }
    
    fn dfs_cycle_detection(
        &self,
        node: usize,
        graph: &HashMap<usize, Vec<usize>>,
        visited: &mut HashSet<usize>,
        recursion_stack: &mut HashSet<usize>,
        path: &mut Vec<usize>,
    ) -> Option<Vec<usize>> {
        visited.insert(node);
        recursion_stack.insert(node);
        path.push(node);
        
        if let Some(dependencies) = graph.get(&node) {
            for &dep in dependencies {
                if !visited.contains(&dep) {
                    if let Some(cycle) = self.dfs_cycle_detection(dep, graph, visited, recursion_stack, path) {
                        return Some(cycle);
                    }
                } else if recursion_stack.contains(&dep) {
                    // Found a cycle - extract the cycle from the path
                    let cycle_start = path.iter().position(|&x| x == dep).unwrap();
                    let mut cycle = path[cycle_start..].to_vec();
                    cycle.push(dep); // Complete the cycle
                    return Some(cycle);
                }
            }
        }
        
        recursion_stack.remove(&node);
        path.pop();
        None
    }
    
    fn dependency_exists(&self, operations: &[RollbackOperation], dependency_index: usize) -> bool {
        dependency_index < operations.len()
    }
    
    fn detect_dependency_conflicts(&self, operations: &[RollbackOperation]) -> Vec<DependencyConflict> {
        let mut conflicts = Vec::new();
        
        // Detect operations that might conflict with each other
        for (i, op1) in operations.iter().enumerate() {
            for (j, op2) in operations.iter().enumerate() {
                if i >= j {
                    continue; // Only check each pair once
                }
                
                // Check for table-level conflicts
                let table1 = op1.affected_table();
                let table2 = op2.affected_table();
                
                if table1 == table2 {
                    match (op1, op2) {
                        (RollbackOperation::DropTable { .. }, RollbackOperation::RecreateTable { .. }) => {
                            // This is actually expected and should be ordered properly
                        }
                        (RollbackOperation::RecreateTable { .. }, RollbackOperation::AddColumn { .. }) => {
                            // Column addition after table recreation might conflict
                            conflicts.push(DependencyConflict {
                                conflicting_operations: vec![i, j],
                                conflict_description: format!(
                                    "Table recreation and column addition on '{}' may conflict",
                                    table1
                                ),
                                resolution_strategy: "Ensure column is included in table recreation definition".to_string(),
                            });
                        }
                        (RollbackOperation::DropColumn { .. }, RollbackOperation::AddColumn { .. }) => {
                            // Drop and add of columns on same table
                            conflicts.push(DependencyConflict {
                                conflicting_operations: vec![i, j],
                                conflict_description: format!(
                                    "Column drop and add operations on table '{}' may conflict",
                                    table1
                                ),
                                resolution_strategy: "Ensure proper ordering of column operations".to_string(),
                            });
                        }
                        _ => {}
                    }
                }
                
                // Check for foreign key conflicts
                match (op1, op2) {
                    (
                        RollbackOperation::AddForeignKey { constraint: fk1, .. },
                        RollbackOperation::DropTable { name, .. }
                    ) => {
                        if &fk1.referenced_table == name {
                            conflicts.push(DependencyConflict {
                                conflicting_operations: vec![i, j],
                                conflict_description: format!(
                                    "Adding foreign key referencing table '{}' conflicts with dropping that table",
                                    name
                                ),
                                resolution_strategy: "Reorder operations: drop foreign key before dropping table".to_string(),
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
        
        conflicts
    }
    
    // Schema compatibility helper methods
    
    fn table_exists(&self, table_name: &str, schema: &DatabaseSchema) -> bool {
        schema.tables.iter().any(|t| t.name == table_name)
    }
    
    fn check_column_type_compatibility(&self, table: &str, column: &ColumnSchema, schema: &DatabaseSchema) -> Option<TypeIncompatibility> {
        if let Some(current_table) = schema.tables.iter().find(|t| t.name == table) {
            if let Some(current_column) = current_table.columns.iter().find(|c| c.name == column.name) {
                if self.is_type_conversion_incompatible(&current_column.column_type, &column.column_type) {
                    return Some(TypeIncompatibility {
                        table: table.to_string(),
                        column: column.name.clone(),
                        source_type: current_column.column_type.clone(),
                        target_type: column.column_type.clone(),
                        is_lossy: self.is_lossy_conversion(&current_column.column_type, &column.column_type),
                        conversion_strategy: self.suggest_conversion_strategy(&current_column.column_type, &column.column_type),
                    });
                }
            }
        }
        None
    }
    
    fn check_index_recreation_capability(&self, table: &str, index: &crate::auto_migration::introspector::IndexSchema) -> Option<IndexCompatibilityIssue> {
        // Check for common index recreation issues
        
        // Check if index columns are still valid (this is a basic check)
        if index.columns.is_empty() {
            return Some(IndexCompatibilityIssue {
                table: table.to_string(),
                index_name: index.name.clone(),
                issue_description: "Index has no columns defined".to_string(),
                can_recreate: false,
            });
        }
        
        // Check if index name is valid
        if index.name.is_empty() {
            return Some(IndexCompatibilityIssue {
                table: table.to_string(),
                index_name: "<unnamed>".to_string(),
                issue_description: "Index has no name defined".to_string(),
                can_recreate: false,
            });
        }
        
        // For SQLite-specific checks, we could add more validation here
        // For now, assume most indexes can be recreated if they have proper definitions
        None
    }
    
    fn is_type_conversion_incompatible(&self, from: &str, to: &str) -> bool {
        // Define incompatible type conversions for SQLite
        match (from.to_uppercase().as_str(), to.to_uppercase().as_str()) {
            // TEXT to numeric types can fail if data is not numeric
            ("TEXT", "INTEGER") | ("TEXT", "REAL") => true,
            
            // BLOB to other types is generally incompatible
            ("BLOB", "TEXT") | ("BLOB", "INTEGER") | ("BLOB", "REAL") => true,
            ("TEXT", "BLOB") | ("INTEGER", "BLOB") | ("REAL", "BLOB") => true,
            
            // Numeric to TEXT is usually safe (implicit conversion)
            ("INTEGER", "TEXT") | ("REAL", "TEXT") => false,
            
            // REAL to INTEGER can lose precision but not incompatible
            ("REAL", "INTEGER") => false,
            
            // INTEGER to REAL is safe
            ("INTEGER", "REAL") => false,
            
            // Same types are always compatible
            (a, b) if a == b => false,
            
            // Default to potentially incompatible for unknown types
            _ => true,
        }
    }
    
    fn is_lossy_conversion(&self, from: &str, to: &str) -> bool {
        // Define lossy conversions for SQLite
        match (from.to_uppercase().as_str(), to.to_uppercase().as_str()) {
            // REAL to INTEGER loses decimal precision
            ("REAL", "INTEGER") => true,
            
            // TEXT to numeric can lose non-numeric data
            ("TEXT", "INTEGER") | ("TEXT", "REAL") => true,
            
            // BLOB conversions are always potentially lossy
            ("BLOB", _) | (_, "BLOB") => true,
            
            // Numeric to TEXT preserves data
            ("INTEGER", "TEXT") | ("REAL", "TEXT") => false,
            
            // INTEGER to REAL preserves data
            ("INTEGER", "REAL") => false,
            
            // Same types are not lossy
            (a, b) if a == b => false,
            
            // Default to potentially lossy for unknown types
            _ => true,
        }
    }
    
    fn suggest_conversion_strategy(&self, from: &str, to: &str) -> Option<String> {
        match (from.to_uppercase().as_str(), to.to_uppercase().as_str()) {
            ("REAL", "INTEGER") => Some("Use CAST() or ROUND() function to convert, data will be truncated".to_string()),
            ("TEXT", "INTEGER") => Some("Validate that all text values are numeric before conversion".to_string()),
            ("TEXT", "REAL") => Some("Validate that all text values are numeric before conversion".to_string()),
            ("INTEGER", "TEXT") => Some("Safe conversion using CAST() function".to_string()),
            ("REAL", "TEXT") => Some("Safe conversion using CAST() function".to_string()),
            ("INTEGER", "REAL") => Some("Safe conversion, no data loss".to_string()),
            ("BLOB", _) => Some("BLOB conversion requires custom handling based on data format".to_string()),
            (_, "BLOB") => Some("Consider if BLOB storage is appropriate for this data type".to_string()),
            (a, b) if a == b => Some("No conversion needed".to_string()),
            _ => Some("Manual data validation recommended before type conversion".to_string()),
        }
    }
    
    // Performance analysis helper methods
    
    fn estimate_table_size(&self, table: &str, schema: &DatabaseSchema) -> Option<f64> {
        // This is a basic estimation - in a real implementation, this would
        // query the database for actual table statistics
        if let Some(table_schema) = schema.tables.iter().find(|t| t.name == table) {
            // Basic estimation based on column count and types
            let column_count = table_schema.columns.len() as f64;
            let index_count = table_schema.indexes.len() as f64;
            
            // Rough estimation: assume 1KB per column per row, 1000 rows average
            // Plus index overhead
            let estimated_size_mb = (column_count * 1.0 * 1000.0 / 1024.0) + (index_count * 2.0);
            
            Some(estimated_size_mb.max(1.0)) // Minimum 1MB
        } else {
            None
        }
    }
    
    fn generate_mitigation_suggestions(&self, issues: &[RollbackValidationIssue], blocking_issues: &[RollbackBlockingIssue]) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        if !blocking_issues.is_empty() {
            suggestions.push("Resolve all blocking issues before attempting rollback".to_string());
        }
        
        let critical_count = issues.iter().filter(|i| i.severity == RollbackRiskSeverity::Critical).count();
        if critical_count > 0 {
            suggestions.push(format!("Review {} critical issues and ensure data backups exist", critical_count));
        }
        
        let high_count = issues.iter().filter(|i| i.severity == RollbackRiskSeverity::High).count();
        if high_count > 0 {
            suggestions.push(format!("Consider data preservation strategies for {} high-risk operations", high_count));
        }
        
        suggestions.push("Test rollback procedure in a non-production environment first".to_string());
        suggestions.push("Ensure adequate backup storage is available".to_string());
        
        suggestions
    }
}

impl Default for RollbackRiskAssessor {
    fn default() -> Self {
        Self::new()
    }
}