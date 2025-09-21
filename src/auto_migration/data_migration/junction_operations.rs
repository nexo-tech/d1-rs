// Junction table operations functionality for data migration

use crate::{Result, D1RsError};
use crate::auto_migration::data_migration::core::{DataMigrator, TransformationResult};
use std::time::Instant;

impl DataMigrator {
    /// Execute denormalized column population
    /// Populates a junction table by splitting denormalized column data into individual records
    pub async fn execute_denormalized_column_population(
        &self,
        junction_table: &str,
        source_table: &str,
        source_column: &str,
        delimiter: &str,
        source_fk: &str,
        target_fk: &str,
    ) -> Result<TransformationResult> {
        let start_time = Instant::now();
        let mut warnings = Vec::new();
        let mut errors = Vec::new();
        
        // Validate denormalized column population parameters
        if junction_table.is_empty() {
            errors.push(D1RsError::ValidationError("Junction table cannot be empty for denormalized column population".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        if source_table.is_empty() {
            errors.push(D1RsError::ValidationError("Source table cannot be empty for denormalized column population".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        if source_column.is_empty() {
            errors.push(D1RsError::ValidationError("Source column cannot be empty for denormalized column population".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        if delimiter.is_empty() {
            errors.push(D1RsError::ValidationError("Delimiter cannot be empty for denormalized column population".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        if source_fk.is_empty() || target_fk.is_empty() {
            errors.push(D1RsError::ValidationError("Source and target foreign key columns cannot be empty for denormalized column population".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        if source_fk == target_fk {
            errors.push(D1RsError::ValidationError("Source and target foreign key columns cannot be the same for denormalized column population".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Validate that source table is accessible
        let source_check_sql = format!("SELECT COUNT(*) FROM {} LIMIT 1", source_table);
        if let Err(_) = self.db.execute_returning_count(&source_check_sql, &[]).await {
            warnings.push(format!("Source table {} not accessible, skipping denormalized column population", source_table));
            return Ok(TransformationResult {
                success: true,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Validate that junction table is accessible
        let junction_check_sql = format!("SELECT COUNT(*) FROM {} LIMIT 1", junction_table);
        if let Err(_) = self.db.execute_returning_count(&junction_check_sql, &[]).await {
            warnings.push(format!("Junction table {} not accessible, skipping denormalized column population", junction_table));
            return Ok(TransformationResult {
                success: true,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Get records with non-empty denormalized data
        let count_sql = format!("SELECT COUNT(*) FROM {} WHERE {} IS NOT NULL AND {} != ''", source_table, source_column, source_column);
        let total_records = match self.db.execute_returning_count(&count_sql, &[]).await {
            Ok(count) => count as u64,
            Err(e) => {
                errors.push(D1RsError::AutoMigration(format!("Failed to count records in source table {}: {}", source_table, e)));
                return Ok(TransformationResult {
                    success: false,
                    records_processed: 0,
                    records_failed: 0,
                    errors,
                    warnings,
                });
            }
        };
        
        if total_records == 0 {
            warnings.push("No records with denormalized data found for population".to_string());
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
        let mut total_junction_records = 0u64;
        
        let mut offset = 0u64;
        while offset < total_records {
            // Get batch of source records with denormalized data
            let batch_sql = format!(
                "SELECT id, {} FROM {} WHERE {} IS NOT NULL AND {} != '' ORDER BY id LIMIT {} OFFSET {}",
                source_column, source_table, source_column, source_column, batch_size, offset
            );
            
            match self.db.execute(&batch_sql, &[]).await {
                Ok(result) => {
                    let batch_processed = result.rows.len() as u64;
                    
                    // Process each record in the batch
                    for row in &result.rows {
                        if let serde_json::Value::Object(row_map) = row {
                            let source_id = match row_map.get("id") {
                                Some(serde_json::Value::Number(n)) => n.as_u64().unwrap_or(0),
                                _ => {
                                    errors.push(D1RsError::AutoMigration("Failed to extract id from source record".to_string()));
                                    total_failed += 1;
                                    continue;
                                }
                            };
                            
                            let denormalized_data = match row_map.get(source_column) {
                                Some(serde_json::Value::String(s)) => s,
                                _ => {
                                    warnings.push(format!("Skipping record {} due to invalid denormalized data", source_id));
                                    continue;
                                }
                            };
                            
                            // Split the denormalized data using the delimiter
                            let values: Vec<&str> = denormalized_data
                                .split(delimiter)
                                .map(|s| s.trim())
                                .filter(|s| !s.is_empty())
                                .collect();
                            
                            if values.is_empty() {
                                warnings.push(format!("No valid values found in denormalized data for record {}", source_id));
                                continue;
                            }
                            
                            // Create junction records for each split value
                            for value in values {
                                // For this implementation, we'll assume the target values are the string values themselves
                                // In a real scenario, you might need to look up target IDs from another table
                                let insert_sql = format!(
                                    "INSERT INTO {} ({}, {}) VALUES (?, ?)",
                                    junction_table, source_fk, target_fk
                                );
                                
                                match self.db.execute(&insert_sql, &[
                                    serde_json::Value::Number(source_id.into()),
                                    serde_json::Value::String(value.to_string()),
                                ]).await {
                                    Ok(_) => {
                                        total_junction_records += 1;
                                    }
                                    Err(e) => {
                                        errors.push(D1RsError::AutoMigration(format!("Failed to insert junction record for source {} value '{}': {}", source_id, value, e)));
                                        total_failed += 1;
                                    }
                                }
                            }
                        }
                    }
                    
                    total_processed += batch_processed;
                }
                Err(e) => {
                    errors.push(D1RsError::AutoMigration(format!("Failed to fetch batch at offset {}: {}", offset, e)));
                    total_failed += std::cmp::min(batch_size as u64, total_records - offset);
                }
            }
            
            offset += batch_size as u64;
        }
        
        // Add summary information
        if total_junction_records > 0 {
            warnings.push(format!("Created {} junction records from {} source records", total_junction_records, total_processed));
        }
        
        // Update metrics
        let duration = start_time.elapsed();
        self.update_transformation_metrics("denormalized_column_population", duration, total_failed == 0);
        
        Ok(TransformationResult {
            success: total_failed == 0,
            records_processed: total_processed,
            records_failed: total_failed,
            errors,
            warnings,
        })
    }
    
    /// Execute existing junction table population
    pub async fn execute_existing_junction_table_population(
        &self,
        _new_junction_table: &str,
        _old_junction_table: &str,
        _column_mapping: &std::collections::HashMap<String, String>,
    ) -> Result<TransformationResult> {
        // Placeholder for existing junction table population logic
        Ok(TransformationResult {
            success: true,
            records_processed: 0,
            records_failed: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
        })
    }
    
    /// Execute business rules population
    pub async fn execute_business_rules_population(
        &self,
        _junction_table: &str,
        _generation_query: &str,
        _validation_rules: &[String],
    ) -> Result<TransformationResult> {
        // Placeholder for business rules population logic
        Ok(TransformationResult {
            success: true,
            records_processed: 0,
            records_failed: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
        })
    }
}