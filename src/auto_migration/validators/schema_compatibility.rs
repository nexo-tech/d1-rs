//! Schema compatibility validation
//! 
//! Ensures schema changes maintain compatibility with existing code and don't break functionality.

use crate::Result;
use super::super::{MigrationPlan, MigrationOperation};
use super::super::introspector::{ColumnSchema, TableSchema, IndexSchema, ForeignKeySchema};

/// Schema compatibility validator
pub struct SchemaCompatibilityValidator;

impl SchemaCompatibilityValidator {
    pub fn new() -> Self {
        Self
    }
    
    /// Validate schema compatibility for entire migration plan
    pub fn validate_compatibility(&self, plan: &MigrationPlan) -> Result<SchemaCompatibilityResult> {
        let mut result = SchemaCompatibilityResult::new();
        
        for operation in &plan.operations {
            self.validate_operation_compatibility(operation, &mut result)?;
        }
        
        self.analyze_cross_operation_compatibility(&plan.operations, &mut result)?;
        
        Ok(result)
    }
    
    /// Validate compatibility for individual operation
    fn validate_operation_compatibility(&self, operation: &MigrationOperation, result: &mut SchemaCompatibilityResult) -> Result<()> {
        match operation {
            MigrationOperation::CreateTable { definition } => {
                self.validate_create_table_compatibility(definition, result)?;
            }
            MigrationOperation::DropTable { name } => {
                self.validate_drop_table_compatibility(name, result)?;
            }
            MigrationOperation::AddColumn { table, column } => {
                self.validate_add_column_compatibility(table, column, result)?;
            }
            MigrationOperation::DropColumn { table, column } => {
                self.validate_drop_column_compatibility(table, column, result)?;
            }
            MigrationOperation::ModifyColumn { table, column, changes } => {
                self.validate_modify_column_compatibility(table, column, changes, result)?;
            }
            MigrationOperation::CreateIndex { table, index } => {
                self.validate_create_index_compatibility(table, index, result)?;
            }
            MigrationOperation::DropIndex { name } => {
                self.validate_drop_index_compatibility(name, result)?;
            }
            MigrationOperation::AddForeignKey { constraint } => {
                self.validate_add_foreign_key_compatibility(constraint, result)?;
            }
            MigrationOperation::DropForeignKey { table, constraint_name } => {
                self.validate_drop_foreign_key_compatibility(table, constraint_name, result)?;
            }
            MigrationOperation::RenameTable { old_name, new_name } => {
                self.validate_rename_table_compatibility(old_name, new_name, result)?;
            }
            MigrationOperation::RenameColumn { table, old_name, new_name } => {
                self.validate_rename_column_compatibility(table, old_name, new_name, result)?;
            }
        }
        
        Ok(())
    }
    
    /// Validate table creation compatibility
    fn validate_create_table_compatibility(&self, definition: &TableSchema, result: &mut SchemaCompatibilityResult) -> Result<()> {
        // Check for reserved names or conflicting patterns
        if self.is_reserved_table_name(&definition.name) {
            result.add_incompatibility(
                SchemaIncompatibility::ReservedName {
                    entity_type: "table".to_string(),
                    name: definition.name.clone(),
                }
            );
        }
        
        // Validate column compatibility within table
        for column in &definition.columns {
            if self.is_reserved_column_name(&column.name) {
                result.add_incompatibility(
                    SchemaIncompatibility::ReservedName {
                        entity_type: "column".to_string(),
                        name: format!("{}.{}", definition.name, column.name),
                    }
                );
            }
            
            // Check for potentially problematic column types
            self.validate_column_type_compatibility(&definition.name, column, result)?;
        }
        
        Ok(())
    }
    
    /// Validate table dropping compatibility
    fn validate_drop_table_compatibility(&self, name: &str, result: &mut SchemaCompatibilityResult) -> Result<()> {
        // Dropping tables is always a breaking change
        result.add_incompatibility(
            SchemaIncompatibility::BreakingChange {
                change_type: "drop_table".to_string(),
                description: format!("Dropping table '{}' will break code that references it", name),
                severity: CompatibilitySeverity::Critical,
            }
        );
        
        Ok(())
    }
    
    /// Validate column addition compatibility
    fn validate_add_column_compatibility(&self, table: &str, column: &ColumnSchema, result: &mut SchemaCompatibilityResult) -> Result<()> {
        // Adding non-nullable columns without defaults can break existing inserts
        if !column.nullable && column.default_value.is_none() && !column.auto_increment {
            result.add_incompatibility(
                SchemaIncompatibility::BreakingChange {
                    change_type: "add_non_nullable_column".to_string(),
                    description: format!(
                        "Adding non-nullable column '{}' to table '{}' without default value will break existing INSERT statements",
                        column.name, table
                    ),
                    severity: CompatibilitySeverity::High,
                }
            );
        }
        
        // Check for reserved column names
        if self.is_reserved_column_name(&column.name) {
            result.add_incompatibility(
                SchemaIncompatibility::ReservedName {
                    entity_type: "column".to_string(),
                    name: format!("{}.{}", table, column.name),
                }
            );
        }
        
        Ok(())
    }
    
    /// Validate column dropping compatibility
    fn validate_drop_column_compatibility(&self, table: &str, column: &str, result: &mut SchemaCompatibilityResult) -> Result<()> {
        // Dropping columns is always a breaking change
        result.add_incompatibility(
            SchemaIncompatibility::BreakingChange {
                change_type: "drop_column".to_string(),
                description: format!("Dropping column '{}.{}' will break code that references it", table, column),
                severity: CompatibilitySeverity::Critical,
            }
        );
        
        Ok(())
    }
    
    /// Validate column modification compatibility
    fn validate_modify_column_compatibility(&self, table: &str, column: &str, changes: &super::super::ColumnChanges, result: &mut SchemaCompatibilityResult) -> Result<()> {
        // Type changes can break compatibility
        if let Some((old_type, new_type)) = &changes.type_change {
            if !self.are_types_compatible(old_type, new_type) {
                result.add_incompatibility(
                    SchemaIncompatibility::BreakingChange {
                        change_type: "incompatible_type_change".to_string(),
                        description: format!(
                            "Changing column '{}.{}' from {} to {} may break existing code",
                            table, column, old_type, new_type
                        ),
                        severity: CompatibilitySeverity::High,
                    }
                );
            }
        }
        
        // Making columns non-nullable can break existing data
        if let Some((was_nullable, is_nullable)) = changes.null_change {
            if was_nullable && !is_nullable {
                result.add_incompatibility(
                    SchemaIncompatibility::BreakingChange {
                        change_type: "remove_nullability".to_string(),
                        description: format!(
                            "Making column '{}.{}' non-nullable may break existing data with NULL values",
                            table, column
                        ),
                        severity: CompatibilitySeverity::High,
                    }
                );
            }
        }
        
        Ok(())
    }
    
    /// Validate index creation compatibility
    fn validate_create_index_compatibility(&self, table: &str, index: &IndexSchema, result: &mut SchemaCompatibilityResult) -> Result<()> {
        // Creating unique indexes on existing data can fail
        if index.unique {
            result.add_incompatibility(
                SchemaIncompatibility::PotentialFailure {
                    operation: "create_unique_index".to_string(),
                    reason: format!(
                        "Creating unique index '{}' on table '{}' may fail if duplicate values exist",
                        index.name, table
                    ),
                }
            );
        }
        
        Ok(())
    }
    
    /// Validate index dropping compatibility  
    fn validate_drop_index_compatibility(&self, name: &str, result: &mut SchemaCompatibilityResult) -> Result<()> {
        // Dropping indexes can impact query performance but doesn't break functionality
        result.add_incompatibility(
            SchemaIncompatibility::PerformanceImpact {
                operation: "drop_index".to_string(),
                impact: format!("Dropping index '{}' may significantly impact query performance", name),
                severity: CompatibilitySeverity::Medium,
            }
        );
        
        Ok(())
    }
    
    /// Validate foreign key addition compatibility
    fn validate_add_foreign_key_compatibility(&self, constraint: &ForeignKeySchema, result: &mut SchemaCompatibilityResult) -> Result<()> {
        // Adding foreign keys to existing data can fail
        result.add_incompatibility(
            SchemaIncompatibility::PotentialFailure {
                operation: "add_foreign_key".to_string(),
                reason: format!(
                    "Adding foreign key '{}' may fail if referential integrity is violated by existing data",
                    constraint.name
                ),
            }
        );
        
        Ok(())
    }
    
    /// Validate foreign key dropping compatibility
    fn validate_drop_foreign_key_compatibility(&self, table: &str, constraint_name: &str, result: &mut SchemaCompatibilityResult) -> Result<()> {
        // Dropping foreign keys removes data integrity constraints
        result.add_incompatibility(
            SchemaIncompatibility::DataIntegrityRisk {
                operation: "drop_foreign_key".to_string(),
                risk: format!(
                    "Dropping foreign key '{}' from table '{}' removes referential integrity protection",
                    constraint_name, table
                ),
            }
        );
        
        Ok(())
    }
    
    /// Validate table renaming compatibility
    fn validate_rename_table_compatibility(&self, old_name: &str, new_name: &str, result: &mut SchemaCompatibilityResult) -> Result<()> {
        // Table renames break all existing references
        result.add_incompatibility(
            SchemaIncompatibility::BreakingChange {
                change_type: "rename_table".to_string(),
                description: format!(
                    "Renaming table from '{}' to '{}' will break all existing code references",
                    old_name, new_name
                ),
                severity: CompatibilitySeverity::Critical,
            }
        );
        
        Ok(())
    }
    
    /// Validate column renaming compatibility
    fn validate_rename_column_compatibility(&self, table: &str, old_name: &str, new_name: &str, result: &mut SchemaCompatibilityResult) -> Result<()> {
        // Column renames break existing references
        result.add_incompatibility(
            SchemaIncompatibility::BreakingChange {
                change_type: "rename_column".to_string(),
                description: format!(
                    "Renaming column from '{}.{}' to '{}.{}' will break existing code references",
                    table, old_name, table, new_name
                ),
                severity: CompatibilitySeverity::High,
            }
        );
        
        Ok(())
    }
    
    /// Analyze compatibility across multiple operations
    fn analyze_cross_operation_compatibility(&self, operations: &[MigrationOperation], result: &mut SchemaCompatibilityResult) -> Result<()> {
        // Check for operations that depend on each other
        for (i, op1) in operations.iter().enumerate() {
            for op2 in operations.iter().skip(i + 1) {
                self.check_operation_dependencies(op1, op2, result)?;
            }
        }
        
        Ok(())
    }
    
    /// Check dependencies between two operations
    fn check_operation_dependencies(&self, op1: &MigrationOperation, op2: &MigrationOperation, result: &mut SchemaCompatibilityResult) -> Result<()> {
        // Example: Creating foreign key after creating referenced table
        match (op1, op2) {
            (MigrationOperation::AddForeignKey { constraint }, MigrationOperation::CreateTable { definition }) => {
                if constraint.referenced_table == definition.name {
                    result.add_incompatibility(
                        SchemaIncompatibility::OrderingIssue {
                            description: format!(
                                "Foreign key '{}' references table '{}' which is created later in the migration",
                                constraint.name, definition.name
                            ),
                        }
                    );
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Check if table name is reserved
    fn is_reserved_table_name(&self, name: &str) -> bool {
        matches!(name.to_lowercase().as_str(), 
            "user" | "order" | "group" | "index" | "table" | "column" | 
            "database" | "schema" | "constraint" | "trigger" | "view" |
            "sqlite_master" | "sqlite_sequence" | "sqlite_temp_master"
        )
    }
    
    /// Check if column name is reserved
    fn is_reserved_column_name(&self, name: &str) -> bool {
        matches!(name.to_lowercase().as_str(),
            "rowid" | "oid" | "_rowid_" | "sqlite_autoindex"
        )
    }
    
    /// Check if two types are compatible
    fn are_types_compatible(&self, old_type: &str, new_type: &str) -> bool {
        match (old_type.to_uppercase().as_str(), new_type.to_uppercase().as_str()) {
            // Widening numeric types is generally safe
            ("INTEGER", "REAL") => true,
            ("INTEGER", "TEXT") => true,
            ("REAL", "TEXT") => true,
            
            // Narrowing is risky
            ("REAL", "INTEGER") => false,
            ("TEXT", "INTEGER") => false,
            ("TEXT", "REAL") => false,
            
            // Same types are always compatible
            (a, b) if a == b => true,
            
            // Everything else is potentially incompatible
            _ => false,
        }
    }
    
    /// Validate column type compatibility
    fn validate_column_type_compatibility(&self, table: &str, column: &ColumnSchema, result: &mut SchemaCompatibilityResult) -> Result<()> {
        // Check for potentially problematic type combinations
        if column.column_type == "TEXT" && column.auto_increment {
            result.add_incompatibility(
                SchemaIncompatibility::LogicalError {
                    description: format!(
                        "Column '{}.{}' has TEXT type with auto_increment, which is not supported",
                        table, column.name
                    ),
                }
            );
        }
        
        Ok(())
    }
}

/// Schema compatibility validation result
#[derive(Debug, Clone)]
pub struct SchemaCompatibilityResult {
    /// List of detected incompatibilities
    pub incompatibilities: Vec<SchemaIncompatibility>,
    
    /// Whether the schema changes are compatible
    pub is_compatible: bool,
    
    /// Compatibility warnings that don't prevent migration
    pub warnings: Vec<String>,
}

impl SchemaCompatibilityResult {
    fn new() -> Self {
        Self {
            incompatibilities: Vec::new(),
            is_compatible: true,
            warnings: Vec::new(),
        }
    }
    
    fn add_incompatibility(&mut self, incompatibility: SchemaIncompatibility) {
        let is_critical = matches!(incompatibility, 
            SchemaIncompatibility::BreakingChange { severity: CompatibilitySeverity::Critical, .. } |
            SchemaIncompatibility::LogicalError { .. }
        );
        
        if is_critical {
            self.is_compatible = false;
        }
        
        self.incompatibilities.push(incompatibility);
    }
    
    /// Get critical incompatibilities that prevent migration
    pub fn critical_incompatibilities(&self) -> Vec<&SchemaIncompatibility> {
        self.incompatibilities.iter()
            .filter(|inc| inc.is_critical())
            .collect()
    }
}

/// Types of schema incompatibilities
#[derive(Debug, Clone)]
pub enum SchemaIncompatibility {
    /// Breaking changes that will break existing code
    BreakingChange {
        change_type: String,
        description: String,
        severity: CompatibilitySeverity,
    },
    
    /// Reserved names that may cause conflicts
    ReservedName {
        entity_type: String,
        name: String,
    },
    
    /// Operations that may fail during execution
    PotentialFailure {
        operation: String,
        reason: String,
    },
    
    /// Performance impacts
    PerformanceImpact {
        operation: String,
        impact: String,
        severity: CompatibilitySeverity,
    },
    
    /// Data integrity risks
    DataIntegrityRisk {
        operation: String,
        risk: String,
    },
    
    /// Logical errors in schema definition
    LogicalError {
        description: String,
    },
    
    /// Operation ordering issues
    OrderingIssue {
        description: String,
    },
}

impl SchemaIncompatibility {
    fn is_critical(&self) -> bool {
        match self {
            SchemaIncompatibility::BreakingChange { severity, .. } => {
                *severity == CompatibilitySeverity::Critical
            }
            SchemaIncompatibility::LogicalError { .. } => true,
            SchemaIncompatibility::ReservedName { .. } => true,
            _ => false,
        }
    }
}

/// Severity levels for compatibility issues
#[derive(Debug, Clone, PartialEq)]
pub enum CompatibilitySeverity {
    Low,
    Medium,
    High,
    Critical,
}