//! Performance impact validation
//! 
//! Analyzes and estimates performance implications of migration operations.

use crate::Result;
use super::super::{MigrationPlan, MigrationOperation};
use super::super::introspector::{ColumnSchema, TableSchema, IndexSchema};
use std::time::Duration;

/// Performance impact validator
pub struct PerformanceImpactValidator;

impl PerformanceImpactValidator {
    pub fn new() -> Self {
        Self
    }
    
    /// Validate performance impact for entire migration plan
    pub fn validate_performance(&self, plan: &MigrationPlan) -> Result<PerformanceImpactResult> {
        let mut result = PerformanceImpactResult::new();
        
        for operation in &plan.operations {
            self.analyze_operation_performance(operation, &mut result)?;
        }
        
        self.analyze_cumulative_impact(&plan.operations, &mut result)?;
        self.estimate_migration_duration(&plan.operations, &mut result)?;
        
        Ok(result)
    }
    
    /// Analyze performance impact for individual operation
    fn analyze_operation_performance(&self, operation: &MigrationOperation, result: &mut PerformanceImpactResult) -> Result<()> {
        match operation {
            MigrationOperation::CreateTable { definition } => {
                self.analyze_create_table_performance(definition, result)?;
            }
            MigrationOperation::DropTable { name } => {
                self.analyze_drop_table_performance(name, result)?;
            }
            MigrationOperation::AddColumn { table, column } => {
                self.analyze_add_column_performance(table, column, result)?;
            }
            MigrationOperation::DropColumn { table, column } => {
                self.analyze_drop_column_performance(table, column, result)?;
            }
            MigrationOperation::ModifyColumn { table, column, changes } => {
                self.analyze_modify_column_performance(table, column, changes, result)?;
            }
            MigrationOperation::CreateIndex { table, index } => {
                self.analyze_create_index_performance(table, index, result)?;
            }
            MigrationOperation::DropIndex { name } => {
                self.analyze_drop_index_performance(name, result)?;
            }
            MigrationOperation::AddForeignKey { constraint } => {
                self.analyze_add_foreign_key_performance(constraint, result)?;
            }
            MigrationOperation::DropForeignKey { table, constraint_name } => {
                self.analyze_drop_foreign_key_performance(table, constraint_name, result)?;
            }
            MigrationOperation::RenameTable { old_name, new_name } => {
                self.analyze_rename_table_performance(old_name, new_name, result)?;
            }
            MigrationOperation::RenameColumn { table, old_name, new_name } => {
                self.analyze_rename_column_performance(table, old_name, new_name, result)?;
            }
        }
        
        Ok(())
    }
    
    /// Analyze table creation performance
    fn analyze_create_table_performance(&self, definition: &TableSchema, result: &mut PerformanceImpactResult) -> Result<()> {
        let estimated_time = self.estimate_table_creation_time(definition);
        let memory_usage = self.estimate_table_memory_usage(definition);
        
        result.add_performance_impact(
            PerformanceImpact::TableOperation {
                operation_type: "create_table".to_string(),
                table: definition.name.clone(),
                estimated_duration: estimated_time,
                memory_impact: memory_usage,
                cpu_impact: CpuImpact::Low,
                io_impact: IoImpact::Low,
                description: format!("Creating table '{}' with {} columns", definition.name, definition.columns.len()),
            }
        );
        
        // Analyze constraints and indexes that come with table creation
        for column in &definition.columns {
            if column.primary_key || column.unique {
                result.add_performance_impact(
                    PerformanceImpact::IndexOperation {
                        operation_type: "implicit_index_creation".to_string(),
                        index_name: format!("{}_{}_idx", definition.name, column.name),
                        table: definition.name.clone(),
                        estimated_duration: Duration::from_millis(50),
                        impact_level: IndexImpactLevel::Low,
                        description: format!("Implicit index for {}{} column '{}'", 
                            if column.primary_key { "primary key " } else { "" },
                            if column.unique { "unique " } else { "" },
                            column.name
                        ),
                    }
                );
            }
        }
        
        Ok(())
    }
    
    /// Analyze table dropping performance
    fn analyze_drop_table_performance(&self, name: &str, result: &mut PerformanceImpactResult) -> Result<()> {
        result.add_performance_impact(
            PerformanceImpact::TableOperation {
                operation_type: "drop_table".to_string(),
                table: name.to_string(),
                estimated_duration: Duration::from_millis(100), // Usually fast
                memory_impact: MemoryImpact::Low,
                cpu_impact: CpuImpact::Low,
                io_impact: IoImpact::Medium, // Needs to clean up storage
                description: format!("Dropping table '{}' and all associated data", name),
            }
        );
        
        Ok(())
    }
    
    /// Analyze column addition performance
    fn analyze_add_column_performance(&self, table: &str, column: &ColumnSchema, result: &mut PerformanceImpactResult) -> Result<()> {
        let mut estimated_time = Duration::from_millis(100); // Base time
        let mut cpu_impact = CpuImpact::Low;
        let mut io_impact = IoImpact::Low;
        
        // Adding columns with defaults requires updating all existing rows
        if column.default_value.is_some() {
            estimated_time = Duration::from_secs(1); // Much longer for populated tables
            cpu_impact = CpuImpact::Medium;
            io_impact = IoImpact::High;
        }
        
        // Adding non-nullable columns is more complex
        if !column.nullable && column.default_value.is_none() {
            estimated_time = Duration::from_millis(50); // Fast if table is empty, fails if not
        }
        
        result.add_performance_impact(
            PerformanceImpact::ColumnOperation {
                operation_type: "add_column".to_string(),
                table: table.to_string(),
                column: column.name.clone(),
                estimated_duration: estimated_time,
                cpu_impact,
                io_impact,
                description: format!("Adding column '{}' to table '{}'", column.name, table),
                affects_existing_data: column.default_value.is_some(),
            }
        );
        
        Ok(())
    }
    
    /// Analyze column dropping performance
    fn analyze_drop_column_performance(&self, table: &str, column: &str, result: &mut PerformanceImpactResult) -> Result<()> {
        result.add_performance_impact(
            PerformanceImpact::ColumnOperation {
                operation_type: "drop_column".to_string(),
                table: table.to_string(),
                column: column.to_string(),
                estimated_duration: Duration::from_secs(2), // Can be slow for large tables
                cpu_impact: CpuImpact::Medium,
                io_impact: IoImpact::High, // Requires table rewrite in SQLite
                description: format!("Dropping column '{}' from table '{}'", column, table),
                affects_existing_data: true,
            }
        );
        
        Ok(())
    }
    
    /// Analyze column modification performance
    fn analyze_modify_column_performance(&self, table: &str, column: &str, changes: &super::super::ColumnChanges, result: &mut PerformanceImpactResult) -> Result<()> {
        let mut estimated_time = Duration::from_millis(500);
        let mut cpu_impact = CpuImpact::Medium;
        let mut io_impact = IoImpact::Medium;
        
        let mut change_descriptions = Vec::new();
        
        // Type changes require data conversion
        if let Some((old_type, new_type)) = &changes.type_change {
            estimated_time = Duration::from_secs(5); // Can be very slow
            cpu_impact = CpuImpact::High;
            io_impact = IoImpact::High;
            change_descriptions.push(format!("type change from {} to {}", old_type, new_type));
        }
        
        // Nullability changes
        if let Some((was_nullable, is_nullable)) = changes.null_change {
            if was_nullable && !is_nullable {
                estimated_time = estimated_time + Duration::from_secs(1);
                change_descriptions.push("making column non-nullable".to_string());
            }
        }
        
        // Default value changes
        if changes.default_change.is_some() {
            change_descriptions.push("changing default value".to_string());
        }
        
        result.add_performance_impact(
            PerformanceImpact::ColumnOperation {
                operation_type: "modify_column".to_string(),
                table: table.to_string(),
                column: column.to_string(),
                estimated_duration: estimated_time,
                cpu_impact,
                io_impact,
                description: format!("Modifying column '{}' in table '{}': {}", 
                    column, table, change_descriptions.join(", ")),
                affects_existing_data: true,
            }
        );
        
        Ok(())
    }
    
    /// Analyze index creation performance
    fn analyze_create_index_performance(&self, table: &str, index: &IndexSchema, result: &mut PerformanceImpactResult) -> Result<()> {
        let estimated_time = self.estimate_index_creation_time(index);
        let impact_level = if index.unique {
            IndexImpactLevel::High // Unique indexes are more expensive
        } else {
            IndexImpactLevel::Medium
        };
        
        result.add_performance_impact(
            PerformanceImpact::IndexOperation {
                operation_type: "create_index".to_string(),
                index_name: index.name.clone(),
                table: table.to_string(),
                estimated_duration: estimated_time,
                impact_level,
                description: format!("Creating {} index '{}' on table '{}'", 
                    if index.unique { "unique" } else { "standard" }, 
                    index.name, table),
            }
        );
        
        Ok(())
    }
    
    /// Analyze index dropping performance
    fn analyze_drop_index_performance(&self, name: &str, result: &mut PerformanceImpactResult) -> Result<()> {
        result.add_performance_impact(
            PerformanceImpact::IndexOperation {
                operation_type: "drop_index".to_string(),
                index_name: name.to_string(),
                table: "unknown".to_string(), // We don't know the table from just the index name
                estimated_duration: Duration::from_millis(50), // Usually fast
                impact_level: IndexImpactLevel::Low,
                description: format!("Dropping index '{}'", name),
            }
        );
        
        // Add warning about query performance impact
        result.add_performance_impact(
            PerformanceImpact::QueryPerformance {
                operation_type: "index_removal_impact".to_string(),
                affected_queries: format!("Queries that used index '{}'", name),
                performance_degradation: QueryPerformanceDegradation::Significant,
                estimated_slowdown: "10x-1000x slower".to_string(),
                mitigation: "Ensure alternative indexes exist or optimize queries".to_string(),
            }
        );
        
        Ok(())
    }
    
    /// Analyze foreign key addition performance
    fn analyze_add_foreign_key_performance(&self, constraint: &super::super::introspector::ForeignKeySchema, result: &mut PerformanceImpactResult) -> Result<()> {
        result.add_performance_impact(
            PerformanceImpact::ConstraintOperation {
                operation_type: "add_foreign_key".to_string(),
                constraint_name: constraint.name.clone(),
                table: "unknown".to_string(), // ForeignKeySchema doesn't store the source table
                estimated_duration: Duration::from_secs(2), // Validation can be slow
                validation_required: true,
                description: format!("Adding foreign key constraint '{}'", constraint.name),
            }
        );
        
        Ok(())
    }
    
    /// Analyze foreign key dropping performance
    fn analyze_drop_foreign_key_performance(&self, table: &str, constraint_name: &str, result: &mut PerformanceImpactResult) -> Result<()> {
        result.add_performance_impact(
            PerformanceImpact::ConstraintOperation {
                operation_type: "drop_foreign_key".to_string(),
                constraint_name: constraint_name.to_string(),
                table: table.to_string(),
                estimated_duration: Duration::from_millis(100), // Usually fast
                validation_required: false,
                description: format!("Dropping foreign key constraint '{}'", constraint_name),
            }
        );
        
        Ok(())
    }
    
    /// Analyze table renaming performance
    fn analyze_rename_table_performance(&self, old_name: &str, new_name: &str, result: &mut PerformanceImpactResult) -> Result<()> {
        result.add_performance_impact(
            PerformanceImpact::TableOperation {
                operation_type: "rename_table".to_string(),
                table: old_name.to_string(),
                estimated_duration: Duration::from_millis(50), // Usually fast
                memory_impact: MemoryImpact::Low,
                cpu_impact: CpuImpact::Low,
                io_impact: IoImpact::Low,
                description: format!("Renaming table from '{}' to '{}'", old_name, new_name),
            }
        );
        
        Ok(())
    }
    
    /// Analyze column renaming performance
    fn analyze_rename_column_performance(&self, table: &str, old_name: &str, new_name: &str, result: &mut PerformanceImpactResult) -> Result<()> {
        result.add_performance_impact(
            PerformanceImpact::ColumnOperation {
                operation_type: "rename_column".to_string(),
                table: table.to_string(),
                column: old_name.to_string(),
                estimated_duration: Duration::from_secs(1), // May require table rewrite
                cpu_impact: CpuImpact::Medium,
                io_impact: IoImpact::Medium,
                description: format!("Renaming column from '{}' to '{}' in table '{}'", old_name, new_name, table),
                affects_existing_data: false, // Data isn't changed, just column name
            }
        );
        
        Ok(())
    }
    
    /// Analyze cumulative impact of all operations
    fn analyze_cumulative_impact(&self, operations: &[MigrationOperation], result: &mut PerformanceImpactResult) -> Result<()> {
        let table_operations = operations.iter()
            .filter(|op| matches!(op, 
                MigrationOperation::CreateTable { .. } |
                MigrationOperation::DropTable { .. } |
                MigrationOperation::RenameTable { .. }
            ))
            .count();
            
        let column_operations = operations.iter()
            .filter(|op| matches!(op,
                MigrationOperation::AddColumn { .. } |
                MigrationOperation::DropColumn { .. } |
                MigrationOperation::ModifyColumn { .. } |
                MigrationOperation::RenameColumn { .. }
            ))
            .count();
            
        let index_operations = operations.iter()
            .filter(|op| matches!(op,
                MigrationOperation::CreateIndex { .. } |
                MigrationOperation::DropIndex { .. }
            ))
            .count();
        
        result.cumulative_impact = Some(CumulativePerformanceImpact {
            total_operations: operations.len(),
            table_operations,
            column_operations,
            index_operations,
            estimated_total_cpu_time: Duration::from_secs(operations.len() as u64), // Rough estimate
            estimated_peak_memory_usage: (operations.len() * 10) as u64, // MB estimate
            parallel_execution_potential: self.analyze_parallelization_potential(operations),
        });
        
        Ok(())
    }
    
    /// Estimate total migration duration
    fn estimate_migration_duration(&self, operations: &[MigrationOperation], result: &mut PerformanceImpactResult) -> Result<()> {
        let mut total_time = Duration::from_secs(0);
        
        for operation in operations {
            total_time += self.estimate_operation_duration(operation);
        }
        
        // Add overhead for transaction management and validation
        let overhead = Duration::from_millis(operations.len() as u64 * 50);
        total_time += overhead;
        
        result.estimated_total_duration = total_time;
        
        Ok(())
    }
    
    /// Estimate duration for a single operation
    fn estimate_operation_duration(&self, operation: &MigrationOperation) -> Duration {
        match operation {
            MigrationOperation::CreateTable { definition } => {
                self.estimate_table_creation_time(definition)
            }
            MigrationOperation::DropTable { .. } => Duration::from_millis(100),
            MigrationOperation::AddColumn { column, .. } => {
                if column.default_value.is_some() {
                    Duration::from_secs(1) // Updating existing rows
                } else {
                    Duration::from_millis(100)
                }
            }
            MigrationOperation::DropColumn { .. } => Duration::from_secs(2),
            MigrationOperation::ModifyColumn { changes, .. } => {
                if changes.type_change.is_some() {
                    Duration::from_secs(5) // Type conversion is expensive
                } else {
                    Duration::from_millis(500)
                }
            }
            MigrationOperation::CreateIndex { index, .. } => {
                self.estimate_index_creation_time(index)
            }
            MigrationOperation::DropIndex { .. } => Duration::from_millis(50),
            MigrationOperation::AddForeignKey { .. } => Duration::from_secs(2),
            MigrationOperation::DropForeignKey { .. } => Duration::from_millis(100),
            MigrationOperation::RenameTable { .. } => Duration::from_millis(50),
            MigrationOperation::RenameColumn { .. } => Duration::from_secs(1),
        }
    }
    
    /// Estimate table creation time based on complexity
    fn estimate_table_creation_time(&self, definition: &TableSchema) -> Duration {
        let base_time = Duration::from_millis(100);
        let column_time = Duration::from_millis(definition.columns.len() as u64 * 10);
        let constraint_time = Duration::from_millis(
            definition.columns.iter()
                .filter(|c| c.primary_key || c.unique || c.constraints.iter().any(|constraint| matches!(constraint, super::super::introspector::ColumnConstraint::References { .. })))
                .count() as u64 * 50
        );
        
        base_time + column_time + constraint_time
    }
    
    /// Estimate table memory usage
    fn estimate_table_memory_usage(&self, definition: &TableSchema) -> MemoryImpact {
        if definition.columns.len() > 20 {
            MemoryImpact::High
        } else if definition.columns.len() > 10 {
            MemoryImpact::Medium
        } else {
            MemoryImpact::Low
        }
    }
    
    /// Estimate index creation time
    fn estimate_index_creation_time(&self, index: &IndexSchema) -> Duration {
        let base_time = Duration::from_millis(200);
        let unique_penalty = if index.unique {
            Duration::from_millis(300) // Unique indexes take longer
        } else {
            Duration::from_millis(0)
        };
        
        base_time + unique_penalty
    }
    
    /// Analyze potential for parallel execution
    fn analyze_parallelization_potential(&self, operations: &[MigrationOperation]) -> ParallelizationPotential {
        let independent_operations = operations.iter()
            .filter(|op| matches!(op,
                MigrationOperation::CreateIndex { .. } |
                MigrationOperation::AddColumn { .. }
            ))
            .count();
            
        if independent_operations as f64 / operations.len() as f64 > 0.7 {
            ParallelizationPotential::High
        } else if independent_operations as f64 / operations.len() as f64 > 0.3 {
            ParallelizationPotential::Medium
        } else {
            ParallelizationPotential::Low
        }
    }
}

/// Performance impact validation result
#[derive(Debug, Clone)]
pub struct PerformanceImpactResult {
    /// Individual performance impacts identified
    pub performance_impacts: Vec<PerformanceImpact>,
    
    /// Overall performance risk level
    pub risk_level: PerformanceRiskLevel,
    
    /// Estimated total migration duration
    pub estimated_total_duration: Duration,
    
    /// Cumulative impact analysis
    pub cumulative_impact: Option<CumulativePerformanceImpact>,
    
    /// Performance optimization recommendations
    pub optimization_recommendations: Vec<String>,
}

impl PerformanceImpactResult {
    fn new() -> Self {
        Self {
            performance_impacts: Vec::new(),
            risk_level: PerformanceRiskLevel::Low,
            estimated_total_duration: Duration::from_secs(0),
            cumulative_impact: None,
            optimization_recommendations: Vec::new(),
        }
    }
    
    fn add_performance_impact(&mut self, impact: PerformanceImpact) {
        // Update risk level based on impact
        let impact_risk = impact.risk_level();
        if impact_risk > self.risk_level {
            self.risk_level = impact_risk;
        }
        
        // Add optimization recommendations based on impact type
        match &impact {
            PerformanceImpact::IndexOperation { operation_type, .. } => {
                if operation_type == "drop_index" {
                    self.optimization_recommendations.push(
                        "Consider creating alternative indexes before dropping existing ones".to_string()
                    );
                }
            }
            PerformanceImpact::ColumnOperation { affects_existing_data: true, .. } => {
                self.optimization_recommendations.push(
                    "Consider running during low-traffic periods due to data modification".to_string()
                );
            }
            _ => {}
        }
        
        self.performance_impacts.push(impact);
    }
    
    /// Check if migration has high performance impact
    pub fn has_high_impact(&self) -> bool {
        matches!(self.risk_level, PerformanceRiskLevel::High | PerformanceRiskLevel::Critical)
    }
    
    /// Get operations with highest performance impact
    pub fn high_impact_operations(&self) -> Vec<&PerformanceImpact> {
        self.performance_impacts.iter()
            .filter(|impact| impact.risk_level() >= PerformanceRiskLevel::High)
            .collect()
    }
}

/// Types of performance impacts
#[derive(Debug, Clone)]
pub enum PerformanceImpact {
    /// Table-level operations
    TableOperation {
        operation_type: String,
        table: String,
        estimated_duration: Duration,
        memory_impact: MemoryImpact,
        cpu_impact: CpuImpact,
        io_impact: IoImpact,
        description: String,
    },
    
    /// Column-level operations
    ColumnOperation {
        operation_type: String,
        table: String,
        column: String,
        estimated_duration: Duration,
        cpu_impact: CpuImpact,
        io_impact: IoImpact,
        description: String,
        affects_existing_data: bool,
    },
    
    /// Index operations
    IndexOperation {
        operation_type: String,
        index_name: String,
        table: String,
        estimated_duration: Duration,
        impact_level: IndexImpactLevel,
        description: String,
    },
    
    /// Constraint operations
    ConstraintOperation {
        operation_type: String,
        constraint_name: String,
        table: String,
        estimated_duration: Duration,
        validation_required: bool,
        description: String,
    },
    
    /// Query performance impacts
    QueryPerformance {
        operation_type: String,
        affected_queries: String,
        performance_degradation: QueryPerformanceDegradation,
        estimated_slowdown: String,
        mitigation: String,
    },
}

impl PerformanceImpact {
    fn risk_level(&self) -> PerformanceRiskLevel {
        match self {
            PerformanceImpact::TableOperation { estimated_duration, .. } => {
                if *estimated_duration > Duration::from_secs(10) {
                    PerformanceRiskLevel::High
                } else if *estimated_duration > Duration::from_secs(1) {
                    PerformanceRiskLevel::Medium
                } else {
                    PerformanceRiskLevel::Low
                }
            }
            PerformanceImpact::ColumnOperation { estimated_duration, affects_existing_data, .. } => {
                if *affects_existing_data && *estimated_duration > Duration::from_secs(5) {
                    PerformanceRiskLevel::High
                } else if *estimated_duration > Duration::from_secs(2) {
                    PerformanceRiskLevel::Medium
                } else {
                    PerformanceRiskLevel::Low
                }
            }
            PerformanceImpact::IndexOperation { impact_level, .. } => {
                match impact_level {
                    IndexImpactLevel::Low => PerformanceRiskLevel::Low,
                    IndexImpactLevel::Medium => PerformanceRiskLevel::Medium,
                    IndexImpactLevel::High => PerformanceRiskLevel::High,
                }
            }
            PerformanceImpact::QueryPerformance { performance_degradation, .. } => {
                match performance_degradation {
                    QueryPerformanceDegradation::None => PerformanceRiskLevel::Low,
                    QueryPerformanceDegradation::Minor => PerformanceRiskLevel::Low,
                    QueryPerformanceDegradation::Moderate => PerformanceRiskLevel::Medium,
                    QueryPerformanceDegradation::Significant => PerformanceRiskLevel::High,
                    QueryPerformanceDegradation::Severe => PerformanceRiskLevel::Critical,
                }
            }
            _ => PerformanceRiskLevel::Low,
        }
    }
}

/// Cumulative performance impact analysis
#[derive(Debug, Clone)]
pub struct CumulativePerformanceImpact {
    pub total_operations: usize,
    pub table_operations: usize,
    pub column_operations: usize,
    pub index_operations: usize,
    pub estimated_total_cpu_time: Duration,
    pub estimated_peak_memory_usage: u64, // MB
    pub parallel_execution_potential: ParallelizationPotential,
}

/// Performance risk levels
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum PerformanceRiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Memory impact levels
#[derive(Debug, Clone)]
pub enum MemoryImpact {
    Low,
    Medium,
    High,
}

/// CPU impact levels
#[derive(Debug, Clone)]
pub enum CpuImpact {
    Low,
    Medium,
    High,
}

/// I/O impact levels
#[derive(Debug, Clone)]
pub enum IoImpact {
    Low,
    Medium,
    High,
}

/// Index operation impact levels
#[derive(Debug, Clone)]
pub enum IndexImpactLevel {
    Low,
    Medium,
    High,
}

/// Query performance degradation levels
#[derive(Debug, Clone)]
pub enum QueryPerformanceDegradation {
    None,
    Minor,
    Moderate,
    Significant,
    Severe,
}

/// Parallelization potential assessment
#[derive(Debug, Clone)]
pub enum ParallelizationPotential {
    Low,     // Most operations must be sequential
    Medium,  // Some operations can be parallelized
    High,    // Many operations can run in parallel
}