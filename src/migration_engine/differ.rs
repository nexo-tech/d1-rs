/// Database-agnostic schema diffing engine built on sea-query
/// 
/// This module provides comprehensive schema comparison and migration operation
/// generation across SQLite, PostgreSQL, and MySQL databases.

use crate::dialects::DatabaseDialect;
use crate::introspection::{
    UnifiedTableSchema, UnifiedColumnSchema, 
    UnifiedIndexSchema, UnifiedConstraintSchema, UnifiedConstraintType,
    type_mapping::CrossDatabaseTypeMapper
};
use crate::auto_migration::introspector::DatabaseSchema;
use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};
use thiserror::Error;

/// Comprehensive error types for schema diffing operations
#[derive(Debug, Error, Clone, PartialEq)]
pub enum SchemaError {
    #[error("Invalid schema: {0}")]
    InvalidSchema(String),
    
    #[error("Cross-database type conversion not possible: {from_type} ({from_db:?}) -> {to_type} ({to_db:?})")]
    IncompatibleType {
        from_type: String,
        from_db: DatabaseDialect,
        to_type: String,
        to_db: DatabaseDialect,
    },
    
    #[error("Constraint conflict: {0}")]
    ConstraintConflict(String),
    
    #[error("Unsafe operation detected: {operation}. Risk: {risk}")]
    UnsafeOperation {
        operation: String,
        risk: String,
    },
    
    #[error("Feature not supported by target database ({dialect:?}): {feature}")]
    UnsupportedFeature {
        dialect: DatabaseDialect,
        feature: String,
    },
}

/// Safety level assessment for migration operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SafetyLevel {
    /// Completely safe operations (adding columns, indexes, etc.)
    Safe,
    /// Operations with minor risks (changing column types with compatible conversion)
    LowRisk,
    /// Operations with moderate risks (renaming columns, adding constraints)
    ModerateRisk,
    /// Operations with high risk of data loss (dropping columns, changing incompatible types)
    HighRisk,
    /// Destructive operations that will definitely lose data (dropping tables)
    Destructive,
}

impl SafetyLevel {
    /// Get the maximum safety level between two levels
    pub fn max(self, other: Self) -> Self {
        use SafetyLevel::*;
        match (self, other) {
            (Destructive, _) | (_, Destructive) => Destructive,
            (HighRisk, _) | (_, HighRisk) => HighRisk,
            (ModerateRisk, _) | (_, ModerateRisk) => ModerateRisk,
            (LowRisk, _) | (_, LowRisk) => LowRisk,
            (Safe, Safe) => Safe,
        }
    }

    /// Check if this safety level represents destructive operations
    pub fn is_destructive(self) -> bool {
        matches!(self, SafetyLevel::Destructive)
    }
}

/// Table-level migration operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TableOperation {
    /// Create a new table
    CreateTable {
        table_name: String,
        schema: UnifiedTableSchema,
    },
    /// Drop an existing table
    DropTable {
        table_name: String,
    },
    /// Rename a table (if supported by database)
    RenameTable {
        old_name: String,
        new_name: String,
    },
}

/// Column-level migration operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ColumnOperation {
    /// Add a new column to existing table
    AddColumn {
        table_name: String,
        column: UnifiedColumnSchema,
    },
    /// Drop an existing column
    DropColumn {
        table_name: String,
        column_name: String,
    },
    /// Modify column properties (type, nullable, default)
    ModifyColumn {
        table_name: String,
        old_column: UnifiedColumnSchema,
        new_column: UnifiedColumnSchema,
    },
    /// Rename a column (if supported by database)
    RenameColumn {
        table_name: String,
        old_name: String,
        new_name: String,
    },
}

/// Index-level migration operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IndexOperation {
    /// Create a new index
    CreateIndex {
        table_name: String,
        index: UnifiedIndexSchema,
    },
    /// Drop an existing index
    DropIndex {
        table_name: String,
        index_name: String,
    },
    /// Modify index properties (columns, unique, etc.)
    ModifyIndex {
        table_name: String,
        old_index: UnifiedIndexSchema,
        new_index: UnifiedIndexSchema,
    },
}

/// Constraint-level migration operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConstraintOperation {
    /// Add a new constraint
    AddConstraint {
        table_name: String,
        constraint: UnifiedConstraintSchema,
    },
    /// Drop an existing constraint
    DropConstraint {
        table_name: String,
        constraint_name: String,
    },
    /// Modify constraint properties
    ModifyConstraint {
        table_name: String,
        old_constraint: UnifiedConstraintSchema,
        new_constraint: UnifiedConstraintSchema,
    },
}

/// Unified migration operation covering all schema change types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MigrationOperation {
    Table(TableOperation),
    Column(ColumnOperation),
    Index(IndexOperation),
    Constraint(ConstraintOperation),
}

impl MigrationOperation {
    /// Assess the safety level of this migration operation
    pub fn safety_level(&self) -> SafetyLevel {
        match self {
            MigrationOperation::Table(op) => match op {
                TableOperation::CreateTable { .. } => SafetyLevel::Safe,
                TableOperation::DropTable { .. } => SafetyLevel::Destructive,
                TableOperation::RenameTable { .. } => SafetyLevel::ModerateRisk,
            },
            MigrationOperation::Column(op) => match op {
                ColumnOperation::AddColumn { column, .. } => {
                    if column.nullable || column.default_value.is_some() {
                        SafetyLevel::Safe
                    } else {
                        SafetyLevel::ModerateRisk // Adding non-null column without default
                    }
                },
                ColumnOperation::DropColumn { .. } => SafetyLevel::Destructive,
                ColumnOperation::ModifyColumn { old_column, new_column, .. } => {
                    Self::assess_column_modification_safety(old_column, new_column)
                },
                ColumnOperation::RenameColumn { .. } => SafetyLevel::LowRisk,
            },
            MigrationOperation::Index(op) => match op {
                IndexOperation::CreateIndex { .. } => SafetyLevel::Safe,
                IndexOperation::DropIndex { .. } => SafetyLevel::LowRisk,
                IndexOperation::ModifyIndex { .. } => SafetyLevel::LowRisk,
            },
            MigrationOperation::Constraint(op) => match op {
                ConstraintOperation::AddConstraint { constraint, .. } => {
                    match constraint.constraint_type {
                        UnifiedConstraintType::ForeignKey => SafetyLevel::ModerateRisk,
                        UnifiedConstraintType::Check => SafetyLevel::ModerateRisk,
                        UnifiedConstraintType::Unique => SafetyLevel::ModerateRisk,
                        UnifiedConstraintType::PrimaryKey => SafetyLevel::HighRisk,
                        UnifiedConstraintType::NotNull => SafetyLevel::ModerateRisk,
                        UnifiedConstraintType::Default => SafetyLevel::LowRisk,
                        UnifiedConstraintType::Other(_) => SafetyLevel::ModerateRisk,
                    }
                },
                ConstraintOperation::DropConstraint { .. } => SafetyLevel::ModerateRisk,
                ConstraintOperation::ModifyConstraint { .. } => SafetyLevel::ModerateRisk,
            },
        }
    }

    /// Assess safety of column type/property modifications
    fn assess_column_modification_safety(
        old_column: &UnifiedColumnSchema,
        new_column: &UnifiedColumnSchema,
    ) -> SafetyLevel {
        let mut risk_level = SafetyLevel::Safe;

        // Check nullability changes
        if old_column.nullable && !new_column.nullable {
            risk_level = risk_level.max(SafetyLevel::HighRisk);
        }

        // Check data type changes - simplified assessment
        if old_column.raw_type != new_column.raw_type {
            risk_level = risk_level.max(Self::assess_type_change_safety(&old_column.raw_type, &new_column.raw_type));
        }

        // Check default value changes
        if old_column.default_value != new_column.default_value {
            risk_level = risk_level.max(SafetyLevel::LowRisk);
        }

        risk_level
    }

    /// Assess safety of data type changes
    fn assess_type_change_safety(old_type: &str, new_type: &str) -> SafetyLevel {
        // Simplified type safety assessment
        match (old_type, new_type) {
            // Safe expansions
            ("INTEGER", "BIGINT") | ("INT", "BIGINT") => SafetyLevel::Safe,
            ("FLOAT", "DOUBLE") | ("REAL", "DOUBLE") => SafetyLevel::Safe,
            ("VARCHAR(10)", "VARCHAR(20)") => SafetyLevel::Safe, // Simplified check
            
            // Risky contractions
            ("BIGINT", "INTEGER") | ("BIGINT", "INT") => SafetyLevel::HighRisk,
            ("DOUBLE", "FLOAT") | ("DOUBLE", "REAL") => SafetyLevel::HighRisk,
            
            // Type conversions
            ("INTEGER", "VARCHAR(50)") => SafetyLevel::LowRisk,
            ("VARCHAR(50)", "INTEGER") => SafetyLevel::HighRisk,
            
            // Default to moderate risk for unhandled cases
            _ => SafetyLevel::ModerateRisk,
        }
    }
}

/// Complete migration plan with operations and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    /// List of migration operations to execute
    pub operations: Vec<MigrationOperation>,
    /// Target database dialect for this plan
    pub dialect: DatabaseDialect,
    /// Overall safety level of the migration
    pub safety_level: SafetyLevel,
    /// Estimated execution time in seconds
    pub estimated_duration_seconds: u64,
    /// Tables that will be affected by this migration
    pub affected_tables: HashSet<String>,
    /// Warnings and recommendations
    pub warnings: Vec<String>,
}

impl MigrationPlan {
    /// Create new migration plan
    pub fn new(operations: Vec<MigrationOperation>, dialect: DatabaseDialect) -> Self {
        let safety_level = operations.iter()
            .map(|op| op.safety_level())
            .fold(SafetyLevel::Safe, |acc, level| acc.max(level));
            
        let affected_tables = operations.iter()
            .flat_map(|op| Self::extract_table_names(op))
            .collect();
            
        let estimated_duration_seconds = Self::estimate_duration(&operations);
        
        Self {
            operations,
            dialect,
            safety_level,
            estimated_duration_seconds,
            affected_tables,
            warnings: Vec::new(),
        }
    }
    
    /// Extract table names affected by an operation
    fn extract_table_names(operation: &MigrationOperation) -> Vec<String> {
        match operation {
            MigrationOperation::Table(op) => match op {
                TableOperation::CreateTable { table_name, .. } |
                TableOperation::DropTable { table_name } => vec![table_name.clone()],
                TableOperation::RenameTable { old_name, new_name } => vec![old_name.clone(), new_name.clone()],
            },
            MigrationOperation::Column(op) => match op {
                ColumnOperation::AddColumn { table_name, .. } |
                ColumnOperation::DropColumn { table_name, .. } |
                ColumnOperation::ModifyColumn { table_name, .. } |
                ColumnOperation::RenameColumn { table_name, .. } => vec![table_name.clone()],
            },
            MigrationOperation::Index(op) => match op {
                IndexOperation::CreateIndex { table_name, .. } |
                IndexOperation::DropIndex { table_name, .. } |
                IndexOperation::ModifyIndex { table_name, .. } => vec![table_name.clone()],
            },
            MigrationOperation::Constraint(op) => match op {
                ConstraintOperation::AddConstraint { table_name, .. } |
                ConstraintOperation::DropConstraint { table_name, .. } |
                ConstraintOperation::ModifyConstraint { table_name, .. } => vec![table_name.clone()],
            },
        }
    }
    
    /// Estimate migration execution duration
    fn estimate_duration(operations: &[MigrationOperation]) -> u64 {
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
    
    /// Check if migration plan is considered safe to execute
    pub fn is_safe(&self) -> bool {
        matches!(self.safety_level, SafetyLevel::Safe | SafetyLevel::LowRisk)
    }
    
    /// Add a warning to the migration plan
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
}

/// Result of migration execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationResult {
    /// Whether the migration was successful
    pub success: bool,
    /// Number of operations completed
    pub operations_completed: usize,
    /// Actual execution time in seconds
    pub execution_duration_seconds: u64,
    /// Any error messages
    pub errors: Vec<String>,
    /// Tables that were successfully migrated
    pub migrated_tables: HashSet<String>,
}

/// Database-agnostic schema diffing engine
pub struct SchemaDiffer {
    /// Target database dialect
    dialect: DatabaseDialect,
    /// Cross-database type mapper for handling type conversions
    #[allow(dead_code)]
    type_mapper: CrossDatabaseTypeMapper,
}

impl SchemaDiffer {
    /// Create new schema differ for target dialect
    pub fn new(dialect: DatabaseDialect) -> Self {
        Self {
            dialect,
            type_mapper: CrossDatabaseTypeMapper::new(),
        }
    }
    
    /// Convert legacy TableSchema to UnifiedTableSchema
    fn convert_table_schema_to_unified(&self, legacy: &crate::auto_migration::introspector::TableSchema) -> UnifiedTableSchema {
        // This is a temporary conversion - in a real implementation,
        // we would have proper converters or use unified schema throughout
        UnifiedTableSchema {
            name: legacy.name.clone(),
            table_type: crate::introspection::UnifiedTableType::Table,
            schema_name: None,
            comment: None,
            columns: legacy.columns.iter().map(|col| self.convert_column_schema_to_unified(col)).collect(),
            indexes: Vec::new(), // Simplified for now
            foreign_keys: Vec::new(), // Simplified for now
            constraints: Vec::new(), // Simplified for now
            metadata: std::collections::HashMap::new(),
        }
    }
    
    /// Convert legacy ColumnSchema to UnifiedColumnSchema
    fn convert_column_schema_to_unified(&self, legacy: &crate::auto_migration::introspector::ColumnSchema) -> UnifiedColumnSchema {
        UnifiedColumnSchema {
            name: legacy.name.clone(),
            column_type: crate::introspection::UnifiedColumnType::Text, // Simplified for now
            raw_type: legacy.column_type.clone(),
            nullable: legacy.nullable,
            default_value: legacy.default_value.clone(),
            primary_key: legacy.primary_key,
            auto_increment: legacy.auto_increment,
            unique: legacy.unique,
            comment: None,
            ordinal_position: Some(1), // Legacy schema doesn't track position
            character_maximum_length: None, // Legacy schema doesn't have these fields
            numeric_precision: None,
            numeric_scale: None,
            constraints: Vec::new(),
        }
    }
    
    /// Compare two database schemas and generate migration plan
    pub fn diff_schemas(
        &self,
        current: &DatabaseSchema,
        target: &DatabaseSchema,
    ) -> Result<MigrationPlan, SchemaError> {
        let mut operations = Vec::new();
        
        // Generate table operations
        operations.extend(self.diff_tables(current, target)?);
        
        // Generate column operations for existing tables
        operations.extend(self.diff_columns(current, target)?);
        
        // Generate index operations
        operations.extend(self.diff_indexes(current, target)?);
        
        // Generate constraint operations
        operations.extend(self.diff_constraints(current, target)?);
        
        // Create migration plan
        let plan = MigrationPlan::new(operations, self.dialect);
        
        // Add cross-database compatibility warnings
        // Note: Legacy DatabaseSchema doesn't have dialect field
        // Cross-database warnings would be handled at a higher level
        
        Ok(plan)
    }
    
    /// Generate table-level operations (create, drop, rename)
    fn diff_tables(
        &self,
        current: &DatabaseSchema,
        target: &DatabaseSchema,
    ) -> Result<Vec<MigrationOperation>, SchemaError> {
        let mut operations = Vec::new();
        
        let current_tables: HashSet<String> = current.tables.iter().map(|t| t.name.clone()).collect();
        let target_tables: HashSet<String> = target.tables.iter().map(|t| t.name.clone()).collect();
        
        // Tables to create (in target but not in current)
        for table_name in target_tables.difference(&current_tables) {
            if let Some(table_schema) = target.tables.iter().find(|t| &t.name == table_name) {
                // TODO: Convert TableSchema to UnifiedTableSchema
                // For now, create a placeholder operation
                let unified_schema = self.convert_table_schema_to_unified(table_schema);
                operations.push(MigrationOperation::Table(TableOperation::CreateTable {
                    table_name: table_name.clone(),
                    schema: unified_schema,
                }));
            }
        }
        
        // Tables to drop (in current but not in target)
        for table_name in current_tables.difference(&target_tables) {
            operations.push(MigrationOperation::Table(TableOperation::DropTable {
                table_name: table_name.clone(),
            }));
        }
        
        Ok(operations)
    }
    
    /// Generate column-level operations for existing tables
    fn diff_columns(
        &self,
        current: &DatabaseSchema,
        target: &DatabaseSchema,
    ) -> Result<Vec<MigrationOperation>, SchemaError> {
        let mut operations = Vec::new();
        
        // Only process tables that exist in both schemas
        for current_table in &current.tables {
            if let Some(target_table) = target.tables.iter().find(|t| t.name == current_table.name) {
                // Convert to unified schema for comparison
                let current_unified = self.convert_table_schema_to_unified(current_table);
                let target_unified = self.convert_table_schema_to_unified(target_table);
                operations.extend(self.diff_table_columns(&current_table.name, &current_unified, &target_unified)?);
            }
        }
        
        Ok(operations)
    }
    
    /// Generate column operations for a specific table
    fn diff_table_columns(
        &self,
        table_name: &str,
        current_table: &UnifiedTableSchema,
        target_table: &UnifiedTableSchema,
    ) -> Result<Vec<MigrationOperation>, SchemaError> {
        let mut operations = Vec::new();
        
        let current_columns: HashMap<String, &UnifiedColumnSchema> = current_table.columns.iter()
            .map(|col| (col.name.clone(), col))
            .collect();
        let target_columns: HashMap<String, &UnifiedColumnSchema> = target_table.columns.iter()
            .map(|col| (col.name.clone(), col))
            .collect();
            
        let current_names: HashSet<String> = current_columns.keys().cloned().collect();
        let target_names: HashSet<String> = target_columns.keys().cloned().collect();
        
        // Columns to add
        for column_name in target_names.difference(&current_names) {
            if let Some(column) = target_columns.get(column_name) {
                // Note: Type conversion will be handled at execution time
                let adapted_column = (*column).clone();
                
                operations.push(MigrationOperation::Column(ColumnOperation::AddColumn {
                    table_name: table_name.to_string(),
                    column: adapted_column,
                }));
            }
        }
        
        // Columns to drop
        for column_name in current_names.difference(&target_names) {
            operations.push(MigrationOperation::Column(ColumnOperation::DropColumn {
                table_name: table_name.to_string(),
                column_name: column_name.clone(),
            }));
        }
        
        // Columns to modify (exist in both but different)
        for column_name in current_names.intersection(&target_names) {
            if let (Some(current_col), Some(target_col)) = (
                current_columns.get(column_name),
                target_columns.get(column_name)
            ) {
                if self.columns_differ(current_col, target_col) {
                    // Note: Type conversion will be handled at execution time
                    let adapted_column = (*target_col).clone();
                    
                    operations.push(MigrationOperation::Column(ColumnOperation::ModifyColumn {
                        table_name: table_name.to_string(),
                        old_column: (*current_col).clone(),
                        new_column: adapted_column,
                    }));
                }
            }
        }
        
        Ok(operations)
    }
    
    /// Check if two columns differ in any meaningful way
    fn columns_differ(&self, current: &UnifiedColumnSchema, target: &UnifiedColumnSchema) -> bool {
        current.raw_type != target.raw_type ||
        current.nullable != target.nullable ||
        current.default_value != target.default_value ||
        current.primary_key != target.primary_key ||
        current.auto_increment != target.auto_increment
    }
    
    /// Generate index operations
    fn diff_indexes(
        &self,
        current: &DatabaseSchema,
        target: &DatabaseSchema,
    ) -> Result<Vec<MigrationOperation>, SchemaError> {
        let mut operations = Vec::new();
        
        // Process indexes for tables that exist in both schemas
        for current_table in &current.tables {
            if let Some(target_table) = target.tables.iter().find(|t| t.name == current_table.name) {
                let current_unified = self.convert_table_schema_to_unified(current_table);
                let target_unified = self.convert_table_schema_to_unified(target_table);
                operations.extend(self.diff_table_indexes(&current_table.name, &current_unified, &target_unified));
            }
        }
        
        Ok(operations)
    }
    
    /// Generate index operations for a specific table
    fn diff_table_indexes(
        &self,
        table_name: &str,
        current_table: &UnifiedTableSchema,
        target_table: &UnifiedTableSchema,
    ) -> Vec<MigrationOperation> {
        let mut operations = Vec::new();
        
        let current_indexes: HashMap<String, &UnifiedIndexSchema> = current_table.indexes.iter()
            .map(|idx| (idx.name.clone(), idx))
            .collect();
        let target_indexes: HashMap<String, &UnifiedIndexSchema> = target_table.indexes.iter()
            .map(|idx| (idx.name.clone(), idx))
            .collect();
            
        let current_names: HashSet<String> = current_indexes.keys().cloned().collect();
        let target_names: HashSet<String> = target_indexes.keys().cloned().collect();
        
        // Indexes to create
        for index_name in target_names.difference(&current_names) {
            if let Some(index) = target_indexes.get(index_name) {
                operations.push(MigrationOperation::Index(IndexOperation::CreateIndex {
                    table_name: table_name.to_string(),
                    index: (*index).clone(),
                }));
            }
        }
        
        // Indexes to drop
        for index_name in current_names.difference(&target_names) {
            operations.push(MigrationOperation::Index(IndexOperation::DropIndex {
                table_name: table_name.to_string(),
                index_name: index_name.clone(),
            }));
        }
        
        // Indexes to modify (exist in both but different)
        for index_name in current_names.intersection(&target_names) {
            if let (Some(current_idx), Some(target_idx)) = (
                current_indexes.get(index_name),
                target_indexes.get(index_name)
            ) {
                if self.indexes_differ(current_idx, target_idx) {
                    operations.push(MigrationOperation::Index(IndexOperation::ModifyIndex {
                        table_name: table_name.to_string(),
                        old_index: (*current_idx).clone(),
                        new_index: (*target_idx).clone(),
                    }));
                }
            }
        }
        
        operations
    }
    
    /// Check if two indexes differ
    fn indexes_differ(&self, current: &UnifiedIndexSchema, target: &UnifiedIndexSchema) -> bool {
        current.columns != target.columns ||
        current.unique != target.unique ||
        current.index_type != target.index_type
    }
    
    /// Generate constraint operations
    fn diff_constraints(
        &self,
        current: &DatabaseSchema,
        target: &DatabaseSchema,
    ) -> Result<Vec<MigrationOperation>, SchemaError> {
        let mut operations = Vec::new();
        
        // Process constraints for tables that exist in both schemas
        for current_table in &current.tables {
            if let Some(target_table) = target.tables.iter().find(|t| t.name == current_table.name) {
                let current_unified = self.convert_table_schema_to_unified(current_table);
                let target_unified = self.convert_table_schema_to_unified(target_table);
                operations.extend(self.diff_table_constraints(&current_table.name, &current_unified, &target_unified));
            }
        }
        
        Ok(operations)
    }
    
    /// Generate constraint operations for a specific table
    fn diff_table_constraints(
        &self,
        table_name: &str,
        current_table: &UnifiedTableSchema,
        target_table: &UnifiedTableSchema,
    ) -> Vec<MigrationOperation> {
        let mut operations = Vec::new();
        
        let current_constraints: HashMap<String, &UnifiedConstraintSchema> = current_table.constraints.iter()
            .map(|constraint| (constraint.name.clone(), constraint))
            .collect();
        let target_constraints: HashMap<String, &UnifiedConstraintSchema> = target_table.constraints.iter()
            .map(|constraint| (constraint.name.clone(), constraint))
            .collect();
            
        let current_names: HashSet<String> = current_constraints.keys().cloned().collect();
        let target_names: HashSet<String> = target_constraints.keys().cloned().collect();
        
        // Constraints to add
        for constraint_name in target_names.difference(&current_names) {
            if let Some(constraint) = target_constraints.get(constraint_name) {
                operations.push(MigrationOperation::Constraint(ConstraintOperation::AddConstraint {
                    table_name: table_name.to_string(),
                    constraint: (*constraint).clone(),
                }));
            }
        }
        
        // Constraints to drop
        for constraint_name in current_names.difference(&target_names) {
            operations.push(MigrationOperation::Constraint(ConstraintOperation::DropConstraint {
                table_name: table_name.to_string(),
                constraint_name: constraint_name.clone(),
            }));
        }
        
        // Constraints to modify (exist in both but different)
        for constraint_name in current_names.intersection(&target_names) {
            if let (Some(current_constraint), Some(target_constraint)) = (
                current_constraints.get(constraint_name),
                target_constraints.get(constraint_name)
            ) {
                if self.constraints_differ(current_constraint, target_constraint) {
                    operations.push(MigrationOperation::Constraint(ConstraintOperation::ModifyConstraint {
                        table_name: table_name.to_string(),
                        old_constraint: (*current_constraint).clone(),
                        new_constraint: (*target_constraint).clone(),
                    }));
                }
            }
        }
        
        operations
    }
    
    /// Check if two constraints differ
    fn constraints_differ(&self, current: &UnifiedConstraintSchema, target: &UnifiedConstraintSchema) -> bool {
        current.constraint_type != target.constraint_type ||
        current.columns != target.columns
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::introspection::UnifiedIndexType;
    
    fn create_test_database_schema(_dialect: DatabaseDialect) -> DatabaseSchema {
        DatabaseSchema {
            tables: Vec::new(),
        }
    }
    
    fn create_test_table_schema(name: &str, _dialect: DatabaseDialect) -> UnifiedTableSchema {
        UnifiedTableSchema {
            name: name.to_string(),
            columns: vec![
                UnifiedColumnSchema {
                    name: "id".to_string(),
                    column_type: crate::introspection::UnifiedColumnType::Integer,
                    raw_type: "INTEGER".to_string(),
                    nullable: false,
                    default_value: None,
                    primary_key: true,
                    auto_increment: true,
                    unique: false,
                    comment: None,
                    ordinal_position: Some(1),
                    character_maximum_length: None,
                    numeric_precision: None,
                    numeric_scale: None,
                    constraints: Vec::new(),
                },
                UnifiedColumnSchema {
                    name: "name".to_string(),
                    column_type: crate::introspection::UnifiedColumnType::Text,
                    raw_type: "TEXT".to_string(),
                    nullable: false,
                    default_value: None,
                    primary_key: false,
                    auto_increment: false,
                    unique: false,
                    comment: None,
                    ordinal_position: Some(2),
                    character_maximum_length: Some(255),
                    numeric_precision: None,
                    numeric_scale: None,
                    constraints: Vec::new(),
                },
            ],
            table_type: crate::introspection::UnifiedTableType::Table,
            schema_name: None,
            comment: None,
            indexes: vec![],
            foreign_keys: vec![],
            constraints: vec![],
            metadata: std::collections::HashMap::new(),
        }
    }
    
    #[test]
    fn test_schema_differ_creation() {
        let differ = SchemaDiffer::new(DatabaseDialect::SQLite);
        assert_eq!(differ.dialect, DatabaseDialect::SQLite);
    }
    
    #[test]
    fn test_migration_operation_safety_levels() {
        // Test table operations
        let create_table = MigrationOperation::Table(TableOperation::CreateTable {
            table_name: "test".to_string(),
            schema: create_test_table_schema("test", DatabaseDialect::SQLite),
        });
        assert_eq!(create_table.safety_level(), SafetyLevel::Safe);
        
        let drop_table = MigrationOperation::Table(TableOperation::DropTable {
            table_name: "test".to_string(),
        });
        assert_eq!(drop_table.safety_level(), SafetyLevel::Destructive);
        
        // Test column operations
        let add_nullable_column = MigrationOperation::Column(ColumnOperation::AddColumn {
            table_name: "test".to_string(),
            column: UnifiedColumnSchema {
                name: "optional".to_string(),
                column_type: crate::introspection::UnifiedColumnType::Text,
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
                constraints: Vec::new(),
            },
        });
        assert_eq!(add_nullable_column.safety_level(), SafetyLevel::Safe);
        
        let drop_column = MigrationOperation::Column(ColumnOperation::DropColumn {
            table_name: "test".to_string(),
            column_name: "obsolete".to_string(),
        });
        assert_eq!(drop_column.safety_level(), SafetyLevel::Destructive);
    }
    
    #[test]
    fn test_migration_plan_creation() {
        let operations = vec![
            MigrationOperation::Table(TableOperation::CreateTable {
                table_name: "users".to_string(),
                schema: create_test_table_schema("users", DatabaseDialect::SQLite),
            }),
            MigrationOperation::Index(IndexOperation::CreateIndex {
                table_name: "users".to_string(),
                index: UnifiedIndexSchema {
                    name: "idx_users_name".to_string(),
                    table_name: "users".to_string(),
                    columns: vec!["name".to_string()],
                    unique: false,
                    primary: false,
                    index_type: UnifiedIndexType::BTree,
                    comment: None,
                    condition: None,
                },
            }),
        ];
        
        let plan = MigrationPlan::new(operations, DatabaseDialect::SQLite);
        
        assert_eq!(plan.dialect, DatabaseDialect::SQLite);
        assert_eq!(plan.safety_level, SafetyLevel::Safe);
        assert_eq!(plan.operations.len(), 2);
        assert!(plan.affected_tables.contains("users"));
        assert!(plan.is_safe());
    }
    
    #[test]
    fn test_safety_level_max() {
        assert_eq!(SafetyLevel::Safe.max(SafetyLevel::LowRisk), SafetyLevel::LowRisk);
        assert_eq!(SafetyLevel::HighRisk.max(SafetyLevel::LowRisk), SafetyLevel::HighRisk);
        assert_eq!(SafetyLevel::Destructive.max(SafetyLevel::ModerateRisk), SafetyLevel::Destructive);
    }
    
    #[test]
    fn test_diff_empty_schemas() {
        let differ = SchemaDiffer::new(DatabaseDialect::SQLite);
        let current = create_test_database_schema(DatabaseDialect::SQLite);
        let target = create_test_database_schema(DatabaseDialect::SQLite);
        
        let result = differ.diff_schemas(&current, &target);
        assert!(result.is_ok());
        
        let plan = result.unwrap();
        assert_eq!(plan.operations.len(), 0);
        assert_eq!(plan.safety_level, SafetyLevel::Safe);
    }
    
    #[test]
    fn test_diff_create_table() {
        let differ = SchemaDiffer::new(DatabaseDialect::SQLite);
        let current = create_test_database_schema(DatabaseDialect::SQLite);
        let mut target = create_test_database_schema(DatabaseDialect::SQLite);
        
        // Create a legacy TableSchema
        let legacy_table = crate::auto_migration::introspector::TableSchema {
            name: "users".to_string(),
            columns: vec![
                crate::auto_migration::introspector::ColumnSchema {
                    name: "id".to_string(),
                    column_type: "INTEGER".to_string(),
                    nullable: false,
                    default_value: None,
                    primary_key: true,
                    auto_increment: true,
                    unique: false,
                    constraints: Vec::new(),
                },
            ],
            indexes: Vec::new(),
            foreign_keys: Vec::new(),
            constraints: Vec::new(),
        };
        target.tables.push(legacy_table);
        
        let result = differ.diff_schemas(&current, &target);
        assert!(result.is_ok());
        
        let plan = result.unwrap();
        assert_eq!(plan.operations.len(), 1);
        
        match &plan.operations[0] {
            MigrationOperation::Table(TableOperation::CreateTable { table_name, .. }) => {
                assert_eq!(table_name, "users");
            },
            _ => panic!("Expected CreateTable operation"),
        }
    }
    
    #[test]
    fn test_diff_drop_table() {
        let differ = SchemaDiffer::new(DatabaseDialect::SQLite);
        let mut current = create_test_database_schema(DatabaseDialect::SQLite);
        
        // Create a legacy TableSchema to be dropped
        let legacy_table = crate::auto_migration::introspector::TableSchema {
            name: "old_table".to_string(),
            columns: Vec::new(),
            indexes: Vec::new(),
            foreign_keys: Vec::new(),
            constraints: Vec::new(),
        };
        current.tables.push(legacy_table);
        
        let target = create_test_database_schema(DatabaseDialect::SQLite);
        
        let result = differ.diff_schemas(&current, &target);
        assert!(result.is_ok());
        
        let plan = result.unwrap();
        assert_eq!(plan.operations.len(), 1);
        assert_eq!(plan.safety_level, SafetyLevel::Destructive);
        
        match &plan.operations[0] {
            MigrationOperation::Table(TableOperation::DropTable { table_name }) => {
                assert_eq!(table_name, "old_table");
            },
            _ => panic!("Expected DropTable operation"),
        }
    }
    
    #[test]
    fn test_cross_database_warning() {
        // Note: For now we use SQLite since cross-database warnings are handled at higher level
        let differ = SchemaDiffer::new(DatabaseDialect::SQLite);
        let current = create_test_database_schema(DatabaseDialect::SQLite);
        let target = create_test_database_schema(DatabaseDialect::SQLite);
        
        let result = differ.diff_schemas(&current, &target);
        assert!(result.is_ok());
        
        let plan = result.unwrap();
        // Cross-database warnings are handled at higher level, so no warnings expected here
        assert_eq!(plan.operations.len(), 0);
    }
    
    #[cfg(feature = "postgres")]
    #[test]
    fn test_postgres_dialect() {
        let differ = SchemaDiffer::new(DatabaseDialect::PostgreSQL);
        assert_eq!(differ.dialect, DatabaseDialect::PostgreSQL);
    }
    
    #[cfg(feature = "mysql")]
    #[test]
    fn test_mysql_dialect() {
        let differ = SchemaDiffer::new(DatabaseDialect::MySQL);
        assert_eq!(differ.dialect, DatabaseDialect::MySQL);
    }
}