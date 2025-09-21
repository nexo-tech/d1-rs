// Data normalization functionality for data migration

use crate::{Result, D1RsError};
use crate::auto_migration::data_migration::core::{DataMigrator, TransformationResult};
use std::time::Instant;

impl DataMigrator {
    /// Execute data normalization transformation
    pub async fn execute_normalization(
        &self,
        table: &str,
        old_column: &str,
        source_pattern: &str,
        target_fields: &[String],
        extraction_rules: &[String],
    ) -> Result<TransformationResult> {
        let start_time = Instant::now();
        let mut warnings = Vec::new();
        let mut errors = Vec::new();
        
        // Validate normalization parameters
        if target_fields.is_empty() {
            warnings.push("No target fields specified for normalization".to_string());
            return Ok(TransformationResult {
                success: true,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        if target_fields.len() != extraction_rules.len() {
            errors.push(D1RsError::ValidationError("Target fields and extraction rules count mismatch".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Validate extraction rules
        if !self.are_extraction_rules_supported(extraction_rules) {
            warnings.push(format!("Some extraction rules are not supported: {:?}", extraction_rules));
            return Ok(TransformationResult {
                success: true,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        // Get total record count - handle missing table gracefully
        let count_sql = format!("SELECT COUNT(*) FROM {} WHERE {} IS NOT NULL AND {} != ''", table, old_column, old_column);
        let total_records = match self.db.execute_returning_count(&count_sql, &[]).await {
            Ok(count) => count as u64,
            Err(_) => {
                // Table doesn't exist or other error - treat as success with 0 records
                warnings.push(format!("Table {} not accessible, skipping normalization", table));
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
            warnings.push("No records to normalize".to_string());
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
            let normalization_sql = self.generate_normalization_sql(
                table,
                old_column,
                source_pattern,
                target_fields,
                extraction_rules,
                batch_size,
                offset,
            );
            
            match self.db.execute(&normalization_sql, &[]).await {
                Ok(_result) => {
                    let batch_processed = std::cmp::min(batch_size as u64, total_records - offset);
                    total_processed += batch_processed;
                    
                    // Progress tracking can be added here if needed
                }
                Err(e) => {
                    errors.push(D1RsError::AutoMigration(format!("Normalization failed for batch at offset {}: {}", offset, e)));
                    total_failed += std::cmp::min(batch_size as u64, total_records - offset);
                }
            }
            
            offset += batch_size as u64;
        }
        
        // Update metrics
        let duration = start_time.elapsed();
        self.update_transformation_metrics("normalization", duration, total_failed == 0);
        
        Ok(TransformationResult {
            success: total_failed == 0,
            records_processed: total_processed,
            records_failed: total_failed,
            errors,
            warnings,
        })
    }
    
    /// Check if extraction rules are supported
    fn are_extraction_rules_supported(&self, extraction_rules: &[String]) -> bool {
        extraction_rules.iter().all(|rule| {
            match rule.to_uppercase().as_str() {
                // Support common extraction patterns
                "SUBSTR" | "SUBSTRING" => true,
                "TRIM" | "LTRIM" | "RTRIM" => true,
                "SPLIT_PART" => true,
                "REGEX_EXTRACT" => true,
                "SPACE_SPLIT_FIRST" => true,
                "SPACE_SPLIT_LAST" => true,
                "COMMA_SPLIT_FIRST" => true,
                "COMMA_SPLIT_LAST" => true,
                _ if rule.starts_with("SUBSTR(") => true,
                _ if rule.starts_with("TRIM(") => true,
                _ => false,
            }
        })
    }
    
    /// Generate SQL for data normalization
    fn generate_normalization_sql(
        &self,
        table: &str,
        old_column: &str,
        source_pattern: &str,
        target_fields: &[String],
        extraction_rules: &[String],
        batch_size: usize,
        offset: u64,
    ) -> String {
        // Build SET clauses for each target field using its extraction rule
        let set_clauses: Vec<String> = target_fields.iter()
            .zip(extraction_rules.iter())
            .map(|(field, rule)| {
                let extraction_expr = self.generate_extraction_expression(old_column, rule, source_pattern);
                format!("{} = {}", field, extraction_expr)
            })
            .collect();
        
        format!(
            "UPDATE {} SET {} WHERE rowid IN (SELECT rowid FROM {} WHERE {} IS NOT NULL AND {} != '' ORDER BY rowid LIMIT {} OFFSET {})",
            table,
            set_clauses.join(", "),
            table,
            old_column,
            old_column,
            batch_size,
            offset
        )
    }
    
    /// Generate extraction expression based on rule type
    fn generate_extraction_expression(&self, column: &str, rule: &str, _pattern: &str) -> String {
        match rule.to_uppercase().as_str() {
            "SPACE_SPLIT_FIRST" => {
                format!("CASE WHEN INSTR({}, ' ') > 0 THEN SUBSTR({}, 1, INSTR({}, ' ') - 1) ELSE {} END", column, column, column, column)
            }
            "SPACE_SPLIT_LAST" => {
                format!("CASE WHEN INSTR({}, ' ') > 0 THEN SUBSTR({}, INSTR({}, ' ') + 1) ELSE '' END", column, column, column)
            }
            "COMMA_SPLIT_FIRST" => {
                format!("CASE WHEN INSTR({}, ',') > 0 THEN TRIM(SUBSTR({}, 1, INSTR({}, ',') - 1)) ELSE TRIM({}) END", column, column, column, column)
            }
            "COMMA_SPLIT_LAST" => {
                format!("CASE WHEN INSTR({}, ',') > 0 THEN TRIM(SUBSTR({}, INSTR({}, ',') + 1)) ELSE '' END", column, column, column)
            }
            "TRIM" => format!("TRIM({})", column),
            "LTRIM" => format!("LTRIM({})", column),
            "RTRIM" => format!("RTRIM({})", column),
            _ if rule.starts_with("SUBSTR(") => {
                // Handle custom SUBSTR rules like "SUBSTR(1,10)"
                rule.replace("SUBSTR(", &format!("SUBSTR({}, ", column))
            }
            _ if rule.starts_with("TRIM(") => {
                // Handle custom TRIM rules
                rule.replace("TRIM(", &format!("TRIM({}", column)).replace(")", ")")
            }
            _ => {
                // Default: return the column as-is if rule is not recognized
                column.to_string()
            }
        }
    }
}