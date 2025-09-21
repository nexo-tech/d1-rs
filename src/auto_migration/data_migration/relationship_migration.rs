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
    /// Recreates business logic between source and target tables using custom SQL queries and validation rules
    pub async fn execute_business_logic_recreation(
        &self,
        source_table: &str,
        target_table: &str,
        recreation_query: &str,
        validation_rules: &[String],
    ) -> Result<TransformationResult> {
        let start_time = Instant::now();
        let mut warnings = Vec::new();
        let mut errors = Vec::new();
        
        // Validate business logic recreation parameters
        if source_table.is_empty() {
            errors.push(D1RsError::ValidationError("Source table cannot be empty for business logic recreation".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        if target_table.is_empty() {
            errors.push(D1RsError::ValidationError("Target table cannot be empty for business logic recreation".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        if recreation_query.is_empty() {
            errors.push(D1RsError::ValidationError("Recreation query cannot be empty for business logic recreation".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Validate that recreation query contains expected table references
        let query_lower = recreation_query.to_lowercase();
        if !query_lower.contains(&source_table.to_lowercase()) {
            warnings.push(format!("Recreation query does not reference source table '{}', this may be intentional", source_table));
        }
        
        if !query_lower.contains(&target_table.to_lowercase()) {
            warnings.push(format!("Recreation query does not reference target table '{}', this may be intentional", target_table));
        }
        
        // Basic SQL injection protection - check for dangerous patterns
        let query_trimmed = query_lower.trim();
        let dangerous_patterns = vec!["drop", "delete", "truncate", "alter", "create"];
        for pattern in dangerous_patterns {
            if query_lower.contains(pattern) && !query_trimmed.starts_with("insert") && !query_trimmed.starts_with("update") && !query_trimmed.starts_with("select") {
                errors.push(D1RsError::ValidationError(format!("Recreation query contains potentially dangerous SQL pattern: '{}'", pattern)));
                return Ok(TransformationResult {
                    success: false,
                    records_processed: 0,
                    records_failed: 0,
                    errors,
                    warnings,
                });
            }
        }
        
        // Validate source table exists and is accessible
        let source_check_sql = format!("SELECT COUNT(*) FROM {} LIMIT 1", source_table);
        if let Err(_) = self.db.execute_returning_count(&source_check_sql, &[]).await {
            warnings.push(format!("Source table {} not accessible, skipping business logic recreation", source_table));
            return Ok(TransformationResult {
                success: true,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Validate target table exists and is accessible
        let target_check_sql = format!("SELECT COUNT(*) FROM {} LIMIT 1", target_table);
        if let Err(_) = self.db.execute_returning_count(&target_check_sql, &[]).await {
            warnings.push(format!("Target table {} not accessible, skipping business logic recreation", target_table));
            return Ok(TransformationResult {
                success: true,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Get count of records that would be affected by the recreation query
        // Try to estimate by counting source table records
        let source_count_sql = format!("SELECT COUNT(*) FROM {}", source_table);
        let estimated_records = match self.db.execute_returning_count(&source_count_sql, &[]).await {
            Ok(count) => count as u64,
            Err(_) => {
                warnings.push("Could not estimate number of records to process".to_string());
                0
            }
        };
        
        if estimated_records == 0 {
            warnings.push("No records found in source table for business logic recreation".to_string());
            return Ok(TransformationResult {
                success: true,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Execute the recreation query
        let mut total_processed = 0u64;
        let mut total_failed = 0u64;
        
        match self.db.execute(recreation_query, &[]).await {
            Ok(_result) => {
                // Query executed successfully
                total_processed = estimated_records;
                
                // Apply validation rules if provided
                if !validation_rules.is_empty() {
                    let mut validation_failures = 0u64;
                    
                    for (i, rule) in validation_rules.iter().enumerate() {
                        if rule.trim().is_empty() {
                            continue;
                        }
                        
                        // Execute validation rule as a query that should return count of violations
                        match self.db.execute_returning_count(rule, &[]).await {
                            Ok(violation_count) => {
                                if violation_count > 0 {
                                    warnings.push(format!("Validation rule {} found {} violations: {}", i + 1, violation_count, rule));
                                    validation_failures += violation_count as u64;
                                }
                            }
                            Err(e) => {
                                errors.push(D1RsError::AutoMigration(format!("Validation rule {} failed to execute: {}", i + 1, e)));
                            }
                        }
                    }
                    
                    if validation_failures > 0 {
                        warnings.push(format!("Business logic recreation completed but {} validation violations were found", validation_failures));
                    }
                }
            }
            Err(e) => {
                errors.push(D1RsError::AutoMigration(format!("Business logic recreation query failed: {}", e)));
                total_failed = estimated_records;
            }
        }
        
        // Update metrics
        let duration = start_time.elapsed();
        self.update_transformation_metrics("business_logic_recreation", duration, total_failed == 0);
        
        Ok(TransformationResult {
            success: total_failed == 0,
            records_processed: total_processed,
            records_failed: total_failed,
            errors,
            warnings,
        })
    }
    
    /// Execute cascade migration
    /// Executes migration operations across multiple tables in dependency order to maintain referential integrity
    pub async fn execute_cascade_migration(
        &self,
        dependency_order: &[String],
        cascade_rules: &std::collections::HashMap<String, String>,
    ) -> Result<TransformationResult> {
        let start_time = Instant::now();
        let mut warnings = Vec::new();
        let mut errors = Vec::new();
        
        // Validate cascade migration parameters
        if dependency_order.is_empty() {
            errors.push(D1RsError::ValidationError("Dependency order cannot be empty for cascade migration".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        if cascade_rules.is_empty() {
            errors.push(D1RsError::ValidationError("Cascade rules cannot be empty for cascade migration".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Validate that all tables in dependency order have corresponding rules
        for table in dependency_order {
            if table.is_empty() {
                errors.push(D1RsError::ValidationError("Table name in dependency order cannot be empty".to_string()));
                return Ok(TransformationResult {
                    success: false,
                    records_processed: 0,
                    records_failed: 0,
                    errors,
                    warnings,
                });
            }
            
            if !cascade_rules.contains_key(table) {
                warnings.push(format!("Table '{}' in dependency order has no corresponding cascade rule", table));
            }
        }
        
        // Validate cascade rules for basic SQL injection protection
        for (table_name, rule) in cascade_rules {
            if table_name.is_empty() {
                errors.push(D1RsError::ValidationError("Table name in cascade rules cannot be empty".to_string()));
                return Ok(TransformationResult {
                    success: false,
                    records_processed: 0,
                    records_failed: 0,
                    errors,
                    warnings,
                });
            }
            
            if rule.is_empty() {
                warnings.push(format!("Cascade rule for table '{}' is empty", table_name));
                continue;
            }
            
            // Basic SQL injection protection for cascade rules
            let rule_lower = rule.to_lowercase();
            let rule_trimmed = rule_lower.trim();
            let dangerous_patterns = vec!["drop", "truncate", "alter"];
            
            for pattern in dangerous_patterns {
                if rule_lower.contains(pattern) && !rule_trimmed.starts_with("insert") && !rule_trimmed.starts_with("update") && !rule_trimmed.starts_with("select") && !rule_trimmed.starts_with("delete") {
                    errors.push(D1RsError::ValidationError(format!("Cascade rule for table '{}' contains potentially dangerous SQL pattern: '{}'", table_name, pattern)));
                    return Ok(TransformationResult {
                        success: false,
                        records_processed: 0,
                        records_failed: 0,
                        errors,
                        warnings,
                    });
                }
            }
        }
        
        // Execute cascade migration in dependency order
        let mut total_processed = 0u64;
        let mut total_failed = 0u64;
        let mut processed_tables = Vec::new();
        
        for table in dependency_order {
            // Check if table is accessible
            let table_check_sql = format!("SELECT COUNT(*) FROM {} LIMIT 1", table);
            match self.db.execute_returning_count(&table_check_sql, &[]).await {
                Ok(_) => {
                    // Table exists and is accessible
                    if let Some(cascade_rule) = cascade_rules.get(table) {
                        if cascade_rule.trim().is_empty() {
                            warnings.push(format!("Skipping table '{}' due to empty cascade rule", table));
                            continue;
                        }
                        
                        // Get record count before applying rule (for estimation)
                        let pre_count_sql = format!("SELECT COUNT(*) FROM {}", table);
                        let estimated_records = match self.db.execute_returning_count(&pre_count_sql, &[]).await {
                            Ok(count) => count as u64,
                            Err(_) => {
                                warnings.push(format!("Could not estimate record count for table '{}'", table));
                                0
                            }
                        };
                        
                        // Execute the cascade rule
                        match self.db.execute(cascade_rule, &[]).await {
                            Ok(_result) => {
                                // Rule executed successfully
                                total_processed += estimated_records;
                                processed_tables.push(table.clone());
                                
                                // Log the successful operation
                                if estimated_records > 0 {
                                    warnings.push(format!("Successfully applied cascade rule to table '{}' (estimated {} records)", table, estimated_records));
                                }
                            }
                            Err(e) => {
                                errors.push(D1RsError::AutoMigration(format!("Cascade rule failed for table '{}': {}", table, e)));
                                total_failed += estimated_records;
                                
                                // Depending on failure strategy, we might want to continue or stop
                                // For cascade migrations, failures are often critical, so we'll continue but track them
                                warnings.push(format!("Continuing cascade migration despite failure in table '{}'", table));
                            }
                        }
                    } else {
                        warnings.push(format!("Table '{}' has no cascade rule, skipping", table));
                    }
                }
                Err(_) => {
                    warnings.push(format!("Table '{}' not accessible, skipping in cascade migration", table));
                }
            }
        }
        
        // Check for unused cascade rules
        let mut unused_rules = Vec::new();
        for table_name in cascade_rules.keys() {
            if !dependency_order.contains(table_name) {
                unused_rules.push(table_name.clone());
            }
        }
        
        if !unused_rules.is_empty() {
            warnings.push(format!("The following tables have cascade rules but are not in dependency order: {}", unused_rules.join(", ")));
        }
        
        // Summary information
        if !processed_tables.is_empty() {
            warnings.push(format!("Cascade migration processed {} tables in order: {}", processed_tables.len(), processed_tables.join(" → ")));
        }
        
        // Update metrics
        let duration = start_time.elapsed();
        self.update_transformation_metrics("cascade_migration", duration, total_failed == 0);
        
        Ok(TransformationResult {
            success: total_failed == 0,
            records_processed: total_processed,
            records_failed: total_failed,
            errors,
            warnings,
        })
    }
}