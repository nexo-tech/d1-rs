// Value mapping functionality for data migration

use crate::{Result, D1RsError};
use crate::auto_migration::data_migration::core::{DataMigrator, TransformationResult};
use std::collections::HashMap;
use std::time::Instant;

impl DataMigrator {
    /// Execute value mapping transformation
    pub async fn execute_value_mapping(
        &self,
        table: &str,
        old_column: &str,
        new_column: &str,
        mapping_table: &HashMap<String, String>,
        default_value: &Option<String>,
    ) -> Result<TransformationResult> {
        let start_time = Instant::now();
        let mut warnings = Vec::new();
        let mut errors = Vec::new();
        
        // Validate value mapping parameters
        if mapping_table.is_empty() && default_value.is_none() {
            warnings.push("No mapping table provided and no default value specified".to_string());
            return Ok(TransformationResult {
                success: true,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        if old_column.is_empty() || new_column.is_empty() {
            errors.push(D1RsError::ValidationError("Source and target columns cannot be empty for value mapping".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Get total record count - handle missing table gracefully
        let count_sql = format!("SELECT COUNT(*) FROM {} WHERE {} IS NOT NULL", table, old_column);
        let total_records = match self.db.execute_returning_count(&count_sql, &[]).await {
            Ok(count) => count as u64,
            Err(_) => {
                // Table doesn't exist or other error - treat as success with 0 records
                warnings.push(format!("Table {} not accessible, skipping value mapping", table));
                return Ok(TransformationResult {
                    success: true,
                    records_processed: 0,
                    records_failed: 0,
                    errors,
                    warnings,
                });
            }
        };
        
        if total_records == 0 {
            warnings.push("No records to map values for".to_string());
            return Ok(TransformationResult {
                success: true,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Process records in batches
        let batch_size = self.config.batch_size;
        let mut total_processed = 0u64;
        let mut total_failed = 0u64;
        
        let mut offset = 0u64;
        while offset < total_records {
            let mapping_sql = self.generate_value_mapping_sql(
                table,
                old_column,
                new_column,
                mapping_table,
                default_value,
                batch_size,
                offset,
            );
            
            match self.db.execute(&mapping_sql, &[]).await {
                Ok(_result) => {
                    let batch_processed = std::cmp::min(batch_size as u64, total_records - offset);
                    total_processed += batch_processed;
                }
                Err(e) => {
                    errors.push(D1RsError::AutoMigration(format!("Value mapping failed for batch at offset {}: {}", offset, e)));
                    total_failed += std::cmp::min(batch_size as u64, total_records - offset);
                }
            }
            
            offset += batch_size as u64;
        }
        
        // Update metrics
        let duration = start_time.elapsed();
        self.update_transformation_metrics("value_mapping", duration, total_failed == 0);
        
        Ok(TransformationResult {
            success: total_failed == 0,
            records_processed: total_processed,
            records_failed: total_failed,
            errors,
            warnings,
        })
    }
    
    /// Generate SQL for value mapping operation
    fn generate_value_mapping_sql(
        &self,
        table: &str,
        old_column: &str,
        new_column: &str,
        mapping_table: &HashMap<String, String>,
        default_value: &Option<String>,
        batch_size: usize,
        offset: u64,
    ) -> String {
        // Build the CASE expression for mapping values
        let mut case_expr = String::from("CASE");
        
        // Add mapping cases
        for (old_value, new_value) in mapping_table {
            // Escape single quotes in SQL string literals
            let escaped_old = old_value.replace("'", "''");
            let escaped_new = new_value.replace("'", "''");
            case_expr.push_str(&format!(" WHEN {} = '{}' THEN '{}'", old_column, escaped_old, escaped_new));
        }
        
        // Add default case
        match default_value {
            Some(default) => {
                let escaped_default = default.replace("'", "''");
                case_expr.push_str(&format!(" ELSE '{}'", escaped_default));
            }
            None => {
                // If no default provided, keep original value for unmapped cases
                case_expr.push_str(&format!(" ELSE {}", old_column));
            }
        }
        
        case_expr.push_str(" END");
        
        format!(
            "UPDATE {} SET {} = {} WHERE rowid IN (SELECT rowid FROM {} WHERE {} IS NOT NULL ORDER BY rowid LIMIT {} OFFSET {})",
            table, new_column, case_expr, table, old_column, batch_size, offset
        )
    }
}