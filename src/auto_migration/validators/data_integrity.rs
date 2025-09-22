//! Data integrity validation
//! 
//! Ensures migrations don't cause data loss or corruption during schema changes.

use crate::Result;
use super::super::{MigrationPlan, MigrationOperation};
use super::super::introspector::{ColumnSchema, TableSchema};

/// Data integrity validator
pub struct DataIntegrityValidator;

impl DataIntegrityValidator {
    pub fn new() -> Self {
        Self
    }
    
    /// Validate data integrity for entire migration plan
    pub fn validate_integrity(&self, plan: &MigrationPlan) -> Result<DataIntegrityResult> {
        let mut result = DataIntegrityResult::new();
        
        for operation in &plan.operations {
            self.validate_operation_integrity(operation, &mut result)?;
        }
        
        self.analyze_data_preservation(&plan.operations, &mut result)?;
        
        Ok(result)
    }
    
    /// Validate data integrity for individual operation
    fn validate_operation_integrity(&self, operation: &MigrationOperation, result: &mut DataIntegrityResult) -> Result<()> {
        match operation {
            MigrationOperation::CreateTable { definition } => {
                self.validate_create_table_integrity(definition, result)?;
            }
            MigrationOperation::DropTable { name } => {
                self.validate_drop_table_integrity(name, result)?;
            }
            MigrationOperation::AddColumn { table, column } => {
                self.validate_add_column_integrity(table, column, result)?;
            }
            MigrationOperation::DropColumn { table, column } => {
                self.validate_drop_column_integrity(table, column, result)?;
            }
            MigrationOperation::ModifyColumn { table, column, changes } => {
                self.validate_modify_column_integrity(table, column, changes, result)?;
            }
            MigrationOperation::CreateIndex { table, index } => {
                self.validate_create_index_integrity(table, index, result)?;
            }
            MigrationOperation::DropIndex { name } => {
                self.validate_drop_index_integrity(name, result)?;
            }
            MigrationOperation::AddForeignKey { constraint } => {
                self.validate_add_foreign_key_integrity(constraint, result)?;
            }
            MigrationOperation::DropForeignKey { table, constraint_name } => {
                self.validate_drop_foreign_key_integrity(table, constraint_name, result)?;
            }
            MigrationOperation::RenameTable { old_name, new_name } => {
                self.validate_rename_table_integrity(old_name, new_name, result)?;
            }
            MigrationOperation::RenameColumn { table, old_name, new_name } => {
                self.validate_rename_column_integrity(table, old_name, new_name, result)?;
            }
        }
        
        Ok(())
    }
    
    /// Validate table creation integrity
    fn validate_create_table_integrity(&self, definition: &TableSchema, result: &mut DataIntegrityResult) -> Result<()> {
        // Check for potential constraint violations
        let mut primary_key_count = 0;
        let mut unique_columns = Vec::new();
        
        for column in &definition.columns {
            if column.primary_key {
                primary_key_count += 1;
            }
            
            if column.unique {
                unique_columns.push(&column.name);
            }
            
            // Check for logical inconsistencies
            if column.nullable && column.primary_key {
                result.add_integrity_issue(
                    IntegrityIssue::ConstraintViolation {
                        table: definition.name.clone(),
                        description: format!(
                            "Primary key column '{}' cannot be nullable",
                            column.name
                        ),
                        severity: IntegritySeverity::High,
                    }
                );
            }
            
            // Check auto_increment constraints
            if column.auto_increment && !column.primary_key && column.column_type != "INTEGER" {
                result.add_integrity_issue(
                    IntegrityIssue::ConstraintViolation {
                        table: definition.name.clone(),
                        description: format!(
                            "Auto-increment column '{}' should be INTEGER PRIMARY KEY",
                            column.name
                        ),
                        severity: IntegritySeverity::Medium,
                    }
                );
            }
        }
        
        // Tables should have exactly one primary key
        if primary_key_count == 0 {
            result.add_integrity_issue(
                IntegrityIssue::MissingConstraint {
                    table: definition.name.clone(),
                    constraint_type: "primary_key".to_string(),
                    recommendation: "Consider adding a primary key for better data integrity".to_string(),
                }
            );
        } else if primary_key_count > 1 {
            result.add_integrity_issue(
                IntegrityIssue::ConstraintViolation {
                    table: definition.name.clone(),
                    description: format!("Table has {} primary key columns, only one is allowed", primary_key_count),
                    severity: IntegritySeverity::High,
                }
            );
        }
        
        Ok(())
    }
    
    /// Validate table dropping integrity
    fn validate_drop_table_integrity(&self, name: &str, result: &mut DataIntegrityResult) -> Result<()> {
        // Dropping tables causes permanent data loss
        result.add_integrity_issue(
            IntegrityIssue::DataLoss {
                operation: "drop_table".to_string(),
                affected_data: format!("All data in table '{}'", name),
                severity: IntegritySeverity::Critical,
                reversible: false,
            }
        );
        
        Ok(())
    }
    
    /// Validate column addition integrity
    fn validate_add_column_integrity(&self, table: &str, column: &ColumnSchema, result: &mut DataIntegrityResult) -> Result<()> {
        // Adding non-nullable columns without defaults to existing data
        if !column.nullable && column.default_value.is_none() && !column.auto_increment {
            result.add_integrity_issue(
                IntegrityIssue::ConstraintViolation {
                    table: table.to_string(),
                    description: format!(
                        "Adding non-nullable column '{}' without default value will fail if table contains existing rows",
                        column.name
                    ),
                    severity: IntegritySeverity::High,
                }
            );
        }
        
        // Check for potential unique constraint violations
        if column.unique {
            result.add_integrity_issue(
                IntegrityIssue::PotentialDataCorruption {
                    operation: "add_unique_column".to_string(),
                    risk: format!(
                        "Adding unique column '{}' to table '{}' with existing data may fail or require data cleanup",
                        column.name, table
                    ),
                    mitigation: "Ensure all existing rows will have unique values for this column".to_string(),
                }
            );
        }
        
        Ok(())
    }
    
    /// Validate column dropping integrity
    fn validate_drop_column_integrity(&self, table: &str, column: &str, result: &mut DataIntegrityResult) -> Result<()> {
        // Dropping columns causes permanent data loss
        result.add_integrity_issue(
            IntegrityIssue::DataLoss {
                operation: "drop_column".to_string(),
                affected_data: format!("All data in column '{}.{}'", table, column),
                severity: IntegritySeverity::Critical,
                reversible: false,
            }
        );
        
        Ok(())
    }
    
    /// Validate column modification integrity
    fn validate_modify_column_integrity(&self, table: &str, column: &str, changes: &super::super::ColumnChanges, result: &mut DataIntegrityResult) -> Result<()> {
        // Type changes can cause data loss or corruption
        if let Some((old_type, new_type)) = &changes.type_change {
            let conversion_safety = self.analyze_type_conversion_safety(old_type, new_type);
            
            match conversion_safety {
                TypeConversionSafety::Safe => {
                    // No issues
                }
                TypeConversionSafety::Lossy => {
                    result.add_integrity_issue(
                        IntegrityIssue::PotentialDataLoss {
                            operation: "type_conversion".to_string(),
                            description: format!(
                                "Converting column '{}.{}' from {} to {} may lose precision or data",
                                table, column, old_type, new_type
                            ),
                            affected_data: "Existing column values".to_string(),
                        }
                    );
                }
                TypeConversionSafety::Dangerous => {
                    result.add_integrity_issue(
                        IntegrityIssue::DataLoss {
                            operation: "unsafe_type_conversion".to_string(),
                            affected_data: format!("Column '{}.{}' data may be corrupted or lost", table, column),
                            severity: IntegritySeverity::High,
                            reversible: false,
                        }
                    );
                }
            }
        }
        
        // Making columns non-nullable can fail with existing NULL data
        if let Some((was_nullable, is_nullable)) = changes.null_change {
            if was_nullable && !is_nullable {
                result.add_integrity_issue(
                    IntegrityIssue::ConstraintViolation {
                        table: table.to_string(),
                        description: format!(
                            "Making column '{}.{}' non-nullable will fail if existing rows contain NULL values",
                            table, column
                        ),
                        severity: IntegritySeverity::High,
                    }
                );
            }
        }
        
        // Default value changes impact new inserts
        if let Some((old_default, new_default)) = &changes.default_change {
            match (old_default, new_default) {
                (Some(_), None) => {
                    result.add_integrity_issue(
                        IntegrityIssue::BehaviorChange {
                            operation: "remove_default_value".to_string(),
                            change: format!(
                                "Removing default value from column '{}.{}' changes insertion behavior",
                                table, column
                            ),
                            impact: "New inserts without explicit value will fail".to_string(),
                        }
                    );
                }
                (old_val, new_val) if old_val != new_val => {
                    result.add_integrity_issue(
                        IntegrityIssue::BehaviorChange {
                            operation: "change_default_value".to_string(),
                            change: format!(
                                "Changing default value for column '{}.{}' affects new inserts",
                                table, column
                            ),
                            impact: "New inserts without explicit value will use new default".to_string(),
                        }
                    );
                }
                _ => {}
            }
        }
        
        Ok(())
    }
    
    /// Validate index creation integrity
    fn validate_create_index_integrity(&self, table: &str, index: &super::super::introspector::IndexSchema, result: &mut DataIntegrityResult) -> Result<()> {
        // Unique indexes on existing data can fail
        if index.unique {
            result.add_integrity_issue(
                IntegrityIssue::PotentialDataCorruption {
                    operation: "create_unique_index".to_string(),
                    risk: format!(
                        "Creating unique index '{}' on table '{}' will fail if duplicate values exist",
                        index.name, table
                    ),
                    mitigation: "Ensure data uniqueness before creating index".to_string(),
                }
            );
        }
        
        Ok(())
    }
    
    /// Validate index dropping integrity
    fn validate_drop_index_integrity(&self, name: &str, result: &mut DataIntegrityResult) -> Result<()> {
        // Dropping indexes doesn't affect data but impacts performance
        result.add_integrity_issue(
            IntegrityIssue::PerformanceImpact {
                operation: "drop_index".to_string(),
                impact: format!("Dropping index '{}' may significantly slow queries", name),
                affected_queries: "Queries that rely on this index".to_string(),
            }
        );
        
        Ok(())
    }
    
    /// Validate foreign key addition integrity
    fn validate_add_foreign_key_integrity(&self, constraint: &super::super::introspector::ForeignKeySchema, result: &mut DataIntegrityResult) -> Result<()> {
        // Adding foreign keys to existing data can fail
        result.add_integrity_issue(
            IntegrityIssue::ConstraintViolation {
                table: "unknown".to_string(), // ForeignKeySchema doesn't store the source table
                description: format!(
                    "Adding foreign key '{}' will fail if existing data violates referential integrity",
                    constraint.name
                ),
                severity: IntegritySeverity::High,
            }
        );
        
        Ok(())
    }
    
    /// Validate foreign key dropping integrity
    fn validate_drop_foreign_key_integrity(&self, table: &str, constraint_name: &str, result: &mut DataIntegrityResult) -> Result<()> {
        // Dropping foreign keys removes data integrity protection
        result.add_integrity_issue(
            IntegrityIssue::IntegrityWeakening {
                operation: "drop_foreign_key".to_string(),
                description: format!(
                    "Dropping foreign key '{}' from table '{}' removes referential integrity protection",
                    constraint_name, table
                ),
                consequences: "Invalid references may be inserted without validation".to_string(),
            }
        );
        
        Ok(())
    }
    
    /// Validate table renaming integrity
    fn validate_rename_table_integrity(&self, old_name: &str, new_name: &str, result: &mut DataIntegrityResult) -> Result<()> {
        // Table renames preserve data but may break references
        result.add_integrity_issue(
            IntegrityIssue::BehaviorChange {
                operation: "rename_table".to_string(),
                change: format!("Table renamed from '{}' to '{}'", old_name, new_name),
                impact: "All references to old table name will break".to_string(),
            }
        );
        
        Ok(())
    }
    
    /// Validate column renaming integrity
    fn validate_rename_column_integrity(&self, table: &str, old_name: &str, new_name: &str, result: &mut DataIntegrityResult) -> Result<()> {
        // Column renames preserve data but may break references
        result.add_integrity_issue(
            IntegrityIssue::BehaviorChange {
                operation: "rename_column".to_string(),
                change: format!("Column renamed from '{}.{}' to '{}.{}'", table, old_name, table, new_name),
                impact: "All references to old column name will break".to_string(),
            }
        );
        
        Ok(())
    }
    
    /// Analyze data preservation across multiple operations
    fn analyze_data_preservation(&self, operations: &[MigrationOperation], result: &mut DataIntegrityResult) -> Result<()> {
        let mut data_loss_operations = 0;
        let mut tables_affected = std::collections::HashSet::new();
        
        for operation in operations {
            match operation {
                MigrationOperation::DropTable { name } => {
                    data_loss_operations += 1;
                    tables_affected.insert(name.clone());
                }
                MigrationOperation::DropColumn { table, .. } => {
                    data_loss_operations += 1;
                    tables_affected.insert(table.clone());
                }
                MigrationOperation::ModifyColumn { table, changes, .. } => {
                    if let Some((old_type, new_type)) = &changes.type_change {
                        if !matches!(self.analyze_type_conversion_safety(old_type, new_type), TypeConversionSafety::Safe) {
                            data_loss_operations += 1;
                            tables_affected.insert(table.clone());
                        }
                    }
                }
                _ => {}
            }
        }
        
        if data_loss_operations > 0 {
            result.add_integrity_issue(
                IntegrityIssue::MigrationRisk {
                    total_operations: operations.len(),
                    risky_operations: data_loss_operations,
                    affected_tables: tables_affected.len(),
                    recommendation: "Consider creating backup before executing migration".to_string(),
                }
            );
        }
        
        Ok(())
    }
    
    /// Analyze safety of type conversions
    fn analyze_type_conversion_safety(&self, from_type: &str, to_type: &str) -> TypeConversionSafety {
        match (from_type.to_uppercase().as_str(), to_type.to_uppercase().as_str()) {
            // Safe conversions (no data loss)
            ("INTEGER", "TEXT") => TypeConversionSafety::Safe,
            ("REAL", "TEXT") => TypeConversionSafety::Safe,
            ("INTEGER", "REAL") => TypeConversionSafety::Safe,
            
            // Lossy conversions (potential precision loss)
            ("REAL", "INTEGER") => TypeConversionSafety::Lossy,
            ("TEXT", "INTEGER") => TypeConversionSafety::Lossy,
            ("TEXT", "REAL") => TypeConversionSafety::Lossy,
            
            // Dangerous conversions
            ("BLOB", _) => TypeConversionSafety::Dangerous,
            (_, "BLOB") => TypeConversionSafety::Dangerous,
            
            // Same types are always safe
            (a, b) if a == b => TypeConversionSafety::Safe,
            
            // Unknown conversions are considered dangerous
            _ => TypeConversionSafety::Dangerous,
        }
    }
}

/// Data integrity validation result
#[derive(Debug, Clone)]
pub struct DataIntegrityResult {
    /// List of detected integrity issues
    pub integrity_issues: Vec<IntegrityIssue>,
    
    /// Whether the migration preserves data integrity
    pub preserves_integrity: bool,
    
    /// Overall data loss risk assessment
    pub data_loss_risk: DataLossRisk,
    
    /// Recommended actions to preserve integrity
    pub recommendations: Vec<String>,
}

impl DataIntegrityResult {
    fn new() -> Self {
        Self {
            integrity_issues: Vec::new(),
            preserves_integrity: true,
            data_loss_risk: DataLossRisk::None,
            recommendations: Vec::new(),
        }
    }
    
    fn add_integrity_issue(&mut self, issue: IntegrityIssue) {
        let affects_integrity = issue.is_critical();
        let has_data_loss = issue.involves_data_loss();
        
        if affects_integrity {
            self.preserves_integrity = false;
        }
        
        if has_data_loss {
            self.data_loss_risk = match self.data_loss_risk {
                DataLossRisk::None => DataLossRisk::Low,
                DataLossRisk::Low => DataLossRisk::Medium,
                DataLossRisk::Medium => DataLossRisk::High,
                DataLossRisk::High => DataLossRisk::Critical,
                DataLossRisk::Critical => DataLossRisk::Critical,
            };
        }
        
        self.integrity_issues.push(issue);
    }
    
    /// Check if there's risk of data loss
    pub fn has_data_loss_risk(&self) -> bool {
        self.data_loss_risk != DataLossRisk::None
    }
    
    /// Get critical integrity issues
    pub fn critical_issues(&self) -> Vec<&IntegrityIssue> {
        self.integrity_issues.iter()
            .filter(|issue| issue.is_critical())
            .collect()
    }
}

/// Types of data integrity issues
#[derive(Debug, Clone)]
pub enum IntegrityIssue {
    /// Permanent data loss
    DataLoss {
        operation: String,
        affected_data: String,
        severity: IntegritySeverity,
        reversible: bool,
    },
    
    /// Potential data loss or corruption
    PotentialDataLoss {
        operation: String,
        description: String,
        affected_data: String,
    },
    
    /// Data corruption risks
    PotentialDataCorruption {
        operation: String,
        risk: String,
        mitigation: String,
    },
    
    /// Constraint violations
    ConstraintViolation {
        table: String,
        description: String,
        severity: IntegritySeverity,
    },
    
    /// Missing important constraints
    MissingConstraint {
        table: String,
        constraint_type: String,
        recommendation: String,
    },
    
    /// Behavior changes that affect data handling
    BehaviorChange {
        operation: String,
        change: String,
        impact: String,
    },
    
    /// Performance impacts on data access
    PerformanceImpact {
        operation: String,
        impact: String,
        affected_queries: String,
    },
    
    /// Weakening of data integrity protections
    IntegrityWeakening {
        operation: String,
        description: String,
        consequences: String,
    },
    
    /// Overall migration risk assessment
    MigrationRisk {
        total_operations: usize,
        risky_operations: usize,
        affected_tables: usize,
        recommendation: String,
    },
}

impl IntegrityIssue {
    fn is_critical(&self) -> bool {
        match self {
            IntegrityIssue::DataLoss { severity, .. } => {
                matches!(severity, IntegritySeverity::High | IntegritySeverity::Critical)
            }
            IntegrityIssue::PotentialDataLoss { .. } => true,
            IntegrityIssue::ConstraintViolation { severity, .. } => {
                matches!(severity, IntegritySeverity::High | IntegritySeverity::Critical)
            }
            _ => false,
        }
    }
    
    fn involves_data_loss(&self) -> bool {
        matches!(self, 
            IntegrityIssue::DataLoss { .. } |
            IntegrityIssue::PotentialDataLoss { .. } |
            IntegrityIssue::PotentialDataCorruption { .. }
        )
    }
}

/// Severity levels for integrity issues
#[derive(Debug, Clone, PartialEq)]
pub enum IntegritySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Data loss risk levels
#[derive(Debug, Clone, PartialEq)]
pub enum DataLossRisk {
    None,
    Low,
    Medium,
    High,
    Critical,
}

/// Safety assessment for type conversions
#[derive(Debug, Clone, PartialEq)]
enum TypeConversionSafety {
    /// Conversion is safe, no data loss
    Safe,
    /// Conversion may lose precision but no data corruption
    Lossy,
    /// Conversion may cause data corruption or complete loss
    Dangerous,
}