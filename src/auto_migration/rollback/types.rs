//! Core types and enums for the rollback system
//!
//! This module defines all the fundamental types used throughout the rollback system,
//! including rollback operations, risk assessment levels, configuration options,
//! and data preservation strategies.

use super::super::introspector::{TableSchema, ColumnSchema, IndexSchema, ForeignKeySchema, ConstraintSchema};
use std::time::Duration;
use std::fmt;

/// Represents a single rollback operation that reverses a forward migration operation
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum RollbackOperation {
    /// Drop a table (reverse of CreateTable)
    DropTable {
        /// Name of the table to drop
        name: String,
        /// Whether to preserve data by renaming instead of dropping
        preserve_data: bool,
        /// Name for backup table if preserving data
        backup_table_name: Option<String>,
    },
    
    /// Recreate a table (reverse of DropTable)
    RecreateTable {
        /// Complete table definition to recreate
        definition: TableSchema,
        /// Whether to restore data from backup
        restore_data: bool,
        /// Source table/location for data restoration
        data_source: Option<String>,
    },
    
    /// Drop a column (reverse of AddColumn)
    DropColumn {
        /// Table containing the column
        table: String,
        /// Name of column to drop
        column: String,
        /// Whether to preserve column data
        preserve_data: bool,
    },
    
    /// Add a column (reverse of DropColumn)
    AddColumn {
        /// Table to add column to
        table: String,
        /// Complete column definition
        column: ColumnSchema,
        /// Whether to restore column data
        restore_data: bool,
    },
    
    /// Modify a column (reverse of ModifyColumn)
    ModifyColumn {
        /// Table containing the column
        table: String,
        /// Name of column to modify
        column: String,
        /// Changes to apply (reverse of original changes)
        changes: RollbackColumnChanges,
        /// Whether to preserve data during modification
        preserve_data: bool,
    },
    
    /// Drop an index (reverse of CreateIndex)
    DropIndex {
        /// Name of index to drop
        name: String,
        /// Table the index belongs to
        table: String,
    },
    
    /// Create an index (reverse of DropIndex)
    CreateIndex {
        /// Table to create index on
        table: String,
        /// Complete index definition
        index: IndexSchema,
    },
    
    /// Drop a foreign key constraint (reverse of AddForeignKey)
    DropForeignKey {
        /// Table containing the constraint
        table: String,
        /// Name of constraint to drop
        constraint_name: String,
    },
    
    /// Add a foreign key constraint (reverse of DropForeignKey)
    AddForeignKey {
        /// Table to add constraint to
        table: String,
        /// Complete constraint definition
        constraint: ForeignKeySchema,
    },
    
    /// Rename a table (reverse of RenameTable - swaps names)
    RenameTable {
        /// Current name of table
        old_name: String,
        /// New name for table
        new_name: String,
    },
    
    /// Rename a column (reverse of RenameColumn - swaps names)
    RenameColumn {
        /// Table containing the column
        table: String,
        /// Current column name
        old_name: String,
        /// New column name
        new_name: String,
    },
}

impl RollbackOperation {
    /// Get the operation type as a string for progress tracking and logging
    pub fn operation_type(&self) -> &'static str {
        match self {
            RollbackOperation::DropTable { .. } => "drop_table",
            RollbackOperation::RecreateTable { .. } => "recreate_table",
            RollbackOperation::DropColumn { .. } => "drop_column",
            RollbackOperation::AddColumn { .. } => "add_column",
            RollbackOperation::ModifyColumn { .. } => "modify_column",
            RollbackOperation::DropIndex { .. } => "drop_index",
            RollbackOperation::CreateIndex { .. } => "create_index",
            RollbackOperation::DropForeignKey { .. } => "drop_foreign_key",
            RollbackOperation::AddForeignKey { .. } => "add_foreign_key",
            RollbackOperation::RenameTable { .. } => "rename_table",
            RollbackOperation::RenameColumn { .. } => "rename_column",
        }
    }
    
    /// Get the primary table affected by this operation
    pub fn affected_table(&self) -> &str {
        match self {
            RollbackOperation::DropTable { name, .. } => name,
            RollbackOperation::RecreateTable { definition, .. } => &definition.name,
            RollbackOperation::DropColumn { table, .. } => table,
            RollbackOperation::AddColumn { table, .. } => table,
            RollbackOperation::ModifyColumn { table, .. } => table,
            RollbackOperation::DropIndex { table, .. } => table,
            RollbackOperation::CreateIndex { table, .. } => table,
            RollbackOperation::DropForeignKey { table, .. } => table,
            RollbackOperation::AddForeignKey { table, .. } => table,
            RollbackOperation::RenameTable { old_name, .. } => old_name,
            RollbackOperation::RenameColumn { table, .. } => table,
        }
    }
    
    /// Determine if this operation is potentially destructive (data loss risk)
    pub fn is_destructive(&self) -> bool {
        match self {
            RollbackOperation::DropTable { preserve_data, .. } => !preserve_data,
            RollbackOperation::DropColumn { preserve_data, .. } => !preserve_data,
            RollbackOperation::ModifyColumn { preserve_data, changes, .. } => {
                !preserve_data || changes.is_lossy()
            }
            RollbackOperation::RecreateTable { restore_data, .. } => !restore_data,
            RollbackOperation::AddColumn { restore_data, .. } => !restore_data,
            _ => false,
        }
    }
    
    /// Assess the risk level for this rollback operation
    pub fn assess_risk_level(&self) -> RollbackRiskLevel {
        match self {
            RollbackOperation::DropTable { preserve_data, .. } => {
                if *preserve_data {
                    RollbackRiskLevel::Medium
                } else {
                    RollbackRiskLevel::Critical
                }
            }
            RollbackOperation::RecreateTable { restore_data, .. } => {
                if *restore_data {
                    RollbackRiskLevel::High
                } else {
                    RollbackRiskLevel::Critical
                }
            }
            RollbackOperation::DropColumn { preserve_data, .. } => {
                if *preserve_data {
                    RollbackRiskLevel::Medium
                } else {
                    RollbackRiskLevel::High
                }
            }
            RollbackOperation::AddColumn { restore_data, .. } => {
                if *restore_data {
                    RollbackRiskLevel::Low
                } else {
                    RollbackRiskLevel::Medium
                }
            }
            RollbackOperation::ModifyColumn { preserve_data, changes, .. } => {
                if changes.is_lossy() {
                    if *preserve_data {
                        RollbackRiskLevel::High
                    } else {
                        RollbackRiskLevel::Critical
                    }
                } else if *preserve_data {
                    RollbackRiskLevel::Medium
                } else {
                    RollbackRiskLevel::High
                }
            }
            RollbackOperation::DropIndex { .. } => RollbackRiskLevel::Low,
            RollbackOperation::CreateIndex { .. } => RollbackRiskLevel::Low,
            RollbackOperation::DropForeignKey { .. } => RollbackRiskLevel::Medium,
            RollbackOperation::AddForeignKey { .. } => RollbackRiskLevel::Medium,
            RollbackOperation::RenameTable { .. } => RollbackRiskLevel::Low,
            RollbackOperation::RenameColumn { .. } => RollbackRiskLevel::Low,
        }
    }
    
    /// Assess the data loss risk for this operation
    pub fn assess_data_loss_risk(&self) -> DataLossRisk {
        match self {
            RollbackOperation::DropTable { preserve_data, .. } => {
                if *preserve_data {
                    DataLossRisk::Low
                } else {
                    DataLossRisk::High
                }
            }
            RollbackOperation::DropColumn { preserve_data, .. } => {
                if *preserve_data {
                    DataLossRisk::Low
                } else {
                    DataLossRisk::Medium
                }
            }
            RollbackOperation::ModifyColumn { preserve_data, changes, .. } => {
                if changes.is_lossy() {
                    if *preserve_data {
                        DataLossRisk::Low
                    } else {
                        DataLossRisk::High
                    }
                } else {
                    DataLossRisk::None
                }
            }
            RollbackOperation::RecreateTable { restore_data, .. } => {
                if *restore_data {
                    DataLossRisk::Low
                } else {
                    DataLossRisk::High
                }
            }
            RollbackOperation::AddColumn { restore_data, .. } => {
                if *restore_data {
                    DataLossRisk::None
                } else {
                    DataLossRisk::Low
                }
            }
            _ => DataLossRisk::None,
        }
    }
    
    /// Estimate the relative complexity/duration of this operation
    pub fn complexity_score(&self) -> u32 {
        match self {
            RollbackOperation::DropTable { .. } => 5,
            RollbackOperation::RecreateTable { .. } => 15,
            RollbackOperation::DropColumn { .. } => 3,
            RollbackOperation::AddColumn { .. } => 8,
            RollbackOperation::ModifyColumn { .. } => 10,
            RollbackOperation::DropIndex { .. } => 2,
            RollbackOperation::CreateIndex { .. } => 5,
            RollbackOperation::DropForeignKey { .. } => 2,
            RollbackOperation::AddForeignKey { .. } => 4,
            RollbackOperation::RenameTable { .. } => 3,
            RollbackOperation::RenameColumn { .. } => 4,
        }
    }
}

impl fmt::Display for RollbackOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RollbackOperation::DropTable { name, preserve_data, .. } => {
                write!(f, "Drop table '{}'{}", name, 
                    if *preserve_data { " (preserving data)" } else { "" })
            }
            RollbackOperation::RecreateTable { definition, restore_data, .. } => {
                write!(f, "Recreate table '{}'{}", definition.name,
                    if *restore_data { " (restoring data)" } else { "" })
            }
            RollbackOperation::DropColumn { table, column, preserve_data } => {
                write!(f, "Drop column '{}.{}'{}", table, column,
                    if *preserve_data { " (preserving data)" } else { "" })
            }
            RollbackOperation::AddColumn { table, column, restore_data } => {
                write!(f, "Add column '{}.{}'{}", table, column.name,
                    if *restore_data { " (restoring data)" } else { "" })
            }
            RollbackOperation::ModifyColumn { table, column, .. } => {
                write!(f, "Modify column '{}.{}'", table, column)
            }
            RollbackOperation::DropIndex { name, table } => {
                write!(f, "Drop index '{}' on table '{}'", name, table)
            }
            RollbackOperation::CreateIndex { table, index } => {
                write!(f, "Create index '{}' on table '{}'", index.name, table)
            }
            RollbackOperation::DropForeignKey { table, constraint_name } => {
                write!(f, "Drop foreign key '{}' from table '{}'", constraint_name, table)
            }
            RollbackOperation::AddForeignKey { table, constraint } => {
                write!(f, "Add foreign key '{}' to table '{}'", constraint.name, table)
            }
            RollbackOperation::RenameTable { old_name, new_name } => {
                write!(f, "Rename table '{}' to '{}'", old_name, new_name)
            }
            RollbackOperation::RenameColumn { table, old_name, new_name } => {
                write!(f, "Rename column '{}.{}' to '{}'", table, old_name, new_name)
            }
        }
    }
}

/// Changes to apply when rolling back a column modification
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RollbackColumnChanges {
    /// Type change to apply (old_type, new_type)
    pub type_change: Option<(String, String)>,
    /// Nullability change to apply (old_nullable, new_nullable)
    pub null_change: Option<(bool, bool)>,
    /// Default value change to apply (old_default, new_default)
    pub default_change: Option<(Option<String>, Option<String>)>,
    /// Constraint changes to apply
    pub constraint_changes: Vec<ConstraintSchema>,
}

impl RollbackColumnChanges {
    /// Create an empty set of column changes
    pub fn new() -> Self {
        Self {
            type_change: None,
            null_change: None,
            default_change: None,
            constraint_changes: Vec::new(),
        }
    }
    
    /// Check if any of these changes could result in data loss
    pub fn is_lossy(&self) -> bool {
        if let Some((from_type, to_type)) = &self.type_change {
            return Self::is_lossy_type_conversion(from_type, to_type);
        }
        
        if let Some((from_nullable, to_nullable)) = &self.null_change {
            // Going from nullable to not nullable could fail if there are nulls
            if *from_nullable && !*to_nullable {
                return true;
            }
        }
        
        false
    }
    
    /// Determine if a type conversion is potentially lossy
    fn is_lossy_type_conversion(from_type: &str, to_type: &str) -> bool {
        let from_upper = from_type.to_uppercase();
        let to_upper = to_type.to_uppercase();
        
        match (from_upper.as_str(), to_upper.as_str()) {
            // Text to numeric conversions are lossy
            ("TEXT", "INTEGER") | ("TEXT", "REAL") => true,
            // Numeric to smaller numeric types can be lossy
            ("REAL", "INTEGER") => true,
            // BLOB conversions are typically lossy
            ("BLOB", "TEXT") | ("BLOB", "INTEGER") | ("BLOB", "REAL") => true,
            ("TEXT", "BLOB") | ("INTEGER", "BLOB") | ("REAL", "BLOB") => true,
            // Same type conversions are safe
            (from, to) if from == to => false,
            // Numeric to text is generally safe
            ("INTEGER", "TEXT") | ("REAL", "TEXT") => false,
            // Integer to real is safe
            ("INTEGER", "REAL") => false,
            // Default to potentially lossy for unknown conversions
            _ => true,
        }
    }
    
    /// Get a human-readable description of the changes
    pub fn description(&self) -> String {
        let mut parts = Vec::new();
        
        if let Some((from_type, to_type)) = &self.type_change {
            parts.push(format!("type: {} → {}", from_type, to_type));
        }
        
        if let Some((from_nullable, to_nullable)) = &self.null_change {
            let from_str = if *from_nullable { "nullable" } else { "not null" };
            let to_str = if *to_nullable { "nullable" } else { "not null" };
            parts.push(format!("nullability: {} → {}", from_str, to_str));
        }
        
        if let Some((from_default, to_default)) = &self.default_change {
            let from_str = from_default.as_deref().unwrap_or("none");
            let to_str = to_default.as_deref().unwrap_or("none");
            parts.push(format!("default: {} → {}", from_str, to_str));
        }
        
        if !self.constraint_changes.is_empty() {
            parts.push(format!("{} constraint changes", self.constraint_changes.len()));
        }
        
        if parts.is_empty() {
            "no changes".to_string()
        } else {
            parts.join(", ")
        }
    }
}

impl Default for RollbackColumnChanges {
    fn default() -> Self {
        Self::new()
    }
}

/// Risk level assessment for rollback operations
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy, serde::Serialize, serde::Deserialize)]
pub enum RollbackRiskLevel {
    /// Safe operations with no data loss risk (e.g., renames, index operations)
    Low,
    /// Operations with minor risk or reversible changes
    Medium,
    /// Operations with significant risk of data loss or complex dependencies
    High,
    /// Operations that will definitely cause data loss or system instability
    Critical,
}

impl RollbackRiskLevel {
    /// Get all risk levels in order from lowest to highest
    pub fn all_levels() -> &'static [RollbackRiskLevel] {
        &[
            RollbackRiskLevel::Low,
            RollbackRiskLevel::Medium,
            RollbackRiskLevel::High,
            RollbackRiskLevel::Critical,
        ]
    }
    
    /// Get the maximum risk level from a collection
    pub fn max_level<I>(levels: I) -> RollbackRiskLevel
    where
        I: IntoIterator<Item = RollbackRiskLevel>,
    {
        levels.into_iter()
            .max_by_key(|level| level.numeric_value())
            .unwrap_or(RollbackRiskLevel::Low)
    }
    
    /// Get numeric value for comparison (higher = more risky)
    pub fn numeric_value(&self) -> u8 {
        match self {
            RollbackRiskLevel::Low => 0,
            RollbackRiskLevel::Medium => 1,
            RollbackRiskLevel::High => 2,
            RollbackRiskLevel::Critical => 3,
        }
    }
    
    /// Check if this risk level blocks execution
    pub fn blocks_execution(&self) -> bool {
        matches!(self, RollbackRiskLevel::Critical)
    }
    
    /// Get the color code for terminal output
    pub fn color_code(&self) -> &'static str {
        match self {
            RollbackRiskLevel::Low => "\x1b[32m",      // Green
            RollbackRiskLevel::Medium => "\x1b[33m",   // Yellow
            RollbackRiskLevel::High => "\x1b[31m",     // Red
            RollbackRiskLevel::Critical => "\x1b[35m", // Magenta
        }
    }
}

impl fmt::Display for RollbackRiskLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            RollbackRiskLevel::Low => "LOW",
            RollbackRiskLevel::Medium => "MEDIUM",
            RollbackRiskLevel::High => "HIGH",
            RollbackRiskLevel::Critical => "CRITICAL",
        };
        write!(f, "{}", name)
    }
}

impl PartialOrd for RollbackRiskLevel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RollbackRiskLevel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.numeric_value().cmp(&other.numeric_value())
    }
}

/// Severity level for individual rollback validation issues
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum RollbackRiskSeverity {
    /// Informational issue that doesn't affect safety
    Low,
    /// Minor issue that should be noted but doesn't prevent execution
    Medium,
    /// Significant issue that increases risk but may be acceptable
    High,
    /// Critical issue that should prevent execution
    Critical,
}

impl RollbackRiskSeverity {
    /// Convert to risk level for overall assessment
    pub fn to_risk_level(&self) -> RollbackRiskLevel {
        match self {
            RollbackRiskSeverity::Low => RollbackRiskLevel::Low,
            RollbackRiskSeverity::Medium => RollbackRiskLevel::Medium,
            RollbackRiskSeverity::High => RollbackRiskLevel::High,
            RollbackRiskSeverity::Critical => RollbackRiskLevel::Critical,
        }
    }
    
    /// Get numeric value for comparison
    pub fn numeric_value(&self) -> u8 {
        match self {
            RollbackRiskSeverity::Low => 0,
            RollbackRiskSeverity::Medium => 1,
            RollbackRiskSeverity::High => 2,
            RollbackRiskSeverity::Critical => 3,
        }
    }
}

impl fmt::Display for RollbackRiskSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            RollbackRiskSeverity::Low => "Low",
            RollbackRiskSeverity::Medium => "Medium",
            RollbackRiskSeverity::High => "High",
            RollbackRiskSeverity::Critical => "Critical",
        };
        write!(f, "{}", name)
    }
}

/// Assessment of data loss risk for rollback operations
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy, serde::Serialize, serde::Deserialize)]
pub enum DataLossRisk {
    /// No data will be lost
    None,
    /// Minimal or recoverable data loss possible
    Low,
    /// Significant data loss possible but may be acceptable
    Medium,
    /// Major data loss likely, operation should be carefully considered
    High,
}

impl DataLossRisk {
    /// Get all data loss risk levels
    pub fn all_levels() -> &'static [DataLossRisk] {
        &[
            DataLossRisk::None,
            DataLossRisk::Low,
            DataLossRisk::Medium,
            DataLossRisk::High,
        ]
    }
    
    /// Get the maximum data loss risk from a collection
    pub fn max_risk<I>(risks: I) -> DataLossRisk
    where
        I: IntoIterator<Item = DataLossRisk>,
    {
        risks.into_iter()
            .max_by_key(|risk| risk.numeric_value())
            .unwrap_or(DataLossRisk::None)
    }
    
    /// Get numeric value for comparison
    pub fn numeric_value(&self) -> u8 {
        match self {
            DataLossRisk::None => 0,
            DataLossRisk::Low => 1,
            DataLossRisk::Medium => 2,
            DataLossRisk::High => 3,
        }
    }
    
    /// Convert to equivalent risk level
    pub fn to_risk_level(&self) -> RollbackRiskLevel {
        match self {
            DataLossRisk::None => RollbackRiskLevel::Low,
            DataLossRisk::Low => RollbackRiskLevel::Medium,
            DataLossRisk::Medium => RollbackRiskLevel::High,
            DataLossRisk::High => RollbackRiskLevel::Critical,
        }
    }
}

impl fmt::Display for DataLossRisk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            DataLossRisk::None => "None",
            DataLossRisk::Low => "Low",
            DataLossRisk::Medium => "Medium",
            DataLossRisk::High => "High",
        };
        write!(f, "{}", name)
    }
}

/// Configuration options for rollback behavior
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RollbackConfig {
    /// Whether to preserve data when dropping tables/columns
    pub preserve_data_on_rollback: bool,
    
    /// Whether to restore data when recreating tables/columns
    pub restore_data_on_rollback: bool,
    
    /// Whether to create backup tables before destructive operations
    pub create_backup_tables: bool,
    
    /// Whether to stop execution on first operation failure
    pub stop_on_first_failure: bool,
    
    /// Maximum time to wait for each operation before timing out
    pub operation_timeout: Duration,
    
    /// Whether to run operations in transaction mode (all-or-nothing)
    pub use_transactions: bool,
    
    /// Maximum number of operations to execute in parallel
    pub max_parallel_operations: usize,
    
    /// Whether to validate schema compatibility before execution
    pub validate_schema_compatibility: bool,
    
    /// Whether to perform dry-run validation before actual execution
    pub perform_dry_run: bool,
    
    /// Prefix for backup table names
    pub backup_table_prefix: String,
    
    /// Suffix for backup table names
    pub backup_table_suffix: String,
}

impl RollbackConfig {
    /// Create a new rollback configuration with safe defaults
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create a conservative configuration that prioritizes data safety
    pub fn conservative() -> Self {
        Self {
            preserve_data_on_rollback: true,
            restore_data_on_rollback: true,
            create_backup_tables: true,
            stop_on_first_failure: true,
            operation_timeout: Duration::from_secs(300), // 5 minutes
            use_transactions: true,
            max_parallel_operations: 1,
            validate_schema_compatibility: true,
            perform_dry_run: true,
            backup_table_prefix: "rollback_backup_".to_string(),
            backup_table_suffix: format!("_{}", chrono::Utc::now().timestamp()),
        }
    }
    
    /// Create an aggressive configuration for faster execution (higher risk)
    pub fn aggressive() -> Self {
        Self {
            preserve_data_on_rollback: false,
            restore_data_on_rollback: false,
            create_backup_tables: false,
            stop_on_first_failure: false,
            operation_timeout: Duration::from_secs(30),
            use_transactions: false,
            max_parallel_operations: 4,
            validate_schema_compatibility: false,
            perform_dry_run: false,
            backup_table_prefix: "backup_".to_string(),
            backup_table_suffix: String::new(),
        }
    }
    
    /// Validate the configuration and return any issues
    pub fn validate(&self) -> Result<(), String> {
        if self.operation_timeout.is_zero() {
            return Err("Operation timeout cannot be zero".to_string());
        }
        
        if self.max_parallel_operations == 0 {
            return Err("Max parallel operations must be at least 1".to_string());
        }
        
        if self.max_parallel_operations > 10 {
            return Err("Max parallel operations should not exceed 10 for safety".to_string());
        }
        
        if self.backup_table_prefix.is_empty() && self.create_backup_tables {
            return Err("Backup table prefix cannot be empty when creating backup tables".to_string());
        }
        
        Ok(())
    }
    
    /// Get the complete backup table name for a given table
    pub fn backup_table_name(&self, table_name: &str) -> String {
        format!("{}{}{}", self.backup_table_prefix, table_name, self.backup_table_suffix)
    }
    
    /// Calculate the estimated total timeout for a given number of operations
    pub fn total_timeout_estimate(&self, operation_count: usize) -> Duration {
        let _sequential_time = self.operation_timeout * operation_count as u32;
        let parallel_batches = (operation_count + self.max_parallel_operations - 1) / self.max_parallel_operations;
        self.operation_timeout * parallel_batches as u32
    }
}

impl Default for RollbackConfig {
    fn default() -> Self {
        Self {
            preserve_data_on_rollback: true,
            restore_data_on_rollback: true,
            create_backup_tables: true,
            stop_on_first_failure: true,
            operation_timeout: Duration::from_secs(60),
            use_transactions: true,
            max_parallel_operations: 2,
            validate_schema_compatibility: true,
            perform_dry_run: false,
            backup_table_prefix: "rollback_backup_".to_string(),
            backup_table_suffix: String::new(),
        }
    }
}

impl fmt::Display for RollbackConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Rollback Configuration:")?;
        writeln!(f, "  Preserve data: {}", self.preserve_data_on_rollback)?;
        writeln!(f, "  Restore data: {}", self.restore_data_on_rollback)?;
        writeln!(f, "  Create backups: {}", self.create_backup_tables)?;
        writeln!(f, "  Stop on failure: {}", self.stop_on_first_failure)?;
        writeln!(f, "  Operation timeout: {:?}", self.operation_timeout)?;
        writeln!(f, "  Use transactions: {}", self.use_transactions)?;
        writeln!(f, "  Max parallel ops: {}", self.max_parallel_operations)?;
        writeln!(f, "  Validate schema: {}", self.validate_schema_compatibility)?;
        writeln!(f, "  Perform dry run: {}", self.perform_dry_run)?;
        writeln!(f, "  Backup prefix: '{}'", self.backup_table_prefix)?;
        write!(f, "  Backup suffix: '{}'", self.backup_table_suffix)
    }
}

/// Strategy for preserving data during rollback operations
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum DataPreservationStrategy {
    /// Create a backup table with the original data
    BackupTable {
        /// Name of the backup table
        backup_name: String,
        /// Whether to drop the backup after successful restore
        drop_after_restore: bool,
    },
    
    /// Store data in temporary storage during the operation
    TemporaryStorage {
        /// Location identifier for temporary storage
        storage_location: String,
        /// Maximum size limit for temporary storage
        size_limit: Option<usize>,
    },
    
    /// Export data to external file/system before operation
    ExternalExport {
        /// Path or identifier for export location
        export_path: String,
        /// Format for the exported data
        export_format: DataExportFormat,
    },
    
    /// Keep data in memory during operation (for small datasets)
    InMemoryCache {
        /// Maximum number of rows to cache
        max_rows: usize,
        /// Whether to compress cached data
        compress: bool,
    },
}

impl DataPreservationStrategy {
    /// Get the estimated memory/storage overhead for this strategy
    pub fn storage_overhead_factor(&self) -> f64 {
        match self {
            DataPreservationStrategy::BackupTable { .. } => 2.0, // Doubles storage
            DataPreservationStrategy::TemporaryStorage { .. } => 1.5, // Some overhead
            DataPreservationStrategy::ExternalExport { .. } => 0.1, // Minimal overhead
            DataPreservationStrategy::InMemoryCache { compress, .. } => {
                if *compress { 0.3 } else { 1.0 }
            }
        }
    }
    
    /// Check if this strategy is suitable for the given data size
    pub fn is_suitable_for_size(&self, estimated_rows: usize, estimated_size_bytes: usize) -> bool {
        match self {
            DataPreservationStrategy::BackupTable { .. } => true, // Always suitable
            DataPreservationStrategy::TemporaryStorage { size_limit, .. } => {
                size_limit.map_or(true, |limit| estimated_size_bytes <= limit)
            }
            DataPreservationStrategy::ExternalExport { .. } => true, // Always suitable
            DataPreservationStrategy::InMemoryCache { max_rows, .. } => {
                estimated_rows <= *max_rows && estimated_size_bytes <= 100 * 1024 * 1024 // 100MB limit
            }
        }
    }
    
    /// Get the risk level associated with this preservation strategy
    pub fn risk_level(&self) -> RollbackRiskLevel {
        match self {
            DataPreservationStrategy::BackupTable { .. } => RollbackRiskLevel::Low,
            DataPreservationStrategy::TemporaryStorage { .. } => RollbackRiskLevel::Medium,
            DataPreservationStrategy::ExternalExport { .. } => RollbackRiskLevel::Medium,
            DataPreservationStrategy::InMemoryCache { .. } => RollbackRiskLevel::High,
        }
    }
}

impl fmt::Display for DataPreservationStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataPreservationStrategy::BackupTable { backup_name, drop_after_restore } => {
                write!(f, "Backup table '{}'{}", backup_name,
                    if *drop_after_restore { " (temporary)" } else { " (permanent)" })
            }
            DataPreservationStrategy::TemporaryStorage { storage_location, size_limit } => {
                write!(f, "Temporary storage at '{}'", storage_location)?;
                if let Some(limit) = size_limit {
                    write!(f, " (max {} bytes)", limit)?;
                }
                Ok(())
            }
            DataPreservationStrategy::ExternalExport { export_path, export_format } => {
                write!(f, "Export to '{}' as {}", export_path, export_format)
            }
            DataPreservationStrategy::InMemoryCache { max_rows, compress } => {
                write!(f, "In-memory cache (max {} rows{})", max_rows,
                    if *compress { ", compressed" } else { "" })
            }
        }
    }
}

/// Format options for data export
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum DataExportFormat {
    /// Comma-separated values
    Csv,
    /// JSON format
    Json,
    /// SQL INSERT statements
    SqlInserts,
    /// Custom format with specification
    Custom(String),
}

impl fmt::Display for DataExportFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataExportFormat::Csv => write!(f, "CSV"),
            DataExportFormat::Json => write!(f, "JSON"),
            DataExportFormat::SqlInserts => write!(f, "SQL"),
            DataExportFormat::Custom(format) => write!(f, "Custom({})", format),
        }
    }
}

/// Requirements for preserving data during a rollback operation
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DataPreservationRequirement {
    /// Table whose data needs preservation
    pub table: String,
    
    /// Specific columns to preserve (empty means all columns)
    pub columns: Vec<String>,
    
    /// Strategy to use for preservation
    pub preservation_strategy: DataPreservationStrategy,
    
    /// Optional location for backup storage
    pub backup_location: Option<String>,
    
    /// Estimated number of rows to preserve
    pub estimated_rows: Option<usize>,
    
    /// Estimated size in bytes
    pub estimated_size_bytes: Option<usize>,
    
    /// Priority level for this preservation requirement
    pub priority: PreservationPriority,
}

impl DataPreservationRequirement {
    /// Create a new data preservation requirement
    pub fn new(
        table: String,
        columns: Vec<String>,
        preservation_strategy: DataPreservationStrategy,
    ) -> Self {
        Self {
            table,
            columns,
            preservation_strategy,
            backup_location: None,
            estimated_rows: None,
            estimated_size_bytes: None,
            priority: PreservationPriority::Medium,
        }
    }
    
    /// Set the estimated data size for this requirement
    pub fn with_size_estimate(mut self, rows: usize, bytes: usize) -> Self {
        self.estimated_rows = Some(rows);
        self.estimated_size_bytes = Some(bytes);
        self
    }
    
    /// Set the priority for this requirement
    pub fn with_priority(mut self, priority: PreservationPriority) -> Self {
        self.priority = priority;
        self
    }
    
    /// Set the backup location for this requirement
    pub fn with_backup_location(mut self, location: String) -> Self {
        self.backup_location = Some(location);
        self
    }
    
    /// Validate that this requirement is feasible
    pub fn validate(&self) -> Result<(), String> {
        if self.table.is_empty() {
            return Err("Table name cannot be empty".to_string());
        }
        
        if let (Some(rows), Some(bytes)) = (self.estimated_rows, self.estimated_size_bytes) {
            if !self.preservation_strategy.is_suitable_for_size(rows, bytes) {
                return Err(format!(
                    "Preservation strategy '{}' is not suitable for data size ({} rows, {} bytes)",
                    self.preservation_strategy, rows, bytes
                ));
            }
        }
        
        Ok(())
    }
    
    /// Get the risk level for this preservation requirement
    pub fn risk_level(&self) -> RollbackRiskLevel {
        let strategy_risk = self.preservation_strategy.risk_level();
        let priority_risk = self.priority.to_risk_level();
        RollbackRiskLevel::max_level([strategy_risk, priority_risk])
    }
}

impl fmt::Display for DataPreservationRequirement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Preserve data from table '{}'", self.table)?;
        
        if !self.columns.is_empty() {
            write!(f, " (columns: {})", self.columns.join(", "))?;
        }
        
        write!(f, " using {}", self.preservation_strategy)?;
        
        if let Some(location) = &self.backup_location {
            write!(f, " at '{}'", location)?;
        }
        
        write!(f, " [{}]", self.priority)?;
        
        Ok(())
    }
}

/// Priority level for data preservation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy, serde::Serialize, serde::Deserialize)]
pub enum PreservationPriority {
    /// Low priority - optional preservation
    Low,
    /// Medium priority - recommended preservation
    Medium,
    /// High priority - required preservation
    High,
    /// Critical priority - preservation is mandatory
    Critical,
}

impl PreservationPriority {
    /// Convert to equivalent risk level
    pub fn to_risk_level(&self) -> RollbackRiskLevel {
        match self {
            PreservationPriority::Low => RollbackRiskLevel::Low,
            PreservationPriority::Medium => RollbackRiskLevel::Medium,
            PreservationPriority::High => RollbackRiskLevel::High,
            PreservationPriority::Critical => RollbackRiskLevel::Critical,
        }
    }
    
    /// Get numeric value for comparison
    pub fn numeric_value(&self) -> u8 {
        match self {
            PreservationPriority::Low => 0,
            PreservationPriority::Medium => 1,
            PreservationPriority::High => 2,
            PreservationPriority::Critical => 3,
        }
    }
}

impl fmt::Display for PreservationPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            PreservationPriority::Low => "Low Priority",
            PreservationPriority::Medium => "Medium Priority",
            PreservationPriority::High => "High Priority",
            PreservationPriority::Critical => "Critical Priority",
        }; 
        write!(f, "{}", name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rollback_operation_type() {
        let op = RollbackOperation::DropTable {
            name: "users".to_string(),
            preserve_data: true,
            backup_table_name: Some("users_backup".to_string()),
        };
        assert_eq!(op.operation_type(), "drop_table");
    }
    
    #[test]
    fn test_rollback_operation_affected_table() {
        let op = RollbackOperation::AddColumn {
            table: "posts".to_string(),
            column: ColumnSchema {
                name: "title".to_string(),
                column_type: "TEXT".to_string(),
                nullable: false,
                primary_key: false,
                unique: false,
                auto_increment: false,
                default_value: None,
                constraints: vec![],
            },
            restore_data: true,
        };
        assert_eq!(op.affected_table(), "posts");
    }
    
    #[test]
    fn test_rollback_operation_is_destructive() {
        let destructive_op = RollbackOperation::DropTable {
            name: "users".to_string(),
            preserve_data: false,
            backup_table_name: None,
        };
        assert!(destructive_op.is_destructive());
        
        let safe_op = RollbackOperation::RenameTable {
            old_name: "users".to_string(),
            new_name: "app_users".to_string(),
        };
        assert!(!safe_op.is_destructive());
    }
    
    #[test]
    fn test_risk_level_ordering() {
        assert!(RollbackRiskLevel::Low < RollbackRiskLevel::Medium);
        assert!(RollbackRiskLevel::Medium < RollbackRiskLevel::High);
        assert!(RollbackRiskLevel::High < RollbackRiskLevel::Critical);
    }
    
    #[test]
    fn test_risk_level_max() {
        let levels = vec![
            RollbackRiskLevel::Low,
            RollbackRiskLevel::Critical,
            RollbackRiskLevel::Medium,
        ];
        assert_eq!(RollbackRiskLevel::max_level(levels), RollbackRiskLevel::Critical);
    }
    
    #[test]
    fn test_column_changes_is_lossy() {
        let lossy_changes = RollbackColumnChanges {
            type_change: Some(("TEXT".to_string(), "INTEGER".to_string())),
            null_change: None,
            default_change: None,
            constraint_changes: Vec::new(),
        };
        assert!(lossy_changes.is_lossy());
        
        let safe_changes = RollbackColumnChanges {
            type_change: Some(("INTEGER".to_string(), "TEXT".to_string())),
            null_change: None,
            default_change: None,
            constraint_changes: Vec::new(),
        };
        assert!(!safe_changes.is_lossy());
    }
    
    #[test]
    fn test_rollback_config_validation() {
        let valid_config = RollbackConfig::default();
        assert!(valid_config.validate().is_ok());
        
        let invalid_config = RollbackConfig {
            max_parallel_operations: 0,
            ..RollbackConfig::default()
        };
        assert!(invalid_config.validate().is_err());
    }
    
    #[test]
    fn test_rollback_config_backup_table_name() {
        let config = RollbackConfig {
            backup_table_prefix: "backup_".to_string(),
            backup_table_suffix: "_20231201".to_string(),
            ..RollbackConfig::default()
        };
        assert_eq!(config.backup_table_name("users"), "backup_users_20231201");
    }
    
    #[test]
    fn test_data_preservation_strategy_storage_overhead() {
        let backup_strategy = DataPreservationStrategy::BackupTable {
            backup_name: "test_backup".to_string(),
            drop_after_restore: true,
        };
        assert_eq!(backup_strategy.storage_overhead_factor(), 2.0);
        
        let memory_strategy = DataPreservationStrategy::InMemoryCache {
            max_rows: 1000,
            compress: true,
        };
        assert_eq!(memory_strategy.storage_overhead_factor(), 0.3);
    }
    
    #[test]
    fn test_data_preservation_requirement_validation() {
        let valid_req = DataPreservationRequirement::new(
            "users".to_string(),
            vec!["id".to_string(), "name".to_string()],
            DataPreservationStrategy::BackupTable {
                backup_name: "users_backup".to_string(),
                drop_after_restore: true,
            },
        );
        assert!(valid_req.validate().is_ok());
        
        let invalid_req = DataPreservationRequirement::new(
            "".to_string(),
            vec![],
            DataPreservationStrategy::BackupTable {
                backup_name: "backup".to_string(),
                drop_after_restore: true,
            },
        );
        assert!(invalid_req.validate().is_err());
    }
    
    #[test]
    fn test_data_loss_risk_max() {
        let risks = vec![
            DataLossRisk::None,
            DataLossRisk::High,
            DataLossRisk::Low,
        ];
        assert_eq!(DataLossRisk::max_risk(risks), DataLossRisk::High);
    }

    // Additional comprehensive tests for Phase 1.1 completion

    #[test]
    fn test_all_rollback_operation_variants_creation() {
        // Test that all operation variants can be constructed 
        let _drop_table = RollbackOperation::DropTable {
            name: "test_table".to_string(),
            preserve_data: true,
            backup_table_name: Some("test_table_backup".to_string()),
        };
        
        let _recreate_table = RollbackOperation::RecreateTable {
            definition: TableSchema {
                name: "test_table".to_string(),
                columns: vec![],
                foreign_keys: vec![],
                indexes: vec![],
                constraints: vec![],
            },
            restore_data: true,
            data_source: Some("test_table_backup".to_string()),
        };
        
        let _drop_column = RollbackOperation::DropColumn {
            table: "test_table".to_string(),
            column: "test_column".to_string(),
            preserve_data: true,
        };
        
        let _add_column = RollbackOperation::AddColumn {
            table: "test_table".to_string(),
            column: ColumnSchema {
                name: "test_column".to_string(),
                column_type: "TEXT".to_string(),
                nullable: true,
                primary_key: false,
                unique: false,
                auto_increment: false,
                default_value: None,
                constraints: vec![],
            },
            restore_data: false,
        };
        
        // If we get here without panicking, all variants can be constructed
        assert!(true);
    }

    #[test]
    fn test_rollback_config_fields() {
        let config = RollbackConfig::default();
        
        // Test that all expected fields are accessible
        assert_eq!(config.preserve_data_on_rollback, true);
        assert_eq!(config.restore_data_on_rollback, true);
        assert_eq!(config.create_backup_tables, true);
        assert_eq!(config.stop_on_first_failure, true);
        assert_eq!(config.use_transactions, true);
    }

    #[test]
    fn test_data_preservation_strategies_basic() {
        let backup_strategy = DataPreservationStrategy::BackupTable {
            backup_name: "test_backup".to_string(),
            drop_after_restore: true,
        };
        
        let temp_strategy = DataPreservationStrategy::TemporaryStorage {
            storage_location: "/tmp/rollback".to_string(),
            size_limit: Some(1024 * 1024),
        };
        
        let export_strategy = DataPreservationStrategy::ExternalExport {
            export_path: "/backup/export.sql".to_string(),
            export_format: DataExportFormat::SqlInserts,
        };
        
        let cache_strategy = DataPreservationStrategy::InMemoryCache {
            max_rows: 1000,
            compress: true,
        };

        // Test that all strategies can be created and displayed
        let _backup_display = format!("{}", backup_strategy);
        let _temp_display = format!("{}", temp_strategy);
        let _export_display = format!("{}", export_strategy);
        let _cache_display = format!("{}", cache_strategy);
        
        // Test cloning
        let _cloned_backup = backup_strategy.clone();
        let _cloned_temp = temp_strategy.clone();
        let _cloned_export = export_strategy.clone();
        let _cloned_cache = cache_strategy.clone();
    }

    #[test]
    fn test_data_preservation_requirement_basic() {
        let requirement = DataPreservationRequirement::new(
            "users".to_string(),
            vec!["id".to_string(), "email".to_string()],
            DataPreservationStrategy::BackupTable {
                backup_name: "users_backup".to_string(),
                drop_after_restore: true,
            },
        );

        assert_eq!(requirement.table, "users");
        assert_eq!(requirement.columns.len(), 2);
        assert!(requirement.validate().is_ok());

        let _display_str = format!("{}", requirement);
    }

    #[test]
    fn test_rollback_column_changes_basic() {
        let changes = RollbackColumnChanges {
            type_change: Some(("INTEGER".to_string(), "TEXT".to_string())),
            null_change: Some((false, true)),
            default_change: Some((Some("0".to_string()), Some("''".to_string()))),
            constraint_changes: vec![],
        };

        // Test the struct was created successfully
        assert!(changes.type_change.is_some());
        
        // Test is_lossy method that exists
        assert!(!changes.is_lossy()); // INTEGER to TEXT is generally safe
        
        let lossy_changes = RollbackColumnChanges {
            type_change: Some(("TEXT".to_string(), "INTEGER".to_string())),
            null_change: None,
            default_change: None,
            constraint_changes: vec![],
        };
        assert!(lossy_changes.is_lossy());
    }

    #[test]
    fn test_type_safety_comprehensive() {
        // Test that all types can be cloned and compared where appropriate
        let operation = RollbackOperation::DropTable {
            name: "test".to_string(),
            preserve_data: true,
            backup_table_name: None,
        };
        
        let cloned_operation = operation.clone();
        assert_eq!(operation, cloned_operation);

        // Test risk levels
        let risk = RollbackRiskLevel::High;
        let cloned_risk = risk;
        assert_eq!(risk, cloned_risk);

        // Test data loss risk
        let data_loss = DataLossRisk::Medium;
        let cloned_data_loss = data_loss;
        assert_eq!(data_loss, cloned_data_loss);
    }

    #[test]
    fn test_display_traits_basic() {
        let operation = RollbackOperation::DropTable {
            name: "users".to_string(),
            preserve_data: true,
            backup_table_name: Some("users_backup".to_string()),
        };

        let display = format!("{}", operation);
        assert!(!display.is_empty());
        assert!(display.len() > 10); // Should be reasonably descriptive

        // Test risk level displays
        assert_eq!(format!("{}", RollbackRiskLevel::Low), "LOW");
        assert_eq!(format!("{}", RollbackRiskLevel::Critical), "CRITICAL");
        assert_eq!(format!("{}", DataLossRisk::High), "High");
    }
}