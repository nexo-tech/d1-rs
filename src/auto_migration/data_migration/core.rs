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
    
    /// Execute custom transformation logic using registered transformation functions
    pub async fn execute_custom_transformation(
        &self,
        migration_name: &str,
        source_query: &str,
        transformation_logic: &str,
        target_operations: &[String],
    ) -> Result<TransformationResult> {
        let start_time = std::time::Instant::now();
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        
        // Input validation
        if migration_name.is_empty() {
            return Err(D1RsError::ValidationError("Migration name cannot be empty".to_string()));
        }
        
        if source_query.trim().is_empty() {
            return Err(D1RsError::ValidationError("Source query cannot be empty".to_string()));
        }
        
        if transformation_logic.trim().is_empty() {
            return Err(D1RsError::ValidationError("Transformation logic cannot be empty".to_string()));
        }
        
        if target_operations.is_empty() {
            return Err(D1RsError::ValidationError("At least one target operation must be specified".to_string()));
        }
        
        // Validate SQL syntax (basic check for injection protection)
        if source_query.to_lowercase().contains(";") && source_query.matches(';').count() > 1 {
            return Err(D1RsError::ValidationError("Source query should not contain multiple statements".to_string()));
        }
        
        // Check if custom transformation function exists
        let transformation_function = self.config.custom_transformations.get(transformation_logic)
            .ok_or_else(|| D1RsError::ValidationError(
                format!("Custom transformation '{}' not found in configuration", transformation_logic)
            ))?;
        
        // Validate transformation configuration
        if let Err(e) = transformation_function.validate_config() {
            return Err(D1RsError::ValidationError(
                format!("Custom transformation validation failed: {}", e)
            ));
        }
        
        warnings.push(format!("Starting custom migration: {}", migration_name));
        warnings.push(format!("Using transformation function: {}", transformation_function.name()));
        
        let mut total_processed = 0u64;
        let mut total_failed = 0u64;
        let mut total_successful = 0u64;
        
        // Execute source query to get input data
        let source_result = match self.db.execute(source_query, &[]).await {
            Ok(result) => result,
            Err(e) => {
                errors.push(D1RsError::AutoMigration(
                    format!("Failed to execute source query for custom migration '{}': {}", migration_name, e)
                ));
                let duration = start_time.elapsed();
                self.update_transformation_metrics("custom_transformation", duration, false);
                
                return Ok(TransformationResult {
                    success: false,
                    records_processed: 0,
                    records_failed: 1,
                    errors,
                    warnings,
                });
            }
        };
        
        warnings.push(format!("Source query returned {} rows", source_result.rows.len()));
        
        let batch_size = self.config.batch_size;
        
        // Process source data in batches
        for (batch_index, chunk) in source_result.rows.chunks(batch_size).enumerate() {
            warnings.push(format!("Processing batch {} with {} records", batch_index + 1, chunk.len()));
            
            for (row_index, row) in chunk.iter().enumerate() {
                total_processed += 1;
                
                // Convert row to HashMap for transformation
                let mut input_data = HashMap::new();
                if let serde_json::Value::Object(row_map) = row {
                    for (key, value) in row_map {
                        input_data.insert(key.clone(), value.clone());
                    }
                } else {
                    warnings.push(format!("Skipping non-object row {} in batch {}", row_index + 1, batch_index + 1));
                    total_failed += 1;
                    continue;
                }
                
                // Apply custom transformation
                let transformed_data = {
                    let context = self.context.borrow();
                    match transformation_function.transform(&input_data, &*context) {
                        Ok(data) => data,
                        Err(e) => {
                            errors.push(D1RsError::AutoMigration(
                                format!("Custom transformation failed for row {} in batch {}: {}", row_index + 1, batch_index + 1, e)
                            ));
                            total_failed += 1;
                            continue;
                        }
                    }
                };
                
                // Execute target operations with transformed data
                let mut operation_success = true;
                for (op_index, operation) in target_operations.iter().enumerate() {
                    // Replace placeholders in operation with transformed data
                    let mut final_operation = operation.clone();
                    for (key, value) in &transformed_data {
                        let placeholder = format!("${}", key);
                        let value_str = match value {
                            serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
                            serde_json::Value::Number(n) => n.to_string(),
                            serde_json::Value::Bool(b) => if *b { "1".to_string() } else { "0".to_string() },
                            serde_json::Value::Null => "NULL".to_string(),
                            _ => format!("'{}'", value.to_string().replace('\'', "''")),
                        };
                        final_operation = final_operation.replace(&placeholder, &value_str);
                    }
                    
                    // Execute target operation
                    match self.db.execute(&final_operation, &[]).await {
                        Ok(_) => {
                            // Operation succeeded
                        }
                        Err(e) => {
                            errors.push(D1RsError::AutoMigration(
                                format!("Target operation {} failed for row {} in batch {}: {}", op_index + 1, row_index + 1, batch_index + 1, e)
                            ));
                            operation_success = false;
                            break;
                        }
                    }
                }
                
                if operation_success {
                    total_successful += 1;
                } else {
                    total_failed += 1;
                }
                
                // Check timeout
                if start_time.elapsed() > self.config.max_transformation_time {
                    warnings.push(format!("Custom transformation timeout reached after processing {} records", total_processed));
                    break;
                }
            }
            
            // Check timeout between batches
            if start_time.elapsed() > self.config.max_transformation_time {
                warnings.push(format!("Custom transformation timeout reached after {} batches", batch_index + 1));
                break;
            }
        }
        
        // Generate summary
        warnings.push(format!("Custom migration '{}' completed", migration_name));
        warnings.push(format!("Processed: {}, Successful: {}, Failed: {}", total_processed, total_successful, total_failed));
        
        if total_failed > 0 {
            warnings.push(format!("Failed to process {} out of {} records", total_failed, total_processed));
        }
        
        // Update metrics
        let duration = start_time.elapsed();
        self.update_transformation_metrics("custom_transformation", duration, total_failed == 0);
        
        // Determine success: all records should be processed successfully
        let success = total_failed == 0 && errors.is_empty();
        
        Ok(TransformationResult {
            success,
            records_processed: total_processed,
            records_failed: total_failed,
            errors,
            warnings,
        })
    }
}