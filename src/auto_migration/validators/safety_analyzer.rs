//! Migration safety analysis
//! 
//! Comprehensive safety checks for all migration operations with detailed risk assessment.

use crate::Result;
use super::super::{MigrationPlan, MigrationOperation};
use super::super::introspector::{ColumnSchema, TableSchema, IndexSchema, ForeignKeySchema};

/// Migration safety analyzer
pub struct SafetyAnalyzer;

impl SafetyAnalyzer {
    pub fn new() -> Self {
        Self
    }
    
    /// Perform comprehensive safety analysis of migration plan
    pub fn analyze_safety(&self, plan: &MigrationPlan) -> Result<SafetyAnalysisResult> {
        let mut result = SafetyAnalysisResult::new();
        
        // Analyze individual operations
        for operation in &plan.operations {
            self.analyze_operation_safety(operation, &mut result)?;
        }
        
        // Analyze operation sequences and dependencies
        self.analyze_operation_sequences(&plan.operations, &mut result)?;
        
        // Analyze rollback safety (only if there are operations to rollback)
        if !plan.operations.is_empty() {
            self.analyze_rollback_safety(&plan.rollback_plan, &mut result)?;
        }
        
        // Perform comprehensive risk assessment
        self.perform_risk_assessment(&mut result)?;
        
        Ok(result)
    }
    
    /// Analyze safety for individual operation
    fn analyze_operation_safety(&self, operation: &MigrationOperation, result: &mut SafetyAnalysisResult) -> Result<()> {
        match operation {
            MigrationOperation::CreateTable { definition } => {
                self.analyze_create_table_safety(definition, result)?;
            }
            MigrationOperation::DropTable { name } => {
                self.analyze_drop_table_safety(name, result)?;
            }
            MigrationOperation::AddColumn { table, column } => {
                self.analyze_add_column_safety(table, column, result)?;
            }
            MigrationOperation::DropColumn { table, column } => {
                self.analyze_drop_column_safety(table, column, result)?;
            }
            MigrationOperation::ModifyColumn { table, column, changes } => {
                self.analyze_modify_column_safety(table, column, changes, result)?;
            }
            MigrationOperation::CreateIndex { table, index } => {
                self.analyze_create_index_safety(table, index, result)?;
            }
            MigrationOperation::DropIndex { name } => {
                self.analyze_drop_index_safety(name, result)?;
            }
            MigrationOperation::AddForeignKey { constraint } => {
                self.analyze_add_foreign_key_safety(constraint, result)?;
            }
            MigrationOperation::DropForeignKey { table, constraint_name } => {
                self.analyze_drop_foreign_key_safety(table, constraint_name, result)?;
            }
            MigrationOperation::RenameTable { old_name, new_name } => {
                self.analyze_rename_table_safety(old_name, new_name, result)?;
            }
            MigrationOperation::RenameColumn { table, old_name, new_name } => {
                self.analyze_rename_column_safety(table, old_name, new_name, result)?;
            }
        }
        
        Ok(())
    }
    
    /// Analyze table creation safety
    fn analyze_create_table_safety(&self, definition: &TableSchema, result: &mut SafetyAnalysisResult) -> Result<()> {
        let mut safety_issues = Vec::new();
        
        // Check for potential naming conflicts
        if self.is_system_table_name(&definition.name) {
            safety_issues.push(SafetyIssue::NamingConflict {
                entity_type: "table".to_string(),
                name: definition.name.clone(),
                conflict_type: "system_reserved".to_string(),
                severity: SafetySeverity::Critical,
            });
        }
        
        // Validate table structure
        let mut has_primary_key = false;
        let mut unique_constraints = 0;
        
        for column in &definition.columns {
            if column.primary_key {
                has_primary_key = true;
            }
            if column.unique {
                unique_constraints += 1;
            }
            
            // Check for problematic column definitions
            self.validate_column_definition(&definition.name, column, &mut safety_issues)?;
        }
        
        // Warn about tables without primary keys
        if !has_primary_key {
            safety_issues.push(SafetyIssue::StructuralIssue {
                issue_type: "missing_primary_key".to_string(),
                description: format!("Table '{}' has no primary key, which may impact replication and referential integrity", definition.name),
                severity: SafetySeverity::Medium,
                recommendation: "Consider adding a primary key column".to_string(),
            });
        }
        
        // Warn about excessive unique constraints
        if unique_constraints > 5 {
            safety_issues.push(SafetyIssue::PerformanceRisk {
                operation: "create_table".to_string(),
                risk_type: "excessive_constraints".to_string(),
                description: format!("Table '{}' has {} unique constraints, which may impact INSERT performance", definition.name, unique_constraints),
                impact_level: SafetyImpactLevel::Medium,
            });
        }
        
        result.add_safety_issues(safety_issues);
        Ok(())
    }
    
    /// Analyze table dropping safety
    fn analyze_drop_table_safety(&self, name: &str, result: &mut SafetyAnalysisResult) -> Result<()> {
        let safety_issues = vec![
            SafetyIssue::DataLossRisk {
                operation: "drop_table".to_string(),
                affected_data: format!("All data in table '{}'", name),
                severity: SafetySeverity::Critical,
                reversible: false,
                backup_recommended: true,
            },
            SafetyIssue::DependencyRisk {
                operation: "drop_table".to_string(),
                dependency_type: "foreign_keys".to_string(),
                description: format!("Dropping table '{}' may break foreign key references from other tables", name),
                mitigation: "Check and remove dependent foreign keys first".to_string(),
            },
        ];
        
        result.add_safety_issues(safety_issues);
        Ok(())
    }
    
    /// Analyze column addition safety
    fn analyze_add_column_safety(&self, table: &str, column: &ColumnSchema, result: &mut SafetyAnalysisResult) -> Result<()> {
        let mut safety_issues = Vec::new();
        
        // Non-nullable columns without defaults on existing tables
        if !column.nullable && column.default_value.is_none() && !column.auto_increment {
            safety_issues.push(SafetyIssue::ConstraintViolationRisk {
                operation: "add_column".to_string(),
                constraint_type: "not_null".to_string(),
                description: format!("Adding non-nullable column '{}' to table '{}' without default will fail if table has existing data", column.name, table),
                severity: SafetySeverity::High,
                workaround: "Add a default value or make column nullable".to_string(),
            });
        }
        
        // Unique columns on existing data
        if column.unique {
            safety_issues.push(SafetyIssue::ConstraintViolationRisk {
                operation: "add_unique_column".to_string(),
                constraint_type: "unique".to_string(),
                description: format!("Adding unique column '{}' may fail if default values would create duplicates", column.name),
                severity: SafetySeverity::Medium,
                workaround: "Ensure uniqueness of default values or existing data".to_string(),
            });
        }
        
        // Validate column definition
        self.validate_column_definition(table, column, &mut safety_issues)?;
        
        result.add_safety_issues(safety_issues);
        Ok(())
    }
    
    /// Analyze column dropping safety
    fn analyze_drop_column_safety(&self, table: &str, column: &str, result: &mut SafetyAnalysisResult) -> Result<()> {
        let safety_issues = vec![
            SafetyIssue::DataLossRisk {
                operation: "drop_column".to_string(),
                affected_data: format!("All data in column '{}.{}'", table, column),
                severity: SafetySeverity::Critical,
                reversible: false,
                backup_recommended: true,
            },
            SafetyIssue::DependencyRisk {
                operation: "drop_column".to_string(),
                dependency_type: "application_code".to_string(),
                description: format!("Dropping column '{}.{}' will break application code that references it", table, column),
                mitigation: "Update application code before dropping column".to_string(),
            },
        ];
        
        result.add_safety_issues(safety_issues);
        Ok(())
    }
    
    /// Analyze column modification safety
    fn analyze_modify_column_safety(&self, table: &str, column: &str, changes: &super::super::ColumnChanges, result: &mut SafetyAnalysisResult) -> Result<()> {
        let mut safety_issues = Vec::new();
        
        // Type changes
        if let Some((old_type, new_type)) = &changes.type_change {
            let conversion_risk = self.assess_type_conversion_risk(old_type, new_type);
            
            match conversion_risk {
                TypeConversionRisk::Safe => {
                    // No issues
                }
                TypeConversionRisk::DataLoss => {
                    safety_issues.push(SafetyIssue::DataLossRisk {
                        operation: "type_conversion".to_string(),
                        affected_data: format!("Column '{}.{}' values may be truncated or lost", table, column),
                        severity: SafetySeverity::High,
                        reversible: false,
                        backup_recommended: true,
                    });
                }
                TypeConversionRisk::PrecisionLoss => {
                    safety_issues.push(SafetyIssue::DataIntegrityRisk {
                        operation: "type_conversion".to_string(),
                        risk_type: "precision_loss".to_string(),
                        description: format!("Converting column '{}.{}' from {} to {} may lose precision", table, column, old_type, new_type),
                        severity: SafetySeverity::Medium,
                    });
                }
                TypeConversionRisk::ConversionFailure => {
                    safety_issues.push(SafetyIssue::OperationFailureRisk {
                        operation: "type_conversion".to_string(),
                        failure_reason: format!("Cannot convert existing data from {} to {}", old_type, new_type),
                        probability: FailureProbability::High,
                        impact: FailureImpact::Critical,
                    });
                }
            }
        }
        
        // Nullability changes
        if let Some((was_nullable, is_nullable)) = changes.null_change {
            if was_nullable && !is_nullable {
                safety_issues.push(SafetyIssue::ConstraintViolationRisk {
                    operation: "remove_nullability".to_string(),
                    constraint_type: "not_null".to_string(),
                    description: format!("Making column '{}.{}' non-nullable will fail if NULL values exist", table, column),
                    severity: SafetySeverity::High,
                    workaround: "Update NULL values before applying constraint".to_string(),
                });
            }
        }
        
        result.add_safety_issues(safety_issues);
        Ok(())
    }
    
    /// Analyze index creation safety
    fn analyze_create_index_safety(&self, table: &str, index: &IndexSchema, result: &mut SafetyAnalysisResult) -> Result<()> {
        let mut safety_issues = Vec::new();
        
        // Unique indexes on existing data
        if index.unique {
            safety_issues.push(SafetyIssue::ConstraintViolationRisk {
                operation: "create_unique_index".to_string(),
                constraint_type: "unique".to_string(),
                description: format!("Creating unique index '{}' on table '{}' will fail if duplicate values exist", index.name, table),
                severity: SafetySeverity::Medium,
                workaround: "Remove duplicate values before creating index".to_string(),
            });
        }
        
        // Performance impact of index creation
        safety_issues.push(SafetyIssue::PerformanceRisk {
            operation: "create_index".to_string(),
            risk_type: "blocking_operation".to_string(),
            description: format!("Creating index '{}' may block table access during creation", index.name),
            impact_level: SafetyImpactLevel::Medium,
        });
        
        result.add_safety_issues(safety_issues);
        Ok(())
    }
    
    /// Analyze index dropping safety
    fn analyze_drop_index_safety(&self, name: &str, result: &mut SafetyAnalysisResult) -> Result<()> {
        let safety_issues = vec![
            SafetyIssue::PerformanceRisk {
                operation: "drop_index".to_string(),
                risk_type: "query_degradation".to_string(),
                description: format!("Dropping index '{}' may severely impact query performance", name),
                impact_level: SafetyImpactLevel::High,
            },
        ];
        
        result.add_safety_issues(safety_issues);
        Ok(())
    }
    
    /// Analyze foreign key addition safety
    fn analyze_add_foreign_key_safety(&self, constraint: &ForeignKeySchema, result: &mut SafetyAnalysisResult) -> Result<()> {
        let safety_issues = vec![
            SafetyIssue::ConstraintViolationRisk {
                operation: "add_foreign_key".to_string(),
                constraint_type: "foreign_key".to_string(),
                description: format!("Adding foreign key '{}' will fail if referential integrity is violated", constraint.name),
                severity: SafetySeverity::High,
                workaround: "Ensure all references are valid before adding constraint".to_string(),
            },
            SafetyIssue::PerformanceRisk {
                operation: "add_foreign_key".to_string(),
                risk_type: "constraint_validation".to_string(),
                description: "Foreign key validation may be slow on large tables".to_string(),
                impact_level: SafetyImpactLevel::Medium,
            },
        ];
        
        result.add_safety_issues(safety_issues);
        Ok(())
    }
    
    /// Analyze foreign key dropping safety
    fn analyze_drop_foreign_key_safety(&self, _table: &str, constraint_name: &str, result: &mut SafetyAnalysisResult) -> Result<()> {
        let safety_issues = vec![
            SafetyIssue::DataIntegrityRisk {
                operation: "drop_foreign_key".to_string(),
                risk_type: "referential_integrity_loss".to_string(),
                description: format!("Dropping foreign key '{}' removes referential integrity protection", constraint_name),
                severity: SafetySeverity::Medium,
            },
        ];
        
        result.add_safety_issues(safety_issues);
        Ok(())
    }
    
    /// Analyze table renaming safety
    fn analyze_rename_table_safety(&self, old_name: &str, new_name: &str, result: &mut SafetyAnalysisResult) -> Result<()> {
        let mut safety_issues = Vec::new();
        
        // Check for naming conflicts
        if self.is_system_table_name(new_name) {
            safety_issues.push(SafetyIssue::NamingConflict {
                entity_type: "table".to_string(),
                name: new_name.to_string(),
                conflict_type: "system_reserved".to_string(),
                severity: SafetySeverity::High,
            });
        }
        
        safety_issues.push(SafetyIssue::DependencyRisk {
            operation: "rename_table".to_string(),
            dependency_type: "application_references".to_string(),
            description: format!("Renaming table from '{}' to '{}' will break all application references", old_name, new_name),
            mitigation: "Update all application code to use new table name".to_string(),
        });
        
        result.add_safety_issues(safety_issues);
        Ok(())
    }
    
    /// Analyze column renaming safety
    fn analyze_rename_column_safety(&self, table: &str, old_name: &str, new_name: &str, result: &mut SafetyAnalysisResult) -> Result<()> {
        let safety_issues = vec![
            SafetyIssue::DependencyRisk {
                operation: "rename_column".to_string(),
                dependency_type: "application_references".to_string(),
                description: format!("Renaming column from '{}.{}' to '{}.{}' will break application references", table, old_name, table, new_name),
                mitigation: "Update application code to use new column name".to_string(),
            },
        ];
        
        result.add_safety_issues(safety_issues);
        Ok(())
    }
    
    /// Analyze operation sequences for dependencies and conflicts
    fn analyze_operation_sequences(&self, operations: &[MigrationOperation], result: &mut SafetyAnalysisResult) -> Result<()> {
        // Check for dependency violations
        for (i, operation) in operations.iter().enumerate() {
            for (j, other_operation) in operations.iter().enumerate() {
                if i != j {
                    self.check_operation_dependency(operation, other_operation, i, j, result)?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Check dependencies between two operations
    fn check_operation_dependency(&self, op1: &MigrationOperation, op2: &MigrationOperation, index1: usize, index2: usize, result: &mut SafetyAnalysisResult) -> Result<()> {
        // Check for operations that must be in specific order
        match (op1, op2) {
            // Foreign key must be created after referenced table
            (MigrationOperation::AddForeignKey { constraint }, MigrationOperation::CreateTable { definition }) => {
                if constraint.referenced_table == definition.name && index1 < index2 {
                    result.add_safety_issue(SafetyIssue::OrderingViolation {
                        operation1: format!("add_foreign_key({})", constraint.name),
                        operation2: format!("create_table({})", definition.name),
                        violation_type: "dependency_order".to_string(),
                        description: "Foreign key references table that is created later".to_string(),
                    });
                }
            }
            // Column must exist before creating index on it
            (MigrationOperation::CreateIndex { index, .. }, MigrationOperation::AddColumn { table, column }) => {
                if index.columns.contains(&column.name) && Some(table.to_string()) == index.table_name && index1 < index2 {
                    result.add_safety_issue(SafetyIssue::OrderingViolation {
                        operation1: format!("create_index({})", index.name),
                        operation2: format!("add_column({}.{})", table, column.name),
                        violation_type: "dependency_order".to_string(),
                        description: "Index references column that is added later".to_string(),
                    });
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Analyze rollback safety
    fn analyze_rollback_safety(&self, rollback_operations: &[MigrationOperation], result: &mut SafetyAnalysisResult) -> Result<()> {
        if rollback_operations.is_empty() {
            result.add_safety_issue(SafetyIssue::RollbackRisk {
                risk_type: "no_rollback_plan".to_string(),
                description: "No rollback plan available for this migration".to_string(),
                severity: SafetySeverity::High,
                impact: "Migration cannot be automatically reversed if issues occur".to_string(),
            });
            return Ok(());
        }
        
        // Check for non-reversible operations in rollback plan
        for operation in rollback_operations {
            if self.is_destructive_operation(operation) {
                result.add_safety_issue(SafetyIssue::RollbackRisk {
                    risk_type: "destructive_rollback".to_string(),
                    description: format!("Rollback contains destructive operation: {:?}", operation),
                    severity: SafetySeverity::Medium,
                    impact: "Rollback may cause additional data loss".to_string(),
                });
            }
        }
        
        Ok(())
    }
    
    /// Perform comprehensive risk assessment
    fn perform_risk_assessment(&self, result: &mut SafetyAnalysisResult) -> Result<()> {
        let critical_issues = result.safety_issues.iter()
            .filter(|issue| issue.severity() == SafetySeverity::Critical)
            .count();
            
        let high_issues = result.safety_issues.iter()
            .filter(|issue| issue.severity() == SafetySeverity::High)
            .count();
            
        let data_loss_risks = result.safety_issues.iter()
            .filter(|issue| matches!(issue, SafetyIssue::DataLossRisk { .. }))
            .count();
        
        // Calculate overall risk level
        result.overall_risk_level = if critical_issues > 0 || data_loss_risks > 2 {
            OverallRiskLevel::Critical
        } else if high_issues > 3 || data_loss_risks > 0 {
            OverallRiskLevel::High
        } else if high_issues > 0 {
            OverallRiskLevel::Medium
        } else {
            OverallRiskLevel::Low
        };
        
        // Generate safety recommendations
        result.safety_recommendations = self.generate_safety_recommendations(&result.safety_issues);
        
        Ok(())
    }
    
    /// Validate individual column definition
    fn validate_column_definition(&self, table: &str, column: &ColumnSchema, safety_issues: &mut Vec<SafetyIssue>) -> Result<()> {
        // Check for reserved column names
        if self.is_reserved_column_name(&column.name) {
            safety_issues.push(SafetyIssue::NamingConflict {
                entity_type: "column".to_string(),
                name: format!("{}.{}", table, column.name),
                conflict_type: "reserved_name".to_string(),
                severity: SafetySeverity::High,
            });
        }
        
        // Check for logical inconsistencies
        if column.nullable && column.primary_key {
            safety_issues.push(SafetyIssue::StructuralIssue {
                issue_type: "logical_inconsistency".to_string(),
                description: format!("Primary key column '{}.{}' cannot be nullable", table, column.name),
                severity: SafetySeverity::High,
                recommendation: "Remove nullable constraint from primary key column".to_string(),
            });
        }
        
        if column.auto_increment && column.column_type != "INTEGER" {
            safety_issues.push(SafetyIssue::StructuralIssue {
                issue_type: "logical_inconsistency".to_string(),
                description: format!("Auto-increment column '{}.{}' should be INTEGER type", table, column.name),
                severity: SafetySeverity::Medium,
                recommendation: "Change column type to INTEGER for auto-increment".to_string(),
            });
        }
        
        Ok(())
    }
    
    /// Check if table name is a system table
    fn is_system_table_name(&self, name: &str) -> bool {
        matches!(name.to_lowercase().as_str(),
            "sqlite_master" | "sqlite_sequence" | "sqlite_temp_master" |
            "sqlite_stat1" | "sqlite_stat2" | "sqlite_stat3" | "sqlite_stat4"
        )
    }
    
    /// Check if column name is reserved
    fn is_reserved_column_name(&self, name: &str) -> bool {
        matches!(name.to_lowercase().as_str(),
            "rowid" | "oid" | "_rowid_"
        )
    }
    
    /// Assess type conversion risk
    fn assess_type_conversion_risk(&self, from_type: &str, to_type: &str) -> TypeConversionRisk {
        match (from_type.to_uppercase().as_str(), to_type.to_uppercase().as_str()) {
            // Safe conversions
            ("INTEGER", "TEXT") => TypeConversionRisk::Safe,
            ("REAL", "TEXT") => TypeConversionRisk::Safe,
            ("INTEGER", "REAL") => TypeConversionRisk::Safe,
            
            // Precision loss
            ("REAL", "INTEGER") => TypeConversionRisk::PrecisionLoss,
            
            // Potential data loss
            ("TEXT", "INTEGER") => TypeConversionRisk::DataLoss,
            ("TEXT", "REAL") => TypeConversionRisk::DataLoss,
            
            // Conversion failure
            ("BLOB", _) => TypeConversionRisk::ConversionFailure,
            (_, "BLOB") => TypeConversionRisk::ConversionFailure,
            
            // Same types are safe
            (a, b) if a == b => TypeConversionRisk::Safe,
            
            // Unknown conversions are risky
            _ => TypeConversionRisk::DataLoss,
        }
    }
    
    /// Check if operation is destructive
    fn is_destructive_operation(&self, operation: &MigrationOperation) -> bool {
        matches!(operation,
            MigrationOperation::DropTable { .. } |
            MigrationOperation::DropColumn { .. }
        )
    }
    
    /// Generate safety recommendations based on issues
    fn generate_safety_recommendations(&self, issues: &[SafetyIssue]) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        let has_data_loss = issues.iter().any(|issue| matches!(issue, SafetyIssue::DataLossRisk { .. }));
        let has_critical_issues = issues.iter().any(|issue| issue.severity() == SafetySeverity::Critical);
        let has_constraint_risks = issues.iter().any(|issue| matches!(issue, SafetyIssue::ConstraintViolationRisk { .. }));
        
        if has_data_loss {
            recommendations.push("Create a full database backup before executing migration".to_string());
        }
        
        if has_critical_issues {
            recommendations.push("Review all critical issues and resolve before proceeding".to_string());
        }
        
        if has_constraint_risks {
            recommendations.push("Validate data integrity and constraints before migration".to_string());
        }
        
        recommendations.push("Test migration on staging environment first".to_string());
        recommendations.push("Plan for rollback procedure in case of issues".to_string());
        
        recommendations
    }
}

/// Safety analysis result
#[derive(Debug, Clone)]
pub struct SafetyAnalysisResult {
    /// All safety issues identified
    pub safety_issues: Vec<SafetyIssue>,
    
    /// Overall risk level assessment
    pub overall_risk_level: OverallRiskLevel,
    
    /// Safety recommendations
    pub safety_recommendations: Vec<String>,
    
    /// Whether migration is safe to execute
    pub is_safe_to_execute: bool,
}

impl SafetyAnalysisResult {
    fn new() -> Self {
        Self {
            safety_issues: Vec::new(),
            overall_risk_level: OverallRiskLevel::Low,
            safety_recommendations: Vec::new(),
            is_safe_to_execute: true,
        }
    }
    
    fn add_safety_issue(&mut self, issue: SafetyIssue) {
        let is_critical = issue.severity() == SafetySeverity::Critical;
        
        if is_critical {
            self.is_safe_to_execute = false;
        }
        
        self.safety_issues.push(issue);
    }
    
    fn add_safety_issues(&mut self, issues: Vec<SafetyIssue>) {
        for issue in issues {
            self.add_safety_issue(issue);
        }
    }
    
    /// Get critical safety issues that block migration
    pub fn critical_issues(&self) -> Vec<&SafetyIssue> {
        self.safety_issues.iter()
            .filter(|issue| issue.severity() == SafetySeverity::Critical)
            .collect()
    }
    
    /// Get high priority safety issues  
    pub fn high_priority_issues(&self) -> Vec<&SafetyIssue> {
        self.safety_issues.iter()
            .filter(|issue| matches!(issue.severity(), SafetySeverity::High | SafetySeverity::Critical))
            .collect()
    }
}

/// Types of safety issues
#[derive(Debug, Clone)]
pub enum SafetyIssue {
    /// Risk of permanent data loss
    DataLossRisk {
        operation: String,
        affected_data: String,
        severity: SafetySeverity,
        reversible: bool,
        backup_recommended: bool,
    },
    
    /// Risk of constraint violations during migration
    ConstraintViolationRisk {
        operation: String,
        constraint_type: String,
        description: String,
        severity: SafetySeverity,
        workaround: String,
    },
    
    /// Data integrity risks
    DataIntegrityRisk {
        operation: String,
        risk_type: String,
        description: String,
        severity: SafetySeverity,
    },
    
    /// Performance-related risks
    PerformanceRisk {
        operation: String,
        risk_type: String,
        description: String,
        impact_level: SafetyImpactLevel,
    },
    
    /// Naming conflicts with system or reserved names
    NamingConflict {
        entity_type: String,
        name: String,
        conflict_type: String,
        severity: SafetySeverity,
    },
    
    /// Structural issues in schema definition
    StructuralIssue {
        issue_type: String,
        description: String,
        severity: SafetySeverity,
        recommendation: String,
    },
    
    /// Dependency-related risks
    DependencyRisk {
        operation: String,
        dependency_type: String,
        description: String,
        mitigation: String,
    },
    
    /// Operation ordering violations
    OrderingViolation {
        operation1: String,
        operation2: String,
        violation_type: String,
        description: String,
    },
    
    /// Risk of operation failure
    OperationFailureRisk {
        operation: String,
        failure_reason: String,
        probability: FailureProbability,
        impact: FailureImpact,
    },
    
    /// Rollback-related risks
    RollbackRisk {
        risk_type: String,
        description: String,
        severity: SafetySeverity,
        impact: String,
    },
}

impl SafetyIssue {
    fn severity(&self) -> SafetySeverity {
        match self {
            SafetyIssue::DataLossRisk { severity, .. } => *severity,
            SafetyIssue::ConstraintViolationRisk { severity, .. } => *severity,
            SafetyIssue::DataIntegrityRisk { severity, .. } => *severity,
            SafetyIssue::NamingConflict { severity, .. } => *severity,
            SafetyIssue::StructuralIssue { severity, .. } => *severity,
            SafetyIssue::RollbackRisk { severity, .. } => *severity,
            SafetyIssue::PerformanceRisk { impact_level, .. } => {
                match impact_level {
                    SafetyImpactLevel::Low => SafetySeverity::Low,
                    SafetyImpactLevel::Medium => SafetySeverity::Medium,
                    SafetyImpactLevel::High => SafetySeverity::High,
                }
            }
            SafetyIssue::OperationFailureRisk { impact, .. } => {
                match impact {
                    FailureImpact::Minor => SafetySeverity::Low,
                    FailureImpact::Moderate => SafetySeverity::Medium,
                    FailureImpact::Severe => SafetySeverity::High,
                    FailureImpact::Critical => SafetySeverity::Critical,
                }
            }
            _ => SafetySeverity::Medium,
        }
    }
}

/// Safety severity levels
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SafetySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Overall risk level assessment
#[derive(Debug, Clone, PartialEq)]
pub enum OverallRiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Safety impact levels
#[derive(Debug, Clone)]
pub enum SafetyImpactLevel {
    Low,
    Medium,
    High,
}

/// Type conversion risk assessment
#[derive(Debug, Clone, PartialEq)]
enum TypeConversionRisk {
    Safe,
    PrecisionLoss,
    DataLoss,
    ConversionFailure,
}

/// Failure probability levels
#[derive(Debug, Clone)]
pub enum FailureProbability {
    Low,
    Medium,
    High,
    Certain,
}

/// Failure impact levels
#[derive(Debug, Clone)]
pub enum FailureImpact {
    Minor,
    Moderate,
    Severe,
    Critical,
}