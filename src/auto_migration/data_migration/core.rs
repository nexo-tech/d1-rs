// Core data migration functionality and structures

use crate::{D1Client, Result, D1RsError};
use crate::auto_migration::DatabaseSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Data migration engine - handles complex data transformations during schema changes
/// Provides type-safe data migration with comprehensive error handling and rollback support
pub struct DataMigrator {
    /// Database client for executing migration operations
    pub(crate) db: D1Client,
    
    /// Configuration for data migration behavior
    pub(crate) config: DataMigrationConfig,
    
    /// Migration context with current state
    pub(crate) context: std::cell::RefCell<MigrationContext>,
}

/// Configuration for data migration operations
pub struct DataMigrationConfig {
    /// Batch size for processing large datasets
    pub batch_size: usize,
    
    /// Maximum time to spend on a single transformation
    pub max_transformation_time: Duration,
    
    /// Whether to create backups before destructive operations
    pub create_backups: bool,
    
    /// Strategy for handling transformation failures
    pub failure_strategy: FailureStrategy,
    
    /// Whether to verify data integrity after transformations
    pub verify_integrity: bool,
    
    /// Custom transformation functions
    pub custom_transformations: HashMap<String, Box<dyn TransformationFunction>>,
}

impl std::fmt::Debug for DataMigrationConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataMigrationConfig")
            .field("batch_size", &self.batch_size)
            .field("max_transformation_time", &self.max_transformation_time)
            .field("create_backups", &self.create_backups)
            .field("failure_strategy", &self.failure_strategy)
            .field("verify_integrity", &self.verify_integrity)
            .field("custom_transformations", &format!("{} transformations", self.custom_transformations.len()))
            .finish()
    }
}

/// Strategy for handling transformation failures
#[derive(Debug, Clone, PartialEq)]
pub enum FailureStrategy {
    /// Stop immediately on first failure
    StopOnFailure,
    /// Skip failed records and continue
    SkipFailures,
    /// Retry failed records with backoff
    RetryWithBackoff,
    /// Use default values for failed transformations
    UseDefaults,
}

/// Context information for the current migration
#[derive(Debug, Clone)]
pub struct MigrationContext {
    /// Current migration identifier
    pub migration_id: String,
    
    /// Schema before migration
    pub source_schema: DatabaseSchema,
    
    /// Schema after migration
    pub target_schema: DatabaseSchema,
    
    /// Statistics about the migration progress
    pub statistics: MigrationStatistics,
    
    /// Temporary tables created during migration
    pub temporary_tables: Vec<String>,
    
    /// Backup table names for rollback
    pub backup_tables: HashMap<String, String>,
}

/// Statistics about migration progress
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MigrationStatistics {
    /// Total number of records processed
    pub total_records_processed: u64,
    
    /// Number of successful transformations
    pub successful_transformations: u64,
    
    /// Number of failed transformations
    pub failed_transformations: u64,
    
    /// Time spent on migration
    pub elapsed_time: Duration,
    
    /// Memory usage during migration
    pub peak_memory_usage: u64,
    
    /// Transformation performance by type
    pub transformation_metrics: HashMap<String, TransformationMetrics>,
}

/// Performance metrics for a specific transformation type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationMetrics {
    pub execution_count: u64,
    pub total_duration: Duration,
    pub average_duration: Duration,
    pub success_rate: f64,
    pub error_count: u64,
}

/// Different types of data transformations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataTransformation {
    /// Transform column data during type changes
    ColumnTransformation {
        table: String,
        old_column: String,
        new_column: String,
        transformation_type: ColumnTransformationType,
    },
    
    /// Migrate relationship data during FK changes
    RelationshipMigration {
        source_table: String,
        target_table: String,
        old_fk_column: String,
        new_fk_column: String,
        migration_strategy: RelationshipMigrationStrategy,
    },
    
    /// Populate junction tables for M2M relationships
    JunctionTablePopulation {
        junction_table: String,
        source_table: String,
        target_table: String,
        source_fk: String,
        target_fk: String,
        data_source: JunctionDataSource,
    },
    
    /// Custom data migration logic
    CustomMigration {
        migration_name: String,
        source_query: String,
        transformation_logic: String,
        target_operations: Vec<String>,
    },
}

/// Types of column transformations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColumnTransformationType {
    /// Type conversion (e.g., INTEGER to TEXT)
    TypeConversion {
        from_type: String,
        to_type: String,
        conversion_function: String,
    },
    
    /// Data normalization (e.g., split full_name into first_name, last_name)
    Normalization {
        source_pattern: String,
        target_fields: Vec<String>,
        extraction_rules: Vec<String>,
    },
    
    /// Data aggregation (e.g., combine first_name, last_name into full_name)
    Aggregation {
        source_fields: Vec<String>,
        target_field: String,
        aggregation_function: String,
    },
    
    /// Value mapping (e.g., status codes to descriptions)
    ValueMapping {
        mapping_table: HashMap<String, String>,
        default_value: Option<String>,
    },
    
    /// Format transformation (e.g., date format changes)
    FormatTransformation {
        source_format: String,
        target_format: String,
        format_function: String,
    },
}

/// Strategies for migrating relationship data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipMigrationStrategy {
    /// Direct FK value copy
    DirectCopy,
    
    /// Map old IDs to new IDs using lookup table
    IdMapping {
        mapping_table: String,
        old_id_column: String,
        new_id_column: String,
    },
    
    /// Recreate relationships based on business logic
    BusinessLogicRecreation {
        recreation_query: String,
        validation_rules: Vec<String>,
    },
    
    /// Cascade migration with dependency resolution
    CascadeMigration {
        dependency_order: Vec<String>,
        cascade_rules: HashMap<String, String>,
    },
}

/// Sources for junction table data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JunctionDataSource {
    /// Extract from denormalized columns
    DenormalizedColumns {
        source_table: String,
        source_column: String,
        delimiter: String,
    },
    
    /// Copy from existing junction table
    ExistingJunctionTable {
        source_junction_table: String,
        column_mapping: HashMap<String, String>,
    },
    
    /// Generate from business rules
    BusinessRules {
        generation_query: String,
        validation_rules: Vec<String>,
    },
    
    /// Import from external data source
    ExternalSource {
        source_identifier: String,
        import_format: String,
        mapping_rules: HashMap<String, String>,
    },
}

/// Result of a data migration operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataMigrationResult {
    /// Whether the migration was successful
    pub success: bool,
    
    /// Transformations that were applied
    pub applied_transformations: Vec<DataTransformation>,
    
    /// Migration statistics
    pub statistics: MigrationStatistics,
    
    /// Any errors that occurred
    pub errors: Vec<crate::D1RsError>,
    
    /// Warnings about potential data issues
    pub warnings: Vec<String>,
    
    /// Rollback information if needed
    pub rollback_info: Option<RollbackInfo>,
}

/// Information needed for rollback operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackInfo {
    /// Backup table names
    pub backup_tables: HashMap<String, String>,
    
    /// Temporary tables to clean up
    pub temporary_tables: Vec<String>,
    
    /// Operations to reverse
    pub reverse_operations: Vec<String>,
    
    /// Data snapshots for restoration
    pub data_snapshots: HashMap<String, String>,
}

/// Trait for custom transformation functions
pub trait TransformationFunction: Send + Sync {
    /// Execute the transformation
    fn transform(
        &self,
        input_data: &HashMap<String, serde_json::Value>,
        context: &MigrationContext,
    ) -> Result<HashMap<String, serde_json::Value>>;
    
    /// Validate the transformation configuration
    fn validate_config(&self) -> Result<()>;
    
    /// Get the transformation name
    fn name(&self) -> &str;
}

/// Result of a single transformation operation
#[derive(Debug, Clone)]
pub struct TransformationResult {
    /// Whether the transformation was successful
    pub success: bool,
    
    /// Number of records processed
    pub records_processed: u64,
    
    /// Number of records that failed
    pub records_failed: u64,
    
    /// Any errors that occurred
    pub errors: Vec<D1RsError>,
    
    /// Warnings about data issues
    pub warnings: Vec<String>,
}

impl DataMigrator {
    /// Create a new data migrator with the specified configuration
    pub fn new(db: D1Client, config: DataMigrationConfig) -> Self {
        Self {
            db,
            config,
            context: std::cell::RefCell::new(MigrationContext {
                migration_id: String::new(),
                source_schema: DatabaseSchema { tables: Vec::new() },
                target_schema: DatabaseSchema { tables: Vec::new() },
                statistics: MigrationStatistics::default(),
                temporary_tables: Vec::new(),
                backup_tables: HashMap::new(),
            }),
        }
    }
    
    /// Initialize migration context with source and target schemas
    pub fn initialize_migration(
        &self,
        migration_id: String,
        source_schema: DatabaseSchema,
        target_schema: DatabaseSchema,
    ) -> Result<()> {
        let mut context = self.context.borrow_mut();
        context.migration_id = migration_id;
        context.source_schema = source_schema;
        context.target_schema = target_schema;
        context.statistics = MigrationStatistics::default();
        context.temporary_tables.clear();
        context.backup_tables.clear();
        
        Ok(())
    }
    
    /// Update transformation metrics
    pub(crate) fn update_transformation_metrics(&self, transformation_type: &str, duration: Duration, success: bool) {
        let mut context = self.context.borrow_mut();
        let metrics = context.statistics.transformation_metrics
            .entry(transformation_type.to_string())
            .or_insert_with(|| TransformationMetrics {
                execution_count: 0,
                total_duration: Duration::ZERO,
                average_duration: Duration::ZERO,
                success_rate: 0.0,
                error_count: 0,
            });
        
        metrics.execution_count += 1;
        metrics.total_duration += duration;
        metrics.average_duration = metrics.total_duration / metrics.execution_count as u32;
        
        if !success {
            metrics.error_count += 1;
        }
        
        metrics.success_rate = (metrics.execution_count - metrics.error_count) as f64 / metrics.execution_count as f64;
    }
}