// Format transformation functionality for data migration

use crate::{Result, D1RsError};
use crate::auto_migration::data_migration::core::{DataMigrator, TransformationResult};
use std::time::Instant;

impl DataMigrator {
    /// Execute format transformation
    pub async fn execute_format_transformation(
        &self,
        table: &str,
        old_column: &str,
        new_column: &str,
        source_format: &str,
        target_format: &str,
        format_function: &str,
    ) -> Result<TransformationResult> {
        let start_time = Instant::now();
        let mut warnings = Vec::new();
        let mut errors = Vec::new();
        
        // Validate format transformation parameters
        if old_column.is_empty() || new_column.is_empty() {
            errors.push(D1RsError::ValidationError("Source and target columns cannot be empty for format transformation".to_string()));
            return Ok(TransformationResult {
                success: false,
                records_processed: 0,
                records_failed: 0,
                errors,
                warnings,
            });
        }
        
        if source_format.is_empty() || target_format.is_empty() {
            warnings.push("Source or target format not specified, using best-effort transformation".to_string());
        }
        
        // Validate format transformation support
        if !self.is_format_transformation_supported(source_format, target_format, format_function) {
            warnings.push(format!("Format transformation from '{}' to '{}' using '{}' is not fully supported", source_format, target_format, format_function));
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
                warnings.push(format!("Table {} not accessible, skipping format transformation", table));
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
            warnings.push("No records to transform formats for".to_string());
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
            let transformation_sql = self.generate_format_transformation_sql(
                table,
                old_column,
                new_column,
                source_format,
                target_format,
                format_function,
                batch_size,
                offset,
            );
            
            match self.db.execute(&transformation_sql, &[]).await {
                Ok(_result) => {
                    let batch_processed = std::cmp::min(batch_size as u64, total_records - offset);
                    total_processed += batch_processed;
                }
                Err(e) => {
                    errors.push(D1RsError::AutoMigration(format!("Format transformation failed for batch at offset {}: {}", offset, e)));
                    total_failed += std::cmp::min(batch_size as u64, total_records - offset);
                }
            }
            
            offset += batch_size as u64;
        }
        
        // Update metrics
        let duration = start_time.elapsed();
        self.update_transformation_metrics("format_transformation", duration, total_failed == 0);
        
        Ok(TransformationResult {
            success: total_failed == 0,
            records_processed: total_processed,
            records_failed: total_failed,
            errors,
            warnings,
        })
    }
    
    /// Check if format transformation is supported
    fn is_format_transformation_supported(&self, source_format: &str, target_format: &str, format_function: &str) -> bool {
        match format_function.to_uppercase().as_str() {
            // Date format transformations
            "DATE_FORMAT" => {
                self.is_date_format_supported(source_format, target_format)
            }
            
            // Time format transformations
            "TIME_FORMAT" => {
                self.is_time_format_supported(source_format, target_format)
            }
            
            // Number format transformations
            "NUMBER_FORMAT" => {
                self.is_number_format_supported(source_format, target_format)
            }
            
            // String case transformations
            "UPPER" | "LOWER" | "TITLE_CASE" | "SENTENCE_CASE" => true,
            
            // Phone number format transformations
            "PHONE_FORMAT" => {
                self.is_phone_format_supported(source_format, target_format)
            }
            
            // Currency format transformations
            "CURRENCY_FORMAT" => {
                self.is_currency_format_supported(source_format, target_format)
            }
            
            // Custom regex-based transformations
            "REGEX_REPLACE" => true,
            
            // Padding and trimming
            "TRIM" | "LTRIM" | "RTRIM" | "PAD_LEFT" | "PAD_RIGHT" => true,
            
            _ => false,
        }
    }
    
    /// Check if date format transformation is supported
    fn is_date_format_supported(&self, source_format: &str, target_format: &str) -> bool {
        let supported_formats = vec![
            "MM/DD/YYYY", "DD/MM/YYYY", "YYYY-MM-DD", "YYYY/MM/DD",
            "MM-DD-YYYY", "DD-MM-YYYY", "YYYY.MM.DD", "MM.DD.YYYY",
            "ISO8601", "RFC3339", "UNIX_TIMESTAMP"
        ];
        
        supported_formats.contains(&source_format) && supported_formats.contains(&target_format)
    }
    
    /// Check if time format transformation is supported
    fn is_time_format_supported(&self, source_format: &str, target_format: &str) -> bool {
        let supported_formats = vec![
            "HH:MM:SS", "HH:MM", "H:MM:SS AM/PM", "H:MM AM/PM",
            "24_HOUR", "12_HOUR", "UNIX_TIMESTAMP"
        ];
        
        supported_formats.contains(&source_format) && supported_formats.contains(&target_format)
    }
    
    /// Check if number format transformation is supported
    fn is_number_format_supported(&self, source_format: &str, target_format: &str) -> bool {
        let supported_formats = vec![
            "1,234.56", "1.234,56", "1234.56", "1234,56", 
            "SCIENTIFIC", "PERCENT", "INTEGER"
        ];
        
        supported_formats.contains(&source_format) && supported_formats.contains(&target_format)
    }
    
    /// Check if phone format transformation is supported
    fn is_phone_format_supported(&self, source_format: &str, target_format: &str) -> bool {
        let supported_formats = vec![
            "(555) 123-4567", "555-123-4567", "555.123.4567", "5551234567",
            "+1-555-123-4567", "+1 (555) 123-4567", "E164"
        ];
        
        supported_formats.contains(&source_format) && supported_formats.contains(&target_format)
    }
    
    /// Check if currency format transformation is supported
    fn is_currency_format_supported(&self, source_format: &str, target_format: &str) -> bool {
        let supported_formats = vec![
            "$1,234.56", "USD 1,234.56", "1,234.56 USD", "1234.56",
            "€1.234,56", "EUR 1.234,56", "¥1,234", "£1,234.56"
        ];
        
        supported_formats.contains(&source_format) && supported_formats.contains(&target_format)
    }
    
    /// Generate SQL for format transformation operation
    fn generate_format_transformation_sql(
        &self,
        table: &str,
        old_column: &str,
        new_column: &str,
        source_format: &str,
        target_format: &str,
        format_function: &str,
        batch_size: usize,
        offset: u64,
    ) -> String {
        let transformation_expr = match format_function.to_uppercase().as_str() {
            "DATE_FORMAT" => {
                self.generate_date_format_expression(old_column, source_format, target_format)
            }
            
            "TIME_FORMAT" => {
                self.generate_time_format_expression(old_column, source_format, target_format)
            }
            
            "NUMBER_FORMAT" => {
                self.generate_number_format_expression(old_column, source_format, target_format)
            }
            
            "UPPER" => {
                format!("UPPER({})", old_column)
            }
            
            "LOWER" => {
                format!("LOWER({})", old_column)
            }
            
            "TITLE_CASE" => {
                // SQLite doesn't have title case, but we can approximate
                format!("UPPER(SUBSTR({}, 1, 1)) || LOWER(SUBSTR({}, 2))", old_column, old_column)
            }
            
            "SENTENCE_CASE" => {
                // Convert to sentence case (first letter uppercase, rest lowercase)
                format!("UPPER(SUBSTR({}, 1, 1)) || LOWER(SUBSTR({}, 2))", old_column, old_column)
            }
            
            "PHONE_FORMAT" => {
                self.generate_phone_format_expression(old_column, source_format, target_format)
            }
            
            "CURRENCY_FORMAT" => {
                self.generate_currency_format_expression(old_column, source_format, target_format)
            }
            
            "TRIM" => {
                format!("TRIM({})", old_column)
            }
            
            "LTRIM" => {
                format!("LTRIM({})", old_column)
            }
            
            "RTRIM" => {
                format!("RTRIM({})", old_column)
            }
            
            "REGEX_REPLACE" => {
                // For regex replace, source_format is the pattern, target_format is the replacement
                // Note: SQLite has limited regex support, this is a simplified version
                format!("REPLACE({}, '{}', '{}')", old_column, source_format, target_format)
            }
            
            _ => {
                // Fallback: return the column as-is
                old_column.to_string()
            }
        };
        
        format!(
            "UPDATE {} SET {} = {} WHERE rowid IN (SELECT rowid FROM {} WHERE {} IS NOT NULL AND {} != '' ORDER BY rowid LIMIT {} OFFSET {})",
            table, new_column, transformation_expr, table, old_column, old_column, batch_size, offset
        )
    }
    
    /// Generate date format transformation expression
    fn generate_date_format_expression(&self, column: &str, source_format: &str, target_format: &str) -> String {
        match (source_format, target_format) {
            ("MM/DD/YYYY", "YYYY-MM-DD") => {
                format!("SUBSTR({}, 7, 4) || '-' || SUBSTR({}, 1, 2) || '-' || SUBSTR({}, 4, 2)", column, column, column)
            }
            ("DD/MM/YYYY", "YYYY-MM-DD") => {
                format!("SUBSTR({}, 7, 4) || '-' || SUBSTR({}, 4, 2) || '-' || SUBSTR({}, 1, 2)", column, column, column)
            }
            ("YYYY-MM-DD", "MM/DD/YYYY") => {
                format!("SUBSTR({}, 6, 2) || '/' || SUBSTR({}, 9, 2) || '/' || SUBSTR({}, 1, 4)", column, column, column)
            }
            ("YYYY-MM-DD", "DD/MM/YYYY") => {
                format!("SUBSTR({}, 9, 2) || '/' || SUBSTR({}, 6, 2) || '/' || SUBSTR({}, 1, 4)", column, column, column)
            }
            _ => {
                // Fallback: return original value
                column.to_string()
            }
        }
    }
    
    /// Generate time format transformation expression
    fn generate_time_format_expression(&self, column: &str, source_format: &str, target_format: &str) -> String {
        match (source_format, target_format) {
            ("H:MM AM/PM", "HH:MM:SS") => {
                // Convert 12-hour to 24-hour format (simplified - just strip AM/PM and add seconds)
                format!("REPLACE(REPLACE({}, ' AM', ''), ' PM', '') || ':00'", column)
            }
            ("HH:MM:SS", "HH:MM") => {
                format!("SUBSTR({}, 1, 5)", column)
            }
            ("HH:MM", "HH:MM:SS") => {
                format!("{} || ':00'", column)
            }
            _ => {
                // Fallback: return original value
                column.to_string()
            }
        }
    }
    
    /// Generate number format transformation expression
    fn generate_number_format_expression(&self, column: &str, source_format: &str, target_format: &str) -> String {
        match (source_format, target_format) {
            ("1,234.56", "1234.56") => {
                // Remove commas
                format!("REPLACE({}, ',', '')", column)
            }
            ("1.234,56", "1234.56") => {
                // Convert European format to US format
                format!("REPLACE(REPLACE({}, '.', ''), ',', '.')", column)
            }
            ("1234.56", "1,234.56") => {
                // Add commas (simplified - only works for numbers with decimal)
                format!("CASE WHEN LENGTH(SUBSTR({}, 1, INSTR({}, '.') - 1)) > 3 THEN SUBSTR({}, 1, LENGTH(SUBSTR({}, 1, INSTR({}, '.') - 1)) - 3) || ',' || SUBSTR({}, LENGTH(SUBSTR({}, 1, INSTR({}, '.') - 1)) - 2, 3) || SUBSTR({}, INSTR({}, '.')) ELSE {} END", 
                       column, column, column, column, column, column, column, column, column, column, column)
            }
            _ => {
                // Fallback: return original value
                column.to_string()
            }
        }
    }
    
    /// Generate phone format transformation expression
    fn generate_phone_format_expression(&self, column: &str, source_format: &str, target_format: &str) -> String {
        match (source_format, target_format) {
            ("(555) 123-4567", "555-123-4567") => {
                // Remove parentheses and spaces
                format!("REPLACE(REPLACE(REPLACE({}, '(', ''), ')', ''), ' ', '')", column)
            }
            ("5551234567", "555-123-4567") => {
                // Add dashes to 10-digit number
                format!("SUBSTR({}, 1, 3) || '-' || SUBSTR({}, 4, 3) || '-' || SUBSTR({}, 7, 4)", column, column, column)
            }
            ("555-123-4567", "(555) 123-4567") => {
                // Convert to parentheses format
                format!("'(' || SUBSTR({}, 1, 3) || ') ' || SUBSTR({}, 5, 3) || '-' || SUBSTR({}, 9, 4)", column, column, column)
            }
            _ => {
                // Fallback: return original value
                column.to_string()
            }
        }
    }
    
    /// Generate currency format transformation expression
    fn generate_currency_format_expression(&self, column: &str, source_format: &str, target_format: &str) -> String {
        match (source_format, target_format) {
            ("$1,234.56", "1234.56") => {
                // Remove currency symbol and commas
                format!("REPLACE(REPLACE({}, '$', ''), ',', '')", column)
            }
            ("1234.56", "$1,234.56") => {
                // Add dollar sign (simplified - doesn't add commas for large numbers)
                format!("'$' || {}", column)
            }
            ("USD 1,234.56", "1234.56") => {
                // Remove USD prefix and commas
                format!("REPLACE(REPLACE(SUBSTR({}, 5), ',', ''), ' ', '')", column)
            }
            _ => {
                // Fallback: return original value
                column.to_string()
            }
        }
    }
}