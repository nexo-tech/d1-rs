// Relationship migration functionality for data migration

use crate::{Result, D1RsError};
use crate::auto_migration::data_migration::core::{DataMigrator, TransformationResult};
use std::time::Instant;

impl DataMigrator {
    /// Execute direct foreign key copy
    /// Copies values from old foreign key column to new foreign key column in the source table
    pub async fn execute_direct_fk_copy(
        &self,
        source_table: &str,
        old_fk_column: &str,
        new_fk_column: &str,
    ) -> Result<TransformationResult> {
        let start_time = Instant::now();
        let mut warnings = Vec::new();
        let mut errors = Vec::new();
        
        // Validate direct FK copy parameters
        if source_table.is_empty() {
            errors.push(D1RsError::ValidationError("Source table cannot be empty for direct FK copy".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        if old_fk_column.is_empty() || new_fk_column.is_empty() {
            errors.push(D1RsError::ValidationError("Source and target FK columns cannot be empty for direct FK copy".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        if old_fk_column == new_fk_column {
            warnings.push("Source and target FK columns are the same, no copy needed".to_string());
            return Ok(TransformationResult {
                success: true,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Get total record count - handle missing table gracefully
        let count_sql = format!("SELECT COUNT(*) FROM {} WHERE {} IS NOT NULL", source_table, old_fk_column);
        let total_records = match self.db.execute_returning_count(&count_sql, &[]).await {
            Ok(count) => count as u64,
            Err(_) => {
                // Table doesn't exist or other error - treat as success with 0 records
                warnings.push(format!("Table {} not accessible, skipping direct FK copy", source_table));
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
            warnings.push("No records to copy FK values for".to_string());
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
            let copy_sql = format!(
                "UPDATE {} SET {} = {} WHERE rowid IN (SELECT rowid FROM {} WHERE {} IS NOT NULL ORDER BY rowid LIMIT {} OFFSET {})",
                source_table, new_fk_column, old_fk_column, source_table, old_fk_column, batch_size, offset
            );
            
            match self.db.execute(&copy_sql, &[]).await {
                Ok(_result) => {
                    let batch_processed = std::cmp::min(batch_size as u64, total_records - offset);
                    total_processed += batch_processed;
                }
                Err(e) => {
                    errors.push(D1RsError::AutoMigration(format!("Direct FK copy failed for batch at offset {}: {}", offset, e)));
                    total_failed += std::cmp::min(batch_size as u64, total_records - offset);
                }
            }
            
            offset += batch_size as u64;
        }
        
        // Update metrics
        let duration = start_time.elapsed();
        self.update_transformation_metrics("direct_fk_copy", duration, total_failed == 0);
        
        Ok(TransformationResult {
            success: total_failed == 0,
            records_processed: total_processed,
            records_failed: total_failed,
            errors,
            warnings,
        })
    }
    
    /// Execute ID mapping migration
    /// Migrates IDs from old values to new values using a mapping table
    pub async fn execute_id_mapping_migration(
        &self,
        source_table: &str,
        target_table: &str,
        mapping_table: &str,
        old_id_column: &str,
        new_id_column: &str,
    ) -> Result<TransformationResult> {
        let start_time = Instant::now();
        let mut warnings = Vec::new();
        let mut errors = Vec::new();
        
        // Validate ID mapping migration parameters
        if source_table.is_empty() {
            errors.push(D1RsError::ValidationError("Source table cannot be empty for ID mapping migration".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        if target_table.is_empty() {
            errors.push(D1RsError::ValidationError("Target table cannot be empty for ID mapping migration".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        if mapping_table.is_empty() {
            errors.push(D1RsError::ValidationError("Mapping table cannot be empty for ID mapping migration".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        if old_id_column.is_empty() || new_id_column.is_empty() {
            errors.push(D1RsError::ValidationError("Old and new ID columns cannot be empty for ID mapping migration".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        if old_id_column == new_id_column {
            warnings.push("Old and new ID columns are the same, no migration needed".to_string());
            return Ok(TransformationResult {
                success: true,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Validate mapping table exists and has expected structure
        let mapping_check_sql = format!("SELECT COUNT(*) FROM {} LIMIT 1", mapping_table);
        if let Err(_) = self.db.execute_returning_count(&mapping_check_sql, &[]).await {
            warnings.push(format!("Mapping table {} not accessible, skipping ID mapping migration", mapping_table));
            return Ok(TransformationResult {
                success: true,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Check if mapping table has any data
        let mapping_count_sql = format!("SELECT COUNT(*) FROM {}", mapping_table);
        let mapping_records = match self.db.execute_returning_count(&mapping_count_sql, &[]).await {
            Ok(count) => count as u64,
            Err(_) => {
                warnings.push(format!("Cannot count records in mapping table {}", mapping_table));
                return Ok(TransformationResult {
                    success: true,
                    records_processed: 0,
                    records_failed: 0,
                    errors,
                    warnings,
                });
            }
        };
        
        if mapping_records == 0 {
            warnings.push("Mapping table is empty, no ID mappings available".to_string());
            return Ok(TransformationResult {
                success: true,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Get total record count in source table that need migration
        let count_sql = format!("SELECT COUNT(*) FROM {} WHERE {} IS NOT NULL", source_table, old_id_column);
        let total_records = match self.db.execute_returning_count(&count_sql, &[]).await {
            Ok(count) => count as u64,
            Err(_) => {
                // Source table doesn't exist or other error
                warnings.push(format!("Source table {} not accessible, skipping ID mapping migration", source_table));
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
            warnings.push("No records to migrate IDs for in source table".to_string());
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
        let mut unmapped_ids = 0u64;
        
        let mut offset = 0u64;
        while offset < total_records {
            // Build SQL to update IDs using mapping table
            // This joins the source table with the mapping table to get new IDs
            let migration_sql = format!(
                "UPDATE {} SET {} = (
                    SELECT new_id FROM {} WHERE old_id = {}.{}
                ) WHERE rowid IN (
                    SELECT s.rowid FROM {} s
                    WHERE s.{} IS NOT NULL 
                    ORDER BY s.rowid 
                    LIMIT {} OFFSET {}
                )",
                source_table, new_id_column,
                mapping_table, source_table, old_id_column,
                source_table, old_id_column,
                batch_size, offset
            );
            
            match self.db.execute(&migration_sql, &[]).await {
                Ok(_result) => {
                    let batch_processed = std::cmp::min(batch_size as u64, total_records - offset);
                    total_processed += batch_processed;
                    
                    // Check for unmapped IDs in this batch
                    let unmapped_check_sql = format!(
                        "SELECT COUNT(*) FROM {} 
                         WHERE rowid IN (
                             SELECT s.rowid FROM {} s
                             WHERE s.{} IS NOT NULL AND s.{} IS NULL
                             ORDER BY s.rowid 
                             LIMIT {} OFFSET {}
                         )",
                        source_table, source_table, old_id_column, new_id_column,
                        batch_size, offset
                    );
                    
                    if let Ok(unmapped_count) = self.db.execute_returning_count(&unmapped_check_sql, &[]).await {
                        unmapped_ids += unmapped_count as u64;
                    }
                }
                Err(e) => {
                    errors.push(D1RsError::AutoMigration(format!("ID mapping migration failed for batch at offset {}: {}", offset, e)));
                    total_failed += std::cmp::min(batch_size as u64, total_records - offset);
                }
            }
            
            offset += batch_size as u64;
        }
        
        // Add warning if some IDs couldn't be mapped
        if unmapped_ids > 0 {
            warnings.push(format!("{} records had IDs that could not be mapped (no corresponding entry in mapping table)", unmapped_ids));
        }
        
        // Update metrics
        let duration = start_time.elapsed();
        self.update_transformation_metrics("id_mapping_migration", duration, total_failed == 0);
        
        Ok(TransformationResult {
            success: total_failed == 0,
            records_processed: total_processed,
            records_failed: total_failed,
            errors,
            warnings,
        })
    }
    
    /// Execute business logic recreation
    pub async fn execute_business_logic_recreation(
        &self,
        _source_table: &str,
        _target_table: &str,
        _recreation_query: &str,
        _validation_rules: &[String],
    ) -> Result<TransformationResult> {
        // Placeholder for business logic recreation
        Ok(TransformationResult {
            success: true,
            records_processed: 0,
            records_failed: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
        })
    }
    
    /// Execute cascade migration
    pub async fn execute_cascade_migration(
        &self,
        _dependency_order: &[String],
        _cascade_rules: &std::collections::HashMap<String, String>,
    ) -> Result<TransformationResult> {
        // Placeholder for cascade migration logic
        Ok(TransformationResult {
            success: true,
            records_processed: 0,
            records_failed: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
        })
    }
}