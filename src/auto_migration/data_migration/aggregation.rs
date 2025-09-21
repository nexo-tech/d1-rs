// Data aggregation functionality for data migration

use crate::{Result, D1RsError};
use crate::auto_migration::data_migration::core::{DataMigrator, TransformationResult};
use std::time::Instant;

impl DataMigrator {
    /// Execute data aggregation transformation
    pub async fn execute_aggregation(
        &self,
        table: &str,
        source_fields: &[String],
        target_field: &str,
        aggregation_function: &str,
    ) -> Result<TransformationResult> {
        let start_time = Instant::now();
        let mut warnings = Vec::new();
        let mut errors = Vec::new();
        
        // Validate aggregation parameters
        if source_fields.is_empty() {
            warnings.push("No source fields specified for aggregation".to_string());
            return Ok(TransformationResult {
                success: true,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        if target_field.is_empty() {
            errors.push(D1RsError::ValidationError("Target field cannot be empty for aggregation".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Validate aggregation function
        if !self.is_aggregation_function_supported(aggregation_function) {
            warnings.push(format!("Aggregation function '{}' is not supported", aggregation_function));
            return Ok(TransformationResult {
                success: true,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Build condition to check for non-null source fields
        let non_null_conditions: Vec<String> = source_fields.iter()
            .map(|field| format!("{} IS NOT NULL", field))
            .collect();
        let where_clause = non_null_conditions.join(" AND ");
        
        // Get total record count - handle missing table gracefully
        let count_sql = format!("SELECT COUNT(*) FROM {} WHERE {}", table, where_clause);
        let total_records = match self.db.execute_returning_count(&count_sql, &[]).await {
            Ok(count) => count as u64,
            Err(_) => {
                // Table doesn't exist or other error - treat as success with 0 records
                warnings.push(format!("Table {} not accessible, skipping aggregation", table));
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
            warnings.push("No records to aggregate".to_string());
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
            let aggregation_sql = self.generate_aggregation_sql(
                table,
                source_fields,
                target_field,
                aggregation_function,
                &where_clause,
                batch_size,
                offset,
            );
            
            match self.db.execute(&aggregation_sql, &[]).await {
                Ok(_result) => {
                    let batch_processed = std::cmp::min(batch_size as u64, total_records - offset);
                    total_processed += batch_processed;
                }
                Err(e) => {
                    errors.push(D1RsError::AutoMigration(format!("Aggregation failed for batch at offset {}: {}", offset, e)));
                    total_failed += std::cmp::min(batch_size as u64, total_records - offset);
                }
            }
            
            offset += batch_size as u64;
        }
        
        // Update metrics
        let duration = start_time.elapsed();
        self.update_transformation_metrics("aggregation", duration, total_failed == 0);
        
        Ok(TransformationResult {
            success: total_failed == 0,
            records_processed: total_processed,
            records_failed: total_failed,
            errors,
            warnings,
        })
    }
    
    /// Check if aggregation function is supported
    fn is_aggregation_function_supported(&self, aggregation_function: &str) -> bool {
        match aggregation_function.to_uppercase().as_str() {
            // Standard SQL aggregation functions
            "SUM" => true,
            "AVG" | "AVERAGE" => true,
            "COUNT" => true,
            "MIN" | "MINIMUM" => true,
            "MAX" | "MAXIMUM" => true,
            
            // String aggregation functions
            "CONCAT" | "GROUP_CONCAT" => true,
            "STRING_AGG" => true,
            
            // Mathematical aggregation functions
            "TOTAL" => true,  // SQLite's total function (like sum but returns 0.0 for empty set)
            
            _ => false,
        }
    }
    
    /// Generate SQL for aggregation operation
    fn generate_aggregation_sql(
        &self,
        table: &str,
        source_fields: &[String],
        target_field: &str,
        aggregation_function: &str,
        where_clause: &str,
        batch_size: usize,
        offset: u64,
    ) -> String {
        let aggregation_expr = match aggregation_function.to_uppercase().as_str() {
            "SUM" => {
                if source_fields.len() == 1 {
                    format!("SUM({})", source_fields[0])
                } else {
                    // Sum multiple fields - use addition
                    format!("({})", source_fields.join(" + "))
                }
            }
            "AVG" | "AVERAGE" => {
                if source_fields.len() == 1 {
                    format!("AVG({})", source_fields[0])
                } else {
                    // Average of multiple fields - use addition and division
                    format!("(({}) / {})", source_fields.join(" + "), source_fields.len())
                }
            }
            "COUNT" => {
                // Count non-null values in source fields
                let non_null_conditions: Vec<String> = source_fields.iter()
                    .map(|field| format!("{} IS NOT NULL", field))
                    .collect();
                format!("CASE WHEN {} THEN 1 ELSE 0 END", non_null_conditions.join(" AND "))
            }
            "MIN" | "MINIMUM" => {
                if source_fields.len() == 1 {
                    // For single field, use aggregate function in a subquery
                    format!("(SELECT MIN({}) FROM {} WHERE rowid = {}.rowid)", source_fields[0], table, table)
                } else {
                    // For multiple fields in same row, use scalar MIN function
                    format!("MIN({})", source_fields.join(", "))
                }
            }
            "MAX" | "MAXIMUM" => {
                if source_fields.len() == 1 {
                    // For single field, use aggregate function in a subquery
                    format!("(SELECT MAX({}) FROM {} WHERE rowid = {}.rowid)", source_fields[0], table, table)
                } else {
                    // For multiple fields in same row, use scalar MAX function
                    format!("MAX({})", source_fields.join(", "))
                }
            }
            "CONCAT" | "GROUP_CONCAT" => {
                // Concatenate fields with a separator
                format!("({})", source_fields.iter()
                    .map(|field| format!("COALESCE({}, '')", field))
                    .collect::<Vec<_>>()
                    .join(" || ' ' || "))
            }
            "STRING_AGG" => {
                // String aggregation with comma separator
                format!("({})", source_fields.iter()
                    .map(|field| format!("COALESCE({}, '')", field))
                    .collect::<Vec<_>>()
                    .join(" || ', ' || "))
            }
            "TOTAL" => {
                if source_fields.len() == 1 {
                    format!("TOTAL({})", source_fields[0])
                } else {
                    // Total of multiple fields - use addition with COALESCE for null handling
                    format!("({})", source_fields.iter()
                        .map(|field| format!("COALESCE({}, 0)", field))
                        .collect::<Vec<_>>()
                        .join(" + "))
                }
            }
            _ => {
                // Fallback: just use the first source field
                source_fields.first().map(|f| f.to_string()).unwrap_or_default()
            }
        };
        
        format!(
            "UPDATE {} SET {} = {} WHERE rowid IN (SELECT rowid FROM {} WHERE {} ORDER BY rowid LIMIT {} OFFSET {})",
            table, target_field, aggregation_expr, table, where_clause, batch_size, offset
        )
    }
}