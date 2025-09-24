// Junction table operations functionality for data migration

use crate::{Result, D1RsError};
use crate::backends::QueryResult;
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
                    let batch_processed = result.rows().len() as u64;
                    
                    // Process each record in the batch
                    for row in result.rows() {
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
    /// Copies data from an existing junction table to a new junction table with column mapping
    pub async fn execute_existing_junction_table_population(
        &self,
        new_junction_table: &str,
        old_junction_table: &str,
        column_mapping: &std::collections::HashMap<String, String>,
    ) -> Result<TransformationResult> {
        let start_time = Instant::now();
        let mut warnings = Vec::new();
        let mut errors = Vec::new();
        
        // Validate existing junction table population parameters
        if new_junction_table.is_empty() {
            errors.push(D1RsError::ValidationError("New junction table cannot be empty for existing junction table population".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        if old_junction_table.is_empty() {
            errors.push(D1RsError::ValidationError("Old junction table cannot be empty for existing junction table population".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        if new_junction_table == old_junction_table {
            errors.push(D1RsError::ValidationError("New and old junction tables cannot be the same for existing junction table population".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        if column_mapping.is_empty() {
            errors.push(D1RsError::ValidationError("Column mapping cannot be empty for existing junction table population".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Validate column mapping parameters
        for (old_col, new_col) in column_mapping {
            if old_col.is_empty() || new_col.is_empty() {
                errors.push(D1RsError::ValidationError("Column names in mapping cannot be empty for existing junction table population".to_string()));
                return Ok(TransformationResult {
                    success: false,
                    records_processed: 0,
                    records_failed: 0,
                    errors,
                    warnings,
                });
            }
            
            if old_col == new_col {
                warnings.push(format!("Column mapping maps '{}' to itself, this has no effect", old_col));
            }
        }
        
        // Check for duplicate mappings
        let mut new_columns: std::collections::HashSet<&String> = std::collections::HashSet::new();
        for new_col in column_mapping.values() {
            if !new_columns.insert(new_col) {
                errors.push(D1RsError::ValidationError(format!("Duplicate target column '{}' in column mapping for existing junction table population", new_col)));
                return Ok(TransformationResult {
                    success: false,
                    records_processed: 0,
                    records_failed: 0,
                    errors,
                    warnings,
                });
            }
        }
        
        // Validate that old junction table is accessible
        let old_check_sql = format!("SELECT COUNT(*) FROM {} LIMIT 1", old_junction_table);
        if let Err(_) = self.db.execute_returning_count(&old_check_sql, &[]).await {
            warnings.push(format!("Old junction table {} not accessible, skipping existing junction table population", old_junction_table));
            return Ok(TransformationResult {
                success: true,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Validate that new junction table is accessible
        let new_check_sql = format!("SELECT COUNT(*) FROM {} LIMIT 1", new_junction_table);
        if let Err(_) = self.db.execute_returning_count(&new_check_sql, &[]).await {
            warnings.push(format!("New junction table {} not accessible, skipping existing junction table population", new_junction_table));
            return Ok(TransformationResult {
                success: true,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Get total record count from old junction table
        let count_sql = format!("SELECT COUNT(*) FROM {}", old_junction_table);
        let total_records = match self.db.execute_returning_count(&count_sql, &[]).await {
            Ok(count) => count as u64,
            Err(e) => {
                errors.push(D1RsError::AutoMigration(format!("Failed to count records in old junction table {}: {}", old_junction_table, e)));
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
            warnings.push("No records found in old junction table for population".to_string());
            return Ok(TransformationResult {
                success: true,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Build column lists for SQL generation
        let old_columns: Vec<&String> = column_mapping.keys().collect();
        let new_columns: Vec<&String> = old_columns.iter().map(|&old_col| column_mapping.get(old_col).unwrap()).collect();
        
        let old_columns_sql = old_columns.iter().map(|&s| s.as_str()).collect::<Vec<&str>>().join(", ");
        let new_columns_sql = new_columns.iter().map(|&s| s.as_str()).collect::<Vec<&str>>().join(", ");
        let placeholders = vec!["?"; new_columns.len()].join(", ");
        
        // Process records in batches
        let batch_size = self.config.batch_size;
        let mut total_processed = 0u64;
        let mut total_failed = 0u64;
        let mut total_copied_records = 0u64;
        
        let mut offset = 0u64;
        while offset < total_records {
            // Get batch of records from old junction table
            let batch_sql = format!(
                "SELECT {} FROM {} ORDER BY rowid LIMIT {} OFFSET {}",
                old_columns_sql, old_junction_table, batch_size, offset
            );
            
            match self.db.execute(&batch_sql, &[]).await {
                Ok(result) => {
                    let batch_processed = result.rows().len() as u64;
                    
                    // Process each record in the batch
                    for row in result.rows() {
                        if let serde_json::Value::Object(row_map) = row {
                            // Extract values according to column mapping
                            let mut values = Vec::new();
                            let mut skip_record = false;
                            
                            for old_col in &old_columns {
                                match row_map.get(*old_col) {
                                    Some(value) => values.push(value.clone()),
                                    None => {
                                        warnings.push(format!("Column '{}' not found in record, skipping", old_col));
                                        skip_record = true;
                                        break;
                                    }
                                }
                            }
                            
                            if skip_record {
                                continue;
                            }
                            
                            // Insert record into new junction table
                            let insert_sql = format!(
                                "INSERT INTO {} ({}) VALUES ({})",
                                new_junction_table, new_columns_sql, placeholders
                            );
                            
                            match self.db.execute(&insert_sql, &values).await {
                                Ok(_) => {
                                    total_copied_records += 1;
                                }
                                Err(e) => {
                                    errors.push(D1RsError::AutoMigration(format!("Failed to insert record into new junction table {}: {}", new_junction_table, e)));
                                    total_failed += 1;
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
        if total_copied_records > 0 {
            warnings.push(format!("Copied {} records from {} to {} with column mapping", total_copied_records, old_junction_table, new_junction_table));
        }
        
        if !column_mapping.is_empty() {
            let mapping_summary: Vec<String> = column_mapping.iter()
                .map(|(old, new)| format!("{} → {}", old, new))
                .collect();
            warnings.push(format!("Applied column mappings: {}", mapping_summary.join(", ")));
        }
        
        // Update metrics
        let duration = start_time.elapsed();
        self.update_transformation_metrics("existing_junction_table_population", duration, total_failed == 0);
        
        Ok(TransformationResult {
            success: total_failed == 0,
            records_processed: total_processed,
            records_failed: total_failed,
            errors,
            warnings,
        })
    }
    
    /// Execute business rules population
    /// Populates a junction table using a generation query and validates the results with business rules
    pub async fn execute_business_rules_population(
        &self,
        junction_table: &str,
        generation_query: &str,
        validation_rules: &[String],
    ) -> Result<TransformationResult> {
        let start_time = Instant::now();
        let mut warnings = Vec::new();
        let mut errors = Vec::new();
        
        // Validate business rules population parameters
        if junction_table.is_empty() {
            errors.push(D1RsError::ValidationError("Junction table cannot be empty for business rules population".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        if generation_query.is_empty() {
            errors.push(D1RsError::ValidationError("Generation query cannot be empty for business rules population".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Validate generation query safety (prevent dangerous operations)
        let query_trimmed = generation_query.trim().to_uppercase();
        if query_trimmed.contains("DROP ") || query_trimmed.contains("TRUNCATE ") || 
           query_trimmed.contains("ALTER ") || query_trimmed.contains("DELETE ") {
            errors.push(D1RsError::ValidationError("Generation query contains potentially dangerous SQL patterns for business rules population".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Ensure generation query is an INSERT statement
        if !query_trimmed.starts_with("INSERT ") {
            errors.push(D1RsError::ValidationError("Generation query must be an INSERT statement for business rules population".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Validate that generation query targets the correct junction table
        if !query_trimmed.contains(&format!("INTO {}", junction_table.to_uppercase())) &&
           !query_trimmed.contains(&format!("INTO `{}`", junction_table)) &&
           !query_trimmed.contains(&format!("INTO \"{}\"", junction_table)) {
            errors.push(D1RsError::ValidationError(format!("Generation query does not target the specified junction table '{}' for business rules population", junction_table)));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Validate validation rules for safety
        for (index, rule) in validation_rules.iter().enumerate() {
            if rule.is_empty() {
                errors.push(D1RsError::ValidationError(format!("Validation rule {} cannot be empty for business rules population", index + 1)));
                return Ok(TransformationResult {
                    success: false,
                    records_processed: 0,
                    records_failed: 0,
                    errors,
                    warnings,
                });
            }
            
            let rule_trimmed = rule.trim().to_uppercase();
            if rule_trimmed.contains("DROP ") || rule_trimmed.contains("TRUNCATE ") || 
               rule_trimmed.contains("ALTER ") || rule_trimmed.contains("DELETE ") ||
               rule_trimmed.contains("INSERT ") || rule_trimmed.contains("UPDATE ") {
                errors.push(D1RsError::ValidationError(format!("Validation rule {} contains potentially dangerous SQL patterns for business rules population", index + 1)));
                return Ok(TransformationResult {
                    success: false,
                    records_processed: 0,
                    records_failed: 0,
                    errors,
                    warnings,
                });
            }
        }
        
        // Validate that junction table is accessible
        let junction_check_sql = format!("SELECT COUNT(*) FROM {} LIMIT 1", junction_table);
        if let Err(_) = self.db.execute_returning_count(&junction_check_sql, &[]).await {
            warnings.push(format!("Junction table {} not accessible, skipping business rules population", junction_table));
            return Ok(TransformationResult {
                success: true,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Get initial record count to track what was added
        let initial_count_sql = format!("SELECT COUNT(*) FROM {}", junction_table);
        let initial_records = match self.db.execute_returning_count(&initial_count_sql, &[]).await {
            Ok(count) => count as u64,
            Err(e) => {
                warnings.push(format!("Could not get initial record count for junction table {}: {}", junction_table, e));
                0
            }
        };
        
        // Execute the generation query
        let mut total_processed = 0u64;
        let mut total_failed = 0u64;
        let mut generation_success = true;
        
        match self.db.execute(generation_query, &[]).await {
            Ok(_result) => {
                // Generation query executed successfully
                warnings.push("Generation query executed successfully".to_string());
                
                // Get final record count to see how many records were generated
                let final_count_sql = format!("SELECT COUNT(*) FROM {}", junction_table);
                match self.db.execute_returning_count(&final_count_sql, &[]).await {
                    Ok(count) => {
                        let final_records = count as u64;
                        total_processed = final_records.saturating_sub(initial_records);
                        warnings.push(format!("Generated {} new records in junction table {}", total_processed, junction_table));
                    }
                    Err(e) => {
                        warnings.push(format!("Could not get final record count for junction table {}: {}", junction_table, e));
                        // Estimate that at least some records were processed
                        total_processed = 1;
                    }
                }
            }
            Err(e) => {
                errors.push(D1RsError::AutoMigration(format!("Generation query failed for business rules population: {}", e)));
                total_failed += 1;
                generation_success = false;
            }
        }
        
        // Apply validation rules if generation was successful
        let mut validation_failures = 0u64;
        if generation_success && !validation_rules.is_empty() {
            warnings.push(format!("Applying {} validation rules to verify business rules compliance", validation_rules.len()));
            
            for (index, validation_rule) in validation_rules.iter().enumerate() {
                match self.db.execute(validation_rule, &[]).await {
                    Ok(result) => {
                        // Check if validation rule returned results (depends on the rule type)
                        if let Some(first_row) = result.rows().first() {
                            if let serde_json::Value::Object(row_map) = first_row {
                                // If the validation rule returns a count, check if it's zero (which might indicate failures)
                                if let Some(serde_json::Value::Number(count)) = row_map.values().next() {
                                    if let Some(count_val) = count.as_u64() {
                                        if count_val == 0 {
                                            validation_failures += 1;
                                            warnings.push(format!("Validation rule {} detected potential issues (returned 0)", index + 1));
                                        } else {
                                            warnings.push(format!("Validation rule {} passed (returned {})", index + 1, count_val));
                                        }
                                    }
                                } else {
                                    warnings.push(format!("Validation rule {} executed successfully", index + 1));
                                }
                            }
                        } else {
                            warnings.push(format!("Validation rule {} executed with no results", index + 1));
                        }
                    }
                    Err(e) => {
                        errors.push(D1RsError::AutoMigration(format!("Validation rule {} failed for business rules population: {}", index + 1, e)));
                        validation_failures += 1;
                    }
                }
            }
            
            if validation_failures > 0 {
                warnings.push(format!("{} out of {} validation rules failed or detected issues", validation_failures, validation_rules.len()));
            } else {
                warnings.push("All validation rules passed successfully".to_string());
            }
        }
        
        // Add summary information
        if total_processed > 0 {
            warnings.push(format!("Business rules population completed: {} records generated in {}", total_processed, junction_table));
        }
        
        if !validation_rules.is_empty() {
            warnings.push(format!("Validation summary: {} rules applied, {} failures detected", validation_rules.len(), validation_failures));
        }
        
        // Update metrics
        let duration = start_time.elapsed();
        self.update_transformation_metrics("business_rules_population", duration, total_failed == 0 && validation_failures == 0);
        
        // Determine success: generation must succeed and validation failures should be minimal
        let success = generation_success && total_failed == 0;
        
        Ok(TransformationResult {
            success,
            records_processed: total_processed,
            records_failed: total_failed + validation_failures,
            errors,
            warnings,
        })
    }
    
    /// Execute external source population
    /// Populates a junction table from external data sources (JSON, CSV, etc.)
    pub async fn execute_external_source_population(
        &self,
        junction_table: &str,
        source_data: &str,
        source_format: &str,
        column_mapping: &std::collections::HashMap<String, String>,
        validation_rules: &[String],
    ) -> Result<TransformationResult> {
        let start_time = Instant::now();
        let mut warnings = Vec::new();
        let mut errors = Vec::new();
        
        // Validate external source population parameters
        if junction_table.is_empty() {
            errors.push(D1RsError::ValidationError("Junction table cannot be empty for external source population".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        if source_data.is_empty() {
            errors.push(D1RsError::ValidationError("Source data cannot be empty for external source population".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        if source_format.is_empty() {
            errors.push(D1RsError::ValidationError("Source format cannot be empty for external source population".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Validate supported source formats
        let supported_formats = ["json", "csv", "array"];
        if !supported_formats.contains(&source_format.to_lowercase().as_str()) {
            errors.push(D1RsError::ValidationError(format!("Unsupported source format '{}' for external source population. Supported formats: {}", source_format, supported_formats.join(", "))));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        if column_mapping.is_empty() {
            errors.push(D1RsError::ValidationError("Column mapping cannot be empty for external source population".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Validate column mapping parameters
        for (source_col, target_col) in column_mapping {
            if source_col.is_empty() || target_col.is_empty() {
                errors.push(D1RsError::ValidationError("Column names in mapping cannot be empty for external source population".to_string()));
                return Ok(TransformationResult {
                    success: false,
                    records_processed: 0,
                    records_failed: 0,
                    errors,
                    warnings,
                });
            }
        }
        
        // Check for duplicate target columns
        let mut target_columns: std::collections::HashSet<&String> = std::collections::HashSet::new();
        for target_col in column_mapping.values() {
            if !target_columns.insert(target_col) {
                errors.push(D1RsError::ValidationError(format!("Duplicate target column '{}' in column mapping for external source population", target_col)));
                return Ok(TransformationResult {
                    success: false,
                    records_processed: 0,
                    records_failed: 0,
                    errors,
                    warnings,
                });
            }
        }
        
        // Validate validation rules for safety
        for (index, rule) in validation_rules.iter().enumerate() {
            if rule.is_empty() {
                errors.push(D1RsError::ValidationError(format!("Validation rule {} cannot be empty for external source population", index + 1)));
                return Ok(TransformationResult {
                    success: false,
                    records_processed: 0,
                    records_failed: 0,
                    errors,
                    warnings,
                });
            }
            
            let rule_trimmed = rule.trim().to_uppercase();
            if rule_trimmed.contains("DROP ") || rule_trimmed.contains("TRUNCATE ") || 
               rule_trimmed.contains("ALTER ") || rule_trimmed.contains("DELETE ") ||
               rule_trimmed.contains("INSERT ") || rule_trimmed.contains("UPDATE ") {
                errors.push(D1RsError::ValidationError(format!("Validation rule {} contains potentially dangerous SQL patterns for external source population", index + 1)));
                return Ok(TransformationResult {
                    success: false,
                    records_processed: 0,
                    records_failed: 0,
                    errors,
                    warnings,
                });
            }
        }
        
        // Validate that junction table is accessible
        let junction_check_sql = format!("SELECT COUNT(*) FROM {} LIMIT 1", junction_table);
        if let Err(_) = self.db.execute_returning_count(&junction_check_sql, &[]).await {
            warnings.push(format!("Junction table {} not accessible, skipping external source population", junction_table));
            return Ok(TransformationResult {
                success: true,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Parse external source data based on format
        let parsed_records = match self.parse_external_source_data(source_data, source_format) {
            Ok(records) => records,
            Err(e) => {
                errors.push(D1RsError::AutoMigration(format!("Failed to parse external source data: {}", e)));
                return Ok(TransformationResult {
                    success: false,
                    records_processed: 0,
                    records_failed: 0,
                    errors,
                    warnings,
                });
            }
        };
        
        if parsed_records.is_empty() {
            warnings.push("No records found in external source data".to_string());
            return Ok(TransformationResult {
                success: true,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        warnings.push(format!("Parsed {} records from external source (format: {})", parsed_records.len(), source_format));
        
        // Get initial record count
        let initial_count_sql = format!("SELECT COUNT(*) FROM {}", junction_table);
        let _initial_records = match self.db.execute_returning_count(&initial_count_sql, &[]).await {
            Ok(count) => count as u64,
            Err(e) => {
                warnings.push(format!("Could not get initial record count for junction table {}: {}", junction_table, e));
                0
            }
        };
        
        // Process records in batches
        let batch_size = self.config.batch_size;
        let mut total_processed = 0u64;
        let mut total_failed = 0u64;
        let mut total_inserted = 0u64;
        
        // Build column lists for SQL generation
        let target_columns: Vec<&String> = column_mapping.values().collect();
        let target_columns_sql = target_columns.iter().map(|&s| s.as_str()).collect::<Vec<&str>>().join(", ");
        let placeholders = vec!["?"; target_columns.len()].join(", ");
        
        let insert_sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            junction_table, target_columns_sql, placeholders
        );
        
        // Process records in batches
        for chunk in parsed_records.chunks(batch_size) {
            for record in chunk {
                total_processed += 1;
                
                // Map source columns to target columns and extract values
                let mut values = Vec::new();
                let mut skip_record = false;
                
                for source_col in column_mapping.keys() {
                    match record.get(source_col) {
                        Some(value) => values.push(value.clone()),
                        None => {
                            warnings.push(format!("Source column '{}' not found in record {}, skipping", source_col, total_processed));
                            skip_record = true;
                            break;
                        }
                    }
                }
                
                if skip_record {
                    total_failed += 1;
                    continue;
                }
                
                // Insert record into junction table
                match self.db.execute(&insert_sql, &values).await {
                    Ok(_) => {
                        total_inserted += 1;
                    }
                    Err(e) => {
                        errors.push(D1RsError::AutoMigration(format!("Failed to insert external record {} into junction table {}: {}", total_processed, junction_table, e)));
                        total_failed += 1;
                    }
                }
            }
        }
        
        // Apply validation rules if provided
        let mut validation_failures = 0u64;
        if !validation_rules.is_empty() {
            warnings.push(format!("Applying {} validation rules to verify external source data integrity", validation_rules.len()));
            
            for (index, validation_rule) in validation_rules.iter().enumerate() {
                match self.db.execute(validation_rule, &[]).await {
                    Ok(result) => {
                        // Check if validation rule returned results
                        if let Some(first_row) = result.rows().first() {
                            if let serde_json::Value::Object(row_map) = first_row {
                                // If the validation rule returns a count, check if it's zero
                                if let Some(serde_json::Value::Number(count)) = row_map.values().next() {
                                    if let Some(count_val) = count.as_u64() {
                                        if count_val == 0 {
                                            validation_failures += 1;
                                            warnings.push(format!("Validation rule {} detected potential issues (returned 0)", index + 1));
                                        } else {
                                            warnings.push(format!("Validation rule {} passed (returned {})", index + 1, count_val));
                                        }
                                    }
                                } else {
                                    warnings.push(format!("Validation rule {} executed successfully", index + 1));
                                }
                            }
                        } else {
                            warnings.push(format!("Validation rule {} executed with no results", index + 1));
                        }
                    }
                    Err(e) => {
                        errors.push(D1RsError::AutoMigration(format!("Validation rule {} failed for external source population: {}", index + 1, e)));
                        validation_failures += 1;
                    }
                }
            }
            
            if validation_failures > 0 {
                warnings.push(format!("{} out of {} validation rules failed or detected issues", validation_failures, validation_rules.len()));
            } else {
                warnings.push("All validation rules passed successfully".to_string());
            }
        }
        
        // Add summary information
        if total_inserted > 0 {
            warnings.push(format!("External source population completed: {} records inserted into {} from {} format", total_inserted, junction_table, source_format));
        }
        
        if total_failed > 0 {
            warnings.push(format!("Failed to process {} out of {} external records", total_failed, total_processed));
        }
        
        if !validation_rules.is_empty() {
            warnings.push(format!("Validation summary: {} rules applied, {} failures detected", validation_rules.len(), validation_failures));
        }
        
        // Update metrics
        let duration = start_time.elapsed();
        self.update_transformation_metrics("external_source_population", duration, total_failed == 0 && validation_failures == 0);
        
        // Determine success: most records should be processed successfully
        let success_threshold = 0.8; // 80% success rate required
        let success_rate = if total_processed > 0 {
            (total_processed - total_failed) as f64 / total_processed as f64
        } else {
            1.0
        };
        
        let success = success_rate >= success_threshold && validation_failures == 0;
        
        Ok(TransformationResult {
            success,
            records_processed: total_processed,
            records_failed: total_failed + validation_failures,
            errors,
            warnings,
        })
    }
    
    /// Parse external source data based on format
    fn parse_external_source_data(&self, source_data: &str, source_format: &str) -> Result<Vec<std::collections::HashMap<String, serde_json::Value>>> {
        match source_format.to_lowercase().as_str() {
            "json" => {
                // Try to parse as JSON array
                match serde_json::from_str::<Vec<serde_json::Value>>(source_data) {
                    Ok(json_array) => {
                        let mut records = Vec::new();
                        for item in json_array {
                            if let serde_json::Value::Object(obj) = item {
                                // Convert serde_json::Map to HashMap
                                let hashmap: std::collections::HashMap<String, serde_json::Value> = obj.into_iter().collect();
                                records.push(hashmap);
                            } else {
                                return Err(D1RsError::ValidationError("JSON array must contain objects".to_string()));
                            }
                        }
                        Ok(records)
                    }
                    Err(_) => {
                        // Try to parse as single JSON object
                        match serde_json::from_str::<serde_json::Value>(source_data) {
                            Ok(serde_json::Value::Object(obj)) => {
                                // Convert serde_json::Map to HashMap
                                let hashmap: std::collections::HashMap<String, serde_json::Value> = obj.into_iter().collect();
                                Ok(vec![hashmap])
                            },
                            Ok(_) => Err(D1RsError::ValidationError("JSON must be an object or array of objects".to_string())),
                            Err(e) => Err(D1RsError::ValidationError(format!("Invalid JSON format: {}", e))),
                        }
                    }
                }
            }
            "csv" => {
                self.parse_csv_data(source_data)
            }
            "array" => {
                // Parse as newline-separated JSON objects
                let mut records = Vec::new();
                for line in source_data.lines() {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<serde_json::Value>(line) {
                        Ok(serde_json::Value::Object(obj)) => {
                            // Convert serde_json::Map to HashMap
                            let hashmap: std::collections::HashMap<String, serde_json::Value> = obj.into_iter().collect();
                            records.push(hashmap);
                        },
                        Ok(_) => return Err(D1RsError::ValidationError("Array format must contain JSON objects".to_string())),
                        Err(e) => return Err(D1RsError::ValidationError(format!("Invalid JSON object in array format: {}", e))),
                    }
                }
                Ok(records)
            }
            _ => Err(D1RsError::ValidationError(format!("Unsupported source format: {}", source_format))),
        }
    }
    
    /// Parse CSV data into records
    fn parse_csv_data(&self, csv_data: &str) -> Result<Vec<std::collections::HashMap<String, serde_json::Value>>> {
        let lines: Vec<&str> = csv_data.lines().collect();
        if lines.is_empty() {
            return Ok(Vec::new());
        }
        
        // First line is headers
        let headers: Vec<&str> = lines[0].split(',').map(|h| h.trim()).collect();
        if headers.is_empty() {
            return Err(D1RsError::ValidationError("CSV must have header row".to_string()));
        }
        
        let mut records = Vec::new();
        
        // Process data rows
        for (line_num, line) in lines.iter().skip(1).enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            
            let values: Vec<&str> = line.split(',').map(|v| v.trim()).collect();
            if values.len() != headers.len() {
                return Err(D1RsError::ValidationError(format!("CSV row {} has {} values but {} headers", line_num + 2, values.len(), headers.len())));
            }
            
            let mut record = std::collections::HashMap::new();
            for (header, value) in headers.iter().zip(values.iter()) {
                // Try to parse as number, otherwise treat as string
                let json_value = if let Ok(num) = value.parse::<i64>() {
                    serde_json::Value::Number(num.into())
                } else if let Ok(float) = value.parse::<f64>() {
                    serde_json::Value::Number(serde_json::Number::from_f64(float).unwrap_or_else(|| 0.into()))
                } else if value.eq_ignore_ascii_case("true") {
                    serde_json::Value::Bool(true)
                } else if value.eq_ignore_ascii_case("false") {
                    serde_json::Value::Bool(false)
                } else if value.eq_ignore_ascii_case("null") || value.is_empty() {
                    serde_json::Value::Null
                } else {
                    serde_json::Value::String(value.to_string())
                };
                
                record.insert(header.to_string(), json_value);
            }
            
            records.push(record);
        }
        
        Ok(records)
    }
}