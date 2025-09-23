//! Rollback plan generation engine for converting forward migrations to rollback plans
//!
//! This module provides comprehensive rollback plan generation capabilities, including
//! operation reversal logic, schema reconstruction, data preservation analysis, and
//! dependency resolution for safe rollback execution.

use super::types::{RollbackOperation, RollbackConfig, RollbackColumnChanges, DataPreservationRequirement, DataPreservationStrategy, DataExportFormat};
use super::plan::{RollbackPlan, PreExecutionCheck, PostExecutionValidation, CheckType, ValidationType, CheckFailureAction, ValidationFailureAction};
use crate::auto_migration::{MigrationPlan, MigrationOperation, DatabaseSchema, TableSchema, ColumnSchema, ForeignKeySchema};
use crate::Result;
use std::time::{Duration, SystemTime};
use std::collections::{HashMap, HashSet};

/// Error types specific to rollback plan generation
#[derive(Debug, Clone, PartialEq)]
pub enum GeneratorError {
    /// Missing schema element needed for rollback generation
    MissingSchemaElement {
        element_type: String,
        element_name: String,
        context: String,
    },
    
    /// Unsupported migration operation for rollback
    UnsupportedOperation {
        operation_type: String,
        reason: String,
    },
    
    /// Schema inconsistency detected during generation
    SchemaInconsistency {
        description: String,
        affected_elements: Vec<String>,
    },
    
    /// Data preservation requirement cannot be satisfied
    DataPreservationImpossible {
        table: String,
        reason: String,
        suggested_alternative: Option<String>,
    },
    
    /// Circular dependency detected in operations
    CircularDependency {
        cycle_description: String,
        affected_operations: Vec<String>,
    },
    
    /// Configuration validation error
    ConfigurationError {
        field: String,
        message: String,
    },
}

impl std::fmt::Display for GeneratorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GeneratorError::MissingSchemaElement { element_type, element_name, context } => {
                write!(f, "Missing {} '{}' in schema context: {}", element_type, element_name, context)
            }
            GeneratorError::UnsupportedOperation { operation_type, reason } => {
                write!(f, "Unsupported operation '{}': {}", operation_type, reason)
            }
            GeneratorError::SchemaInconsistency { description, affected_elements } => {
                write!(f, "Schema inconsistency: {} (affects: {})", description, affected_elements.join(", "))
            }
            GeneratorError::DataPreservationImpossible { table, reason, suggested_alternative } => {
                match suggested_alternative {
                    Some(alt) => write!(f, "Data preservation impossible for table '{}': {} (try: {})", table, reason, alt),
                    None => write!(f, "Data preservation impossible for table '{}': {}", table, reason),
                }
            }
            GeneratorError::CircularDependency { cycle_description, affected_operations } => {
                write!(f, "Circular dependency detected: {} (operations: {})", cycle_description, affected_operations.join(", "))
            }
            GeneratorError::ConfigurationError { field, message } => {
                write!(f, "Configuration error in field '{}': {}", field, message)
            }
        }
    }
}

impl std::error::Error for GeneratorError {}

/// Comprehensive rollback plan generator with schema analysis and dependency resolution
pub struct RollbackOperationGenerator {
    /// Configuration for rollback generation behavior
    config: RollbackConfig,
    
    /// Cache for schema lookups to improve performance
    schema_cache: std::cell::RefCell<HashMap<String, CachedSchemaElement>>,
    
    /// Operation complexity scoring for duration estimation
    complexity_weights: OperationComplexityWeights,
}

/// Cached schema elements for performance optimization
#[derive(Debug, Clone)]
enum CachedSchemaElement {
    Table(TableSchema),
    Column { column: ColumnSchema },
    ForeignKey { foreign_key: ForeignKeySchema },
}

/// Weights for calculating operation complexity and duration
#[derive(Debug, Clone)]
pub struct OperationComplexityWeights {
    /// Base time per operation in seconds
    pub base_operation_time: u64,
    
    /// Multiplier for operations involving data
    pub data_operation_multiplier: f64,
    
    /// Multiplier for operations requiring schema reconstruction
    pub schema_reconstruction_multiplier: f64,
    
    /// Multiplier for operations with foreign key dependencies
    pub foreign_key_multiplier: f64,
    
    /// Time per estimated row affected (in microseconds)
    pub per_row_time_microseconds: u64,
}

impl Default for OperationComplexityWeights {
    fn default() -> Self {
        Self {
            base_operation_time: 5,
            data_operation_multiplier: 2.5,
            schema_reconstruction_multiplier: 1.8,
            foreign_key_multiplier: 1.5,
            per_row_time_microseconds: 10,
        }
    }
}

/// Analysis result for data preservation requirements
#[derive(Debug, Clone)]
struct DataPreservationAnalysis {
    /// Tables requiring data preservation
    required_preservations: Vec<DataPreservationRequirement>,
}

/// Result of dependency analysis for operation ordering
#[derive(Debug, Clone)]
struct DependencyAnalysis {
    /// Operations in execution order (reverse of dependency order)
    execution_order: Vec<usize>,
    
    /// Operations that can be executed in parallel
    parallel_groups: Vec<Vec<usize>>,
}

impl RollbackOperationGenerator {
    /// Create a new rollback operation generator with default configuration
    pub fn new() -> Self {
        Self {
            config: RollbackConfig::default(),
            schema_cache: std::cell::RefCell::new(HashMap::new()),
            complexity_weights: OperationComplexityWeights::default(),
        }
    }
    
    /// Create a new rollback operation generator with custom configuration
    pub fn with_config(config: RollbackConfig) -> Self {
        Self {
            config,
            schema_cache: std::cell::RefCell::new(HashMap::new()),
            complexity_weights: OperationComplexityWeights::default(),
        }
    }
    
    /// Set custom complexity weights for duration estimation
    pub fn with_complexity_weights(mut self, weights: OperationComplexityWeights) -> Self {
        self.complexity_weights = weights;
        self
    }
    
    /// Generate complete rollback plan from forward migration plan
    pub fn generate_rollback_operations(
        &self,
        forward_plan: &MigrationPlan,
        current_schema: &DatabaseSchema,
    ) -> Result<RollbackPlan> {
        // Validate inputs
        self.validate_generation_inputs(forward_plan, current_schema)?;
        
        // Clear schema cache for fresh generation
        self.schema_cache.borrow_mut().clear();
        
        // Generate reverse operations for each forward operation
        let mut rollback_operations = Vec::new();
        for (index, forward_op) in forward_plan.operations.iter().enumerate() {
            match self.generate_reverse_operation(forward_op, current_schema) {
                Ok(rollback_op) => rollback_operations.push(rollback_op),
                Err(e) => {
                    return Err(crate::D1RsError::ValidationError(
                        format!("Failed to generate reverse operation for operation {}: {}", index, e)
                    ));
                }
            }
        }
        
        // Reverse the order since rollback should undo operations in reverse
        rollback_operations.reverse();
        
        // Analyze dependencies and optimize execution order
        let dependency_analysis = self.analyze_dependencies(&rollback_operations, current_schema)?;
        let ordered_operations = self.apply_dependency_ordering(rollback_operations, &dependency_analysis);
        
        // Analyze data preservation requirements
        let preservation_analysis = self.analyze_data_preservation(&ordered_operations, current_schema)?;
        
        // Generate pre-execution checks
        let pre_execution_checks = self.generate_pre_execution_checks(&ordered_operations, current_schema)?;
        
        // Generate post-execution validations
        let post_execution_validations = self.generate_post_execution_validations(&ordered_operations, current_schema)?;
        
        // Estimate total duration
        let _estimated_duration = self.estimate_plan_duration(&ordered_operations, &dependency_analysis)?;
        
        // Create plan ID
        let plan_id = format!("rollback_{}", 
            forward_plan.operations.len(),
        );
        
        // Create the rollback plan
        let mut plan = RollbackPlan::new(
            plan_id,
            ordered_operations,
            Some("generated_migration".to_string()),
            self.config.clone(),
            format!("Auto-generated rollback for migration with {} operations", forward_plan.operations.len()),
        );
        
        // Add data preservation requirements
        for requirement in preservation_analysis.required_preservations {
            plan.add_data_preservation_requirement(requirement);
        }
        
        // Add pre-execution checks
        for check in pre_execution_checks {
            plan.add_pre_execution_check(check);
        }
        
        // Add post-execution validations
        for validation in post_execution_validations {
            plan.add_post_execution_validation(validation);
        }
        
        Ok(plan)
    }
    
    /// Generate reverse operation for a single forward operation
    fn generate_reverse_operation(
        &self,
        forward_operation: &MigrationOperation,
        current_schema: &DatabaseSchema,
    ) -> Result<RollbackOperation> {
        match forward_operation {
            MigrationOperation::CreateTable { definition, .. } => {
                // CreateTable → DropTable with data preservation
                Ok(RollbackOperation::DropTable {
                    name: definition.name.clone(),
                    preserve_data: self.config.preserve_data_on_rollback,
                    backup_table_name: if self.config.create_backup_tables {
                        Some(format!("{}_backup_{}", definition.name, SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()))
                    } else {
                        None
                    },
                })
            }
            
            MigrationOperation::DropTable { name, .. } => {
                // DropTable → RecreateTable with schema reconstruction
                let table_schema = self.find_table_schema(name, current_schema)?;
                Ok(RollbackOperation::RecreateTable {
                    definition: table_schema,
                    restore_data: self.config.restore_data_on_rollback,
                    data_source: if self.config.create_backup_tables {
                        Some(format!("{}_backup", name))
                    } else {
                        None
                    },
                })
            }
            
            MigrationOperation::AddColumn { table, column, .. } => {
                // AddColumn → DropColumn with data preservation
                Ok(RollbackOperation::DropColumn {
                    table: table.clone(),
                    column: column.name.clone(),
                    preserve_data: self.config.preserve_data_on_rollback,
                })
            }
            
            MigrationOperation::DropColumn { table, column, .. } => {
                // DropColumn → AddColumn with data restoration
                let column_schema = self.find_column_schema(table, column, current_schema)?;
                Ok(RollbackOperation::AddColumn {
                    table: table.clone(),
                    column: column_schema,
                    restore_data: self.config.restore_data_on_rollback,
                })
            }
            
            MigrationOperation::ModifyColumn { table, column, changes, .. } => {
                // ModifyColumn → ModifyColumn with reverse changes
                let reverse_changes = RollbackColumnChanges {
                    type_change: changes.type_change.as_ref().map(|(old, new)| (new.clone(), old.clone())),
                    null_change: changes.null_change.map(|(old, new)| (new, old)),
                    default_change: changes.default_change.as_ref().map(|(old, new)| (new.clone(), old.clone())),
                    constraint_changes: Vec::new(), // TODO: Convert constraint changes
                };
                Ok(RollbackOperation::ModifyColumn {
                    table: table.clone(),
                    column: column.clone(),
                    changes: reverse_changes,
                    preserve_data: self.config.preserve_data_on_rollback,
                })
            }
            
            MigrationOperation::CreateIndex { table, index, .. } => {
                // CreateIndex → DropIndex
                Ok(RollbackOperation::DropIndex {
                    name: index.name.clone(),
                    table: table.clone(),
                })
            }
            
            MigrationOperation::DropIndex { name, .. } => {
                // DropIndex → CreateIndex with schema lookup - need to find table
                // For now, return error as we can't determine table from just index name
                Err(crate::D1RsError::ValidationError(
                    format!("Cannot generate rollback for DropIndex '{}' - table information required", name)
                ))
            }
            
            MigrationOperation::AddForeignKey { constraint } => {
                // AddForeignKey → DropForeignKey
                // Find which table this foreign key belongs to by looking at the constraint columns
                let table_name = self.find_table_for_foreign_key_columns(&constraint.columns, current_schema)?;
                Ok(RollbackOperation::DropForeignKey {
                    table: table_name,
                    constraint_name: constraint.name.clone(),
                })
            }
            
            MigrationOperation::DropForeignKey { table, constraint_name, .. } => {
                // DropForeignKey → AddForeignKey with constraint reconstruction
                let foreign_key_schema = self.find_foreign_key_schema(table, constraint_name, current_schema)?;
                Ok(RollbackOperation::AddForeignKey {
                    table: table.clone(),
                    constraint: foreign_key_schema,
                })
            }
            
            MigrationOperation::RenameTable { old_name, new_name, .. } => {
                // RenameTable → RenameTable with name swap
                Ok(RollbackOperation::RenameTable {
                    old_name: new_name.clone(),
                    new_name: old_name.clone(),
                })
            }
            
            MigrationOperation::RenameColumn { table, old_name, new_name, .. } => {
                // RenameColumn → RenameColumn with name swap
                Ok(RollbackOperation::RenameColumn {
                    table: table.clone(),
                    old_name: new_name.clone(),
                    new_name: old_name.clone(),
                })
            }
        }
    }
    
    /// Find table schema from current database schema
    fn find_table_schema(&self, table_name: &str, current_schema: &DatabaseSchema) -> Result<TableSchema> {
        // Check cache first
        let cache_key = format!("table:{}", table_name);
        if let Some(CachedSchemaElement::Table(table_schema)) = self.schema_cache.borrow().get(&cache_key) {
            return Ok(table_schema.clone());
        }
        
        // Search in current schema
        for table in &current_schema.tables {
            if table.name == table_name {
                // Cache the result
                self.schema_cache.borrow_mut().insert(
                    cache_key,
                    CachedSchemaElement::Table(table.clone())
                );
                return Ok(table.clone());
            }
        }
        
        Err(crate::D1RsError::ValidationError(format!(
            "Missing schema element: table '{}' not found during rollback generation",
            table_name
        )))
    }
    
    /// Find column schema from table definition
    fn find_column_schema(&self, table_name: &str, column_name: &str, current_schema: &DatabaseSchema) -> Result<ColumnSchema> {
        let cache_key = format!("column:{}:{}", table_name, column_name);
        if let Some(CachedSchemaElement::Column { column }) = self.schema_cache.borrow().get(&cache_key) {
            return Ok(column.clone());
        }
        
        let table_schema = self.find_table_schema(table_name, current_schema)?;
        for column in &table_schema.columns {
            if column.name == column_name {
                self.schema_cache.borrow_mut().insert(
                    cache_key,
                    CachedSchemaElement::Column {
                        column: column.clone(),
                    }
                );
                return Ok(column.clone());
            }
        }
        
        Err(crate::D1RsError::ValidationError(format!(
            "Missing schema element: column '{}.{}' not found during rollback generation",
            table_name, column_name
        )))
    }
    
    
    /// Find which table contains the specified foreign key columns
    fn find_table_for_foreign_key_columns(&self, columns: &[String], current_schema: &DatabaseSchema) -> Result<String> {
        for table in &current_schema.tables {
            let table_column_names: Vec<&String> = table.columns.iter().map(|c| &c.name).collect();
            
            // Check if all foreign key columns exist in this table
            let all_columns_exist = columns.iter().all(|fk_col| {
                table_column_names.contains(&fk_col)
            });
            
            if all_columns_exist {
                return Ok(table.name.clone());
            }
        }
        
        Err(crate::D1RsError::ValidationError(format!(
            "Cannot find table containing foreign key columns: {:?}",
            columns
        )))
    }
    
    /// Find foreign key schema from table definition
    fn find_foreign_key_schema(&self, table_name: &str, constraint_name: &str, current_schema: &DatabaseSchema) -> Result<ForeignKeySchema> {
        let cache_key = format!("fk:{}:{}", table_name, constraint_name);
        if let Some(CachedSchemaElement::ForeignKey { foreign_key }) = self.schema_cache.borrow().get(&cache_key) {
            return Ok(foreign_key.clone());
        }
        
        let table_schema = self.find_table_schema(table_name, current_schema)?;
        for fk in &table_schema.foreign_keys {
            if fk.name == constraint_name {
                self.schema_cache.borrow_mut().insert(
                    cache_key,
                    CachedSchemaElement::ForeignKey {
                        foreign_key: fk.clone(),
                    }
                );
                return Ok(fk.clone());
            }
        }
        
        Err(crate::D1RsError::ValidationError(format!(
            "Missing schema element: foreign_key '{}.{}' not found during rollback generation",
            table_name, constraint_name
        )))
    }
    
    
    /// Validate inputs for rollback generation
    fn validate_generation_inputs(&self, forward_plan: &MigrationPlan, current_schema: &DatabaseSchema) -> Result<()> {
        if forward_plan.operations.is_empty() {
            return Err(crate::D1RsError::ValidationError(
                "Forward migration plan contains no operations".to_string()
            ));
        }
        
        if current_schema.tables.is_empty() {
            return Err(crate::D1RsError::ValidationError(
                "Current schema contains no tables".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Analyze dependencies between rollback operations
    fn analyze_dependencies(&self, operations: &[RollbackOperation], _current_schema: &DatabaseSchema) -> Result<DependencyAnalysis> {
        let mut dependencies: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut affected_tables: HashMap<String, Vec<usize>> = HashMap::new();
        
        // Build table-to-operations mapping
        for (index, operation) in operations.iter().enumerate() {
            let table = operation.affected_table();
            affected_tables.entry(table.to_string()).or_default().push(index);
        }
        
        // Analyze dependencies based on operation types and table relationships
        for (index, operation) in operations.iter().enumerate() {
            match operation {
                RollbackOperation::DropForeignKey { .. } => {
                    // Foreign key drops should happen before table structure changes
                    // Find any table structure operations for the same table
                    let table = operation.affected_table();
                    if let Some(table_ops) = affected_tables.get(table) {
                        for &other_index in table_ops {
                            if other_index != index {
                                match &operations[other_index] {
                                    RollbackOperation::DropTable { .. } |
                                    RollbackOperation::RecreateTable { .. } => {
                                        dependencies.entry(other_index).or_default().push(index);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                
                RollbackOperation::DropIndex { .. } => {
                    // Index drops should happen before column/table operations
                    let table = operation.affected_table();
                    if let Some(table_ops) = affected_tables.get(table) {
                        for &other_index in table_ops {
                            if other_index != index {
                                match &operations[other_index] {
                                    RollbackOperation::DropColumn { .. } |
                                    RollbackOperation::DropTable { .. } |
                                    RollbackOperation::RecreateTable { .. } => {
                                        dependencies.entry(other_index).or_default().push(index);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                
                _ => {}
            }
        }
        
        // Create execution order respecting dependencies
        let execution_order = self.topological_sort(operations.len(), &dependencies)?;
        
        // Create parallel groups (operations with no dependencies between them)
        let parallel_groups = self.create_parallel_groups(&execution_order, &dependencies);
        
        Ok(DependencyAnalysis {
            execution_order,
            parallel_groups,
        })
    }
    
    /// Perform topological sort for dependency ordering
    fn topological_sort(&self, operation_count: usize, dependencies: &HashMap<usize, Vec<usize>>) -> Result<Vec<usize>> {
        let mut result = Vec::new();
        let mut visited = vec![false; operation_count];
        let mut visiting = vec![false; operation_count];
        
        for i in 0..operation_count {
            if !visited[i] {
                self.topological_sort_visit(i, dependencies, &mut visited, &mut visiting, &mut result)?;
            }
        }
        
        Ok(result)
    }
    
    /// Helper for topological sort with cycle detection
    fn topological_sort_visit(
        &self,
        node: usize,
        dependencies: &HashMap<usize, Vec<usize>>,
        visited: &mut Vec<bool>,
        visiting: &mut Vec<bool>,
        result: &mut Vec<usize>,
    ) -> Result<()> {
        if visiting[node] {
            return Err(crate::D1RsError::ValidationError(format!(
                "Circular dependency detected involving operation {}", node
            )));
        }
        
        if visited[node] {
            return Ok(());
        }
        
        visiting[node] = true;
        
        if let Some(deps) = dependencies.get(&node) {
            for &dep in deps {
                self.topological_sort_visit(dep, dependencies, visited, visiting, result)?;
            }
        }
        
        visiting[node] = false;
        visited[node] = true;
        result.push(node);
        
        Ok(())
    }
    
    /// Create parallel execution groups
    fn create_parallel_groups(&self, execution_order: &[usize], dependencies: &HashMap<usize, Vec<usize>>) -> Vec<Vec<usize>> {
        let mut groups = Vec::new();
        let mut remaining: HashSet<usize> = execution_order.iter().cloned().collect();
        
        while !remaining.is_empty() {
            let mut current_group = Vec::new();
            let mut to_remove = Vec::new();
            
            for &op_index in &remaining {
                // Check if all dependencies are satisfied (not in remaining)
                let deps_satisfied = dependencies
                    .get(&op_index)
                    .map(|deps| deps.iter().all(|&dep| !remaining.contains(&dep)))
                    .unwrap_or(true);
                
                if deps_satisfied {
                    current_group.push(op_index);
                    to_remove.push(op_index);
                }
            }
            
            for op_index in to_remove {
                remaining.remove(&op_index);
            }
            
            if !current_group.is_empty() {
                groups.push(current_group);
            }
        }
        
        groups
    }
    
    
    /// Apply dependency ordering to operations
    fn apply_dependency_ordering(&self, operations: Vec<RollbackOperation>, analysis: &DependencyAnalysis) -> Vec<RollbackOperation> {
        let mut ordered_operations = Vec::with_capacity(operations.len());
        
        for &index in &analysis.execution_order {
            if index < operations.len() {
                ordered_operations.push(operations[index].clone());
            }
        }
        
        // If any operations weren't included in the ordering, append them
        for (original_index, operation) in operations.iter().enumerate() {
            if !analysis.execution_order.contains(&original_index) {
                ordered_operations.push(operation.clone());
            }
        }
        
        ordered_operations
    }
    
    /// Analyze data preservation requirements for operations
    fn analyze_data_preservation(&self, operations: &[RollbackOperation], _current_schema: &DatabaseSchema) -> Result<DataPreservationAnalysis> {
        let mut required_preservations = Vec::new();
        
        for operation in operations {
            match operation {
                RollbackOperation::DropTable { name, preserve_data, .. } => {
                    if *preserve_data {
                        let requirement = DataPreservationRequirement::new(
                            name.clone(),
                            vec!["*".to_string()], // All columns
                            DataPreservationStrategy::BackupTable {
                                backup_name: format!("{}_backup", name),
                                drop_after_restore: true,
                            },
                        );
                        required_preservations.push(requirement);
                    } else {
                    }
                }
                
                RollbackOperation::DropColumn { table, column, preserve_data, .. } => {
                    if *preserve_data {
                        let requirement = DataPreservationRequirement::new(
                            table.clone(),
                            vec![column.clone()],
                            DataPreservationStrategy::ExternalExport {
                                export_path: format!("./backups/{}_{}_data.json", table, column),
                                export_format: DataExportFormat::Json,
                            },
                        );
                        required_preservations.push(requirement);
                    } else {
                    }
                }
                
                RollbackOperation::ModifyColumn { table, column, preserve_data, changes, .. } => {
                    if changes.is_lossy() && *preserve_data {
                        let requirement = DataPreservationRequirement::new(
                            table.clone(),
                            vec![column.clone()],
                            DataPreservationStrategy::TemporaryStorage {
                                storage_location: format!("./temp/{}_{}_backup", table, column),
                                size_limit: Some(100 * 1024 * 1024), // 100MB limit
                            },
                        );
                        required_preservations.push(requirement);
                    } else if changes.is_lossy() {
                    }
                }
                
                _ => {
                    // Non-destructive operations don't require special preservation
                }
            }
        }
        
        Ok(DataPreservationAnalysis {
            required_preservations,
        })
    }
    
    /// Generate pre-execution checks for the rollback plan
    fn generate_pre_execution_checks(&self, operations: &[RollbackOperation], _current_schema: &DatabaseSchema) -> Result<Vec<PreExecutionCheck>> {
        let mut checks = Vec::new();
        let mut check_counter = 0;
        
        // Add table existence checks for operations that depend on tables
        let mut checked_tables = HashSet::new();
        for operation in operations {
            let table_name = operation.affected_table();
            if !checked_tables.contains(table_name) {
                match operation {
                    RollbackOperation::DropColumn { .. } |
                    RollbackOperation::AddColumn { .. } |
                    RollbackOperation::ModifyColumn { .. } |
                    RollbackOperation::CreateIndex { .. } |
                    RollbackOperation::DropIndex { .. } |
                    RollbackOperation::AddForeignKey { .. } |
                    RollbackOperation::DropForeignKey { .. } => {
                        check_counter += 1;
                        checks.push(
                            PreExecutionCheck::new(
                                format!("table_exists_{}", check_counter),
                                CheckType::TableExists,
                                format!("Verify table '{}' exists before rollback", table_name),
                                true,
                            )
                            .with_sql_query(
                                format!("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{}'", table_name),
                                "1".to_string(),
                            )
                            .with_failure_action(CheckFailureAction::Stop)
                        );
                        checked_tables.insert(table_name.to_string());
                    }
                    _ => {}
                }
            }
        }
        
        // Add backup availability checks
        for operation in operations {
            match operation {
                RollbackOperation::RecreateTable { data_source: Some(backup_table), .. } => {
                    check_counter += 1;
                    checks.push(
                        PreExecutionCheck::new(
                            format!("backup_available_{}", check_counter),
                            CheckType::BackupAvailability,
                            format!("Verify backup table '{}' exists for data restoration", backup_table),
                            true,
                        )
                        .with_sql_query(
                            format!("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{}'", backup_table),
                            "1".to_string(),
                        )
                        .with_failure_action(CheckFailureAction::Stop)
                    );
                }
                _ => {}
            }
        }
        
        // Add disk space check if there are preservation requirements
        let has_preservation = operations.iter().any(|op| match op {
            RollbackOperation::DropTable { preserve_data, .. } |
            RollbackOperation::DropColumn { preserve_data, .. } |
            RollbackOperation::ModifyColumn { preserve_data, .. } => *preserve_data,
            _ => false,
        });
        
        if has_preservation {
            check_counter += 1;
            checks.push(
                PreExecutionCheck::new(
                    format!("disk_space_{}", check_counter),
                    CheckType::DiskSpace,
                    "Verify sufficient disk space for data preservation".to_string(),
                    true,
                )
                .with_failure_action(CheckFailureAction::Stop)
            );
        }
        
        Ok(checks)
    }
    
    /// Generate post-execution validations for the rollback plan
    fn generate_post_execution_validations(&self, operations: &[RollbackOperation], _current_schema: &DatabaseSchema) -> Result<Vec<PostExecutionValidation>> {
        let mut validations = Vec::new();
        let mut validation_counter = 0;
        
        // Add schema validation for table operations
        let mut validated_tables = HashSet::new();
        for operation in operations {
            let table_name = operation.affected_table();
            if !validated_tables.contains(table_name) {
                match operation {
                    RollbackOperation::RecreateTable { .. } |
                    RollbackOperation::AddColumn { .. } |
                    RollbackOperation::DropColumn { .. } |
                    RollbackOperation::ModifyColumn { .. } => {
                        validation_counter += 1;
                        validations.push(
                            PostExecutionValidation::new(
                                format!("schema_validation_{}", validation_counter),
                                ValidationType::SchemaValidation,
                                format!("Verify schema integrity for table '{}'", table_name),
                                true,
                            )
                            .with_sql_query(
                                format!("PRAGMA table_info({})", table_name),
                                "Schema validation successful".to_string(),
                            )
                            .with_failure_action(ValidationFailureAction::MarkFailed)
                        );
                        validated_tables.insert(table_name.to_string());
                    }
                    _ => {}
                }
            }
        }
        
        // Add constraint validation for foreign key operations
        for operation in operations {
            match operation {
                RollbackOperation::AddForeignKey { table, constraint, .. } => {
                    validation_counter += 1;
                    validations.push(
                        PostExecutionValidation::new(
                            format!("fk_validation_{}", validation_counter),
                            ValidationType::ForeignKeyValidation,
                            format!("Verify foreign key constraint '{}' on table '{}'", constraint.name, table),
                            true,
                        )
                        .with_failure_action(ValidationFailureAction::MarkFailed)
                    );
                }
                _ => {}
            }
        }
        
        // Add data integrity validation for operations with data restoration
        for operation in operations {
            match operation {
                RollbackOperation::RecreateTable { definition, restore_data: true, .. } => {
                    validation_counter += 1;
                    validations.push(
                        PostExecutionValidation::new(
                            format!("data_integrity_{}", validation_counter),
                            ValidationType::DataIntegrity,
                            format!("Verify data integrity for recreated table '{}'", definition.name),
                            false, // Non-required since data might not be perfectly restored
                        )
                        .with_sql_query(
                            format!("SELECT COUNT(*) FROM {}", definition.name),
                            "Data count validation".to_string(),
                        )
                        .with_failure_action(ValidationFailureAction::WarnButContinue)
                    );
                }
                _ => {}
            }
        }
        
        Ok(validations)
    }
    
    /// Estimate total duration for the rollback plan
    fn estimate_plan_duration(&self, operations: &[RollbackOperation], analysis: &DependencyAnalysis) -> Result<Duration> {
        let mut total_duration = Duration::from_secs(0);
        
        // If we can execute in parallel, use the critical path
        if analysis.parallel_groups.len() > 1 {
            // Calculate duration for each parallel group and take the maximum
            let mut max_group_duration = Duration::from_secs(0);
            
            for group in &analysis.parallel_groups {
                let mut group_duration = Duration::from_secs(0);
                
                for &op_index in group {
                    if op_index < operations.len() {
                        let op_duration = self.estimate_operation_duration(&operations[op_index]);
                        group_duration = std::cmp::max(group_duration, op_duration);
                    }
                }
                
                max_group_duration = max_group_duration + group_duration;
            }
            
            total_duration = max_group_duration;
        } else {
            // Sequential execution - sum all operation durations
            for operation in operations {
                total_duration += self.estimate_operation_duration(operation);
            }
        }
        
        // Add overhead for safety checks and validations
        let overhead = Duration::from_secs((operations.len() as u64) * 2); // 2 seconds per operation for checks
        total_duration += overhead;
        
        Ok(total_duration)
    }
    
    /// Estimate duration for a single operation
    fn estimate_operation_duration(&self, operation: &RollbackOperation) -> Duration {
        let base_time = Duration::from_secs(self.complexity_weights.base_operation_time);
        
        let complexity_multiplier = match operation {
            RollbackOperation::DropTable { preserve_data: true, .. } |
            RollbackOperation::RecreateTable { restore_data: true, .. } => {
                self.complexity_weights.data_operation_multiplier
            }
            
            RollbackOperation::RecreateTable { .. } => {
                self.complexity_weights.schema_reconstruction_multiplier
            }
            
            RollbackOperation::AddForeignKey { .. } |
            RollbackOperation::DropForeignKey { .. } => {
                self.complexity_weights.foreign_key_multiplier
            }
            
            RollbackOperation::ModifyColumn { changes, .. } => {
                if changes.is_lossy() {
                    self.complexity_weights.data_operation_multiplier
                } else {
                    1.0
                }
            }
            
            _ => 1.0,
        };
        
        let estimated_millis = (base_time.as_millis() as f64 * complexity_multiplier) as u64;
        Duration::from_millis(estimated_millis)
    }
}

impl Default for RollbackOperationGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generator_creation() {
        let generator = RollbackOperationGenerator::new();
        assert_eq!(generator.config.preserve_data_on_rollback, true);
        
        let custom_config = RollbackConfig {
            preserve_data_on_rollback: false,
            ..RollbackConfig::default()
        };
        
        let custom_generator = RollbackOperationGenerator::with_config(custom_config.clone());
        assert_eq!(custom_generator.config.preserve_data_on_rollback, false);
    }
    
    #[test]
    fn test_topological_sort_simple() {
        let generator = RollbackOperationGenerator::new();
        let mut dependencies = HashMap::new();
        dependencies.insert(1, vec![0]); // Operation 1 depends on operation 0
        
        let result = generator.topological_sort(2, &dependencies).unwrap();
        assert_eq!(result, vec![0, 1]);
    }
    
    #[test]
    fn test_topological_sort_cycle_detection() {
        let generator = RollbackOperationGenerator::new();
        let mut dependencies = HashMap::new();
        dependencies.insert(0, vec![1]);
        dependencies.insert(1, vec![0]); // Circular dependency
        
        let result = generator.topological_sort(2, &dependencies);
        assert!(result.is_err());
        
        if let Err(crate::D1RsError::ValidationError(err_msg)) = result {
            assert!(err_msg.contains("Circular dependency detected"));
        } else {
            panic!("Expected ValidationError with circular dependency message");
        }
    }
    
    #[test]
    fn test_operation_duration_estimation() {
        let generator = RollbackOperationGenerator::new();
        
        let simple_operation = RollbackOperation::RenameTable {
            old_name: "old".to_string(),
            new_name: "new".to_string(),
        };
        
        let complex_operation = RollbackOperation::RecreateTable {
            definition: TableSchema {
                name: "test_table".to_string(),
                columns: vec![],
                foreign_keys: vec![],
                indexes: vec![],
                constraints: vec![],
            },
            restore_data: true,
            data_source: Some("backup_table".to_string()),
        };
        
        let simple_duration = generator.estimate_operation_duration(&simple_operation);
        let complex_duration = generator.estimate_operation_duration(&complex_operation);
        
        assert!(complex_duration > simple_duration);
    }
    
    #[test]
    fn test_parallel_group_creation() {
        let generator = RollbackOperationGenerator::new();
        let execution_order = vec![0, 1, 2, 3];
        let mut dependencies = HashMap::new();
        dependencies.insert(2, vec![0]); // Operation 2 depends on 0
        dependencies.insert(3, vec![1]); // Operation 3 depends on 1
        
        let groups = generator.create_parallel_groups(&execution_order, &dependencies);
        
        // Should have two groups: [0, 1] and [2, 3]
        assert_eq!(groups.len(), 2);
        assert!(groups[0].contains(&0) && groups[0].contains(&1));
        assert!(groups[1].contains(&2) && groups[1].contains(&3));
    }
    
    
    #[test]
    fn test_data_preservation_analysis() {
        let generator = RollbackOperationGenerator::new();
        
        let operations = vec![
            RollbackOperation::DropTable {
                name: "test_table".to_string(),
                preserve_data: true,
                backup_table_name: Some("test_table_backup".to_string()),
            },
            RollbackOperation::DropColumn {
                table: "users".to_string(),
                column: "temp_col".to_string(),
                preserve_data: false,
            },
        ];
        
        let schema = DatabaseSchema {
            tables: vec![],
        };
        
        let analysis = generator.analyze_data_preservation(&operations, &schema).unwrap();
        
        assert_eq!(analysis.required_preservations.len(), 1);
    }
    
    #[test]
    fn test_pre_execution_checks_generation() {
        let generator = RollbackOperationGenerator::new();
        
        let operations = vec![
            RollbackOperation::DropColumn {
                table: "users".to_string(),
                column: "temp_col".to_string(),
                preserve_data: true,
            },
            RollbackOperation::RecreateTable {
                definition: TableSchema {
                    name: "test_table".to_string(),
                    columns: vec![],
                    foreign_keys: vec![],
                    indexes: vec![],
                    constraints: vec![],
                },
                restore_data: true,
                data_source: Some("test_table_backup".to_string()),
            },
        ];
        
        let schema = DatabaseSchema {
            tables: vec![],
        };
        
        let checks = generator.generate_pre_execution_checks(&operations, &schema).unwrap();
        
        // Should generate table existence check, backup availability check, and disk space check
        assert!(checks.len() >= 3);
        
        let has_table_check = checks.iter().any(|c| matches!(c.check_type, CheckType::TableExists));
        let has_backup_check = checks.iter().any(|c| matches!(c.check_type, CheckType::BackupAvailability));
        let has_disk_check = checks.iter().any(|c| matches!(c.check_type, CheckType::DiskSpace));
        
        assert!(has_table_check);
        assert!(has_backup_check);
        assert!(has_disk_check);
    }
    
    #[test]
    fn test_post_execution_validations_generation() {
        let generator = RollbackOperationGenerator::new();
        
        let operations = vec![
            RollbackOperation::RecreateTable {
                definition: TableSchema {
                    name: "test_table".to_string(),
                    columns: vec![],
                    foreign_keys: vec![],
                    indexes: vec![],
                    constraints: vec![],
                },
                restore_data: true,
                data_source: Some("test_table_backup".to_string()),
            },
            RollbackOperation::AddForeignKey {
                table: "users".to_string(),
                constraint: ForeignKeySchema {
                    name: "fk_test".to_string(),
                    columns: vec!["user_id".to_string()],
                    referenced_table: "profiles".to_string(),
                    referenced_columns: vec!["id".to_string()],
                    on_delete: Some("CASCADE".to_string()),
                    on_update: Some("RESTRICT".to_string()),
                },
            },
        ];
        
        let schema = DatabaseSchema {
            tables: vec![],
        };
        
        let validations = generator.generate_post_execution_validations(&operations, &schema).unwrap();
        
        // Should generate schema validation, foreign key validation, and data integrity validation
        assert!(validations.len() >= 3);
        
        let has_schema_validation = validations.iter().any(|v| matches!(v.validation_type, ValidationType::SchemaValidation));
        let has_fk_validation = validations.iter().any(|v| matches!(v.validation_type, ValidationType::ForeignKeyValidation));
        let has_data_validation = validations.iter().any(|v| matches!(v.validation_type, ValidationType::DataIntegrity));
        
        assert!(has_schema_validation);
        assert!(has_fk_validation);
        assert!(has_data_validation);
    }
    
    #[test]
    fn test_generator_error_display() {
        let error = GeneratorError::MissingSchemaElement {
            element_type: "table".to_string(),
            element_name: "users".to_string(),
            context: "rollback generation".to_string(),
        };
        
        let display = format!("{}", error);
        assert!(display.contains("Missing table 'users'"));
        assert!(display.contains("rollback generation"));
    }
    
    #[test]
    fn test_complexity_weights_customization() {
        let custom_weights = OperationComplexityWeights {
            base_operation_time: 10,
            data_operation_multiplier: 3.0,
            schema_reconstruction_multiplier: 2.0,
            foreign_key_multiplier: 1.8,
            per_row_time_microseconds: 20,
        };
        
        let generator = RollbackOperationGenerator::new()
            .with_complexity_weights(custom_weights.clone());
        
        assert_eq!(generator.complexity_weights.base_operation_time, 10);
        assert_eq!(generator.complexity_weights.data_operation_multiplier, 3.0);
    }
}