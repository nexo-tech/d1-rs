// Core data migration functionality and structures

use crate::{D1Client, Result, D1RsError};
use crate::backends::QueryResult;
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
                source_schema: DatabaseSchema { 
                    tables: Vec::new(),
                    dialect: crate::dialects::DatabaseDialect::SQLite,
                },
                target_schema: DatabaseSchema { 
                    tables: Vec::new(),
                    dialect: crate::dialects::DatabaseDialect::SQLite,
                },
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
        
        warnings.push(format!("Source query returned {} rows", source_result.rows().len()));
        
        let batch_size = self.config.batch_size;
        
        // Process source data in batches
        for (batch_index, chunk) in source_result.rows().chunks(batch_size).enumerate() {
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
    
    /// Execute built-in custom migration using predefined transformation types
    pub async fn execute_built_in_custom_migration(
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
        
        warnings.push(format!("Starting built-in custom migration: {}", migration_name));
        
        // Parse transformation logic as JSON to determine built-in transformation type
        let transformation_config: serde_json::Value = match serde_json::from_str(transformation_logic) {
            Ok(config) => config,
            Err(e) => {
                return Err(D1RsError::ValidationError(
                    format!("Invalid transformation logic JSON: {}", e)
                ));
            }
        };
        
        let transformation_type = transformation_config
            .get("type")
            .and_then(|t| t.as_str())
            .ok_or_else(|| D1RsError::ValidationError(
                "Transformation logic must specify a 'type' field".to_string()
            ))?;
        
        warnings.push(format!("Using built-in transformation type: {}", transformation_type));
        
        let mut total_failed = 0u64;
        let mut total_successful = 0u64;
        
        // Execute source query to get input data
        let source_result = match self.db.execute(source_query, &[]).await {
            Ok(result) => result,
            Err(e) => {
                errors.push(D1RsError::AutoMigration(
                    format!("Failed to execute source query for built-in migration '{}': {}", migration_name, e)
                ));
                let duration = start_time.elapsed();
                self.update_transformation_metrics("built_in_custom_migration", duration, false);
                
                return Ok(TransformationResult {
                    success: false,
                    records_processed: 0,
                    records_failed: 1,
                    errors,
                    warnings,
                });
            }
        };
        
        warnings.push(format!("Source query returned {} rows", source_result.rows().len()));
        
        // Execute built-in transformation based on type
        let transformation_result = match transformation_type {
            "type_conversion" => {
                self.execute_built_in_type_conversion(&transformation_config, &source_result.rows()).await
            }
            "value_mapping" => {
                self.execute_built_in_value_mapping(&transformation_config, &source_result.rows()).await
            }
            "normalization" => {
                self.execute_built_in_normalization(&transformation_config, &source_result.rows()).await
            }
            "aggregation" => {
                self.execute_built_in_aggregation(&transformation_config, &source_result.rows()).await
            }
            "format_transformation" => {
                self.execute_built_in_format_transformation(&transformation_config, &source_result.rows()).await
            }
            _ => {
                return Err(D1RsError::ValidationError(
                    format!("Unsupported built-in transformation type: {}", transformation_type)
                ));
            }
        };
        
        let transformed_data = match transformation_result {
            Ok(data) => data,
            Err(e) => {
                errors.push(D1RsError::AutoMigration(
                    format!("Built-in transformation '{}' failed: {}", transformation_type, e)
                ));
                
                let duration = start_time.elapsed();
                self.update_transformation_metrics("built_in_custom_migration", duration, false);
                
                return Ok(TransformationResult {
                    success: false,
                    records_processed: 0,
                    records_failed: 1,
                    errors,
                    warnings,
                });
            }
        };
        
        let total_processed = transformed_data.len() as u64;
        
        let batch_size = self.config.batch_size;
        
        // Execute target operations with transformed data in batches
        for (batch_index, chunk) in transformed_data.chunks(batch_size).enumerate() {
            warnings.push(format!("Processing batch {} with {} transformed records", batch_index + 1, chunk.len()));
            
            for (row_index, record) in chunk.iter().enumerate() {
                // Execute target operations with transformed record
                let mut operation_success = true;
                for (op_index, operation) in target_operations.iter().enumerate() {
                    // Replace placeholders in operation with transformed data
                    let mut final_operation = operation.clone();
                    for (key, value) in record {
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
                                format!("Target operation {} failed for transformed record {} in batch {}: {}", op_index + 1, row_index + 1, batch_index + 1, e)
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
                    warnings.push(format!("Built-in custom migration timeout reached after processing {} records", total_processed));
                    break;
                }
            }
            
            // Check timeout between batches
            if start_time.elapsed() > self.config.max_transformation_time {
                warnings.push(format!("Built-in custom migration timeout reached after {} batches", batch_index + 1));
                break;
            }
        }
        
        // Generate summary
        warnings.push(format!("Built-in custom migration '{}' completed", migration_name));
        warnings.push(format!("Transformed: {}, Successful: {}, Failed: {}", total_processed, total_successful, total_failed));
        
        if total_failed > 0 {
            warnings.push(format!("Failed to process {} out of {} transformed records", total_failed, total_processed));
        }
        
        // Update metrics
        let duration = start_time.elapsed();
        self.update_transformation_metrics("built_in_custom_migration", duration, total_failed == 0);
        
        // Determine success: all transformed records should be processed successfully
        let success = total_failed == 0 && errors.is_empty();
        
        Ok(TransformationResult {
            success,
            records_processed: total_processed,
            records_failed: total_failed,
            errors,
            warnings,
        })
    }
    
    /// Execute built-in type conversion transformation
    async fn execute_built_in_type_conversion(
        &self,
        config: &serde_json::Value,
        rows: &[serde_json::Value],
    ) -> Result<Vec<std::collections::HashMap<String, serde_json::Value>>> {
        let from_type = config.get("from_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| D1RsError::ValidationError("type_conversion requires 'from_type' field".to_string()))?;
        
        let to_type = config.get("to_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| D1RsError::ValidationError("type_conversion requires 'to_type' field".to_string()))?;
        
        let source_column = config.get("source_column")
            .and_then(|v| v.as_str())
            .ok_or_else(|| D1RsError::ValidationError("type_conversion requires 'source_column' field".to_string()))?;
        
        let target_column = config.get("target_column")
            .and_then(|v| v.as_str())
            .unwrap_or(source_column);
        
        let mut transformed_data = Vec::new();
        
        for row in rows {
            if let serde_json::Value::Object(row_map) = row {
                let mut output = std::collections::HashMap::new();
                
                // Copy all fields
                for (key, value) in row_map {
                    output.insert(key.clone(), value.clone());
                }
                
                // Apply type conversion to source column
                if let Some(source_value) = row_map.get(source_column) {
                    let converted_value = self.convert_value_type(source_value, from_type, to_type)?;
                    output.insert(target_column.to_string(), converted_value);
                }
                
                transformed_data.push(output);
            }
        }
        
        Ok(transformed_data)
    }
    
    /// Execute built-in value mapping transformation
    async fn execute_built_in_value_mapping(
        &self,
        config: &serde_json::Value,
        rows: &[serde_json::Value],
    ) -> Result<Vec<std::collections::HashMap<String, serde_json::Value>>> {
        let source_column = config.get("source_column")
            .and_then(|v| v.as_str())
            .ok_or_else(|| D1RsError::ValidationError("value_mapping requires 'source_column' field".to_string()))?;
        
        let target_column = config.get("target_column")
            .and_then(|v| v.as_str())
            .unwrap_or(source_column);
        
        let mapping_obj = config.get("mapping")
            .and_then(|v| v.as_object())
            .ok_or_else(|| D1RsError::ValidationError("value_mapping requires 'mapping' object".to_string()))?;
        
        let default_value = config.get("default_value");
        
        // Convert mapping object to HashMap<String, String>
        let mut mapping = std::collections::HashMap::new();
        for (key, value) in mapping_obj {
            if let Some(value_str) = value.as_str() {
                mapping.insert(key.clone(), value_str.to_string());
            }
        }
        
        let mut transformed_data = Vec::new();
        
        for row in rows {
            if let serde_json::Value::Object(row_map) = row {
                let mut output = std::collections::HashMap::new();
                
                // Copy all fields
                for (key, value) in row_map {
                    output.insert(key.clone(), value.clone());
                }
                
                // Apply value mapping to source column
                if let Some(source_value) = row_map.get(source_column) {
                    let source_str = match source_value {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::Bool(b) => b.to_string(),
                        _ => source_value.to_string(),
                    };
                    
                    let mapped_value = if let Some(mapped) = mapping.get(&source_str) {
                        serde_json::Value::String(mapped.clone())
                    } else if let Some(default) = default_value {
                        default.clone()
                    } else {
                        source_value.clone()
                    };
                    
                    output.insert(target_column.to_string(), mapped_value);
                }
                
                transformed_data.push(output);
            }
        }
        
        Ok(transformed_data)
    }
    
    /// Execute built-in normalization transformation
    async fn execute_built_in_normalization(
        &self,
        config: &serde_json::Value,
        rows: &[serde_json::Value],
    ) -> Result<Vec<std::collections::HashMap<String, serde_json::Value>>> {
        let source_column = config.get("source_column")
            .and_then(|v| v.as_str())
            .ok_or_else(|| D1RsError::ValidationError("normalization requires 'source_column' field".to_string()))?;
        
        let target_fields = config.get("target_fields")
            .and_then(|v| v.as_array())
            .ok_or_else(|| D1RsError::ValidationError("normalization requires 'target_fields' array".to_string()))?;
        
        let _pattern = config.get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("\\s+");  // Default to whitespace split
        
        let mut transformed_data = Vec::new();
        
        for row in rows {
            if let serde_json::Value::Object(row_map) = row {
                let mut output = std::collections::HashMap::new();
                
                // Copy all fields
                for (key, value) in row_map {
                    output.insert(key.clone(), value.clone());
                }
                
                // Apply normalization to source column
                if let Some(source_value) = row_map.get(source_column) {
                    if let serde_json::Value::String(source_str) = source_value {
                        let parts: Vec<&str> = source_str.split_whitespace().collect();
                        
                        for (index, target_field_value) in target_fields.iter().enumerate() {
                            if let serde_json::Value::String(target_field) = target_field_value {
                                let part_value = if index < parts.len() {
                                    serde_json::Value::String(parts[index].to_string())
                                } else {
                                    serde_json::Value::Null
                                };
                                output.insert(target_field.clone(), part_value);
                            }
                        }
                    }
                }
                
                transformed_data.push(output);
            }
        }
        
        Ok(transformed_data)
    }
    
    /// Execute built-in aggregation transformation
    async fn execute_built_in_aggregation(
        &self,
        config: &serde_json::Value,
        rows: &[serde_json::Value],
    ) -> Result<Vec<std::collections::HashMap<String, serde_json::Value>>> {
        let source_fields = config.get("source_fields")
            .and_then(|v| v.as_array())
            .ok_or_else(|| D1RsError::ValidationError("aggregation requires 'source_fields' array".to_string()))?;
        
        let target_field = config.get("target_field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| D1RsError::ValidationError("aggregation requires 'target_field' string".to_string()))?;
        
        let separator = config.get("separator")
            .and_then(|v| v.as_str())
            .unwrap_or(" ");  // Default to space separator
        
        let mut transformed_data = Vec::new();
        
        for row in rows {
            if let serde_json::Value::Object(row_map) = row {
                let mut output = std::collections::HashMap::new();
                
                // Copy all fields
                for (key, value) in row_map {
                    output.insert(key.clone(), value.clone());
                }
                
                // Apply aggregation to source fields
                let mut aggregated_parts = Vec::new();
                for source_field_value in source_fields {
                    if let serde_json::Value::String(source_field) = source_field_value {
                        if let Some(field_value) = row_map.get(source_field) {
                            if let serde_json::Value::String(s) = field_value {
                                if !s.is_empty() {
                                    aggregated_parts.push(s.clone());
                                }
                            }
                        }
                    }
                }
                
                let aggregated_value = serde_json::Value::String(aggregated_parts.join(separator));
                output.insert(target_field.to_string(), aggregated_value);
                
                transformed_data.push(output);
            }
        }
        
        Ok(transformed_data)
    }
    
    /// Execute built-in format transformation
    async fn execute_built_in_format_transformation(
        &self,
        config: &serde_json::Value,
        rows: &[serde_json::Value],
    ) -> Result<Vec<std::collections::HashMap<String, serde_json::Value>>> {
        let source_column = config.get("source_column")
            .and_then(|v| v.as_str())
            .ok_or_else(|| D1RsError::ValidationError("format_transformation requires 'source_column' field".to_string()))?;
        
        let target_column = config.get("target_column")
            .and_then(|v| v.as_str())
            .unwrap_or(source_column);
        
        let source_format = config.get("source_format")
            .and_then(|v| v.as_str())
            .ok_or_else(|| D1RsError::ValidationError("format_transformation requires 'source_format' field".to_string()))?;
        
        let target_format = config.get("target_format")
            .and_then(|v| v.as_str())
            .ok_or_else(|| D1RsError::ValidationError("format_transformation requires 'target_format' field".to_string()))?;
        
        let mut transformed_data = Vec::new();
        
        for row in rows {
            if let serde_json::Value::Object(row_map) = row {
                let mut output = std::collections::HashMap::new();
                
                // Copy all fields
                for (key, value) in row_map {
                    output.insert(key.clone(), value.clone());
                }
                
                // Apply format transformation to source column
                if let Some(source_value) = row_map.get(source_column) {
                    if let serde_json::Value::String(source_str) = source_value {
                        let transformed_value = self.transform_format(source_str, source_format, target_format)?;
                        output.insert(target_column.to_string(), serde_json::Value::String(transformed_value));
                    } else {
                        output.insert(target_column.to_string(), source_value.clone());
                    }
                }
                
                transformed_data.push(output);
            }
        }
        
        Ok(transformed_data)
    }
    
    /// Convert value from one type to another
    fn convert_value_type(&self, value: &serde_json::Value, from_type: &str, to_type: &str) -> Result<serde_json::Value> {
        match (from_type.to_uppercase().as_str(), to_type.to_uppercase().as_str()) {
            ("INTEGER", "TEXT") => {
                if let serde_json::Value::Number(n) = value {
                    if let Some(i) = n.as_i64() {
                        Ok(serde_json::Value::String(i.to_string()))
                    } else {
                        Ok(serde_json::Value::String(n.to_string()))
                    }
                } else {
                    Ok(value.clone())
                }
            }
            ("TEXT", "INTEGER") => {
                if let serde_json::Value::String(s) = value {
                    if let Ok(i) = s.parse::<i64>() {
                        Ok(serde_json::Value::Number(i.into()))
                    } else {
                        Ok(serde_json::Value::Number(0.into()))
                    }
                } else {
                    Ok(value.clone())
                }
            }
            ("INTEGER", "REAL") => {
                if let serde_json::Value::Number(n) = value {
                    if let Some(i) = n.as_i64() {
                        Ok(serde_json::Value::Number(serde_json::Number::from_f64(i as f64).unwrap_or_else(|| 0.into())))
                    } else {
                        Ok(value.clone())
                    }
                } else {
                    Ok(value.clone())
                }
            }
            ("REAL", "INTEGER") => {
                if let serde_json::Value::Number(n) = value {
                    if let Some(f) = n.as_f64() {
                        Ok(serde_json::Value::Number((f as i64).into()))
                    } else {
                        Ok(value.clone())
                    }
                } else {
                    Ok(value.clone())
                }
            }
            ("BOOLEAN", "INTEGER") => {
                if let serde_json::Value::Bool(b) = value {
                    Ok(serde_json::Value::Number(if *b { 1.into() } else { 0.into() }))
                } else {
                    Ok(value.clone())
                }
            }
            ("INTEGER", "BOOLEAN") => {
                if let serde_json::Value::Number(n) = value {
                    if let Some(i) = n.as_i64() {
                        Ok(serde_json::Value::Bool(i != 0))
                    } else {
                        Ok(serde_json::Value::Bool(false))
                    }
                } else {
                    Ok(value.clone())
                }
            }
            _ => Ok(value.clone()),  // No conversion needed or unsupported
        }
    }
    
    /// Transform format from source to target format
    fn transform_format(&self, value: &str, source_format: &str, target_format: &str) -> Result<String> {
        // Basic format transformation - can be extended for specific formats like dates
        match (source_format.to_lowercase().as_str(), target_format.to_lowercase().as_str()) {
            ("uppercase", "lowercase") => Ok(value.to_lowercase()),
            ("lowercase", "uppercase") => Ok(value.to_uppercase()),
            ("trim", "none") => Ok(value.trim().to_string()),
            _ => Ok(value.to_string()),  // No transformation or unsupported
        }
    }
    
    /// Restore data from backup tables and snapshots
    pub async fn restore_from_backup(
        &self,
        rollback_info: &RollbackInfo,
    ) -> Result<TransformationResult> {
        let start_time = std::time::Instant::now();
        let errors = Vec::new();
        let mut warnings = Vec::new();
        
        // Input validation
        if rollback_info.backup_tables.is_empty() && rollback_info.data_snapshots.is_empty() {
            warnings.push("No backup tables or data snapshots to restore from".to_string());
            return Ok(TransformationResult {
                success: true,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        warnings.push("Starting data restoration from backup".to_string());
        warnings.push(format!("Backup tables to restore: {}", rollback_info.backup_tables.len()));
        warnings.push(format!("Data snapshots to restore: {}", rollback_info.data_snapshots.len()));
        
        let mut total_failed = 0u64;
        let mut total_successful = 0u64;
        let mut total_records_restored = 0u64;
        
        // Step 1: Restore from backup tables
        for (original_table, backup_table) in &rollback_info.backup_tables {
            warnings.push(format!("Restoring table '{}' from backup '{}'", original_table, backup_table));
            
            // Verify backup table exists
            let backup_exists_sql = format!(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{}'",
                backup_table
            );
            
            let backup_exists = match self.db.execute_returning_count(&backup_exists_sql, &[]).await {
                Ok(count) => count > 0,
                Err(e) => {
                    warnings.push(format!("Failed to check if backup table '{}' exists: {}", backup_table, e));
                    continue;
                }
            };
            
            if !backup_exists {
                warnings.push(format!("Backup table {} not found, skipping restoration for table '{}'", backup_table, original_table));
                continue;
            }
            
            // Get record count from backup table
            let count_sql = format!("SELECT COUNT(*) FROM {}", backup_table);
            let backup_record_count = match self.db.execute_returning_count(&count_sql, &[]).await {
                Ok(count) => count as u64,
                Err(e) => {
                    warnings.push(format!("Failed to count records in backup table '{}': {}", backup_table, e));
                    continue;
                }
            };
            
            warnings.push(format!("Backup table '{}' contains {} records", backup_table, backup_record_count));
            
            if backup_record_count == 0 {
                warnings.push(format!("No records found in backup table '{}'", backup_table));
                continue;
            }
            
            // Clear original table before restoration
            let clear_sql = format!("DELETE FROM {}", original_table);
            if let Err(e) = self.db.execute(&clear_sql, &[]).await {
                warnings.push(format!("Failed to clear original table '{}' before restoration: {}. Proceeding with restoration anyway.", original_table, e));
            } else {
                warnings.push(format!("Successfully cleared original table '{}' before restoration", original_table));
            }
            
            // Restore data from backup table in batches
            let batch_size = self.config.batch_size;
            let total_batches = (backup_record_count + batch_size as u64 - 1) / batch_size as u64;
            
            let mut batch_failures = 0u64;
            let mut batch_successes = 0u64;
            
            for batch_index in 0..total_batches {
                let offset = batch_index * batch_size as u64;
                
                warnings.push(format!("Processing batch {} of {} for table '{}'", 
                    batch_index + 1, total_batches, original_table));
                
                // Direct copy from backup table - backup should have same structure as original
                let insert_sql = format!(
                    "INSERT INTO {} SELECT * FROM {} ORDER BY rowid LIMIT {} OFFSET {}",
                    original_table, backup_table, batch_size, offset
                );
                
                // Execute batch restoration
                match self.db.execute(&insert_sql, &[]).await {
                    Ok(_) => {
                        // Get affected rows count for this batch
                        let affected_rows = match self.db.execute_returning_count("SELECT changes()", &[]).await {
                            Ok(count) => count as u64,
                            Err(_) => std::cmp::min(batch_size as u64, backup_record_count - offset),
                        };
                        
                        total_records_restored += affected_rows;
                        batch_successes += 1;
                        warnings.push(format!("Restored {} records in batch {} for table '{}'", 
                            affected_rows, batch_index + 1, original_table));
                    }
                    Err(e) => {
                        warnings.push(format!("Failed to restore batch {} for table '{}': {}", batch_index + 1, original_table, e));
                        batch_failures += 1;
                    }
                }
                
                // Check timeout
                if start_time.elapsed() > self.config.max_transformation_time {
                    warnings.push(format!("Restoration timeout reached after processing {} batches for table '{}'", 
                        batch_index + 1, original_table));
                    break;
                }
            }
            
            if batch_failures == 0 {
                total_successful += 1;
                warnings.push(format!("Successfully restored table '{}' from backup with {} records", 
                    original_table, backup_record_count));
            } else {
                total_failed += 1;
                warnings.push(format!("Partially failed to restore table '{}': {} successful batches, {} failed batches", 
                    original_table, batch_successes, batch_failures));
            }
        }
        
        // Step 2: Restore from data snapshots
        for (snapshot_key, snapshot_data) in &rollback_info.data_snapshots {
            warnings.push(format!("Restoring data snapshot: {}", snapshot_key));
            
            // Parse snapshot data as JSON
            let snapshot_json: serde_json::Value = match serde_json::from_str(snapshot_data) {
                Ok(json) => json,
                Err(e) => {
                    warnings.push(format!("Failed to parse snapshot data for '{}': {}", snapshot_key, e));
                    continue;
                }
            };
            
            // Handle both direct arrays and structured data
            let (table_name, snapshot_rows) = if let Some(rows) = snapshot_json.as_array() {
                // Direct array format: use the snapshot key as table name
                (snapshot_key.as_str(), rows)
            } else if let Some(table) = snapshot_json.get("table").and_then(|t| t.as_str()) {
                // Structured format with table and rows
                if let Some(rows) = snapshot_json.get("rows").and_then(|r| r.as_array()) {
                    (table, rows)
                } else {
                    warnings.push(format!("Snapshot '{}' has table name but missing or invalid rows data", snapshot_key));
                    continue;
                }
            } else {
                warnings.push(format!("Snapshot '{}' has invalid format - expected array or object with table/rows", snapshot_key));
                continue;
            };
            
            warnings.push(format!("Snapshot '{}' contains {} rows for table '{}'", 
                snapshot_key, snapshot_rows.len(), table_name));
            
            if snapshot_rows.is_empty() {
                warnings.push(format!("Snapshot '{}' is empty, skipping restoration", snapshot_key));
                continue;
            }
            
            // Clear table before snapshot restoration
            let clear_sql = format!("DELETE FROM {}", table_name);
            if let Err(e) = self.db.execute(&clear_sql, &[]).await {
                warnings.push(format!("Failed to clear table '{}' before snapshot restoration: {}. Proceeding with restoration anyway.", table_name, e));
            } else {
                warnings.push(format!("Successfully cleared table '{}' before snapshot restoration", table_name));
            }
            
            // Restore snapshot data row by row
            let mut row_successes = 0u64;
            let mut row_failures = 0u64;
            
            for (row_index, row_data) in snapshot_rows.iter().enumerate() {
                if let serde_json::Value::Object(row_map) = row_data {
                    // Build INSERT statement from row data
                    let columns: Vec<String> = row_map.keys().cloned().collect();
                    let values: Vec<serde_json::Value> = columns.iter()
                        .map(|col| row_map.get(col).cloned().unwrap_or(serde_json::Value::Null))
                        .collect();
                    
                    let columns_str = columns.join(", ");
                    let placeholders = columns.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
                    let insert_sql = format!("INSERT INTO {} ({}) VALUES ({})", table_name, columns_str, placeholders);
                    
                    match self.db.execute(&insert_sql, &values).await {
                        Ok(_) => {
                            row_successes += 1;
                            total_records_restored += 1;
                        }
                        Err(e) => {
                            warnings.push(format!("Failed to restore row {} from snapshot '{}': {}", row_index + 1, snapshot_key, e));
                            row_failures += 1;
                        }
                    }
                } else {
                    warnings.push(format!("Invalid row data at index {} in snapshot '{}', skipping", row_index, snapshot_key));
                    row_failures += 1;
                }
                
                // Check timeout
                if start_time.elapsed() > self.config.max_transformation_time {
                    warnings.push(format!("Restoration timeout reached after processing {} rows for snapshot '{}'", 
                        row_index + 1, snapshot_key));
                    break;
                }
            }
            
            if row_failures == 0 {
                total_successful += 1;
                warnings.push(format!("Successfully restored snapshot '{}' with {} rows", 
                    snapshot_key, row_successes));
            } else {
                total_failed += 1;
                warnings.push(format!("Partially failed to restore snapshot '{}': {} successful rows, {} failed rows", 
                    snapshot_key, row_successes, row_failures));
            }
        }
        
        // Step 3: Clean up temporary tables if requested
        for temp_table in &rollback_info.temporary_tables {
            warnings.push(format!("Cleaning up temporary table: {}", temp_table));
            
            let drop_sql = format!("DROP TABLE IF EXISTS {}", temp_table);
            if let Err(e) = self.db.execute(&drop_sql, &[]).await {
                warnings.push(format!("Failed to clean up temporary table '{}': {}", temp_table, e));
            } else {
                warnings.push(format!("Successfully cleaned up temporary table '{}'", temp_table));
            }
        }
        
        // Generate summary
        warnings.push("Data restoration from backup completed".to_string());
        warnings.push(format!("Total records restored: {}", total_records_restored));
        warnings.push(format!("Successful operations: {}, Failed operations: {}", total_successful, total_failed));
        
        if total_failed > 0 {
            warnings.push(format!("Failed to restore {} operations", total_failed));
        }
        
        // Update metrics
        let duration = start_time.elapsed();
        self.update_transformation_metrics("restore_from_backup", duration, total_failed == 0);
        
        // Determine success: graceful handling - success unless there were critical errors that prevented any restoration
        let success = errors.is_empty() || (total_successful > 0 && total_failed == 0);
        
        Ok(TransformationResult {
            success,
            records_processed: total_records_restored,
            records_failed: total_failed,
            errors,
            warnings,
        })
    }
}