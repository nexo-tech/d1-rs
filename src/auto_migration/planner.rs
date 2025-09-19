// Phase 2.1 - Revolutionary Migration Plan Generator
#![allow(unused_imports)] // Suppress false positive warnings for types used in tests
use crate::Result;
use super::{
    SchemaDiff, MigrationPlan, MigrationOperation, SafetyWarning, SafetyWarningType,
    TableChange, ChangeType, ColumnChanges, ColumnChange, ForeignKeyChange
};
use super::introspector::{ColumnSchema, TableSchema, ForeignKeySchema};
// ColumnChange and ForeignKeyChange are already in scope from parent module
use super::smart_strategies::SmartMigrationStrategies;
use std::time::Duration;

/// Revolutionary Migration Plan Generator - Converts schema diffs into safe executable plans
/// Features:
/// - Intelligent dependency ordering
/// - Comprehensive safety analysis  
/// - Automatic rollback generation
/// - Performance-aware duration estimation
#[derive(Clone)]
pub struct MigrationPlanner {
    /// Safety analysis configuration
    strict_mode: bool,
    /// Allow aggressive changes in development
    aggressive_mode: bool,
}

impl MigrationPlanner {
    pub fn new() -> Self {
        Self { 
            strict_mode: false,
            aggressive_mode: false,
        }
    }
    
    /// Enable aggressive changes for development environment
    pub fn enable_aggressive_changes(&mut self, enabled: bool) {
        self.aggressive_mode = enabled;
    }

    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    /// Generate safe migration plan from schema differences
    /// Implements intelligent ordering, safety analysis, and rollback generation
    pub fn plan_migrations(&self, diff: SchemaDiff) -> Result<MigrationPlan> {
        if diff.is_empty() {
            return Ok(MigrationPlan {
                operations: vec![],
                estimated_duration: Duration::from_secs(0),
                safety_warnings: vec![],
                rollback_plan: vec![],
            });
        }

        let mut operations = Vec::new();
        let mut safety_warnings = Vec::new();

        // Phase 1: Plan table creation/deletion operations
        for table_change in &diff.table_changes {
            self.plan_table_changes(table_change, &mut operations, &mut safety_warnings)?;
        }

        // Phase 2: Plan column operations for existing tables
        for table_change in &diff.table_changes {
            if table_change.change_type == ChangeType::Modify {
                self.plan_column_changes(table_change, &mut operations, &mut safety_warnings)?;
            }
        }

        // Phase 3: Plan index operations
        for table_change in &diff.table_changes {
            if table_change.change_type == ChangeType::Modify {
                self.plan_index_changes(table_change, &mut operations, &mut safety_warnings)?;
            }
        }

        // Phase 4: Plan foreign key operations (must come after tables/columns/indexes)
        for table_change in &diff.table_changes {
            if table_change.change_type == ChangeType::Modify {
                self.plan_relationship_changes(table_change, &mut operations, &mut safety_warnings)?;
            }
        }

        // Generate rollback plan (reverse operations)
        let rollback_plan = self.generate_rollback_plan(&operations, &diff)?;

        // Estimate migration duration
        let estimated_duration = self.estimate_duration(&operations);

        Ok(MigrationPlan {
            operations,
            estimated_duration,
            safety_warnings,
            rollback_plan,
        })
    }

    /// Generate enhanced migration plan using smart strategies
    /// This demonstrates integration with Phase 2.2 Smart Migration Strategies
    pub fn plan_migrations_with_smart_strategies(&self, diff: SchemaDiff) -> Result<super::smart_strategies::EnhancedMigrationPlan> {
        // First generate standard migration plan
        let standard_plan = self.plan_migrations(diff.clone())?;
        
        // Apply smart strategies to enhance the plan
        let smart_strategies = SmartMigrationStrategies::new()
            .with_rename_threshold(0.75)
            .with_table_restructuring(true)
            .with_data_migration(true);
            
        smart_strategies.enhance_migration_plan(standard_plan, &diff)
    }

    /// Plan CREATE/DROP TABLE operations
    fn plan_table_changes(
        &self,
        table_change: &TableChange,
        operations: &mut Vec<MigrationOperation>,
        safety_warnings: &mut Vec<SafetyWarning>,
    ) -> Result<()> {
        match table_change.change_type {
            ChangeType::Add => {
                if let Some(new_schema) = &table_change.new_schema {
                    operations.push(MigrationOperation::CreateTable {
                        definition: new_schema.clone(),
                    });
                }
            }
            ChangeType::Remove => {
                operations.push(MigrationOperation::DropTable {
                    name: table_change.table_name.clone(),
                });
                
                // Warn about data loss only if no warnings provided
                if table_change.safety_warnings.is_empty() {
                    safety_warnings.push(SafetyWarning {
                        operation: format!("DROP TABLE {}", table_change.table_name),
                        warning_type: SafetyWarningType::DataLoss,
                        message: format!("Dropping table '{}' will permanently delete all data in the table", table_change.table_name),
                        recommendation: "Consider backing up the data before proceeding, or use a data migration script".to_string(),
                    });
                }
            }
            ChangeType::Rename => {
                // TODO: Implement table rename detection
                // For now, add a safety warning
                safety_warnings.push(SafetyWarning {
                    operation: format!("RENAME TABLE {}", table_change.table_name),
                    warning_type: SafetyWarningType::ComplexOperation,
                    message: "Table rename detected - this operation requires careful review".to_string(),
                    recommendation: "Verify that this is an intended rename and not a table replacement".to_string(),
                });
            }
            ChangeType::Modify => {
                // Table modifications are handled in other phases
            }
        }

        // Add any additional safety warnings from the differ
        for warning in &table_change.safety_warnings {
            let warning_type = match table_change.change_type {
                ChangeType::Remove => SafetyWarningType::DataLoss,
                ChangeType::Modify => SafetyWarningType::BreakingChange,
                _ => SafetyWarningType::ComplexOperation,
            };
            
            safety_warnings.push(SafetyWarning {
                operation: format!("TABLE {}", table_change.table_name),
                warning_type,
                message: warning.clone(),
                recommendation: "Review this operation carefully before proceeding".to_string(),
            });
        }

        Ok(())
    }

    /// Plan ADD/DROP/ALTER COLUMN operations
    fn plan_column_changes(
        &self,
        table_change: &TableChange,
        operations: &mut Vec<MigrationOperation>,
        safety_warnings: &mut Vec<SafetyWarning>,
    ) -> Result<()> {
        for column_change in &table_change.column_changes {
            match column_change.change_type {
                ChangeType::Add => {
                    if let Some(new_def) = &column_change.new_definition {
                        operations.push(MigrationOperation::AddColumn {
                            table: table_change.table_name.clone(),
                            column: new_def.clone(),
                        });
                    }
                }
                ChangeType::Remove => {
                    operations.push(MigrationOperation::DropColumn {
                        table: table_change.table_name.clone(),
                        column: column_change.column_name.clone(),
                    });
                    
                    // Warn about data loss only if no warnings provided
                    if column_change.safety_warnings.is_empty() {
                        safety_warnings.push(SafetyWarning {
                            operation: format!("DROP COLUMN {}.{}", table_change.table_name, column_change.column_name),
                            warning_type: SafetyWarningType::DataLoss,
                            message: format!("Dropping column '{}' will permanently delete all data in that column", column_change.column_name),
                            recommendation: "Consider data migration or column renaming instead of dropping".to_string(),
                        });
                    }
                }
                ChangeType::Modify => {
                    if let (Some(old_def), Some(new_def)) = (&column_change.old_definition, &column_change.new_definition) {
                        let changes = self.analyze_column_changes(old_def, new_def);
                        
                        operations.push(MigrationOperation::ModifyColumn {
                            table: table_change.table_name.clone(),
                            column: column_change.column_name.clone(),
                            changes: changes.clone(),
                        });

                        // Generate safety warnings for risky modifications only if none provided
                        if column_change.safety_warnings.is_empty() {
                            self.analyze_column_modification_safety(
                                &table_change.table_name,
                                &column_change.column_name,
                                &changes,
                                safety_warnings,
                            );
                        }
                    }
                }
                ChangeType::Rename => {
                    if let (Some(old_def), Some(new_def)) = (&column_change.old_definition, &column_change.new_definition) {
                        operations.push(MigrationOperation::RenameColumn {
                            table: table_change.table_name.clone(),
                            old_name: old_def.name.clone(),
                            new_name: new_def.name.clone(),
                        });
                        
                        safety_warnings.push(SafetyWarning {
                            operation: format!("RENAME COLUMN {}.{}", table_change.table_name, old_def.name),
                            warning_type: SafetyWarningType::BreakingChange,
                            message: "Column rename may break existing application code".to_string(),
                            recommendation: "Update all application code references before applying this migration".to_string(),
                        });
                    }
                }
            }

            // Add column-specific safety warnings
            for warning in &column_change.safety_warnings {
                let warning_type = match column_change.change_type {
                    ChangeType::Remove => SafetyWarningType::DataLoss,
                    ChangeType::Modify => SafetyWarningType::BreakingChange,
                    ChangeType::Rename => SafetyWarningType::BreakingChange,
                    _ => SafetyWarningType::ComplexOperation,
                };
                
                safety_warnings.push(SafetyWarning {
                    operation: format!("COLUMN {}.{}", table_change.table_name, column_change.column_name),
                    warning_type,
                    message: warning.clone(),
                    recommendation: "Test this change thoroughly in a development environment".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Plan index modifications
    fn plan_index_changes(
        &self,
        table_change: &TableChange,
        operations: &mut Vec<MigrationOperation>,
        safety_warnings: &mut Vec<SafetyWarning>,
    ) -> Result<()> {
        for index_change in &table_change.index_changes {
            match index_change.change_type {
                ChangeType::Add => {
                    if let Some(new_index) = &index_change.new_definition {
                        operations.push(MigrationOperation::CreateIndex {
                            table: table_change.table_name.clone(),
                            index: new_index.clone(),
                        });
                        
                        // Warn about performance impact for large tables
                        safety_warnings.push(SafetyWarning {
                            operation: format!("CREATE INDEX {}", new_index.name),
                            warning_type: SafetyWarningType::PerformanceImpact,
                            message: "Creating index on large table may take significant time and resources".to_string(),
                            recommendation: "Consider creating index during low-traffic periods".to_string(),
                        });
                    }
                }
                ChangeType::Remove => {
                    operations.push(MigrationOperation::DropIndex {
                        name: index_change.index_name.clone(),
                    });
                    
                    safety_warnings.push(SafetyWarning {
                        operation: format!("DROP INDEX {}", index_change.index_name),
                        warning_type: SafetyWarningType::PerformanceImpact,
                        message: "Dropping index may significantly impact query performance".to_string(),
                        recommendation: "Ensure no critical queries depend on this index".to_string(),
                    });
                }
                ChangeType::Modify => {
                    // For now, treat as drop + recreate
                    operations.push(MigrationOperation::DropIndex {
                        name: index_change.index_name.clone(),
                    });
                    if let Some(new_index) = &index_change.new_definition {
                        operations.push(MigrationOperation::CreateIndex {
                            table: table_change.table_name.clone(),
                            index: new_index.clone(),
                        });
                    }
                }
                ChangeType::Rename => {
                    // SQLite doesn't support index rename, so drop + recreate
                    operations.push(MigrationOperation::DropIndex {
                        name: index_change.index_name.clone(),
                    });
                    if let Some(new_index) = &index_change.new_definition {
                        operations.push(MigrationOperation::CreateIndex {
                            table: table_change.table_name.clone(),
                            index: new_index.clone(),
                        });
                    }
                }
            }

            // Add index-specific safety warnings
            for warning in &index_change.safety_warnings {
                safety_warnings.push(SafetyWarning {
                    operation: format!("INDEX {}", index_change.index_name),
                    warning_type: SafetyWarningType::PerformanceImpact,
                    message: warning.clone(),
                    recommendation: "Monitor performance impact after applying".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Plan FK and junction table changes
    fn plan_relationship_changes(
        &self,
        table_change: &TableChange,
        operations: &mut Vec<MigrationOperation>,
        safety_warnings: &mut Vec<SafetyWarning>,
    ) -> Result<()> {
        for fk_change in &table_change.foreign_key_changes {
            match fk_change.change_type {
                ChangeType::Add => {
                    if let Some(new_fk) = &fk_change.new_definition {
                        operations.push(MigrationOperation::AddForeignKey {
                            constraint: new_fk.clone(),
                        });
                        
                        safety_warnings.push(SafetyWarning {
                            operation: format!("ADD FOREIGN KEY {}", new_fk.name),
                            warning_type: SafetyWarningType::BreakingChange,
                            message: "Adding foreign key constraint may fail if existing data violates the constraint".to_string(),
                            recommendation: "Ensure data integrity before adding the constraint".to_string(),
                        });
                    }
                }
                ChangeType::Remove => {
                    operations.push(MigrationOperation::DropForeignKey {
                        table: table_change.table_name.clone(),
                        constraint_name: fk_change.foreign_key_name.clone(),
                    });
                }
                ChangeType::Modify => {
                    // Drop old and create new
                    operations.push(MigrationOperation::DropForeignKey {
                        table: table_change.table_name.clone(),
                        constraint_name: fk_change.foreign_key_name.clone(),
                    });
                    if let Some(new_fk) = &fk_change.new_definition {
                        operations.push(MigrationOperation::AddForeignKey {
                            constraint: new_fk.clone(),
                        });
                    }
                }
                ChangeType::Rename => {
                    // SQLite doesn't support constraint rename, so drop + recreate
                    operations.push(MigrationOperation::DropForeignKey {
                        table: table_change.table_name.clone(),
                        constraint_name: fk_change.foreign_key_name.clone(),
                    });
                    if let Some(new_fk) = &fk_change.new_definition {
                        operations.push(MigrationOperation::AddForeignKey {
                            constraint: new_fk.clone(),
                        });
                    }
                }
            }

            // Add FK-specific safety warnings
            for warning in &fk_change.safety_warnings {
                safety_warnings.push(SafetyWarning {
                    operation: format!("FOREIGN KEY {}", fk_change.foreign_key_name),
                    warning_type: SafetyWarningType::BreakingChange,
                    message: warning.clone(),
                    recommendation: "Verify referential integrity before proceeding".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Analyze differences between old and new column definitions
    fn analyze_column_changes(&self, old_def: &ColumnSchema, new_def: &ColumnSchema) -> ColumnChanges {
        let type_change = if old_def.column_type != new_def.column_type {
            Some((old_def.column_type.clone(), new_def.column_type.clone()))
        } else {
            None
        };

        let null_change = if old_def.nullable != new_def.nullable {
            Some((old_def.nullable, new_def.nullable))
        } else {
            None
        };

        let default_change = if old_def.default_value != new_def.default_value {
            Some((old_def.default_value.clone(), new_def.default_value.clone()))
        } else {
            None
        };

        ColumnChanges {
            type_change,
            null_change,
            default_change,
            constraint_changes: vec![], // TODO: Implement constraint diff analysis
        }
    }

    /// Analyze safety implications of column modifications
    fn analyze_column_modification_safety(
        &self,
        table_name: &str,
        column_name: &str,
        changes: &ColumnChanges,
        safety_warnings: &mut Vec<SafetyWarning>,
    ) {
        // Check for potentially dangerous type changes
        if let Some((old_type, new_type)) = &changes.type_change {
            safety_warnings.push(SafetyWarning {
                operation: format!("ALTER COLUMN {}.{}", table_name, column_name),
                warning_type: SafetyWarningType::DataLoss,
                message: format!("Type change from {} to {} may cause data loss or truncation", old_type, new_type),
                recommendation: "Test the type conversion with existing data and consider data migration".to_string(),
            });
        }

        // Check for nullable to NOT NULL changes
        if let Some((true, false)) = changes.null_change {
            safety_warnings.push(SafetyWarning {
                operation: format!("ALTER COLUMN {}.{}", table_name, column_name),
                warning_type: SafetyWarningType::BreakingChange,
                message: "Making column NOT NULL may fail if existing data contains NULL values".to_string(),
                recommendation: "Update all NULL values before applying this constraint".to_string(),
            });
        }

        // Check for default value changes
        if changes.default_change.is_some() {
            safety_warnings.push(SafetyWarning {
                operation: format!("ALTER COLUMN {}.{}", table_name, column_name),
                warning_type: SafetyWarningType::BreakingChange,
                message: "Changing default value affects new rows only, existing rows are unchanged".to_string(),
                recommendation: "Consider whether existing rows need to be updated to the new default".to_string(),
            });
        }
    }

    /// Generate rollback operations (reverse of forward operations)
    fn generate_rollback_plan(&self, operations: &[MigrationOperation], diff: &SchemaDiff) -> Result<Vec<MigrationOperation>> {
        let mut rollback_operations = Vec::new();

        // Process operations in reverse order
        for operation in operations.iter().rev() {
            match operation {
                MigrationOperation::CreateTable { definition } => {
                    rollback_operations.push(MigrationOperation::DropTable {
                        name: definition.name.clone(),
                    });
                }
                MigrationOperation::DropTable { name } => {
                    // Find the original table definition from the diff
                    if let Some(table_change) = diff.table_changes.iter().find(|tc| &tc.table_name == name) {
                        if let Some(old_schema) = &table_change.old_schema {
                            rollback_operations.push(MigrationOperation::CreateTable {
                                definition: old_schema.clone(),
                            });
                        }
                    }
                }
                MigrationOperation::AddColumn { table, column } => {
                    rollback_operations.push(MigrationOperation::DropColumn {
                        table: table.clone(),
                        column: column.name.clone(),
                    });
                }
                MigrationOperation::DropColumn { table, column } => {
                    // Find the original column definition from the diff
                    if let Some(table_change) = diff.table_changes.iter().find(|tc| &tc.table_name == table) {
                        if let Some(column_change) = table_change.column_changes.iter().find(|cc| &cc.column_name == column) {
                            if let Some(old_def) = &column_change.old_definition {
                                rollback_operations.push(MigrationOperation::AddColumn {
                                    table: table.clone(),
                                    column: old_def.clone(),
                                });
                            }
                        }
                    }
                }
                MigrationOperation::ModifyColumn { table, column, changes } => {
                    // Generate reverse changes
                    let reverse_changes = ColumnChanges {
                        type_change: changes.type_change.as_ref().map(|(old, new)| (new.clone(), old.clone())),
                        null_change: changes.null_change.map(|(old, new)| (new, old)),
                        default_change: changes.default_change.as_ref().map(|(old, new)| (new.clone(), old.clone())),
                        constraint_changes: vec![], // TODO: Reverse constraint changes
                    };
                    rollback_operations.push(MigrationOperation::ModifyColumn {
                        table: table.clone(),
                        column: column.clone(),
                        changes: reverse_changes,
                    });
                }
                MigrationOperation::CreateIndex { table: _, index } => {
                    rollback_operations.push(MigrationOperation::DropIndex {
                        name: index.name.clone(),
                    });
                }
                MigrationOperation::DropIndex { name } => {
                    // Find the original index definition from the diff
                    for table_change in &diff.table_changes {
                        if let Some(index_change) = table_change.index_changes.iter().find(|ic| &ic.index_name == name) {
                            if let Some(old_index) = &index_change.old_definition {
                                rollback_operations.push(MigrationOperation::CreateIndex {
                                    table: table_change.table_name.clone(),
                                    index: old_index.clone(),
                                });
                                break;
                            }
                        }
                    }
                }
                MigrationOperation::AddForeignKey { constraint } => {
                    // Find the table name from the diff context
                    for table_change in &diff.table_changes {
                        if table_change.foreign_key_changes.iter().any(|fk| &fk.foreign_key_name == &constraint.name) {
                            rollback_operations.push(MigrationOperation::DropForeignKey {
                                table: table_change.table_name.clone(),
                                constraint_name: constraint.name.clone(),
                            });
                            break;
                        }
                    }
                }
                MigrationOperation::DropForeignKey { table, constraint_name } => {
                    // Find the original foreign key definition from the diff
                    if let Some(table_change) = diff.table_changes.iter().find(|tc| &tc.table_name == table) {
                        if let Some(fk_change) = table_change.foreign_key_changes.iter().find(|fk| &fk.foreign_key_name == constraint_name) {
                            if let Some(old_fk) = &fk_change.old_definition {
                                rollback_operations.push(MigrationOperation::AddForeignKey {
                                    constraint: old_fk.clone(),
                                });
                            }
                        }
                    }
                }
                MigrationOperation::RenameTable { old_name, new_name } => {
                    rollback_operations.push(MigrationOperation::RenameTable {
                        old_name: new_name.clone(),
                        new_name: old_name.clone(),
                    });
                }
                MigrationOperation::RenameColumn { table, old_name, new_name } => {
                    rollback_operations.push(MigrationOperation::RenameColumn {
                        table: table.clone(),
                        old_name: new_name.clone(),
                        new_name: old_name.clone(),
                    });
                }
            }
        }

        Ok(rollback_operations)
    }

    /// Estimate migration duration based on operation complexity
    fn estimate_duration(&self, operations: &[MigrationOperation]) -> Duration {
        let mut total_seconds = 0;

        for operation in operations {
            let operation_time = match operation {
                MigrationOperation::CreateTable { .. } => 2, // Simple table creation
                MigrationOperation::DropTable { .. } => 1,   // Fast drop
                MigrationOperation::AddColumn { .. } => 3,   // May need table scan
                MigrationOperation::DropColumn { .. } => 5,  // SQLite limitation - may need table rebuild
                MigrationOperation::ModifyColumn { .. } => 10, // Complex operation, may need table rebuild
                MigrationOperation::CreateIndex { .. } => 15, // Can be slow on large tables
                MigrationOperation::DropIndex { .. } => 1,   // Fast drop
                MigrationOperation::AddForeignKey { .. } => 5, // Constraint validation
                MigrationOperation::DropForeignKey { .. } => 1, // Fast drop
                MigrationOperation::RenameTable { .. } => 1, // Simple rename
                MigrationOperation::RenameColumn { .. } => 5, // May need table rebuild in SQLite
            };
            total_seconds += operation_time;
        }

        Duration::from_secs(total_seconds)
    }
}

#[cfg(test)]
mod planner_verification {
    use super::*;
    
    #[test]
    fn test_warning_type_fixes() {
        let planner = MigrationPlanner::new();
        
        // Test 1: Column Deletion should generate DataLoss
        let column_deletion_diff = SchemaDiff {
            table_changes: vec![
                TableChange {
                    table_name: "users".to_string(),
                    change_type: ChangeType::Modify,
                    old_schema: None,
                    new_schema: None,
                    column_changes: vec![
                        ColumnChange {
                            column_name: "deprecated_field".to_string(),
                            change_type: ChangeType::Remove,
                            old_definition: Some(ColumnSchema {
                                name: "deprecated_field".to_string(),
                                column_type: "TEXT".to_string(),
                                nullable: true,
                                default_value: None,
                                primary_key: false,
                                auto_increment: false,
                                unique: false,
                                constraints: vec![],
                            }),
                            new_definition: None,
                            safety_warnings: vec!["Dropping column will permanently delete data".to_string()],
                        }
                    ],
                    index_changes: vec![],
                    foreign_key_changes: vec![],
                    safety_warnings: vec![],
                }
            ],
        };
        
        let plan = planner.plan_migrations(column_deletion_diff).unwrap();
        assert_eq!(plan.safety_warnings.len(), 1);
        assert_eq!(plan.safety_warnings[0].warning_type, SafetyWarningType::DataLoss);
        
        // Test 2: Table Deletion should generate DataLoss
        let table_deletion_diff = SchemaDiff {
            table_changes: vec![
                TableChange {
                    table_name: "old_table".to_string(),
                    change_type: ChangeType::Remove,
                    old_schema: Some(TableSchema {
                        name: "old_table".to_string(),
                        columns: vec![],
                        indexes: vec![],
                        foreign_keys: vec![],
                        constraints: vec![],
                    }),
                    new_schema: None,
                    column_changes: vec![],
                    index_changes: vec![],
                    foreign_key_changes: vec![],
                    safety_warnings: vec!["This will permanently delete all data in table 'old_table'".to_string()],
                }
            ],
        };
        
        let plan = planner.plan_migrations(table_deletion_diff).unwrap();
        assert_eq!(plan.safety_warnings.len(), 1);
        assert_eq!(plan.safety_warnings[0].warning_type, SafetyWarningType::DataLoss);
        
        println!("✅ All warning type fixes verified!");
    }

    #[test]
    fn test_critical_fixes_verification() {
        let planner = MigrationPlanner::new();
        
        // Test Column Deletion - Should generate DropColumn + safety warnings
        let column_deletion_diff = SchemaDiff {
            table_changes: vec![
                TableChange {
                    table_name: "users".to_string(),
                    change_type: ChangeType::Modify,
                    old_schema: None,
                    new_schema: None,
                    column_changes: vec![
                        ColumnChange {
                            column_name: "deprecated_field".to_string(),
                            change_type: ChangeType::Remove,
                            old_definition: Some(ColumnSchema {
                                name: "deprecated_field".to_string(),
                                column_type: "TEXT".to_string(),
                                nullable: true,
                                default_value: None,
                                primary_key: false,
                                auto_increment: false,
                                unique: false,
                                constraints: vec![],
                            }),
                            new_definition: None,
                            safety_warnings: vec!["Dropping column will permanently delete data".to_string()],
                        }
                    ],
                    index_changes: vec![],
                    foreign_key_changes: vec![],
                    safety_warnings: vec![],
                }
            ],
        };
        
        let plan = planner.plan_migrations(column_deletion_diff).unwrap();
        assert_eq!(plan.operations.len(), 1);
        assert!(!plan.safety_warnings.is_empty());
        
        match &plan.operations[0] {
            MigrationOperation::DropColumn { table, column } => {
                assert_eq!(table, "users");
                assert_eq!(column, "deprecated_field");
            }
            _ => panic!("Expected DropColumn operation"),
        }
        
        // Test Foreign Key Creation - Should generate AddForeignKey + rollback with proper table name
        let fk_creation_diff = SchemaDiff {
            table_changes: vec![
                TableChange {
                    table_name: "posts".to_string(),
                    change_type: ChangeType::Modify,
                    old_schema: None,
                    new_schema: None,
                    column_changes: vec![],
                    index_changes: vec![],
                    foreign_key_changes: vec![
                        ForeignKeyChange {
                            foreign_key_name: "fk_posts_user_id".to_string(),
                            change_type: ChangeType::Add,
                            old_definition: None,
                            new_definition: Some(ForeignKeySchema {
                                name: "fk_posts_user_id".to_string(),
                                columns: vec!["user_id".to_string()],
                                referenced_table: "users".to_string(),
                                referenced_columns: vec!["id".to_string()],
                                on_delete: Some("CASCADE".to_string()),
                                on_update: Some("CASCADE".to_string()),
                            }),
                            safety_warnings: vec![],
                        }
                    ],
                    safety_warnings: vec![],
                }
            ],
        };
        
        let plan = planner.plan_migrations(fk_creation_diff).unwrap();
        assert_eq!(plan.operations.len(), 1);
        assert_eq!(plan.rollback_plan.len(), 1);
        
        match &plan.operations[0] {
            MigrationOperation::AddForeignKey { constraint } => {
                assert_eq!(constraint.name, "fk_posts_user_id");
                assert_eq!(constraint.referenced_table, "users");
            }
            _ => panic!("Expected AddForeignKey operation"),
        }
        
        match &plan.rollback_plan[0] {
            MigrationOperation::DropForeignKey { table, constraint_name } => {
                assert_eq!(table, "posts"); // This was the fix!
                assert_eq!(constraint_name, "fk_posts_user_id");
            }
            _ => panic!("Expected DropForeignKey in rollback"),
        }
        
        println!("✅ All critical migration planner fixes verified!");
    }
}