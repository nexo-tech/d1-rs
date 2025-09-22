//! Breaking change detection
//! 
//! Identifies changes that will break existing application code or APIs.

use crate::Result;
use super::super::{MigrationPlan, MigrationOperation};
use super::super::introspector::{ColumnSchema, TableSchema};

/// Breaking change detector
pub struct BreakingChangeDetector;

impl BreakingChangeDetector {
    pub fn new() -> Self {
        Self
    }
    
    /// Detect breaking changes in migration plan
    pub fn detect_breaking_changes(&self, plan: &MigrationPlan) -> Result<BreakingChangeResult> {
        let mut result = BreakingChangeResult::new();
        
        for operation in &plan.operations {
            self.analyze_operation_for_breaking_changes(operation, &mut result)?;
        }
        
        self.analyze_cross_operation_breaking_changes(&plan.operations, &mut result)?;
        self.categorize_breaking_changes(&mut result)?;
        
        Ok(result)
    }
    
    /// Analyze individual operation for breaking changes
    fn analyze_operation_for_breaking_changes(&self, operation: &MigrationOperation, result: &mut BreakingChangeResult) -> Result<()> {
        match operation {
            MigrationOperation::CreateTable { definition } => {
                self.detect_create_table_breaking_changes(definition, result)?;
            }
            MigrationOperation::DropTable { name } => {
                self.detect_drop_table_breaking_changes(name, result)?;
            }
            MigrationOperation::AddColumn { table, column } => {
                self.detect_add_column_breaking_changes(table, column, result)?;
            }
            MigrationOperation::DropColumn { table, column } => {
                self.detect_drop_column_breaking_changes(table, column, result)?;
            }
            MigrationOperation::ModifyColumn { table, column, changes } => {
                self.detect_modify_column_breaking_changes(table, column, changes, result)?;
            }
            MigrationOperation::CreateIndex { table, index } => {
                self.detect_create_index_breaking_changes(table, index, result)?;
            }
            MigrationOperation::DropIndex { name } => {
                self.detect_drop_index_breaking_changes(name, result)?;
            }
            MigrationOperation::AddForeignKey { constraint } => {
                self.detect_add_foreign_key_breaking_changes(constraint, result)?;
            }
            MigrationOperation::DropForeignKey { table, constraint_name } => {
                self.detect_drop_foreign_key_breaking_changes(table, constraint_name, result)?;
            }
            MigrationOperation::RenameTable { old_name, new_name } => {
                self.detect_rename_table_breaking_changes(old_name, new_name, result)?;
            }
            MigrationOperation::RenameColumn { table, old_name, new_name } => {
                self.detect_rename_column_breaking_changes(table, old_name, new_name, result)?;
            }
        }
        
        Ok(())
    }
    
    /// Detect breaking changes in table creation
    fn detect_create_table_breaking_changes(&self, definition: &TableSchema, result: &mut BreakingChangeResult) -> Result<()> {
        // Creating new tables generally doesn't break existing code
        // But check for potential issues with naming
        
        if self.is_commonly_used_table_name(&definition.name) {
            result.add_breaking_change(BreakingChange::NamingConflict {
                change_type: "create_table".to_string(),
                entity_name: definition.name.clone(),
                conflict_reason: "Table name conflicts with common naming patterns".to_string(),
                severity: BreakingSeverity::Low,
                affected_components: vec!["Future code that might expect this table name to be available".to_string()],
            });
        }
        
        Ok(())
    }
    
    /// Detect breaking changes in table dropping
    fn detect_drop_table_breaking_changes(&self, name: &str, result: &mut BreakingChangeResult) -> Result<()> {
        result.add_breaking_change(BreakingChange::EntityRemoval {
            entity_type: "table".to_string(),
            entity_name: name.to_string(),
            severity: BreakingSeverity::Critical,
            affected_components: vec![
                format!("All queries referencing table '{}'", name),
                format!("All ORM/entity classes for table '{}'", name),
                format!("All foreign keys referencing table '{}'", name),
            ],
            impact_description: format!("Complete removal of table '{}' breaks all dependent code", name),
            migration_strategy: Some("Consider deprecation period or data migration to new table".to_string()),
        });
        
        Ok(())
    }
    
    /// Detect breaking changes in column addition
    fn detect_add_column_breaking_changes(&self, table: &str, column: &ColumnSchema, result: &mut BreakingChangeResult) -> Result<()> {
        // Adding columns is generally non-breaking, but check for edge cases
        
        // Non-nullable columns without defaults break INSERT statements
        if !column.nullable && column.default_value.is_none() && !column.auto_increment {
            result.add_breaking_change(BreakingChange::SchemaConstraintChange {
                change_type: "add_non_nullable_column".to_string(),
                table_name: table.to_string(),
                column_name: Some(column.name.clone()),
                constraint_change: "Added non-nullable column without default".to_string(),
                severity: BreakingSeverity::High,
                affected_operations: vec![
                    format!("INSERT statements into table '{}'", table),
                    format!("ORM create operations for table '{}'", table),
                ],
                workaround: "Add default value or make column nullable".to_string(),
            });
        }
        
        // Adding primary key columns to existing table
        if column.primary_key {
            result.add_breaking_change(BreakingChange::SchemaConstraintChange {
                change_type: "add_primary_key_column".to_string(),
                table_name: table.to_string(),
                column_name: Some(column.name.clone()),
                constraint_change: "Added primary key column to existing table".to_string(),
                severity: BreakingSeverity::High,
                affected_operations: vec![
                    format!("Existing primary key handling for table '{}'", table),
                    format!("ORM primary key mappings for table '{}'", table),
                ],
                workaround: "Consider migration strategy for existing data".to_string(),
            });
        }
        
        Ok(())
    }
    
    /// Detect breaking changes in column dropping
    fn detect_drop_column_breaking_changes(&self, table: &str, column: &str, result: &mut BreakingChangeResult) -> Result<()> {
        result.add_breaking_change(BreakingChange::EntityRemoval {
            entity_type: "column".to_string(),
            entity_name: format!("{}.{}", table, column),
            severity: BreakingSeverity::Critical,
            affected_components: vec![
                format!("All SELECT queries referencing column '{}.{}'", table, column),
                format!("All INSERT/UPDATE statements including column '{}.{}'", table, column),
                format!("All ORM field mappings for column '{}.{}'", table, column),
                format!("All application code accessing column '{}.{}'", table, column),
            ],
            impact_description: format!("Removing column '{}.{}' breaks all code that references it", table, column),
            migration_strategy: Some("Deprecate column first, then remove in subsequent migration".to_string()),
        });
        
        Ok(())
    }
    
    /// Detect breaking changes in column modification
    fn detect_modify_column_breaking_changes(&self, table: &str, column: &str, changes: &super::super::ColumnChanges, result: &mut BreakingChangeResult) -> Result<()> {
        // Type changes
        if let Some((old_type, new_type)) = &changes.type_change {
            let compatibility = self.assess_type_change_compatibility(old_type, new_type);
            
            match compatibility {
                TypeChangeCompatibility::Breaking => {
                    result.add_breaking_change(BreakingChange::TypeChange {
                        table_name: table.to_string(),
                        column_name: column.to_string(),
                        old_type: old_type.clone(),
                        new_type: new_type.clone(),
                        severity: BreakingSeverity::High,
                        compatibility_issues: vec![
                            "Application code expecting different data type".to_string(),
                            "Serialization/deserialization logic may break".to_string(),
                            "Validation logic may need updates".to_string(),
                        ],
                        migration_strategy: "Update application code to handle new type before migration".to_string(),
                    });
                }
                TypeChangeCompatibility::PotentiallyBreaking => {
                    result.add_breaking_change(BreakingChange::TypeChange {
                        table_name: table.to_string(),
                        column_name: column.to_string(),
                        old_type: old_type.clone(),
                        new_type: new_type.clone(),
                        severity: BreakingSeverity::Medium,
                        compatibility_issues: vec![
                            "Potential data precision or range issues".to_string(),
                            "Type casting in application may behave differently".to_string(),
                        ],
                        migration_strategy: "Test thoroughly with existing data and application logic".to_string(),
                    });
                }
                TypeChangeCompatibility::Compatible => {
                    // No breaking change
                }
            }
        }
        
        // Nullability changes
        if let Some((was_nullable, is_nullable)) = changes.null_change {
            if was_nullable && !is_nullable {
                result.add_breaking_change(BreakingChange::SchemaConstraintChange {
                    change_type: "remove_nullability".to_string(),
                    table_name: table.to_string(),
                    column_name: Some(column.to_string()),
                    constraint_change: "Column made non-nullable".to_string(),
                    severity: BreakingSeverity::Medium,
                    affected_operations: vec![
                        format!("INSERT/UPDATE operations that don't specify value for '{}.{}'", table, column),
                        format!("Application logic that relies on NULL values in '{}.{}'", table, column),
                    ],
                    workaround: "Ensure all existing NULL values are handled and application code is updated".to_string(),
                });
            }
        }
        
        // Default value changes
        if let Some((old_default, new_default)) = &changes.default_change {
            match (old_default, new_default) {
                (Some(_), None) => {
                    result.add_breaking_change(BreakingChange::BehaviorChange {
                        change_type: "remove_default_value".to_string(),
                        entity_name: format!("{}.{}", table, column),
                        old_behavior: "Column had default value for INSERT operations".to_string(),
                        new_behavior: "Column requires explicit value in INSERT operations".to_string(),
                        severity: BreakingSeverity::Medium,
                        affected_code_patterns: vec![
                            "INSERT statements that omit this column".to_string(),
                            "ORM operations that don't set this field".to_string(),
                        ],
                        impact_description: "INSERT operations may fail if column value not provided".to_string(),
                    });
                }
                (old_val, new_val) if old_val != new_val => {
                    result.add_breaking_change(BreakingChange::BehaviorChange {
                        change_type: "change_default_value".to_string(),
                        entity_name: format!("{}.{}", table, column),
                        old_behavior: format!("Default value was {:?}", old_val),
                        new_behavior: format!("Default value is now {:?}", new_val),
                        severity: BreakingSeverity::Low,
                        affected_code_patterns: vec![
                            "INSERT statements that rely on default values".to_string(),
                            "Application logic that expects specific default values".to_string(),
                        ],
                        impact_description: "Different default values may affect application behavior".to_string(),
                    });
                }
                _ => {}
            }
        }
        
        Ok(())
    }
    
    /// Detect breaking changes in index creation
    fn detect_create_index_breaking_changes(&self, table: &str, index: &super::super::introspector::IndexSchema, result: &mut BreakingChangeResult) -> Result<()> {
        // Unique indexes can break INSERT operations if duplicates exist
        if index.unique {
            result.add_breaking_change(BreakingChange::SchemaConstraintChange {
                change_type: "add_unique_index".to_string(),
                table_name: table.to_string(),
                column_name: None,
                constraint_change: format!("Added unique index '{}' on table '{}'", index.name, table),
                severity: BreakingSeverity::Medium,
                affected_operations: vec![
                    format!("INSERT operations that would create duplicate values for index '{}'", index.name),
                    format!("UPDATE operations that would violate uniqueness constraint"),
                ],
                workaround: "Ensure data uniqueness before creating index".to_string(),
            });
        }
        
        Ok(())
    }
    
    /// Detect breaking changes in index dropping
    fn detect_drop_index_breaking_changes(&self, name: &str, result: &mut BreakingChangeResult) -> Result<()> {
        // Dropping indexes can severely impact performance but doesn't break functionality
        result.add_breaking_change(BreakingChange::PerformanceRegression {
            change_type: "drop_index".to_string(),
            entity_name: name.to_string(),
            performance_impact: "Queries using this index may become significantly slower".to_string(),
            severity: BreakingSeverity::Medium,
            affected_queries: vec![
                format!("Queries that relied on index '{}'", name),
                "Range queries and sorting operations".to_string(),
                "WHERE clauses on indexed columns".to_string(),
            ],
            mitigation_strategy: "Ensure alternative indexes exist or optimize queries".to_string(),
        });
        
        Ok(())
    }
    
    /// Detect breaking changes in foreign key addition
    fn detect_add_foreign_key_breaking_changes(&self, constraint: &super::super::introspector::ForeignKeySchema, result: &mut BreakingChangeResult) -> Result<()> {
        result.add_breaking_change(BreakingChange::SchemaConstraintChange {
            change_type: "add_foreign_key".to_string(),
            table_name: "unknown".to_string(), // ForeignKeySchema doesn't store the source table
            column_name: constraint.columns.first().cloned(),
            constraint_change: format!("Added foreign key constraint '{}'", constraint.name),
            severity: BreakingSeverity::Medium,
            affected_operations: vec![
                format!("INSERT operations into unknown table (foreign key '{}')", constraint.name),
                format!("UPDATE operations on foreign key '{}' columns", constraint.name),
                format!("DELETE operations from referenced table '{}'", constraint.referenced_table),
            ],
            workaround: "Ensure referential integrity before adding constraint".to_string(),
        });
        
        Ok(())
    }
    
    /// Detect breaking changes in foreign key dropping
    fn detect_drop_foreign_key_breaking_changes(&self, table: &str, constraint_name: &str, result: &mut BreakingChangeResult) -> Result<()> {
        result.add_breaking_change(BreakingChange::BehaviorChange {
            change_type: "drop_foreign_key".to_string(),
            entity_name: format!("{}.{}", table, constraint_name),
            old_behavior: "Foreign key constraint enforced referential integrity".to_string(),
            new_behavior: "No referential integrity constraint - invalid references allowed".to_string(),
            severity: BreakingSeverity::Low,
            affected_code_patterns: vec![
                "Application code that relied on database-level referential integrity".to_string(),
                "Data validation logic that assumed foreign key enforcement".to_string(),
            ],
            impact_description: "Data integrity now depends entirely on application-level validation".to_string(),
        });
        
        Ok(())
    }
    
    /// Detect breaking changes in table renaming
    fn detect_rename_table_breaking_changes(&self, old_name: &str, new_name: &str, result: &mut BreakingChangeResult) -> Result<()> {
        result.add_breaking_change(BreakingChange::EntityRename {
            entity_type: "table".to_string(),
            old_name: old_name.to_string(),
            new_name: new_name.to_string(),
            severity: BreakingSeverity::Critical,
            affected_components: vec![
                format!("All SQL queries referencing table '{}'", old_name),
                format!("All ORM entity classes mapped to table '{}'", old_name),
                format!("All database views that reference table '{}'", old_name),
                format!("All foreign keys referencing table '{}'", old_name),
                format!("All application configuration using table name '{}'", old_name),
            ],
            migration_strategy: "Update all references to use new table name before migration".to_string(),
            backwards_compatibility_period: Some("Consider maintaining alias or view for transition period".to_string()),
        });
        
        Ok(())
    }
    
    /// Detect breaking changes in column renaming
    fn detect_rename_column_breaking_changes(&self, table: &str, old_name: &str, new_name: &str, result: &mut BreakingChangeResult) -> Result<()> {
        result.add_breaking_change(BreakingChange::EntityRename {
            entity_type: "column".to_string(),
            old_name: format!("{}.{}", table, old_name),
            new_name: format!("{}.{}", table, new_name),
            severity: BreakingSeverity::High,
            affected_components: vec![
                format!("All SELECT queries referencing column '{}.{}'", table, old_name),
                format!("All INSERT/UPDATE statements using column '{}.{}'", table, old_name),
                format!("All ORM field mappings for column '{}.{}'", table, old_name),
                format!("All indexes that include column '{}.{}'", table, old_name),
            ],
            migration_strategy: "Update all column references before migration".to_string(),
            backwards_compatibility_period: Some("Consider gradual migration with column aliasing".to_string()),
        });
        
        Ok(())
    }
    
    /// Analyze breaking changes across multiple operations
    fn analyze_cross_operation_breaking_changes(&self, operations: &[MigrationOperation], result: &mut BreakingChangeResult) -> Result<()> {
        // Look for patterns that amplify breaking changes
        let table_operations: Vec<_> = operations.iter()
            .filter_map(|op| match op {
                MigrationOperation::DropTable { name } => Some(("drop", name.as_str())),
                MigrationOperation::RenameTable { old_name, .. } => Some(("rename", old_name.as_str())),
                _ => None,
            })
            .collect();
        
        // Multiple table modifications increase complexity
        if table_operations.len() > 3 {
            result.add_breaking_change(BreakingChange::ComplexMigration {
                complexity_type: "multiple_table_changes".to_string(),
                operation_count: operations.len(),
                high_impact_operations: table_operations.len(),
                description: format!("Migration involves {} table-level changes", table_operations.len()),
                severity: BreakingSeverity::Medium,
                coordination_required: vec![
                    "Application deployment coordination".to_string(),
                    "Database connection management".to_string(),
                    "Rollback strategy planning".to_string(),
                ],
                risk_factors: vec![
                    "Higher chance of deployment issues".to_string(),
                    "More complex rollback procedures".to_string(),
                    "Increased testing requirements".to_string(),
                ],
            });
        }
        
        Ok(())
    }
    
    /// Categorize breaking changes by impact and severity
    fn categorize_breaking_changes(&self, result: &mut BreakingChangeResult) -> Result<()> {
        let mut blocking_changes = 0;
        let mut major_changes = 0;
        let mut minor_changes = 0;
        
        for change in &result.breaking_changes {
            match change.severity() {
                BreakingSeverity::Critical => blocking_changes += 1,
                BreakingSeverity::High => major_changes += 1,
                BreakingSeverity::Medium => major_changes += 1,
                BreakingSeverity::Low => minor_changes += 1,
            }
        }
        
        result.impact_summary = BreakingChangeImpactSummary {
            total_breaking_changes: result.breaking_changes.len(),
            blocking_changes,
            major_changes,
            minor_changes,
            overall_impact_level: if blocking_changes > 0 {
                OverallImpactLevel::Blocking
            } else if major_changes > 2 {
                OverallImpactLevel::Major
            } else if major_changes > 0 {
                OverallImpactLevel::Moderate
            } else {
                OverallImpactLevel::Minor
            },
        };
        
        // Generate migration recommendations
        result.migration_recommendations = self.generate_migration_recommendations(&result.breaking_changes);
        
        Ok(())
    }
    
    /// Check if table name is commonly used in applications
    fn is_commonly_used_table_name(&self, name: &str) -> bool {
        matches!(name.to_lowercase().as_str(),
            "users" | "user" | "orders" | "order" | "products" | "product" |
            "accounts" | "account" | "sessions" | "session"
        )
    }
    
    /// Assess compatibility of type changes
    fn assess_type_change_compatibility(&self, old_type: &str, new_type: &str) -> TypeChangeCompatibility {
        match (old_type.to_uppercase().as_str(), new_type.to_uppercase().as_str()) {
            // Compatible changes
            ("INTEGER", "TEXT") => TypeChangeCompatibility::Compatible,
            ("REAL", "TEXT") => TypeChangeCompatibility::Compatible,
            ("INTEGER", "REAL") => TypeChangeCompatibility::Compatible,
            
            // Potentially breaking (precision/range issues)
            ("REAL", "INTEGER") => TypeChangeCompatibility::PotentiallyBreaking,
            ("TEXT", "INTEGER") => TypeChangeCompatibility::PotentiallyBreaking,
            ("TEXT", "REAL") => TypeChangeCompatibility::PotentiallyBreaking,
            
            // Breaking changes
            ("TEXT", "BLOB") => TypeChangeCompatibility::Breaking,
            ("BLOB", "TEXT") => TypeChangeCompatibility::Breaking,
            ("INTEGER", "BLOB") => TypeChangeCompatibility::Breaking,
            ("REAL", "BLOB") => TypeChangeCompatibility::Breaking,
            ("BLOB", "INTEGER") => TypeChangeCompatibility::Breaking,
            ("BLOB", "REAL") => TypeChangeCompatibility::Breaking,
            
            // Same type is compatible
            (a, b) if a == b => TypeChangeCompatibility::Compatible,
            
            // Unknown combinations are potentially breaking
            _ => TypeChangeCompatibility::PotentiallyBreaking,
        }
    }
    
    /// Generate migration recommendations based on breaking changes
    fn generate_migration_recommendations(&self, changes: &[BreakingChange]) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        let has_critical_changes = changes.iter().any(|c| c.severity() == BreakingSeverity::Critical);
        let has_entity_removals = changes.iter().any(|c| matches!(c, BreakingChange::EntityRemoval { .. }));
        let has_renames = changes.iter().any(|c| matches!(c, BreakingChange::EntityRename { .. }));
        
        if has_critical_changes {
            recommendations.push("Plan staged deployment to minimize impact of critical breaking changes".to_string());
        }
        
        if has_entity_removals {
            recommendations.push("Implement deprecation warnings before removing entities".to_string());
        }
        
        if has_renames {
            recommendations.push("Consider maintaining aliases during transition period for renamed entities".to_string());
        }
        
        recommendations.push("Update application code to handle all breaking changes before migration".to_string());
        recommendations.push("Test thoroughly in staging environment with production-like data".to_string());
        recommendations.push("Coordinate deployment with application teams".to_string());
        recommendations.push("Prepare rollback procedures for each breaking change".to_string());
        
        recommendations
    }
}

/// Breaking change detection result
#[derive(Debug, Clone)]
pub struct BreakingChangeResult {
    /// All breaking changes identified
    pub breaking_changes: Vec<BreakingChange>,
    
    /// Summary of breaking change impact
    pub impact_summary: BreakingChangeImpactSummary,
    
    /// Migration recommendations
    pub migration_recommendations: Vec<String>,
    
    /// Whether migration has blocking changes
    pub has_blocking_changes: bool,
}

impl BreakingChangeResult {
    fn new() -> Self {
        Self {
            breaking_changes: Vec::new(),
            impact_summary: BreakingChangeImpactSummary::default(),
            migration_recommendations: Vec::new(),
            has_blocking_changes: false,
        }
    }
    
    fn add_breaking_change(&mut self, change: BreakingChange) {
        let is_blocking = change.severity() == BreakingSeverity::Critical;
        
        if is_blocking {
            self.has_blocking_changes = true;
        }
        
        self.breaking_changes.push(change);
    }
    
    /// Get blocking changes that prevent migration
    pub fn blocking_changes(&self) -> Vec<&BreakingChange> {
        self.breaking_changes.iter()
            .filter(|change| change.severity() == BreakingSeverity::Critical)
            .collect()
    }
    
    /// Get high-impact breaking changes
    pub fn high_impact_changes(&self) -> Vec<&BreakingChange> {
        self.breaking_changes.iter()
            .filter(|change| matches!(change.severity(), BreakingSeverity::High | BreakingSeverity::Critical))
            .collect()
    }
}

/// Types of breaking changes
#[derive(Debug, Clone)]
pub enum BreakingChange {
    /// Complete removal of database entity
    EntityRemoval {
        entity_type: String,
        entity_name: String,
        severity: BreakingSeverity,
        affected_components: Vec<String>,
        impact_description: String,
        migration_strategy: Option<String>,
    },
    
    /// Renaming of database entity
    EntityRename {
        entity_type: String,
        old_name: String,
        new_name: String,
        severity: BreakingSeverity,
        affected_components: Vec<String>,
        migration_strategy: String,
        backwards_compatibility_period: Option<String>,
    },
    
    /// Data type changes
    TypeChange {
        table_name: String,
        column_name: String,
        old_type: String,
        new_type: String,
        severity: BreakingSeverity,
        compatibility_issues: Vec<String>,
        migration_strategy: String,
    },
    
    /// Schema constraint changes
    SchemaConstraintChange {
        change_type: String,
        table_name: String,
        column_name: Option<String>,
        constraint_change: String,
        severity: BreakingSeverity,
        affected_operations: Vec<String>,
        workaround: String,
    },
    
    /// Behavior changes that affect application logic
    BehaviorChange {
        change_type: String,
        entity_name: String,
        old_behavior: String,
        new_behavior: String,
        severity: BreakingSeverity,
        affected_code_patterns: Vec<String>,
        impact_description: String,
    },
    
    /// Performance regressions
    PerformanceRegression {
        change_type: String,
        entity_name: String,
        performance_impact: String,
        severity: BreakingSeverity,
        affected_queries: Vec<String>,
        mitigation_strategy: String,
    },
    
    /// Naming conflicts
    NamingConflict {
        change_type: String,
        entity_name: String,
        conflict_reason: String,
        severity: BreakingSeverity,
        affected_components: Vec<String>,
    },
    
    /// Complex migrations requiring coordination
    ComplexMigration {
        complexity_type: String,
        operation_count: usize,
        high_impact_operations: usize,
        description: String,
        severity: BreakingSeverity,
        coordination_required: Vec<String>,
        risk_factors: Vec<String>,
    },
}

impl BreakingChange {
    fn severity(&self) -> BreakingSeverity {
        match self {
            BreakingChange::EntityRemoval { severity, .. } => *severity,
            BreakingChange::EntityRename { severity, .. } => *severity,
            BreakingChange::TypeChange { severity, .. } => *severity,
            BreakingChange::SchemaConstraintChange { severity, .. } => *severity,
            BreakingChange::BehaviorChange { severity, .. } => *severity,
            BreakingChange::PerformanceRegression { severity, .. } => *severity,
            BreakingChange::NamingConflict { severity, .. } => *severity,
            BreakingChange::ComplexMigration { severity, .. } => *severity,
        }
    }
}

/// Breaking change impact summary
#[derive(Debug, Clone, Default)]
pub struct BreakingChangeImpactSummary {
    pub total_breaking_changes: usize,
    pub blocking_changes: usize,
    pub major_changes: usize,
    pub minor_changes: usize,
    pub overall_impact_level: OverallImpactLevel,
}

/// Breaking change severity levels
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BreakingSeverity {
    Low,      // Minor breaking changes with easy workarounds
    Medium,   // Moderate breaking changes requiring some effort
    High,     // Major breaking changes requiring significant changes
    Critical, // Blocking changes that prevent migration
}

/// Overall impact levels
#[derive(Debug, Clone, PartialEq, Default)]
pub enum OverallImpactLevel {
    #[default]
    Minor,     // Minimal impact, easy to handle
    Moderate,  // Some impact, manageable with planning
    Major,     // Significant impact, requires careful coordination
    Blocking,  // Critical impact, blocks migration until resolved
}

/// Type change compatibility assessment
#[derive(Debug, Clone, PartialEq)]
enum TypeChangeCompatibility {
    Compatible,          // No issues expected
    PotentiallyBreaking, // May have issues depending on data/usage
    Breaking,           // Will likely break existing code
}