/// Database-agnostic migration rollback system
/// 
/// This module provides comprehensive rollback capabilities for safe migration recovery.
/// It generates inverse operations for migration plans while assessing data loss risks
/// and providing detailed warnings for manual intervention when required.

use crate::migration_engine::{
    MigrationPlan, MigrationOperation, TableOperation, ColumnOperation, 
    IndexOperation, ConstraintOperation, SafetyLevel
};
use crate::introspection::UnifiedTableSchema;
use crate::dialects::DatabaseDialect;
use serde::{Serialize, Deserialize};
use thiserror::Error;
use std::collections::HashMap;

/// Comprehensive error types for rollback operations
#[derive(Debug, Error, Clone, PartialEq)]
pub enum RollbackError {
    #[error("Cannot generate rollback: {0}")]
    CannotRollback(String),
    
    #[error("Data loss would occur: {0}")]
    DataLoss(String),
    
    #[error("Rollback validation failed: {0}")]
    ValidationFailed(String),
    
    #[error("Operation not reversible: {0}")]
    NotReversible(String),
    
    #[error("Manual intervention required: {0}")]
    ManualInterventionRequired(String),
    
    #[error("Schema information insufficient for rollback: {0}")]
    InsufficientSchemaInfo(String),
}

/// Risk assessment for potential data loss during rollback
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataLossRisk {
    /// No data loss expected
    None,
    /// Low risk with specific mitigation strategy
    Low(String),
    /// High risk with detailed warning
    High(String),
    /// Data loss is inevitable
    DataLossInevitable(String),
}

impl DataLossRisk {
    /// Check if this risk level represents potential data loss
    pub fn has_risk(&self) -> bool {
        !matches!(self, DataLossRisk::None)
    }
    
    /// Check if data loss is inevitable
    pub fn is_inevitable(&self) -> bool {
        matches!(self, DataLossRisk::DataLossInevitable(_))
    }
    
    /// Get risk description
    pub fn description(&self) -> &str {
        match self {
            DataLossRisk::None => "No data loss risk",
            DataLossRisk::Low(desc) => desc,
            DataLossRisk::High(desc) => desc,
            DataLossRisk::DataLossInevitable(desc) => desc,
        }
    }
}

/// Comprehensive rollback plan with detailed risk assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackPlan {
    /// Unique identifier linking to original migration plan
    pub original_plan_id: String,
    /// Rollback operations to execute (in reverse order)
    pub rollback_operations: Vec<MigrationOperation>,
    /// Overall data loss risk assessment
    pub data_loss_risk: DataLossRisk,
    /// Detailed warnings and recommendations
    pub rollback_warnings: Vec<String>,
    /// Whether manual intervention is required
    pub requires_manual_intervention: bool,
    /// Estimated execution time in seconds
    pub estimated_duration_seconds: u64,
    /// Overall safety level of rollback operations
    pub safety_level: SafetyLevel,
    /// Operations that cannot be rolled back automatically
    pub non_reversible_operations: Vec<String>,
    /// Schema information required for manual rollback
    pub manual_rollback_instructions: Vec<String>,
}

impl RollbackPlan {
    /// Check if this rollback plan is safe to execute automatically
    pub fn is_safe_to_execute(&self) -> bool {
        !self.requires_manual_intervention && 
        !self.data_loss_risk.is_inevitable() &&
        matches!(self.safety_level, SafetyLevel::Safe | SafetyLevel::LowRisk)
    }
    
    /// Check if rollback would result in data loss
    pub fn has_data_loss_risk(&self) -> bool {
        self.data_loss_risk.has_risk()
    }
    
    /// Get the number of operations that can be rolled back
    pub fn rollback_operations_count(&self) -> usize {
        self.rollback_operations.len()
    }
    
    /// Get the number of operations requiring manual intervention
    pub fn manual_operations_count(&self) -> usize {
        self.non_reversible_operations.len()
    }
}

/// Database-agnostic rollback generator
pub struct RollbackGenerator {
    /// Target database dialect
    #[allow(dead_code)] // Reserved for future dialect-specific rollback logic
    dialect: DatabaseDialect,
    /// Cache for table schemas needed for rollback generation
    #[allow(dead_code)] // Reserved for future schema caching optimization
    schema_cache: HashMap<String, UnifiedTableSchema>,
}

impl RollbackGenerator {
    /// Create new rollback generator for specified database dialect
    pub fn new(dialect: DatabaseDialect) -> Self {
        Self {
            dialect,
            schema_cache: HashMap::new(),
        }
    }
    
    /// Generate comprehensive rollback plan from original migration plan
    pub fn generate_rollback_plan(
        &self,
        original_plan: &MigrationPlan,
    ) -> Result<RollbackPlan, RollbackError> {
        let mut rollback_operations = Vec::new();
        let mut warnings = Vec::new();
        let mut data_loss_risk = DataLossRisk::None;
        let mut requires_manual_intervention = false;
        let mut non_reversible_operations = Vec::new();
        let mut manual_instructions = Vec::new();
        
        // Process operations in reverse order for proper rollback sequence
        for (index, operation) in original_plan.operations.iter().enumerate().rev() {
            match self.create_rollback_operation(operation) {
                Ok(rollback_op) => {
                    rollback_operations.push(rollback_op);
                },
                Err(RollbackError::DataLoss(msg)) => {
                    data_loss_risk = DataLossRisk::High(msg.clone());
                    warnings.push(format!("Operation {}: {}", index + 1, msg));
                    requires_manual_intervention = true;
                },
                Err(RollbackError::NotReversible(msg)) => {
                    non_reversible_operations.push(format!("Operation {}: {:?}", index + 1, operation));
                    warnings.push(msg.clone());
                    manual_instructions.push(
                        format!("Manual rollback required for operation {}: {}", index + 1, msg)
                    );
                    requires_manual_intervention = true;
                },
                Err(RollbackError::ManualInterventionRequired(msg)) => {
                    warnings.push(msg.clone());
                    manual_instructions.push(msg);
                    requires_manual_intervention = true;
                },
                Err(RollbackError::InsufficientSchemaInfo(msg)) => {
                    warnings.push(format!("Schema information missing: {}", msg));
                    manual_instructions.push(
                        format!("Provide schema information for proper rollback: {}", msg)
                    );
                    requires_manual_intervention = true;
                },
                Err(e) => return Err(e),
            }
        }
        
        // Calculate overall safety level
        let safety_level = if rollback_operations.is_empty() {
            SafetyLevel::Safe
        } else {
            // For now, simple operations like DropTable are considered safe
            // TODO: Implement proper safety assessment based on operation types
            if requires_manual_intervention || data_loss_risk != DataLossRisk::None {
                SafetyLevel::HighRisk
            } else {
                SafetyLevel::Safe
            }
        };
        
        // Estimate execution duration
        let estimated_duration_seconds = Self::estimate_rollback_duration(&rollback_operations);
        
        // Generate plan ID
        let original_plan_id = format!("rollback_{}", chrono::Utc::now().timestamp());
        
        Ok(RollbackPlan {
            original_plan_id,
            rollback_operations,
            data_loss_risk,
            rollback_warnings: warnings,
            requires_manual_intervention,
            estimated_duration_seconds,
            safety_level,
            non_reversible_operations,
            manual_rollback_instructions: manual_instructions,
        })
    }
    
    /// Create rollback operation for a single migration operation
    fn create_rollback_operation(
        &self,
        operation: &MigrationOperation,
    ) -> Result<MigrationOperation, RollbackError> {
        match operation {
            MigrationOperation::Table(table_op) => {
                self.create_table_rollback(table_op)
            },
            MigrationOperation::Column(column_op) => {
                self.create_column_rollback(column_op)
            },
            MigrationOperation::Index(index_op) => {
                self.create_index_rollback(index_op)
            },
            MigrationOperation::Constraint(constraint_op) => {
                self.create_constraint_rollback(constraint_op)
            },
        }
    }
    
    /// Create rollback operation for table operations
    fn create_table_rollback(
        &self,
        table_op: &TableOperation,
    ) -> Result<MigrationOperation, RollbackError> {
        match table_op {
            TableOperation::CreateTable { table_name, .. } => {
                // Rolling back table creation means dropping the table
                Ok(MigrationOperation::Table(TableOperation::DropTable {
                    table_name: table_name.clone(),
                }))
            },
            TableOperation::DropTable { table_name } => {
                // Cannot rollback table drop without original schema
                Err(RollbackError::DataLoss(
                    format!("Cannot recreate dropped table '{}' without original schema information", table_name)
                ))
            },
            TableOperation::RenameTable { old_name, new_name } => {
                // Reverse the rename operation
                Ok(MigrationOperation::Table(TableOperation::RenameTable {
                    old_name: new_name.clone(),
                    new_name: old_name.clone(),
                }))
            },
        }
    }
    
    /// Create rollback operation for column operations
    fn create_column_rollback(
        &self,
        column_op: &ColumnOperation,
    ) -> Result<MigrationOperation, RollbackError> {
        match column_op {
            ColumnOperation::AddColumn { table_name, .. } => {
                // Rolling back column addition means dropping the column
                // This could cause data loss if the column has data
                Ok(MigrationOperation::Column(ColumnOperation::DropColumn {
                    table_name: table_name.clone(),
                    column_name: column_op.get_column_name(),
                }))
            },
            ColumnOperation::DropColumn { table_name, column_name } => {
                // Cannot rollback column drop without original schema
                Err(RollbackError::DataLoss(
                    format!("Cannot recreate dropped column '{}' in table '{}' without original schema information", 
                            column_name, table_name)
                ))
            },
            ColumnOperation::ModifyColumn { table_name, old_column, new_column } => {
                // Reverse the column modification
                Ok(MigrationOperation::Column(ColumnOperation::ModifyColumn {
                    table_name: table_name.clone(),
                    old_column: new_column.clone(),
                    new_column: old_column.clone(),
                }))
            },
            ColumnOperation::RenameColumn { table_name, old_name, new_name } => {
                // Reverse the rename operation
                Ok(MigrationOperation::Column(ColumnOperation::RenameColumn {
                    table_name: table_name.clone(),
                    old_name: new_name.clone(),
                    new_name: old_name.clone(),
                }))
            },
        }
    }
    
    /// Create rollback operation for index operations
    fn create_index_rollback(
        &self,
        index_op: &IndexOperation,
    ) -> Result<MigrationOperation, RollbackError> {
        match index_op {
            IndexOperation::CreateIndex { table_name, .. } => {
                // Rolling back index creation means dropping the index
                Ok(MigrationOperation::Index(IndexOperation::DropIndex {
                    table_name: table_name.clone(),
                    index_name: index_op.get_index_name(),
                }))
            },
            IndexOperation::DropIndex { table_name, index_name } => {
                // Cannot rollback index drop without original schema
                Err(RollbackError::InsufficientSchemaInfo(
                    format!("Cannot recreate dropped index '{}' on table '{}' without original index definition", 
                            index_name, table_name)
                ))
            },
            IndexOperation::ModifyIndex { table_name, old_index, new_index } => {
                // Reverse the index modification
                Ok(MigrationOperation::Index(IndexOperation::ModifyIndex {
                    table_name: table_name.clone(),
                    old_index: new_index.clone(),
                    new_index: old_index.clone(),
                }))
            },
        }
    }
    
    /// Create rollback operation for constraint operations
    fn create_constraint_rollback(
        &self,
        constraint_op: &ConstraintOperation,
    ) -> Result<MigrationOperation, RollbackError> {
        match constraint_op {
            ConstraintOperation::AddConstraint { table_name, .. } => {
                // Rolling back constraint addition means dropping the constraint
                Ok(MigrationOperation::Constraint(ConstraintOperation::DropConstraint {
                    table_name: table_name.clone(),
                    constraint_name: constraint_op.get_constraint_name(),
                }))
            },
            ConstraintOperation::DropConstraint { table_name, constraint_name } => {
                // Cannot rollback constraint drop without original schema
                Err(RollbackError::InsufficientSchemaInfo(
                    format!("Cannot recreate dropped constraint '{}' on table '{}' without original constraint definition", 
                            constraint_name, table_name)
                ))
            },
            ConstraintOperation::ModifyConstraint { table_name, old_constraint, new_constraint } => {
                // Reverse the constraint modification
                Ok(MigrationOperation::Constraint(ConstraintOperation::ModifyConstraint {
                    table_name: table_name.clone(),
                    old_constraint: new_constraint.clone(),
                    new_constraint: old_constraint.clone(),
                }))
            },
        }
    }
    
    /// Estimate rollback execution duration based on operations
    fn estimate_rollback_duration(operations: &[MigrationOperation]) -> u64 {
        operations.iter().map(|op| match op {
            MigrationOperation::Table(TableOperation::CreateTable { .. }) => 5,
            MigrationOperation::Table(TableOperation::DropTable { .. }) => 2,
            MigrationOperation::Table(TableOperation::RenameTable { .. }) => 1,
            MigrationOperation::Column(ColumnOperation::AddColumn { .. }) => 3,
            MigrationOperation::Column(ColumnOperation::DropColumn { .. }) => 2,
            MigrationOperation::Column(ColumnOperation::ModifyColumn { .. }) => 5,
            MigrationOperation::Column(ColumnOperation::RenameColumn { .. }) => 1,
            MigrationOperation::Index(IndexOperation::CreateIndex { .. }) => 10,
            MigrationOperation::Index(IndexOperation::DropIndex { .. }) => 1,
            MigrationOperation::Index(IndexOperation::ModifyIndex { .. }) => 11,
            MigrationOperation::Constraint(_) => 3,
        }).sum()
    }
    
    /// Validate rollback plan for execution
    pub fn validate_rollback_plan(&self, plan: &RollbackPlan) -> Result<(), RollbackError> {
        // Check if manual intervention is required
        if plan.requires_manual_intervention {
            return Err(RollbackError::ManualInterventionRequired(
                "Rollback plan requires manual intervention before execution".to_string()
            ));
        }
        
        // Check for data loss risks
        if plan.data_loss_risk.is_inevitable() {
            return Err(RollbackError::DataLoss(
                plan.data_loss_risk.description().to_string()
            ));
        }
        
        // Validate operation sequence
        self.validate_operation_sequence(&plan.rollback_operations)?;
        
        Ok(())
    }
    
    /// Validate that rollback operations can be executed in sequence
    fn validate_operation_sequence(&self, operations: &[MigrationOperation]) -> Result<(), RollbackError> {
        // Check for operations that depend on each other
        for (i, operation) in operations.iter().enumerate() {
            match operation {
                MigrationOperation::Column(ColumnOperation::DropColumn { table_name, .. }) => {
                    // Check if table is being created later in sequence
                    if self.table_created_later(table_name, i, operations) {
                        return Err(RollbackError::ValidationFailed(
                            format!("Cannot drop column from table '{}' before table is created in rollback sequence", table_name)
                        ));
                    }
                },
                MigrationOperation::Index(IndexOperation::DropIndex { table_name, .. }) => {
                    // Check if table is being created later in sequence
                    if self.table_created_later(table_name, i, operations) {
                        return Err(RollbackError::ValidationFailed(
                            format!("Cannot drop index from table '{}' before table is created in rollback sequence", table_name)
                        ));
                    }
                },
                _ => {}
            }
        }
        
        Ok(())
    }
    
    /// Check if a table is created later in the operation sequence
    fn table_created_later(&self, table_name: &str, current_index: usize, operations: &[MigrationOperation]) -> bool {
        operations.iter().skip(current_index + 1).any(|op| {
            matches!(op, 
                MigrationOperation::Table(TableOperation::CreateTable { table_name: name, .. }) 
                if name == table_name
            )
        })
    }
}

/// Extension trait for extracting names from operations
trait OperationNameExtractor {
    fn get_column_name(&self) -> String;
    fn get_index_name(&self) -> String;
    fn get_constraint_name(&self) -> String;
}

impl OperationNameExtractor for ColumnOperation {
    fn get_column_name(&self) -> String {
        match self {
            ColumnOperation::AddColumn { column, .. } => column.name.clone(),
            ColumnOperation::DropColumn { column_name, .. } => column_name.clone(),
            ColumnOperation::ModifyColumn { new_column, .. } => new_column.name.clone(),
            ColumnOperation::RenameColumn { new_name, .. } => new_name.clone(),
        }
    }
    
    fn get_index_name(&self) -> String {
        unreachable!("Column operations don't have index names")
    }
    
    fn get_constraint_name(&self) -> String {
        unreachable!("Column operations don't have constraint names")
    }
}

impl OperationNameExtractor for IndexOperation {
    fn get_column_name(&self) -> String {
        unreachable!("Index operations don't have column names")
    }
    
    fn get_index_name(&self) -> String {
        match self {
            IndexOperation::CreateIndex { index, .. } => index.name.clone(),
            IndexOperation::DropIndex { index_name, .. } => index_name.clone(),
            IndexOperation::ModifyIndex { new_index, .. } => new_index.name.clone(),
        }
    }
    
    fn get_constraint_name(&self) -> String {
        unreachable!("Index operations don't have constraint names")
    }
}

impl OperationNameExtractor for ConstraintOperation {
    fn get_column_name(&self) -> String {
        unreachable!("Constraint operations don't have column names")
    }
    
    fn get_index_name(&self) -> String {
        unreachable!("Constraint operations don't have index names")
    }
    
    fn get_constraint_name(&self) -> String {
        match self {
            ConstraintOperation::AddConstraint { constraint, .. } => constraint.name.clone(),
            ConstraintOperation::DropConstraint { constraint_name, .. } => constraint_name.clone(),
            ConstraintOperation::ModifyConstraint { new_constraint, .. } => new_constraint.name.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::introspection::{UnifiedColumnType, UnifiedTableType, UnifiedColumnSchema};
    use std::collections::HashMap;
    
    fn create_test_column(name: &str) -> UnifiedColumnSchema {
        UnifiedColumnSchema {
            name: name.to_string(),
            column_type: UnifiedColumnType::Text,
            raw_type: "TEXT".to_string(),
            nullable: true,
            default_value: None,
            primary_key: false,
            auto_increment: false,
            unique: false,
            comment: None,
            ordinal_position: Some(1),
            character_maximum_length: None,
            numeric_precision: None,
            numeric_scale: None,
            constraints: vec![],
        }
    }
    
    fn create_test_table_schema(name: &str) -> UnifiedTableSchema {
        UnifiedTableSchema {
            name: name.to_string(),
            table_type: UnifiedTableType::Table,
            schema_name: None,
            comment: None,
            columns: vec![create_test_column("id")],
            indexes: vec![],
            foreign_keys: vec![],
            constraints: vec![],
            metadata: HashMap::new(),
        }
    }
    
    #[test]
    fn test_rollback_generator_creation() {
        let generator = RollbackGenerator::new(DatabaseDialect::SQLite);
        assert_eq!(generator.dialect, DatabaseDialect::SQLite);
    }
    
    #[test]
    fn test_data_loss_risk_assessment() {
        assert!(!DataLossRisk::None.has_risk());
        assert!(DataLossRisk::Low("test".to_string()).has_risk());
        assert!(DataLossRisk::High("test".to_string()).has_risk());
        assert!(DataLossRisk::DataLossInevitable("test".to_string()).has_risk());
        
        assert!(!DataLossRisk::None.is_inevitable());
        assert!(DataLossRisk::DataLossInevitable("test".to_string()).is_inevitable());
    }
    
    #[test]
    fn test_table_create_rollback() {
        let generator = RollbackGenerator::new(DatabaseDialect::SQLite);
        
        let create_op = TableOperation::CreateTable {
            table_name: "test_table".to_string(),
            schema: create_test_table_schema("test_table"),
        };
        
        let rollback = generator.create_table_rollback(&create_op).unwrap();
        
        match rollback {
            MigrationOperation::Table(TableOperation::DropTable { table_name }) => {
                assert_eq!(table_name, "test_table");
            },
            _ => panic!("Expected DropTable operation"),
        }
    }
    
    #[test]
    fn test_table_drop_rollback() {
        let generator = RollbackGenerator::new(DatabaseDialect::SQLite);
        
        let drop_op = TableOperation::DropTable {
            table_name: "test_table".to_string(),
        };
        
        let result = generator.create_table_rollback(&drop_op);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RollbackError::DataLoss(_)));
    }
    
    #[test]
    fn test_table_rename_rollback() {
        let generator = RollbackGenerator::new(DatabaseDialect::SQLite);
        
        let rename_op = TableOperation::RenameTable {
            old_name: "old_table".to_string(),
            new_name: "new_table".to_string(),
        };
        
        let rollback = generator.create_table_rollback(&rename_op).unwrap();
        
        match rollback {
            MigrationOperation::Table(TableOperation::RenameTable { old_name, new_name }) => {
                assert_eq!(old_name, "new_table");
                assert_eq!(new_name, "old_table");
            },
            _ => panic!("Expected RenameTable operation"),
        }
    }
    
    #[test]
    fn test_column_add_rollback() {
        let generator = RollbackGenerator::new(DatabaseDialect::SQLite);
        
        let add_op = ColumnOperation::AddColumn {
            table_name: "test_table".to_string(),
            column: create_test_column("new_column"),
        };
        
        let rollback = generator.create_column_rollback(&add_op).unwrap();
        
        match rollback {
            MigrationOperation::Column(ColumnOperation::DropColumn { table_name, column_name }) => {
                assert_eq!(table_name, "test_table");
                assert_eq!(column_name, "new_column");
            },
            _ => panic!("Expected DropColumn operation"),
        }
    }
    
    #[test]
    fn test_column_drop_rollback() {
        let generator = RollbackGenerator::new(DatabaseDialect::SQLite);
        
        let drop_op = ColumnOperation::DropColumn {
            table_name: "test_table".to_string(),
            column_name: "old_column".to_string(),
        };
        
        let result = generator.create_column_rollback(&drop_op);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RollbackError::DataLoss(_)));
    }
    
    #[test]
    fn test_rollback_plan_generation() {
        let generator = RollbackGenerator::new(DatabaseDialect::SQLite);
        
        let operations = vec![
            MigrationOperation::Table(TableOperation::CreateTable {
                table_name: "users".to_string(),
                schema: create_test_table_schema("users"),
            }),
            MigrationOperation::Column(ColumnOperation::AddColumn {
                table_name: "users".to_string(),
                column: create_test_column("email"),
            }),
        ];
        
        let original_plan = MigrationPlan::new(operations, DatabaseDialect::SQLite);
        let rollback_plan = generator.generate_rollback_plan(&original_plan).unwrap();
        
        // Should have rollback operations in reverse order
        assert_eq!(rollback_plan.rollback_operations.len(), 2);
        
        // First rollback operation should drop the column
        match &rollback_plan.rollback_operations[0] {
            MigrationOperation::Column(ColumnOperation::DropColumn { table_name, column_name }) => {
                assert_eq!(table_name, "users");
                assert_eq!(column_name, "email");
            },
            _ => panic!("Expected DropColumn as first rollback operation"),
        }
        
        // Second rollback operation should drop the table
        match &rollback_plan.rollback_operations[1] {
            MigrationOperation::Table(TableOperation::DropTable { table_name }) => {
                assert_eq!(table_name, "users");
            },
            _ => panic!("Expected DropTable as second rollback operation"),
        }
    }
    
    #[test]
    fn test_rollback_plan_safety_assessment() {
        let generator = RollbackGenerator::new(DatabaseDialect::SQLite);
        
        // Safe operations only
        let safe_operations = vec![
            MigrationOperation::Table(TableOperation::CreateTable {
                table_name: "users".to_string(),
                schema: create_test_table_schema("users"),
            }),
        ];
        
        let safe_plan = MigrationPlan::new(safe_operations, DatabaseDialect::SQLite);
        let rollback_plan = generator.generate_rollback_plan(&safe_plan).unwrap();
        
        assert!(rollback_plan.is_safe_to_execute());
        assert!(!rollback_plan.requires_manual_intervention);
    }
    
    #[test]
    fn test_rollback_plan_validation() {
        let generator = RollbackGenerator::new(DatabaseDialect::SQLite);
        
        // Create a rollback plan that should pass validation
        let rollback_plan = RollbackPlan {
            original_plan_id: "test".to_string(),
            rollback_operations: vec![
                MigrationOperation::Table(TableOperation::DropTable {
                    table_name: "test".to_string(),
                }),
            ],
            data_loss_risk: DataLossRisk::None,
            rollback_warnings: vec![],
            requires_manual_intervention: false,
            estimated_duration_seconds: 10,
            safety_level: SafetyLevel::Safe,
            non_reversible_operations: vec![],
            manual_rollback_instructions: vec![],
        };
        
        assert!(generator.validate_rollback_plan(&rollback_plan).is_ok());
    }
    
    #[test]
    fn test_rollback_plan_validation_failure() {
        let generator = RollbackGenerator::new(DatabaseDialect::SQLite);
        
        // Create a rollback plan that requires manual intervention
        let rollback_plan = RollbackPlan {
            original_plan_id: "test".to_string(),
            rollback_operations: vec![],
            data_loss_risk: DataLossRisk::DataLossInevitable("Data will be lost".to_string()),
            rollback_warnings: vec![],
            requires_manual_intervention: true,
            estimated_duration_seconds: 0,
            safety_level: SafetyLevel::Destructive,
            non_reversible_operations: vec![],
            manual_rollback_instructions: vec![],
        };
        
        assert!(generator.validate_rollback_plan(&rollback_plan).is_err());
    }
    
    #[test]
    fn test_operation_sequence_validation() {
        let generator = RollbackGenerator::new(DatabaseDialect::SQLite);
        
        // Valid sequence: drop column before creating table
        let valid_ops = vec![
            MigrationOperation::Column(ColumnOperation::DropColumn {
                table_name: "users".to_string(),
                column_name: "temp".to_string(),
            }),
            MigrationOperation::Table(TableOperation::CreateTable {
                table_name: "users".to_string(),
                schema: create_test_table_schema("users"),
            }),
        ];
        
        // This should fail validation because we're dropping a column before creating the table
        let result = generator.validate_operation_sequence(&valid_ops);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_estimate_rollback_duration() {
        let operations = vec![
            MigrationOperation::Table(TableOperation::CreateTable {
                table_name: "test".to_string(),
                schema: create_test_table_schema("test"),
            }),
            MigrationOperation::Column(ColumnOperation::AddColumn {
                table_name: "test".to_string(),
                column: create_test_column("col"),
            }),
        ];
        
        let duration = RollbackGenerator::estimate_rollback_duration(&operations);
        assert_eq!(duration, 8); // 5 for create table + 3 for add column
    }
}