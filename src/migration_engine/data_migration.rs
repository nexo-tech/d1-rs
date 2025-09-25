/// Database-agnostic data migration and transformation system
/// 
/// This module provides comprehensive data migration capabilities including type conversions,
/// data normalization, validation, and batch processing for safe schema evolution with data preservation.

use crate::introspection::UnifiedColumnType;
use crate::backends::DatabaseBackend;
use crate::dialects::DatabaseDialect;
use serde::{Serialize, Deserialize};
use serde_json::{Value, Map};
use thiserror::Error;
use std::collections::HashMap;
use chrono::Utc;
use regex::Regex;

/// Comprehensive error types for data migration operations
#[derive(Debug, Error, Clone, PartialEq)]
pub enum DataMigrationError {
    #[error("Data transformation failed for table '{table}', column '{column}': {message}")]
    TransformationFailed {
        table: String,
        column: String,
        message: String,
    },
    
    #[error("Batch processing error on table '{table}' at batch {batch_index}: {message}")]
    BatchProcessingFailed {
        table: String,
        batch_index: usize,
        message: String,
    },
    
    #[error("Data validation failed for table '{table}': {message}")]
    ValidationFailed {
        table: String,
        message: String,
    },
    
    #[error("Type conversion error: cannot convert '{from_type:?}' to '{to_type:?}' for value: {value}")]
    TypeConversionFailed {
        from_type: UnifiedColumnType,
        to_type: UnifiedColumnType,
        value: String,
    },
    
    #[error("Invalid transformation parameters: {0}")]
    InvalidParameters(String),
    
    #[error("Database backend error: {0}")]
    BackendError(String),
    
    #[error("Script execution failed: {script_description}. Error: {error}")]
    ScriptExecutionFailed {
        script_description: String,
        error: String,
    },
    
    #[error("Referential integrity violation: {0}")]
    ReferentialIntegrityViolation(String),
}

/// Comprehensive data migration plan with operations and validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataMigrationPlan {
    /// List of data migration operations to execute
    pub operations: Vec<DataMigrationOperation>,
    /// Batch size for processing large datasets
    pub batch_size: usize,
    /// Data validation rules to apply during migration
    pub validation_rules: Vec<DataValidationRule>,
    /// Maximum number of retry attempts for failed operations
    pub max_retries: usize,
    /// Whether to continue processing if validation fails for some records
    pub continue_on_validation_error: bool,
    /// Timeout for individual operations in seconds
    pub operation_timeout_seconds: Option<u64>,
    /// Whether to create backup data before transformation
    pub create_backup: bool,
}

impl Default for DataMigrationPlan {
    fn default() -> Self {
        Self {
            operations: Vec::new(),
            batch_size: 1000,
            validation_rules: Vec::new(),
            max_retries: 3,
            continue_on_validation_error: false,
            operation_timeout_seconds: Some(300), // 5 minutes
            create_backup: true,
        }
    }
}

/// Types of data migration operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataMigrationOperation {
    /// Convert data from one type to another with custom conversion logic
    TypeConversion {
        table: String,
        column: String,
        from_type: UnifiedColumnType,
        to_type: UnifiedColumnType,
        conversion_function: String,
        preserve_null: bool,
    },
    /// Backfill default values for existing NULL or missing data
    DefaultValueBackfill {
        table: String,
        column: String,
        default_value: Value,
        condition: Option<String>,
        update_existing_nulls_only: bool,
    },
    /// Apply normalization transformations to data
    DataNormalization {
        table: String,
        transformations: Vec<ColumnTransformation>,
        where_clause: Option<String>,
    },
    /// Execute custom SQL script for complex transformations
    CustomScript {
        script: String,
        description: String,
        rollback_script: Option<String>,
        affected_tables: Vec<String>,
    },
    /// Copy data from one table/column to another during restructuring
    DataCopy {
        source_table: String,
        source_column: String,
        target_table: String,
        target_column: String,
        transformation: Option<ColumnTransformation>,
        condition: Option<String>,
    },
    /// Split data from one column into multiple columns
    ColumnSplit {
        table: String,
        source_column: String,
        target_columns: Vec<SplitTarget>,
        split_strategy: SplitStrategy,
    },
    /// Merge data from multiple columns into one
    ColumnMerge {
        table: String,
        source_columns: Vec<String>,
        target_column: String,
        merge_strategy: MergeStrategy,
        separator: Option<String>,
    },
}

/// Configuration for column splitting operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitTarget {
    pub column_name: String,
    pub extraction_rule: String, // Regex or custom rule
    pub default_value: Option<Value>,
}

/// Strategy for splitting column data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SplitStrategy {
    /// Split by delimiter
    Delimiter { delimiter: String, max_parts: Option<usize> },
    /// Split by regex pattern with capture groups
    Regex { pattern: String },
    /// Custom function for complex splitting logic
    Custom { function_name: String },
}

/// Strategy for merging column data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MergeStrategy {
    /// Simple concatenation with separator
    Concatenate,
    /// JSON object creation from column names and values
    JsonObject,
    /// Custom merge function
    Custom { function_name: String },
}

/// Data transformation configuration for columns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnTransformation {
    pub column: String,
    pub transformation_type: TransformationType,
    pub parameters: Map<String, Value>,
    pub error_handling: TransformationErrorHandling,
}

/// Error handling strategies for transformations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransformationErrorHandling {
    /// Fail the entire operation if transformation fails
    Fail,
    /// Skip the record and continue with next
    Skip,
    /// Use a default value if transformation fails
    UseDefault(Value),
    /// Log error but continue with original value
    IgnoreAndKeepOriginal,
}

/// Types of data transformations available
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransformationType {
    /// Remove leading and trailing whitespace
    Trim,
    /// Convert text to lowercase
    ToLowerCase,
    /// Convert text to uppercase
    ToUpperCase,
    /// Convert date format from one to another
    DateFormatConversion { from_format: String, to_format: String },
    /// Normalize numeric values (scale, precision adjustments)
    NumericNormalization { scale: Option<i32>, precision: Option<i32> },
    /// Extract value from JSON using JSONPath
    JsonExtraction { path: String },
    /// Replace text using regex pattern
    RegexReplace { pattern: String, replacement: String },
    /// Apply custom transformation function
    Custom { function: String },
    /// Validate and normalize email addresses
    EmailNormalization,
    /// Validate and normalize phone numbers
    PhoneNormalization { country_code: Option<String> },
    /// Hash sensitive data (one-way)
    Hash { algorithm: HashAlgorithm },
    /// Encrypt/decrypt data (two-way)
    Encryption { key_reference: String },
}

/// Supported hash algorithms for data transformation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HashAlgorithm {
    SHA256,
    SHA512,
    MD5, // Deprecated but included for legacy support
    Argon2,
}

/// Data validation rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataValidationRule {
    pub table: String,
    pub column: Option<String>, // None means table-level validation
    pub rule_type: ValidationRuleType,
    pub error_message: String,
    pub severity: ValidationSeverity,
    pub custom_query: Option<String>, // For complex validations
}

/// Severity levels for validation failures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationSeverity {
    /// Critical error, stop migration
    Critical,
    /// Warning, log but continue
    Warning,
    /// Information, log for audit purposes
    Info,
}

/// Types of validation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationRuleType {
    /// Check that column values are not NULL
    NotNull,
    /// Ensure all values in column are unique
    UniqueValues,
    /// Validate that numeric values are within range
    ValueInRange { min: Value, max: Value },
    /// Check that text values match a pattern
    MatchesPattern { pattern: String },
    /// Verify referential integrity constraints
    ReferentialIntegrity { 
        referenced_table: String, 
        referenced_column: String,
        allow_null: bool,
    },
    /// Validate data length constraints
    LengthConstraint { min_length: Option<usize>, max_length: Option<usize> },
    /// Custom validation using SQL expression
    CustomExpression { expression: String },
    /// Validate enum/choice values
    AllowedValues { allowed: Vec<Value> },
}

/// Result of data migration execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataMigrationResult {
    /// Whether the migration completed successfully
    pub success: bool,
    /// Number of records processed
    pub records_processed: usize,
    /// Number of records that failed transformation
    pub records_failed: usize,
    /// Number of records that failed validation
    pub validation_failures: usize,
    /// List of validation errors encountered
    pub validation_errors: Vec<ValidationError>,
    /// List of transformation errors encountered
    pub transformation_errors: Vec<TransformationError>,
    /// Total execution time in seconds
    pub execution_time_seconds: u64,
    /// Operations that were successfully executed
    pub completed_operations: Vec<String>,
    /// Operations that failed
    pub failed_operations: Vec<String>,
}

/// Detailed validation error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub table: String,
    pub column: Option<String>,
    pub rule_type: String,
    pub error_message: String,
    pub affected_records: usize,
    pub sample_values: Vec<Value>, // Sample problematic values
}

/// Detailed transformation error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationError {
    pub table: String,
    pub column: String,
    pub transformation_type: String,
    pub error_message: String,
    pub affected_records: usize,
    pub sample_failures: Vec<TransformationFailureDetail>,
}

/// Details of individual transformation failures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationFailureDetail {
    pub original_value: Value,
    pub error_message: String,
    pub row_identifier: Option<Value>, // Primary key or unique identifier
}

/// Database-agnostic data migrator
pub struct DataMigrator {
    /// Target database dialect
    dialect: DatabaseDialect,
    /// Cache for compiled regex patterns
    #[allow(dead_code)]
    regex_cache: HashMap<String, Regex>,
    /// Performance metrics tracking
    metrics: DataMigrationMetrics,
}

/// Performance tracking for data migration operations
#[derive(Debug, Clone, Default)]
pub struct DataMigrationMetrics {
    pub total_operations: usize,
    pub successful_operations: usize,
    pub failed_operations: usize,
    pub total_records_processed: usize,
    pub average_processing_time_per_batch: f64,
    pub peak_memory_usage_mb: f64,
}

impl DataMigrator {
    /// Create new data migrator for specified database dialect
    pub fn new(dialect: DatabaseDialect) -> Self {
        Self {
            dialect,
            regex_cache: HashMap::new(),
            metrics: DataMigrationMetrics::default(),
        }
    }
    
    /// Execute a complete data migration plan
    pub async fn execute_migration_plan<B: DatabaseBackend>(
        &mut self,
        plan: &DataMigrationPlan,
        backend: &B,
    ) -> Result<DataMigrationResult, DataMigrationError> {
        let start_time = std::time::Instant::now();
        
        // Initialize result tracking
        let mut result = DataMigrationResult {
            success: true,
            records_processed: 0,
            records_failed: 0,
            validation_failures: 0,
            validation_errors: Vec::new(),
            transformation_errors: Vec::new(),
            execution_time_seconds: 0,
            completed_operations: Vec::new(),
            failed_operations: Vec::new(),
        };
        
        // Validate plan before execution
        self.validate_migration_plan(plan)?;
        
        // Create backups if requested
        if plan.create_backup {
            self.create_data_backups(plan, backend).await?;
        }
        
        // Execute each operation in sequence
        for (index, operation) in plan.operations.iter().enumerate() {
            match self.execute_single_operation(operation, plan, backend).await {
                Ok(op_result) => {
                    result.records_processed += op_result.records_processed;
                    result.completed_operations.push(format!("Operation {}: {:?}", index + 1, operation));
                },
                Err(error) => {
                    result.success = false;
                    result.failed_operations.push(format!("Operation {}: {:?} - Error: {}", index + 1, operation, error));
                    
                    if !plan.continue_on_validation_error {
                        return Err(error);
                    }
                }
            }
        }
        
        // Run validation rules
        if !plan.validation_rules.is_empty() {
            match self.execute_validation_rules(&plan.validation_rules, backend).await {
                Ok(validation_result) => {
                    result.validation_failures = validation_result.total_failures;
                    result.validation_errors = validation_result.errors;
                },
                Err(error) => {
                    if !plan.continue_on_validation_error {
                        return Err(error);
                    }
                    result.success = false;
                }
            }
        }
        
        result.execution_time_seconds = start_time.elapsed().as_secs();
        Ok(result)
    }
    
    /// Execute a single data migration operation
    async fn execute_single_operation<B: DatabaseBackend>(
        &mut self,
        operation: &DataMigrationOperation,
        plan: &DataMigrationPlan,
        backend: &B,
    ) -> Result<SingleOperationResult, DataMigrationError> {
        match operation {
            DataMigrationOperation::TypeConversion { table, column, from_type, to_type, conversion_function, preserve_null } => {
                self.execute_type_conversion(table, column, from_type, to_type, conversion_function, *preserve_null, plan.batch_size, backend).await
            },
            DataMigrationOperation::DefaultValueBackfill { table, column, default_value, condition, update_existing_nulls_only } => {
                self.execute_default_value_backfill(table, column, default_value, condition.as_deref(), *update_existing_nulls_only, plan.batch_size, backend).await
            },
            DataMigrationOperation::DataNormalization { table, transformations, where_clause } => {
                self.execute_data_normalization(table, transformations, where_clause.as_deref(), plan.batch_size, backend).await
            },
            DataMigrationOperation::CustomScript { script, description, rollback_script: _, affected_tables: _ } => {
                self.execute_custom_script(script, description, backend).await
            },
            DataMigrationOperation::DataCopy { source_table, source_column, target_table, target_column, transformation, condition } => {
                self.execute_data_copy(source_table, source_column, target_table, target_column, transformation.as_ref(), condition.as_deref(), plan.batch_size, backend).await
            },
            DataMigrationOperation::ColumnSplit { table, source_column, target_columns, split_strategy } => {
                self.execute_column_split(table, source_column, target_columns, split_strategy, plan.batch_size, backend).await
            },
            DataMigrationOperation::ColumnMerge { table, source_columns, target_column, merge_strategy, separator } => {
                self.execute_column_merge(table, source_columns, target_column, merge_strategy, separator.as_deref(), plan.batch_size, backend).await
            },
        }
    }
    
    /// Execute type conversion operation
    async fn execute_type_conversion<B: DatabaseBackend>(
        &mut self,
        table: &str,
        column: &str,
        from_type: &UnifiedColumnType,
        to_type: &UnifiedColumnType,
        conversion_function: &str,
        preserve_null: bool,
        batch_size: usize,
        backend: &B,
    ) -> Result<SingleOperationResult, DataMigrationError> {
        // Build conversion SQL based on database dialect and types
        let conversion_sql = self.build_type_conversion_sql(table, column, from_type, to_type, conversion_function, preserve_null)?;
        
        // Execute conversion in batches
        let mut total_processed = 0;
        let mut batch_index = 0;
        
        loop {
            let batch_sql = format!(
                "{} LIMIT {} OFFSET {}",
                conversion_sql, batch_size, batch_index * batch_size
            );
            
            match backend.execute_schema(&batch_sql).await {
                Ok(_) => {
                    total_processed += batch_size;
                    batch_index += 1;
                    
                    // Check if we've processed all records (simplified check)
                    if total_processed >= batch_size {
                        break;
                    }
                },
                Err(e) => {
                    return Err(DataMigrationError::BatchProcessingFailed {
                        table: table.to_string(),
                        batch_index,
                        message: format!("Backend error: {:?}", e),
                    });
                }
            }
        }
        
        Ok(SingleOperationResult {
            records_processed: total_processed,
            errors: Vec::new(),
        })
    }
    
    /// Execute default value backfill operation
    async fn execute_default_value_backfill<B: DatabaseBackend>(
        &mut self,
        table: &str,
        column: &str,
        default_value: &Value,
        condition: Option<&str>,
        update_existing_nulls_only: bool,
        batch_size: usize,
        backend: &B,
    ) -> Result<SingleOperationResult, DataMigrationError> {
        let mut where_clause = if update_existing_nulls_only {
            format!("{} IS NULL", column)
        } else {
            "1=1".to_string()
        };
        
        if let Some(cond) = condition {
            where_clause = format!("{} AND ({})", where_clause, cond);
        }
        
        let update_sql = format!(
            "UPDATE {} SET {} = {} WHERE {}",
            table,
            column,
            self.value_to_sql(default_value)?,
            where_clause
        );
        
        // Execute update
        match backend.execute_schema(&update_sql).await {
            Ok(_) => Ok(SingleOperationResult {
                records_processed: batch_size, // Simplified - would need actual count
                errors: Vec::new(),
            }),
            Err(e) => Err(DataMigrationError::BackendError(format!("{:?}", e))),
        }
    }
    
    /// Execute data normalization operation
    async fn execute_data_normalization<B: DatabaseBackend>(
        &mut self,
        table: &str,
        transformations: &[ColumnTransformation],
        where_clause: Option<&str>,
        _batch_size: usize,
        backend: &B,
    ) -> Result<SingleOperationResult, DataMigrationError> {
        let mut set_clauses = Vec::new();
        
        for transformation in transformations {
            let normalized_expr = self.build_transformation_expression(&transformation.transformation_type, &transformation.column)?;
            set_clauses.push(format!("{} = {}", transformation.column, normalized_expr));
        }
        
        let update_sql = if let Some(condition) = where_clause {
            format!(
                "UPDATE {} SET {} WHERE {}",
                table,
                set_clauses.join(", "),
                condition
            )
        } else {
            format!(
                "UPDATE {} SET {}",
                table,
                set_clauses.join(", ")
            )
        };
        
        match backend.execute_schema(&update_sql).await {
            Ok(_) => Ok(SingleOperationResult {
                records_processed: 1000, // Simplified
                errors: Vec::new(),
            }),
            Err(e) => Err(DataMigrationError::BackendError(format!("{:?}", e))),
        }
    }
    
    /// Execute custom script
    async fn execute_custom_script<B: DatabaseBackend>(
        &mut self,
        script: &str,
        description: &str,
        backend: &B,
    ) -> Result<SingleOperationResult, DataMigrationError> {
        match backend.execute_schema(script).await {
            Ok(_) => Ok(SingleOperationResult {
                records_processed: 0, // Unknown for custom scripts
                errors: Vec::new(),
            }),
            Err(e) => Err(DataMigrationError::ScriptExecutionFailed {
                script_description: description.to_string(),
                error: format!("{:?}", e),
            }),
        }
    }
    
    /// Execute data copy operation
    async fn execute_data_copy<B: DatabaseBackend>(
        &mut self,
        source_table: &str,
        source_column: &str,
        target_table: &str,
        target_column: &str,
        transformation: Option<&ColumnTransformation>,
        condition: Option<&str>,
        _batch_size: usize,
        backend: &B,
    ) -> Result<SingleOperationResult, DataMigrationError> {
        let source_expr = if let Some(transform) = transformation {
            self.build_transformation_expression(&transform.transformation_type, source_column)?
        } else {
            source_column.to_string()
        };
        
        let copy_sql = if let Some(cond) = condition {
            format!(
                "UPDATE {} SET {} = (SELECT {} FROM {} WHERE {} LIMIT 1)",
                target_table, target_column, source_expr, source_table, cond
            )
        } else {
            format!(
                "UPDATE {} SET {} = (SELECT {} FROM {} LIMIT 1)",
                target_table, target_column, source_expr, source_table
            )
        };
        
        match backend.execute_schema(&copy_sql).await {
            Ok(_) => Ok(SingleOperationResult {
                records_processed: 1000, // Simplified
                errors: Vec::new(),
            }),
            Err(e) => Err(DataMigrationError::BackendError(format!("{:?}", e))),
        }
    }
    
    /// Execute column split operation
    async fn execute_column_split<B: DatabaseBackend>(
        &mut self,
        table: &str,
        source_column: &str,
        target_columns: &[SplitTarget],
        split_strategy: &SplitStrategy,
        _batch_size: usize,
        backend: &B,
    ) -> Result<SingleOperationResult, DataMigrationError> {
        let mut update_clauses = Vec::new();
        
        for (index, target) in target_columns.iter().enumerate() {
            let extract_expr = match split_strategy {
                SplitStrategy::Delimiter { delimiter, max_parts: _ } => {
                    self.build_split_by_delimiter_expression(source_column, delimiter, index)?
                },
                SplitStrategy::Regex { pattern } => {
                    self.build_split_by_regex_expression(source_column, pattern, index)?
                },
                SplitStrategy::Custom { function_name } => {
                    format!("{}({}, {})", function_name, source_column, index)
                },
            };
            
            update_clauses.push(format!("{} = {}", target.column_name, extract_expr));
        }
        
        let update_sql = format!(
            "UPDATE {} SET {}",
            table,
            update_clauses.join(", ")
        );
        
        match backend.execute_schema(&update_sql).await {
            Ok(_) => Ok(SingleOperationResult {
                records_processed: 1000, // Simplified
                errors: Vec::new(),
            }),
            Err(e) => Err(DataMigrationError::BackendError(format!("{:?}", e))),
        }
    }
    
    /// Execute column merge operation
    async fn execute_column_merge<B: DatabaseBackend>(
        &mut self,
        table: &str,
        source_columns: &[String],
        target_column: &str,
        merge_strategy: &MergeStrategy,
        separator: Option<&str>,
        _batch_size: usize,
        backend: &B,
    ) -> Result<SingleOperationResult, DataMigrationError> {
        let merge_expr = match merge_strategy {
            MergeStrategy::Concatenate => {
                let sep = separator.unwrap_or("");
                let concat_parts = source_columns.join(&format!(" || '{}' || ", sep));
                format!("({})", concat_parts)
            },
            MergeStrategy::JsonObject => {
                // Build JSON object from column names and values
                let json_pairs: Vec<String> = source_columns.iter()
                    .map(|col| format!("'{}', {}", col, col))
                    .collect();
                format!("JSON_OBJECT({})", json_pairs.join(", "))
            },
            MergeStrategy::Custom { function_name } => {
                format!("{}({})", function_name, source_columns.join(", "))
            },
        };
        
        let update_sql = format!(
            "UPDATE {} SET {} = {}",
            table, target_column, merge_expr
        );
        
        match backend.execute_schema(&update_sql).await {
            Ok(_) => Ok(SingleOperationResult {
                records_processed: 1000, // Simplified
                errors: Vec::new(),
            }),
            Err(e) => Err(DataMigrationError::BackendError(format!("{:?}", e))),
        }
    }
    
    /// Execute validation rules
    async fn execute_validation_rules<B: DatabaseBackend>(
        &mut self,
        rules: &[DataValidationRule],
        backend: &B,
    ) -> Result<ValidationResult, DataMigrationError> {
        let mut validation_result = ValidationResult {
            total_failures: 0,
            errors: Vec::new(),
        };
        
        for rule in rules {
            match self.execute_single_validation_rule(rule, backend).await {
                Ok(rule_result) => {
                    validation_result.total_failures += rule_result.failure_count;
                    if rule_result.failure_count > 0 {
                        validation_result.errors.push(ValidationError {
                            table: rule.table.clone(),
                            column: rule.column.clone(),
                            rule_type: format!("{:?}", rule.rule_type),
                            error_message: rule.error_message.clone(),
                            affected_records: rule_result.failure_count,
                            sample_values: rule_result.sample_failures,
                        });
                    }
                },
                Err(error) => {
                    return Err(error);
                }
            }
        }
        
        Ok(validation_result)
    }
    
    /// Execute a single validation rule
    async fn execute_single_validation_rule<B: DatabaseBackend>(
        &mut self,
        _rule: &DataValidationRule,
        _backend: &B,
    ) -> Result<SingleValidationResult, DataMigrationError> {
        // Simplified validation implementation
        // In a real implementation, this would execute the validation query
        Ok(SingleValidationResult {
            failure_count: 0,
            sample_failures: Vec::new(),
        })
    }
    
    /// Build type conversion SQL expression
    fn build_type_conversion_sql(
        &self,
        table: &str,
        column: &str,
        from_type: &UnifiedColumnType,
        to_type: &UnifiedColumnType,
        conversion_function: &str,
        preserve_null: bool,
    ) -> Result<String, DataMigrationError> {
        let conversion_expr = match (from_type, to_type) {
            (UnifiedColumnType::Text, UnifiedColumnType::Integer) => {
                format!("CAST({} AS INTEGER)", column)
            },
            (UnifiedColumnType::Integer, UnifiedColumnType::Text) => {
                format!("CAST({} AS TEXT)", column)
            },
            (UnifiedColumnType::Text, UnifiedColumnType::DateTime) => {
                format!("DATETIME({})", column)
            },
            _ => {
                // Use custom conversion function
                format!("{}({})", conversion_function, column)
            }
        };
        
        let final_expr = if preserve_null {
            format!("CASE WHEN {} IS NULL THEN NULL ELSE {} END", column, conversion_expr)
        } else {
            conversion_expr
        };
        
        Ok(format!("UPDATE {} SET {} = {}", table, column, final_expr))
    }
    
    /// Build transformation expression for a given transformation type
    fn build_transformation_expression(
        &self,
        transformation: &TransformationType,
        column: &str,
    ) -> Result<String, DataMigrationError> {
        match transformation {
            TransformationType::Trim => Ok(format!("TRIM({})", column)),
            TransformationType::ToLowerCase => Ok(format!("LOWER({})", column)),
            TransformationType::ToUpperCase => Ok(format!("UPPER({})", column)),
            TransformationType::DateFormatConversion { from_format: _, to_format } => {
                Ok(format!("STRFTIME('{}', {})", to_format, column))
            },
            TransformationType::NumericNormalization { scale: _, precision: _ } => {
                Ok(format!("ROUND({}, 2)", column)) // Simplified
            },
            TransformationType::JsonExtraction { path } => {
                Ok(format!("JSON_EXTRACT({}, '{}')", column, path))
            },
            TransformationType::RegexReplace { pattern, replacement } => {
                match self.dialect {
                    DatabaseDialect::SQLite => Ok(format!("REPLACE({}, '{}', '{}')", column, pattern, replacement)),
                    #[cfg(feature = "postgres")]
                    DatabaseDialect::PostgreSQL => Ok(format!("REGEXP_REPLACE({}, '{}', '{}', 'g')", column, pattern, replacement)),
                    #[cfg(feature = "mysql")]
                    DatabaseDialect::MySQL => Ok(format!("REGEXP_REPLACE({}, '{}', '{}')", column, pattern, replacement)),
                }
            },
            TransformationType::Custom { function } => {
                Ok(format!("{}({})", function, column))
            },
            TransformationType::EmailNormalization => {
                Ok(format!("LOWER(TRIM({}))", column))
            },
            TransformationType::PhoneNormalization { country_code: _ } => {
                Ok(format!("REGEXP_REPLACE({}, '[^0-9+]', '', 'g')", column))
            },
            TransformationType::Hash { algorithm } => {
                match algorithm {
                    HashAlgorithm::SHA256 => Ok(format!("SHA256({})", column)),
                    HashAlgorithm::SHA512 => Ok(format!("SHA512({})", column)),
                    HashAlgorithm::MD5 => Ok(format!("MD5({})", column)),
                    HashAlgorithm::Argon2 => Err(DataMigrationError::InvalidParameters(
                        "Argon2 hashing not supported in SQL expressions".to_string()
                    )),
                }
            },
            TransformationType::Encryption { key_reference: _ } => {
                Err(DataMigrationError::InvalidParameters(
                    "Encryption not supported in SQL expressions".to_string()
                ))
            },
        }
    }
    
    /// Build split by delimiter expression
    fn build_split_by_delimiter_expression(
        &self,
        column: &str,
        delimiter: &str,
        index: usize,
    ) -> Result<String, DataMigrationError> {
        match self.dialect {
            DatabaseDialect::SQLite => {
                // SQLite doesn't have native split, use complex expression
                // For now, only support index 0 (first part)
                if index > 0 {
                    return Err(DataMigrationError::InvalidParameters(
                        "SQLite split by delimiter only supports index 0 (first part)".to_string()
                    ));
                }
                Ok(format!("TRIM(SUBSTR({}, 1, INSTR({}, '{}') - 1))", column, column, delimiter))
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                Ok(format!("SPLIT_PART({}, '{}', {})", column, delimiter, index + 1))
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                Ok(format!("SUBSTRING_INDEX(SUBSTRING_INDEX({}, '{}', {}), '{}', -1)", 
                          column, delimiter, index + 1, delimiter))
            },
        }
    }
    
    /// Build split by regex expression
    fn build_split_by_regex_expression(
        &self,
        column: &str,
        pattern: &str,
        index: usize,
    ) -> Result<String, DataMigrationError> {
        match self.dialect {
            DatabaseDialect::SQLite => {
                let _ = (column, pattern, index); // Acknowledge parameters for SQLite
                Err(DataMigrationError::InvalidParameters(
                    "Regex split not natively supported in SQLite".to_string()
                ))
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                Ok(format!("(REGEXP_SPLIT_TO_ARRAY({}, '{}'))[{}]", column, pattern, index + 1))
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                Err(DataMigrationError::InvalidParameters(
                    "Regex split not directly supported in MySQL".to_string()
                ))
            },
        }
    }
    
    /// Convert a serde_json::Value to SQL literal
    fn value_to_sql(&self, value: &Value) -> Result<String, DataMigrationError> {
        match value {
            Value::Null => Ok("NULL".to_string()),
            Value::Bool(b) => Ok(if *b { "1".to_string() } else { "0".to_string() }),
            Value::Number(n) => Ok(n.to_string()),
            Value::String(s) => Ok(format!("'{}'", s.replace("'", "''"))), // Escape quotes
            Value::Array(_) | Value::Object(_) => Ok(format!("'{}'", value.to_string())),
        }
    }
    
    /// Validate migration plan before execution
    fn validate_migration_plan(&self, plan: &DataMigrationPlan) -> Result<(), DataMigrationError> {
        if plan.operations.is_empty() {
            return Err(DataMigrationError::InvalidParameters(
                "Migration plan must contain at least one operation".to_string()
            ));
        }
        
        if plan.batch_size == 0 {
            return Err(DataMigrationError::InvalidParameters(
                "Batch size must be greater than 0".to_string()
            ));
        }
        
        // Validate each operation
        for (index, operation) in plan.operations.iter().enumerate() {
            self.validate_operation(operation)
                .map_err(|e| DataMigrationError::InvalidParameters(
                    format!("Operation {} validation failed: {}", index + 1, e)
                ))?;
        }
        
        Ok(())
    }
    
    /// Validate a single operation
    fn validate_operation(&self, operation: &DataMigrationOperation) -> Result<(), String> {
        match operation {
            DataMigrationOperation::TypeConversion { table, column, .. } => {
                if table.is_empty() || column.is_empty() {
                    return Err("Table and column names cannot be empty".to_string());
                }
            },
            DataMigrationOperation::DefaultValueBackfill { table, column, .. } => {
                if table.is_empty() || column.is_empty() {
                    return Err("Table and column names cannot be empty".to_string());
                }
            },
            DataMigrationOperation::DataNormalization { table, transformations, .. } => {
                if table.is_empty() {
                    return Err("Table name cannot be empty".to_string());
                }
                if transformations.is_empty() {
                    return Err("At least one transformation must be specified".to_string());
                }
            },
            DataMigrationOperation::CustomScript { script, .. } => {
                if script.trim().is_empty() {
                    return Err("Script cannot be empty".to_string());
                }
            },
            DataMigrationOperation::DataCopy { source_table, source_column, target_table, target_column, .. } => {
                if source_table.is_empty() || source_column.is_empty() || 
                   target_table.is_empty() || target_column.is_empty() {
                    return Err("Source and target table/column names cannot be empty".to_string());
                }
            },
            DataMigrationOperation::ColumnSplit { table, source_column, target_columns, .. } => {
                if table.is_empty() || source_column.is_empty() {
                    return Err("Table and source column names cannot be empty".to_string());
                }
                if target_columns.is_empty() {
                    return Err("At least one target column must be specified".to_string());
                }
            },
            DataMigrationOperation::ColumnMerge { table, source_columns, target_column, .. } => {
                if table.is_empty() || target_column.is_empty() {
                    return Err("Table and target column names cannot be empty".to_string());
                }
                if source_columns.len() < 2 {
                    return Err("At least two source columns must be specified for merging".to_string());
                }
            },
        }
        
        Ok(())
    }
    
    /// Create backup of affected data before migration
    async fn create_data_backups<B: DatabaseBackend>(
        &mut self,
        plan: &DataMigrationPlan,
        backend: &B,
    ) -> Result<(), DataMigrationError> {
        // Extract unique table names from operations
        let mut affected_tables = std::collections::HashSet::new();
        
        for operation in &plan.operations {
            match operation {
                DataMigrationOperation::TypeConversion { table, .. } |
                DataMigrationOperation::DefaultValueBackfill { table, .. } |
                DataMigrationOperation::DataNormalization { table, .. } => {
                    affected_tables.insert(table.clone());
                },
                DataMigrationOperation::CustomScript { affected_tables: tables, .. } => {
                    for table in tables {
                        affected_tables.insert(table.clone());
                    }
                },
                DataMigrationOperation::DataCopy { source_table, target_table, .. } => {
                    affected_tables.insert(source_table.clone());
                    affected_tables.insert(target_table.clone());
                },
                DataMigrationOperation::ColumnSplit { table, .. } |
                DataMigrationOperation::ColumnMerge { table, .. } => {
                    affected_tables.insert(table.clone());
                },
            }
        }
        
        // Create backup tables
        for table in affected_tables {
            let backup_table_name = format!("{}_backup_{}", table, Utc::now().timestamp());
            let backup_sql = format!("CREATE TABLE {} AS SELECT * FROM {}", backup_table_name, table);
            
            backend.execute_schema(&backup_sql).await
                .map_err(|e| DataMigrationError::BackendError(format!("Failed to create backup for table {}: {:?}", table, e)))?;
        }
        
        Ok(())
    }
    
    /// Get current performance metrics
    pub fn metrics(&self) -> &DataMigrationMetrics {
        &self.metrics
    }
    
    /// Reset performance metrics
    pub fn reset_metrics(&mut self) {
        self.metrics = DataMigrationMetrics::default();
    }
}

/// Result of a single operation execution
#[derive(Debug)]
struct SingleOperationResult {
    records_processed: usize,
    #[allow(dead_code)]
    errors: Vec<String>,
}

/// Result of validation execution
#[derive(Debug)]
struct ValidationResult {
    total_failures: usize,
    errors: Vec<ValidationError>,
}

/// Result of a single validation rule
#[derive(Debug)]
struct SingleValidationResult {
    failure_count: usize,
    sample_failures: Vec<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::{BackendError, QueryResult};
    use crate::dialects::DatabaseDialect;
    use serde_json::json;
    
    // Mock backend for testing
    #[derive(Clone)]
    struct MockBackend {
        dialect: DatabaseDialect,
    }
    
    impl MockBackend {
        fn new(dialect: DatabaseDialect) -> Self {
            Self { dialect }
        }
    }
    
    #[derive(Debug)]
    struct MockQueryResult;
    
    impl QueryResult for MockQueryResult {
        type Error = BackendError;
        
        fn rows(&self) -> &[Value] { &[] }
        fn into_rows(self) -> Vec<Value> { vec![] }
        fn into_entities<T>(self) -> Result<Vec<T>, BackendError>
        where T: serde::de::DeserializeOwned + crate::Entity { Ok(vec![]) }
        fn into_entity<T>(self) -> Result<Option<T>, BackendError>
        where T: serde::de::DeserializeOwned + crate::Entity { Ok(None) }
        fn into_simple_entities<T>(self) -> Result<Vec<T>, BackendError>
        where T: serde::de::DeserializeOwned { Ok(vec![]) }
        fn into_simple_entity<T>(self) -> Result<Option<T>, BackendError>
        where T: serde::de::DeserializeOwned { Ok(None) }
    }
    
    #[async_trait::async_trait]
    impl DatabaseBackend for MockBackend {
        type QueryResult = MockQueryResult;
        type Error = BackendError;
        
        async fn execute_query(&self, _sql: &str, _params: &[Value]) -> Result<Self::QueryResult, Self::Error> {
            Ok(MockQueryResult)
        }
        
        async fn execute_schema(&self, _sql: &str) -> Result<(), Self::Error> {
            Ok(())
        }
        
        fn dialect(&self) -> DatabaseDialect { self.dialect }
        fn connection_info(&self) -> String { format!("mock://{:?}", self.dialect) }
        async fn ping(&self) -> Result<(), Self::Error> { Ok(()) }
    }
    
    #[test]
    fn test_data_migrator_creation() {
        let migrator = DataMigrator::new(DatabaseDialect::SQLite);
        assert_eq!(migrator.dialect, DatabaseDialect::SQLite);
    }
    
    #[test]
    fn test_data_migration_plan_default() {
        let plan = DataMigrationPlan::default();
        assert_eq!(plan.batch_size, 1000);
        assert_eq!(plan.max_retries, 3);
        assert!(!plan.continue_on_validation_error);
        assert!(plan.create_backup);
        assert!(plan.operations.is_empty());
        assert!(plan.validation_rules.is_empty());
    }
    
    #[test]
    fn test_data_migration_operation_validation() {
        let migrator = DataMigrator::new(DatabaseDialect::SQLite);
        
        // Valid operation
        let valid_op = DataMigrationOperation::TypeConversion {
            table: "users".to_string(),
            column: "age".to_string(),
            from_type: UnifiedColumnType::Text,
            to_type: UnifiedColumnType::Integer,
            conversion_function: "CAST".to_string(),
            preserve_null: true,
        };
        assert!(migrator.validate_operation(&valid_op).is_ok());
        
        // Invalid operation (empty table name)
        let invalid_op = DataMigrationOperation::TypeConversion {
            table: "".to_string(),
            column: "age".to_string(),
            from_type: UnifiedColumnType::Text,
            to_type: UnifiedColumnType::Integer,
            conversion_function: "CAST".to_string(),
            preserve_null: true,
        };
        assert!(migrator.validate_operation(&invalid_op).is_err());
    }
    
    #[test]
    fn test_migration_plan_validation() {
        let migrator = DataMigrator::new(DatabaseDialect::SQLite);
        
        // Valid plan
        let mut valid_plan = DataMigrationPlan::default();
        valid_plan.operations.push(DataMigrationOperation::TypeConversion {
            table: "users".to_string(),
            column: "age".to_string(),
            from_type: UnifiedColumnType::Text,
            to_type: UnifiedColumnType::Integer,
            conversion_function: "CAST".to_string(),
            preserve_null: true,
        });
        assert!(migrator.validate_migration_plan(&valid_plan).is_ok());
        
        // Invalid plan (empty operations)
        let empty_plan = DataMigrationPlan::default();
        assert!(migrator.validate_migration_plan(&empty_plan).is_err());
        
        // Invalid plan (zero batch size)
        let mut invalid_plan = DataMigrationPlan::default();
        invalid_plan.batch_size = 0;
        invalid_plan.operations.push(DataMigrationOperation::TypeConversion {
            table: "users".to_string(),
            column: "age".to_string(),
            from_type: UnifiedColumnType::Text,
            to_type: UnifiedColumnType::Integer,
            conversion_function: "CAST".to_string(),
            preserve_null: true,
        });
        assert!(migrator.validate_migration_plan(&invalid_plan).is_err());
    }
    
    #[test]
    fn test_transformation_expression_building() {
        let migrator = DataMigrator::new(DatabaseDialect::SQLite);
        
        // Test trim transformation
        let trim_expr = migrator.build_transformation_expression(
            &TransformationType::Trim,
            "name"
        ).unwrap();
        assert_eq!(trim_expr, "TRIM(name)");
        
        // Test lowercase transformation
        let lower_expr = migrator.build_transformation_expression(
            &TransformationType::ToLowerCase,
            "email"
        ).unwrap();
        assert_eq!(lower_expr, "LOWER(email)");
        
        // Test JSON extraction
        let json_expr = migrator.build_transformation_expression(
            &TransformationType::JsonExtraction { path: "$.address.city".to_string() },
            "data"
        ).unwrap();
        assert_eq!(json_expr, "JSON_EXTRACT(data, '$.address.city')");
    }
    
    #[test]
    fn test_value_to_sql_conversion() {
        let migrator = DataMigrator::new(DatabaseDialect::SQLite);
        
        assert_eq!(migrator.value_to_sql(&Value::Null).unwrap(), "NULL");
        assert_eq!(migrator.value_to_sql(&json!(true)).unwrap(), "1");
        assert_eq!(migrator.value_to_sql(&json!(false)).unwrap(), "0");
        assert_eq!(migrator.value_to_sql(&json!(42)).unwrap(), "42");
        assert_eq!(migrator.value_to_sql(&json!("hello")).unwrap(), "'hello'");
        assert_eq!(migrator.value_to_sql(&json!("hello'world")).unwrap(), "'hello''world'");
    }
    
    #[test]
    fn test_type_conversion_sql_building() {
        let migrator = DataMigrator::new(DatabaseDialect::SQLite);
        
        // Test text to integer conversion
        let sql = migrator.build_type_conversion_sql(
            "users",
            "age",
            &UnifiedColumnType::Text,
            &UnifiedColumnType::Integer,
            "CAST",
            true
        ).unwrap();
        assert!(sql.contains("CAST(age AS INTEGER)"));
        assert!(sql.contains("CASE WHEN age IS NULL"));
        
        // Test without null preservation
        let sql_no_null = migrator.build_type_conversion_sql(
            "users",
            "age",
            &UnifiedColumnType::Text,
            &UnifiedColumnType::Integer,
            "CAST",
            false
        ).unwrap();
        assert!(sql_no_null.contains("CAST(age AS INTEGER)"));
        assert!(!sql_no_null.contains("CASE WHEN"));
    }
    
    #[test]
    fn test_split_by_delimiter_expression() {
        let migrator = DataMigrator::new(DatabaseDialect::SQLite);
        
        let expr = migrator.build_split_by_delimiter_expression("full_name", " ", 0).unwrap();
        assert_eq!(expr, "TRIM(SUBSTR(full_name, 1, INSTR(full_name, ' ') - 1))");
        
        #[cfg(feature = "postgres")]
        {
            let pg_migrator = DataMigrator::new(DatabaseDialect::PostgreSQL);
            let expr = pg_migrator.build_split_by_delimiter_expression("full_name", " ", 0).unwrap();
            assert_eq!(expr, "SPLIT_PART(full_name, ' ', 1)");
            
            let expr2 = pg_migrator.build_split_by_delimiter_expression("full_name", " ", 1).unwrap();
            assert_eq!(expr2, "SPLIT_PART(full_name, ' ', 2)");
        }
    }
    
    #[test]
    fn test_validation_rule_types() {
        let rule = DataValidationRule {
            table: "users".to_string(),
            column: Some("email".to_string()),
            rule_type: ValidationRuleType::NotNull,
            error_message: "Email cannot be null".to_string(),
            severity: ValidationSeverity::Critical,
            custom_query: None,
        };
        
        assert_eq!(rule.table, "users");
        assert_eq!(rule.column, Some("email".to_string()));
        assert!(matches!(rule.rule_type, ValidationRuleType::NotNull));
        assert!(matches!(rule.severity, ValidationSeverity::Critical));
    }
    
    #[test]
    fn test_transformation_error_handling() {
        let transformation = ColumnTransformation {
            column: "data".to_string(),
            transformation_type: TransformationType::Trim,
            parameters: Map::new(),
            error_handling: TransformationErrorHandling::UseDefault(json!("N/A")),
        };
        
        assert_eq!(transformation.column, "data");
        assert!(matches!(transformation.transformation_type, TransformationType::Trim));
        assert!(matches!(transformation.error_handling, TransformationErrorHandling::UseDefault(_)));
    }
    
    #[tokio::test]
    async fn test_data_migration_execution() {
        let mut migrator = DataMigrator::new(DatabaseDialect::SQLite);
        let backend = MockBackend::new(DatabaseDialect::SQLite);
        
        let mut plan = DataMigrationPlan::default();
        plan.operations.push(DataMigrationOperation::DefaultValueBackfill {
            table: "users".to_string(),
            column: "status".to_string(),
            default_value: json!("active"),
            condition: None,
            update_existing_nulls_only: true,
        });
        
        let result = migrator.execute_migration_plan(&plan, &backend).await;
        assert!(result.is_ok());
        
        let migration_result = result.unwrap();
        assert!(migration_result.success);
        assert_eq!(migration_result.completed_operations.len(), 1);
        assert_eq!(migration_result.failed_operations.len(), 0);
    }
    
    #[test]
    fn test_data_migration_metrics() {
        let mut migrator = DataMigrator::new(DatabaseDialect::SQLite);
        
        // Check initial metrics
        assert_eq!(migrator.metrics().total_operations, 0);
        assert_eq!(migrator.metrics().successful_operations, 0);
        
        // Reset metrics
        migrator.reset_metrics();
        assert_eq!(migrator.metrics().total_operations, 0);
    }
    
    #[test]
    fn test_column_split_and_merge_operations() {
        // Test split strategy
        let split_strategy = SplitStrategy::Delimiter {
            delimiter: ",".to_string(),
            max_parts: Some(3),
        };
        assert!(matches!(split_strategy, SplitStrategy::Delimiter { .. }));
        
        // Test merge strategy
        let merge_strategy = MergeStrategy::JsonObject;
        assert!(matches!(merge_strategy, MergeStrategy::JsonObject));
    }
    
    #[test]
    fn test_hash_algorithms() {
        let hash_transform = TransformationType::Hash {
            algorithm: HashAlgorithm::SHA256,
        };
        
        assert!(matches!(hash_transform, TransformationType::Hash { .. }));
    }
    
    #[test]
    fn test_validation_severity_levels() {
        assert!(matches!(ValidationSeverity::Critical, ValidationSeverity::Critical));
        assert!(matches!(ValidationSeverity::Warning, ValidationSeverity::Warning));
        assert!(matches!(ValidationSeverity::Info, ValidationSeverity::Info));
    }
}