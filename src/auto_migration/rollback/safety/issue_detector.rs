//! Safety Issue Detection for Rollback Operations
//!
//! This module provides specialized issue detection capabilities that complement the risk assessment
//! engine. It focuses on identifying specific categories of safety issues including data loss,
//! naming conflicts, type conversion problems, and constraint violations.

use super::super::types::{RollbackOperation, RollbackConfig, RollbackRiskSeverity};
use super::risk_assessor::{RollbackValidationIssue, RollbackBlockingIssue};
use crate::auto_migration::DatabaseSchema;
use std::collections::{HashMap, HashSet};

/// Specialized safety issue detector for rollback operations
pub struct SafetyIssueDetector {
    /// Configuration for issue detection behavior
    #[allow(dead_code)]
    config: RollbackConfig,
    
    /// Cache for frequently accessed schema elements
    schema_cache: std::cell::RefCell<HashMap<String, CachedSchemaInfo>>,
}

/// Cached schema information for performance optimization
#[derive(Debug, Clone)]
enum CachedSchemaInfo {
    TableExists(bool),
    #[allow(dead_code)]
    ColumnExists { table: String, exists: bool },
    #[allow(dead_code)]
    IndexExists { table: String, exists: bool },
    #[allow(dead_code)]
    TypeCompatible { from: String, to: String, compatible: bool },
}

/// Categories of safety issues that can be detected
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SafetyIssueCategory {
    DataLoss,
    NamingConflict,
    TypeConversion,
    ConstraintViolation,
    DependencyIssue,
    PerformanceWarning,
}

/// Detailed issue analysis result
#[derive(Debug, Clone)]
pub struct IssueAnalysisResult {
    /// All validation issues found
    pub validation_issues: Vec<RollbackValidationIssue>,
    
    /// Blocking issues that prevent execution
    pub blocking_issues: Vec<RollbackBlockingIssue>,
    
    /// Issues categorized by type
    pub categorized_issues: HashMap<SafetyIssueCategory, Vec<RollbackValidationIssue>>,
    
    /// Overall safety assessment
    pub is_safe_to_execute: bool,
    
    /// Recommended mitigation actions
    pub mitigation_recommendations: Vec<String>,
}

impl SafetyIssueDetector {
    /// Create a new safety issue detector with default configuration
    pub fn new() -> Self {
        Self {
            config: RollbackConfig::default(),
            schema_cache: std::cell::RefCell::new(HashMap::new()),
        }
    }
    
    /// Create a safety issue detector with custom configuration
    pub fn with_config(config: RollbackConfig) -> Self {
        Self {
            config,
            schema_cache: std::cell::RefCell::new(HashMap::new()),
        }
    }
    
    /// Perform comprehensive issue analysis on a rollback operation
    pub fn analyze_operation_safety(
        &self,
        operation: &RollbackOperation,
        schema: &DatabaseSchema,
    ) -> IssueAnalysisResult {
        let mut all_issues = Vec::new();
        let mut blocking_issues = Vec::new();
        
        // Detect different categories of issues
        all_issues.extend(self.detect_data_loss_issues(operation));
        all_issues.extend(self.detect_naming_conflicts(operation, schema));
        all_issues.extend(self.detect_type_conversion_issues(operation));
        all_issues.extend(self.detect_constraint_violations(operation, schema));
        
        // Extract blocking issues
        for (_index, issue) in all_issues.iter().enumerate() {
            if issue.severity == RollbackRiskSeverity::Blocking {
                blocking_issues.push(RollbackBlockingIssue {
                    operation_index: 0, // Will be set by caller if needed
                    description: issue.description.clone(),
                    resolution: issue.mitigation.clone(),
                    auto_resolvable: self.is_auto_resolvable(&issue.operation_type),
                });
            }
        }
        
        // Categorize issues
        let categorized_issues = self.categorize_issues(&all_issues);
        
        // Determine overall safety
        let is_safe_to_execute = blocking_issues.is_empty() && 
            !all_issues.iter().any(|i| i.severity == RollbackRiskSeverity::Critical);
        
        // Generate mitigation recommendations
        let mitigation_recommendations = self.generate_mitigation_recommendations(&all_issues);
        
        IssueAnalysisResult {
            validation_issues: all_issues,
            blocking_issues,
            categorized_issues,
            is_safe_to_execute,
            mitigation_recommendations,
        }
    }
    
    /// Detect potential data loss issues in rollback operations
    pub fn detect_data_loss_issues(&self, operation: &RollbackOperation) -> Vec<RollbackValidationIssue> {
        let mut issues = Vec::new();
        
        match operation {
            RollbackOperation::DropTable { preserve_data, .. } => {
                if !preserve_data {
                    issues.push(RollbackValidationIssue {
                        operation_type: "drop_table".to_string(),
                        description: "Table drop without data preservation will cause permanent data loss".to_string(),
                        severity: RollbackRiskSeverity::Critical,
                        mitigation: "Enable preserve_data option or manually backup table data".to_string(),
                        affected_table: Some(operation.affected_table().to_string()),
                        affected_column: None,
                    });
                }
            }
            
            RollbackOperation::DropColumn { preserve_data, column, .. } => {
                if !preserve_data {
                    issues.push(RollbackValidationIssue {
                        operation_type: "drop_column".to_string(),
                        description: format!("Column '{}' drop without data preservation will cause permanent data loss", column),
                        severity: RollbackRiskSeverity::Critical,
                        mitigation: "Enable preserve_data option or manually backup column data".to_string(),
                        affected_table: Some(operation.affected_table().to_string()),
                        affected_column: Some(column.clone()),
                    });
                }
            }
            
            RollbackOperation::ModifyColumn { changes, preserve_data, column, .. } => {
                if changes.is_lossy() && !preserve_data {
                    issues.push(RollbackValidationIssue {
                        operation_type: "modify_column".to_string(),
                        description: format!("Lossy column modification of '{}' without data preservation", column),
                        severity: RollbackRiskSeverity::Critical,
                        mitigation: "Enable preserve_data option or ensure conversion safety".to_string(),
                        affected_table: Some(operation.affected_table().to_string()),
                        affected_column: Some(column.clone()),
                    });
                }
                
                // Check for specific lossy conversions
                if let Some((from_type, to_type)) = &changes.type_change {
                    if self.is_precision_loss_conversion(from_type, to_type) {
                        issues.push(RollbackValidationIssue {
                            operation_type: "modify_column".to_string(),
                            description: format!("Type conversion from '{}' to '{}' may lose precision", from_type, to_type),
                            severity: RollbackRiskSeverity::High,
                            mitigation: "Validate data compatibility before conversion".to_string(),
                            affected_table: Some(operation.affected_table().to_string()),
                            affected_column: Some(column.clone()),
                        });
                    }
                }
            }
            
            RollbackOperation::RecreateTable { restore_data, .. } => {
                if !restore_data {
                    issues.push(RollbackValidationIssue {
                        operation_type: "recreate_table".to_string(),
                        description: "Table recreation without data restoration will cause data loss".to_string(),
                        severity: RollbackRiskSeverity::Critical,
                        mitigation: "Enable restore_data option or manually backup table data".to_string(),
                        affected_table: Some(operation.affected_table().to_string()),
                        affected_column: None,
                    });
                }
            }
            
            _ => {
                // Other operations are generally safe from data loss perspective
            }
        }
        
        issues
    }
    
    /// Detect naming conflicts that could cause rollback failures
    pub fn detect_naming_conflicts(
        &self,
        operation: &RollbackOperation,
        schema: &DatabaseSchema,
    ) -> Vec<RollbackValidationIssue> {
        let mut issues = Vec::new();
        
        match operation {
            RollbackOperation::RecreateTable { definition, .. } => {
                // Check if table name already exists
                if self.table_exists_in_schema(&definition.name, schema) {
                    issues.push(RollbackValidationIssue {
                        operation_type: "recreate_table".to_string(),
                        description: format!("Table '{}' already exists - recreation will fail", definition.name),
                        severity: RollbackRiskSeverity::Blocking,
                        mitigation: "Drop existing table first or use a different name".to_string(),
                        affected_table: Some(definition.name.clone()),
                        affected_column: None,
                    });
                }
                
                // Check for column name conflicts within the table definition
                let mut column_names = HashSet::new();
                for column in &definition.columns {
                    if !column_names.insert(&column.name) {
                        issues.push(RollbackValidationIssue {
                            operation_type: "recreate_table".to_string(),
                            description: format!("Duplicate column name '{}' in table definition", column.name),
                            severity: RollbackRiskSeverity::Blocking,
                            mitigation: "Remove duplicate column definitions".to_string(),
                            affected_table: Some(definition.name.clone()),
                            affected_column: Some(column.name.clone()),
                        });
                    }
                }
                
                // Check for index name conflicts
                let mut index_names = HashSet::new();
                for index in &definition.indexes {
                    if !index_names.insert(&index.name) {
                        issues.push(RollbackValidationIssue {
                            operation_type: "recreate_table".to_string(),
                            description: format!("Duplicate index name '{}' in table definition", index.name),
                            severity: RollbackRiskSeverity::Blocking,
                            mitigation: "Use unique index names within the table".to_string(),
                            affected_table: Some(definition.name.clone()),
                            affected_column: None,
                        });
                    }
                }
            }
            
            RollbackOperation::AddColumn { table, column, .. } => {
                // Check if column already exists in the table
                if let Some(existing_table) = schema.tables.iter().find(|t| t.name == *table) {
                    if existing_table.columns.iter().any(|c| c.name == column.name) {
                        issues.push(RollbackValidationIssue {
                            operation_type: "add_column".to_string(),
                            description: format!("Column '{}' already exists in table '{}'", column.name, table),
                            severity: RollbackRiskSeverity::Blocking,
                            mitigation: "Use a different column name or modify existing column".to_string(),
                            affected_table: Some(table.clone()),
                            affected_column: Some(column.name.clone()),
                        });
                    }
                }
            }
            
            RollbackOperation::CreateIndex { table, index, .. } => {
                // Check if index name already exists
                if let Some(existing_table) = schema.tables.iter().find(|t| t.name == *table) {
                    if existing_table.indexes.iter().any(|i| i.name == index.name) {
                        issues.push(RollbackValidationIssue {
                            operation_type: "create_index".to_string(),
                            description: format!("Index '{}' already exists on table '{}'", index.name, table),
                            severity: RollbackRiskSeverity::Blocking,
                            mitigation: "Use a different index name or drop existing index first".to_string(),
                            affected_table: Some(table.clone()),
                            affected_column: None,
                        });
                    }
                }
            }
            
            RollbackOperation::RenameTable { old_name, new_name } => {
                // Check if new name already exists
                if self.table_exists_in_schema(new_name, schema) {
                    issues.push(RollbackValidationIssue {
                        operation_type: "rename_table".to_string(),
                        description: format!("Target table name '{}' already exists", new_name),
                        severity: RollbackRiskSeverity::Blocking,
                        mitigation: "Choose a different target name or drop existing table".to_string(),
                        affected_table: Some(old_name.clone()),
                        affected_column: None,
                    });
                }
            }
            
            RollbackOperation::RenameColumn { table, old_name, new_name } => {
                // Check if new column name already exists
                if let Some(existing_table) = schema.tables.iter().find(|t| t.name == *table) {
                    if existing_table.columns.iter().any(|c| c.name == *new_name) {
                        issues.push(RollbackValidationIssue {
                            operation_type: "rename_column".to_string(),
                            description: format!("Target column name '{}' already exists in table '{}'", new_name, table),
                            severity: RollbackRiskSeverity::Blocking,
                            mitigation: "Choose a different target name or drop existing column".to_string(),
                            affected_table: Some(table.clone()),
                            affected_column: Some(old_name.clone()),
                        });
                    }
                }
            }
            
            _ => {
                // Other operations don't typically have naming conflicts
            }
        }
        
        issues
    }
    
    /// Detect type conversion safety issues
    pub fn detect_type_conversion_issues(&self, operation: &RollbackOperation) -> Vec<RollbackValidationIssue> {
        let mut issues = Vec::new();
        
        match operation {
            RollbackOperation::ModifyColumn { changes, column, .. } => {
                if let Some((from_type, to_type)) = &changes.type_change {
                    // Check for incompatible conversions
                    if self.is_incompatible_conversion(from_type, to_type) {
                        issues.push(RollbackValidationIssue {
                            operation_type: "modify_column".to_string(),
                            description: format!(
                                "Type conversion from '{}' to '{}' is incompatible",
                                from_type, to_type
                            ),
                            severity: RollbackRiskSeverity::Blocking,
                            mitigation: "Use a compatible type or implement custom conversion logic".to_string(),
                            affected_table: Some(operation.affected_table().to_string()),
                            affected_column: Some(column.clone()),
                        });
                    }
                    
                    // Check for size reduction issues
                    if self.is_size_reduction_conversion(from_type, to_type) {
                        issues.push(RollbackValidationIssue {
                            operation_type: "modify_column".to_string(),
                            description: format!(
                                "Type conversion from '{}' to '{}' may truncate data",
                                from_type, to_type
                            ),
                            severity: RollbackRiskSeverity::High,
                            mitigation: "Validate data size constraints before conversion".to_string(),
                            affected_table: Some(operation.affected_table().to_string()),
                            affected_column: Some(column.clone()),
                        });
                    }
                    
                    // Check for charset/encoding issues
                    if self.is_charset_conversion_issue(from_type, to_type) {
                        issues.push(RollbackValidationIssue {
                            operation_type: "modify_column".to_string(),
                            description: format!(
                                "Type conversion from '{}' to '{}' may have encoding issues",
                                from_type, to_type
                            ),
                            severity: RollbackRiskSeverity::Warning,
                            mitigation: "Verify character encoding compatibility".to_string(),
                            affected_table: Some(operation.affected_table().to_string()),
                            affected_column: Some(column.clone()),
                        });
                    }
                }
                
                // Check nullable changes
                if let Some((old_nullable, new_nullable)) = changes.null_change {
                    if old_nullable && !new_nullable {
                        issues.push(RollbackValidationIssue {
                            operation_type: "modify_column".to_string(),
                            description: format!(
                                "Making column '{}' non-nullable may fail if NULL values exist",
                                column
                            ),
                            severity: RollbackRiskSeverity::High,
                            mitigation: "Remove NULL values or provide default value before conversion".to_string(),
                            affected_table: Some(operation.affected_table().to_string()),
                            affected_column: Some(column.clone()),
                        });
                    }
                }
            }
            
            RollbackOperation::RecreateTable { definition, .. } => {
                // Check for type compatibility issues within the table
                for column in &definition.columns {
                    if self.is_problematic_type(&column.column_type) {
                        issues.push(RollbackValidationIssue {
                            operation_type: "recreate_table".to_string(),
                            description: format!(
                                "Column '{}' uses problematic type '{}'",
                                column.name, column.column_type
                            ),
                            severity: RollbackRiskSeverity::Warning,
                            mitigation: "Consider using a more standard data type".to_string(),
                            affected_table: Some(definition.name.clone()),
                            affected_column: Some(column.name.clone()),
                        });
                    }
                }
            }
            
            _ => {
                // Other operations don't involve type conversions
            }
        }
        
        issues
    }
    
    /// Detect constraint violation issues
    pub fn detect_constraint_violations(
        &self,
        operation: &RollbackOperation,
        schema: &DatabaseSchema,
    ) -> Vec<RollbackValidationIssue> {
        let mut issues = Vec::new();
        
        match operation {
            RollbackOperation::AddColumn { table, column, .. } => {
                // Check for unique constraint violations
                if column.unique {
                    issues.push(RollbackValidationIssue {
                        operation_type: "add_column".to_string(),
                        description: format!(
                            "Adding unique column '{}' to table with existing data may fail",
                            column.name
                        ),
                        severity: RollbackRiskSeverity::High,
                        mitigation: "Ensure column values will be unique or add with NULL default".to_string(),
                        affected_table: Some(table.clone()),
                        affected_column: Some(column.name.clone()),
                    });
                }
                
                // Check for not-null constraint violations
                if !column.nullable && column.default_value.is_none() {
                    issues.push(RollbackValidationIssue {
                        operation_type: "add_column".to_string(),
                        description: format!(
                            "Adding non-nullable column '{}' without default to table with existing data",
                            column.name
                        ),
                        severity: RollbackRiskSeverity::Blocking,
                        mitigation: "Add default value or make column nullable".to_string(),
                        affected_table: Some(table.clone()),
                        affected_column: Some(column.name.clone()),
                    });
                }
                
                // Check for foreign key constraint issues
                for constraint in &column.constraints {
                    if let crate::auto_migration::introspector::ColumnConstraint::References { table: ref_table, column: _ref_column } = constraint {
                        if !self.table_exists_in_schema(ref_table, schema) {
                            issues.push(RollbackValidationIssue {
                                operation_type: "add_column".to_string(),
                                description: format!(
                                    "Foreign key references non-existent table '{}'",
                                    ref_table
                                ),
                                severity: RollbackRiskSeverity::Blocking,
                                mitigation: "Create referenced table first".to_string(),
                                affected_table: Some(table.clone()),
                                affected_column: Some(column.name.clone()),
                            });
                        }
                    }
                }
            }
            
            RollbackOperation::AddForeignKey { table, constraint, .. } => {
                // Check if both tables exist
                if !self.table_exists_in_schema(table, schema) {
                    issues.push(RollbackValidationIssue {
                        operation_type: "add_foreign_key".to_string(),
                        description: format!("Source table '{}' does not exist", table),
                        severity: RollbackRiskSeverity::Blocking,
                        mitigation: "Create source table before adding foreign key".to_string(),
                        affected_table: Some(table.clone()),
                        affected_column: None,
                    });
                }
                
                if !self.table_exists_in_schema(&constraint.referenced_table, schema) {
                    issues.push(RollbackValidationIssue {
                        operation_type: "add_foreign_key".to_string(),
                        description: format!("Referenced table '{}' does not exist", constraint.referenced_table),
                        severity: RollbackRiskSeverity::Blocking,
                        mitigation: "Create referenced table before adding foreign key".to_string(),
                        affected_table: Some(table.clone()),
                        affected_column: None,
                    });
                }
                
                // Check for referential integrity violations
                issues.push(RollbackValidationIssue {
                    operation_type: "add_foreign_key".to_string(),
                    description: "Adding foreign key may fail if referential integrity is violated".to_string(),
                    severity: RollbackRiskSeverity::High,
                    mitigation: "Validate referential integrity before adding constraint".to_string(),
                    affected_table: Some(table.clone()),
                    affected_column: None,
                });
            }
            
            RollbackOperation::CreateIndex { table, index, .. } => {
                // Check for unique index constraint violations
                if index.unique {
                    issues.push(RollbackValidationIssue {
                        operation_type: "create_index".to_string(),
                        description: format!(
                            "Creating unique index '{}' may fail if duplicate values exist",
                            index.name
                        ),
                        severity: RollbackRiskSeverity::High,
                        mitigation: "Remove duplicate values before creating unique index".to_string(),
                        affected_table: Some(table.clone()),
                        affected_column: None,
                    });
                }
            }
            
            _ => {
                // Other operations don't typically create constraint violations
            }
        }
        
        issues
    }
    
    // Helper methods for issue detection
    
    fn is_precision_loss_conversion(&self, from_type: &str, to_type: &str) -> bool {
        match (from_type.to_uppercase().as_str(), to_type.to_uppercase().as_str()) {
            ("REAL", "INTEGER") => true,
            ("DOUBLE", "REAL") => true,
            ("TEXT", "INTEGER") | ("TEXT", "REAL") => true,
            _ => false,
        }
    }
    
    fn is_incompatible_conversion(&self, from_type: &str, to_type: &str) -> bool {
        match (from_type.to_uppercase().as_str(), to_type.to_uppercase().as_str()) {
            ("BLOB", "TEXT") | ("BLOB", "INTEGER") | ("BLOB", "REAL") => true,
            ("TEXT", "BLOB") | ("INTEGER", "BLOB") | ("REAL", "BLOB") => true,
            _ => false,
        }
    }
    
    fn is_size_reduction_conversion(&self, from_type: &str, to_type: &str) -> bool {
        // Check for size reduction in text types (simplified)
        if from_type.contains("VARCHAR") && to_type.contains("VARCHAR") {
            // This is a simplified check - in practice would parse the size limits
            return from_type.len() > to_type.len(); // Rough approximation
        }
        false
    }
    
    fn is_charset_conversion_issue(&self, from_type: &str, to_type: &str) -> bool {
        // Check for potential charset issues (simplified)
        (from_type.contains("UTF") && !to_type.contains("UTF")) ||
        (!from_type.contains("UTF") && to_type.contains("UTF"))
    }
    
    fn is_problematic_type(&self, type_name: &str) -> bool {
        let problematic_types = [
            "LONGTEXT", "MEDIUMTEXT", "TINYTEXT",
            "LONGBLOB", "MEDIUMBLOB", "TINYBLOB",
            "JSON", "GEOMETRY", "POINT"
        ];
        
        problematic_types.iter().any(|&t| type_name.to_uppercase().contains(t))
    }
    
    fn table_exists_in_schema(&self, table_name: &str, schema: &DatabaseSchema) -> bool {
        // Check cache first
        let cache_key = format!("table_exists:{}", table_name);
        if let Some(CachedSchemaInfo::TableExists(exists)) = self.schema_cache.borrow().get(&cache_key) {
            return *exists;
        }
        
        // Check schema
        let exists = schema.tables.iter().any(|t| t.name == table_name);
        
        // Cache result
        self.schema_cache.borrow_mut().insert(cache_key, CachedSchemaInfo::TableExists(exists));
        
        exists
    }
    
    fn categorize_issues(&self, issues: &[RollbackValidationIssue]) -> HashMap<SafetyIssueCategory, Vec<RollbackValidationIssue>> {
        let mut categorized = HashMap::new();
        
        for issue in issues {
            let category = match issue.operation_type.as_str() {
                "drop_table" | "drop_column" | "modify_column" if issue.description.contains("data loss") => {
                    SafetyIssueCategory::DataLoss
                }
                "recreate_table" | "add_column" | "create_index" | "rename_table" | "rename_column" 
                    if issue.description.contains("already exists") || issue.description.contains("duplicate") => {
                    SafetyIssueCategory::NamingConflict
                }
                "modify_column" if issue.description.contains("conversion") || issue.description.contains("type") => {
                    SafetyIssueCategory::TypeConversion
                }
                "add_column" | "add_foreign_key" | "create_index" 
                    if issue.description.contains("constraint") || issue.description.contains("unique") || issue.description.contains("nullable") => {
                    SafetyIssueCategory::ConstraintViolation
                }
                _ if issue.description.contains("dependency") || issue.description.contains("references") => {
                    SafetyIssueCategory::DependencyIssue
                }
                _ => SafetyIssueCategory::PerformanceWarning,
            };
            
            categorized.entry(category).or_insert_with(Vec::new).push(issue.clone());
        }
        
        categorized
    }
    
    fn generate_mitigation_recommendations(&self, issues: &[RollbackValidationIssue]) -> Vec<String> {
        let mut recommendations = Vec::new();
        let mut seen_recommendations = HashSet::new();
        
        // Extract unique mitigation strategies
        for issue in issues {
            if seen_recommendations.insert(issue.mitigation.clone()) {
                recommendations.push(issue.mitigation.clone());
            }
        }
        
        // Add general safety recommendations
        if !issues.is_empty() {
            recommendations.push("Test rollback operation in a non-production environment first".to_string());
            recommendations.push("Create full database backup before executing rollback".to_string());
        }
        
        // Add severity-specific recommendations
        let has_critical = issues.iter().any(|i| i.severity == RollbackRiskSeverity::Critical);
        if has_critical {
            recommendations.push("Review all critical issues and ensure data preservation strategies are in place".to_string());
        }
        
        let blocking_count = issues.iter().filter(|i| i.severity == RollbackRiskSeverity::Blocking).count();
        if blocking_count > 0 {
            recommendations.push(format!("Resolve {} blocking issues before attempting rollback", blocking_count));
        }
        
        recommendations
    }
    
    fn is_auto_resolvable(&self, operation_type: &str) -> bool {
        matches!(operation_type, "rename_table" | "rename_column")
    }
}

impl Default for SafetyIssueDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auto_migration::{TableSchema, ColumnSchema};
    use crate::auto_migration::rollback::types::RollbackColumnChanges;
    
    fn create_test_schema() -> DatabaseSchema {
        DatabaseSchema {
            tables: vec![
                TableSchema {
                    name: "users".to_string(),
                    columns: vec![
                        ColumnSchema {
                            name: "id".to_string(),
                            column_type: "INTEGER".to_string(),
                            nullable: false,
                            default_value: None,
                            primary_key: true,
                            auto_increment: true,
                            unique: false,
                            constraints: vec![],
                        },
                        ColumnSchema {
                            name: "email".to_string(),
                            column_type: "TEXT".to_string(),
                            nullable: false,
                            default_value: None,
                            primary_key: false,
                            auto_increment: false,
                            unique: true,
                            constraints: vec![],
                        },
                    ],
                    indexes: vec![],
                    foreign_keys: vec![],
                    constraints: vec![],
                },
            ],
        }
    }
    
    #[test]
    fn test_detector_creation() {
        let detector = SafetyIssueDetector::new();
        assert!(detector.schema_cache.borrow().is_empty());
        
        let detector_with_config = SafetyIssueDetector::with_config(RollbackConfig::default());
        assert!(detector_with_config.schema_cache.borrow().is_empty());
    }
    
    #[test]
    fn test_data_loss_detection_drop_table() {
        let detector = SafetyIssueDetector::new();
        
        let operation = RollbackOperation::DropTable {
            name: "users".to_string(),
            preserve_data: false,
            backup_table_name: None,
        };
        
        let issues = detector.detect_data_loss_issues(&operation);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, RollbackRiskSeverity::Critical);
        assert!(issues[0].description.contains("permanent data loss"));
    }
    
    #[test]
    fn test_data_loss_detection_drop_column() {
        let detector = SafetyIssueDetector::new();
        
        let operation = RollbackOperation::DropColumn {
            table: "users".to_string(),
            column: "email".to_string(),
            preserve_data: false,
        };
        
        let issues = detector.detect_data_loss_issues(&operation);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, RollbackRiskSeverity::Critical);
        assert!(issues[0].description.contains("email"));
    }
    
    #[test]
    fn test_naming_conflict_detection() {
        let detector = SafetyIssueDetector::new();
        let schema = create_test_schema();
        
        let operation = RollbackOperation::AddColumn {
            table: "users".to_string(),
            column: ColumnSchema {
                name: "email".to_string(), // Conflicts with existing column
                column_type: "TEXT".to_string(),
                nullable: true,
                default_value: None,
                primary_key: false,
                auto_increment: false,
                unique: false,
                constraints: vec![],
            },
            restore_data: false,
        };
        
        let issues = detector.detect_naming_conflicts(&operation, &schema);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, RollbackRiskSeverity::Blocking);
        assert!(issues[0].description.contains("already exists"));
    }
    
    #[test]
    fn test_type_conversion_detection() {
        let detector = SafetyIssueDetector::new();
        
        let operation = RollbackOperation::ModifyColumn {
            table: "users".to_string(),
            column: "score".to_string(),
            changes: RollbackColumnChanges {
                type_change: Some(("BLOB".to_string(), "TEXT".to_string())),
                null_change: None,
                default_change: None,
                constraint_changes: vec![],
            },
            preserve_data: false,
        };
        
        let issues = detector.detect_type_conversion_issues(&operation);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, RollbackRiskSeverity::Blocking);
        assert!(issues[0].description.contains("incompatible"));
    }
    
    #[test]
    fn test_constraint_violation_detection() {
        let detector = SafetyIssueDetector::new();
        let schema = create_test_schema();
        
        let operation = RollbackOperation::AddColumn {
            table: "users".to_string(),
            column: ColumnSchema {
                name: "age".to_string(),
                column_type: "INTEGER".to_string(),
                nullable: false, // Not nullable without default - will cause constraint violation
                default_value: None,
                primary_key: false,
                auto_increment: false,
                unique: false,
                constraints: vec![],
            },
            restore_data: false,
        };
        
        let issues = detector.detect_constraint_violations(&operation, &schema);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, RollbackRiskSeverity::Blocking);
        assert!(issues[0].description.contains("non-nullable"));
    }
    
    #[test]
    fn test_comprehensive_analysis() {
        let detector = SafetyIssueDetector::new();
        let schema = create_test_schema();
        
        let operation = RollbackOperation::DropColumn {
            table: "users".to_string(),
            column: "email".to_string(),
            preserve_data: false,
        };
        
        let result = detector.analyze_operation_safety(&operation, &schema);
        
        // Should have data loss issue
        assert!(!result.validation_issues.is_empty());
        assert!(result.categorized_issues.contains_key(&SafetyIssueCategory::DataLoss));
        assert!(!result.is_safe_to_execute);
        assert!(!result.mitigation_recommendations.is_empty());
    }
    
    #[test]
    fn test_issue_categorization() {
        let detector = SafetyIssueDetector::new();
        
        let issues = vec![
            RollbackValidationIssue {
                operation_type: "drop_table".to_string(),
                description: "data loss will occur".to_string(),
                severity: RollbackRiskSeverity::Critical,
                mitigation: "backup data".to_string(),
                affected_table: Some("test".to_string()),
                affected_column: None,
            },
            RollbackValidationIssue {
                operation_type: "add_column".to_string(),
                description: "column already exists".to_string(),
                severity: RollbackRiskSeverity::Blocking,
                mitigation: "use different name".to_string(),
                affected_table: Some("test".to_string()),
                affected_column: Some("col".to_string()),
            },
        ];
        
        let categorized = detector.categorize_issues(&issues);
        
        assert!(categorized.contains_key(&SafetyIssueCategory::DataLoss));
        assert!(categorized.contains_key(&SafetyIssueCategory::NamingConflict));
    }
    
    #[test]
    fn test_mitigation_recommendations() {
        let detector = SafetyIssueDetector::new();
        
        let issues = vec![
            RollbackValidationIssue {
                operation_type: "drop_table".to_string(),
                description: "test".to_string(),
                severity: RollbackRiskSeverity::Critical,
                mitigation: "backup first".to_string(),
                affected_table: None,
                affected_column: None,
            },
            RollbackValidationIssue {
                operation_type: "test".to_string(),
                description: "test".to_string(),
                severity: RollbackRiskSeverity::Blocking,
                mitigation: "resolve blocking issue".to_string(),
                affected_table: None,
                affected_column: None,
            },
        ];
        
        let recommendations = detector.generate_mitigation_recommendations(&issues);
        
        assert!(!recommendations.is_empty());
        assert!(recommendations.iter().any(|r| r.contains("backup first")));
        assert!(recommendations.iter().any(|r| r.contains("blocking")));
        assert!(recommendations.iter().any(|r| r.contains("non-production")));
    }
}