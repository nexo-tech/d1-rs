// REVOLUTIONARY Phase 4.2 Comprehensive Test Suite
// Tests the standardized error handling system across all modules

use d1_rs::*;

/// Test the new unified migration error types and formatting

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_error_type_display() {
        // Test that all MigrationErrorType variants have proper Display implementation
        let error_types = vec![
            // Data migration errors
            MigrationErrorType::TypeConversionFailed,
            MigrationErrorType::ForeignKeyViolation,
            MigrationErrorType::DataIntegrityViolation,
            MigrationErrorType::TransformationLogicError,
            MigrationErrorType::DuplicateKeyError,
            MigrationErrorType::ConstraintViolation,
            MigrationErrorType::InsufficientPermissions,
            MigrationErrorType::TimeoutError,
            
            // Schema change errors
            MigrationErrorType::TableRestructuringFailed,
            MigrationErrorType::ColumnModificationFailed,
            MigrationErrorType::IndexCreationFailed,
            MigrationErrorType::RelationshipEvolutionFailed,
            MigrationErrorType::JunctionTableCreationFailed,
            
            // Validation errors
            MigrationErrorType::SchemaValidationFailed,
            MigrationErrorType::BackupCreationFailed,
            MigrationErrorType::RollbackFailed,
            MigrationErrorType::IntegrityCheckFailed,
            
            // Performance errors
            MigrationErrorType::MemoryLimitExceeded,
            MigrationErrorType::ProcessingTimeoutError,
            MigrationErrorType::ResourceExhausted,
        ];
        
        for error_type in error_types {
            let formatted = error_type.to_string();
            assert!(!formatted.is_empty(), "Error type {:?} should have non-empty display", error_type);
            assert!(!formatted.contains("Debug"), "Error type should have human-readable display, not debug: {}", formatted);
            assert!(formatted.len() > 5, "Error message should be descriptive: {}", formatted);
        }
    }

    #[test]
    fn test_data_migration_error_creation() {
        // Test the convenient error creation helpers
        
        let error = D1RsError::data_migration(
            "column_type_conversion",
            MigrationErrorType::TypeConversionFailed,
            "Consider using a custom transformation function"
        );
        
        let error_string = error.to_string();
        assert!(error_string.contains("Data migration failed"));
        assert!(error_string.contains("column_type_conversion"));
        assert!(error_string.contains("Type conversion failed"));
        assert!(error_string.contains("Consider using a custom transformation function"));
    }

    #[test]
    fn test_data_migration_error_with_table() {
        let error = D1RsError::data_migration_with_table(
            "foreign_key_update",
            "users",
            MigrationErrorType::ForeignKeyViolation,
            "Check that all referenced records exist before migration"
        );
        
        let error_string = error.to_string();
        assert!(error_string.contains("on table 'users'"));
        assert!(error_string.contains("Foreign key constraint violation"));
        assert!(error_string.contains("Check that all referenced records exist"));
    }

    #[test]
    fn test_data_migration_error_detailed() {
        let recovery_actions = vec![
            "Backup the affected table".to_string(),
            "Review the transformation logic".to_string(),
            "Check data integrity constraints".to_string(),
        ];
        
        let error = D1RsError::data_migration_detailed(
            "complex_transformation",
            Some("posts".to_string()),
            Some("author_id".to_string()),
            Some("12345".to_string()),
            MigrationErrorType::DataIntegrityViolation,
            "Data integrity check failed during transformation",
            recovery_actions
        );
        
        let error_string = error.to_string();
        assert!(error_string.contains("posts"));
        assert!(error_string.contains("author_id"));
        assert!(error_string.contains("12345"));
        assert!(error_string.contains("Data integrity violation"));
        assert!(error_string.contains("Recovery actions:"));
        assert!(error_string.contains("1. Backup the affected table"));
        assert!(error_string.contains("2. Review the transformation logic"));
        assert!(error_string.contains("3. Check data integrity constraints"));
    }

    #[test]
    fn test_schema_change_error() {
        let recovery_actions = vec![
            "Rollback to previous schema state".to_string(),
            "Check SQLite version compatibility".to_string(),
        ];
        
        let error = D1RsError::schema_change(
            "add_column_with_not_null",
            "users",
            "SQLite does not support adding NOT NULL columns without a DEFAULT value",
            recovery_actions
        );
        
        let error_string = error.to_string();
        assert!(error_string.contains("Schema change failed"));
        assert!(error_string.contains("add_column_with_not_null"));
        assert!(error_string.contains("users"));
        assert!(error_string.contains("SQLite does not support"));
        assert!(error_string.contains("Recovery actions:"));
        assert!(error_string.contains("Rollback to previous schema state"));
    }

    #[test]
    fn test_schema_change_error_with_affected_tables() {
        let recovery_actions = vec![
            "Create backup of all affected tables".to_string(),
        ];
        
        let affected_tables = vec![
            "users".to_string(),
            "posts".to_string(),
            "user_posts_junction".to_string(),
        ];
        
        let error = D1RsError::schema_change_with_affected(
            "restructure_relationships",
            "users",
            "Circular dependency detected in foreign key relationships",
            recovery_actions,
            affected_tables
        );
        
        let error_string = error.to_string();
        assert!(error_string.contains("Affected tables: users, posts, user_posts_junction"));
        assert!(error_string.contains("Circular dependency detected"));
    }

    #[test]
    fn test_migration_validation_error() {
        let issues = vec![
            "Foreign key 'user_id' references non-existent table 'old_users'".to_string(),
            "Column 'status' has incompatible type change from TEXT to INTEGER".to_string(),
        ];
        
        let suggestions = vec![
            "Update foreign key to reference 'users' table".to_string(),
            "Add intermediate migration step for type conversion".to_string(),
        ];
        
        let error = D1RsError::migration_validation(
            "schema_consistency_check",
            issues,
            suggestions,
            true
        );
        
        let error_string = error.to_string();
        assert!(error_string.contains("🔥 CRITICAL"));
        assert!(error_string.contains("schema_consistency_check"));
        assert!(error_string.contains("Issues found:"));
        assert!(error_string.contains("Foreign key 'user_id'"));
        assert!(error_string.contains("Suggestions:"));
        assert!(error_string.contains("Update foreign key to reference"));
    }

    #[test]
    fn test_migration_validation_warning() {
        let issues = vec![
            "Performance warning: Large table 'analytics_events' may take time to migrate".to_string(),
        ];
        
        let suggestions = vec![
            "Consider running migration during low-traffic hours".to_string(),
        ];
        
        let error = D1RsError::migration_validation(
            "performance_analysis",
            issues,
            suggestions,
            false
        );
        
        let error_string = error.to_string();
        assert!(error_string.contains("⚠️  WARNING"));
        assert!(!error_string.contains("CRITICAL"));
        assert!(error_string.contains("performance_analysis"));
    }

    #[test]
    fn test_error_type_equality() {
        // Test that MigrationErrorType implements PartialEq correctly
        assert_eq!(MigrationErrorType::TypeConversionFailed, MigrationErrorType::TypeConversionFailed);
        assert_ne!(MigrationErrorType::TypeConversionFailed, MigrationErrorType::ForeignKeyViolation);
        
        // Test with match patterns
        let error_type = MigrationErrorType::DataIntegrityViolation;
        match error_type {
            MigrationErrorType::DataIntegrityViolation => {
                // This should match
            },
            _ => panic!("Pattern matching should work correctly"),
        }
    }

    #[test]
    fn test_comprehensive_error_formatting() {
        // Test that error formatting includes all necessary elements for debugging
        
        let complex_error = D1RsError::data_migration_detailed(
            "multi_table_transformation",
            Some("complex_table".to_string()),
            Some("complex_column".to_string()),
            Some("complex_record_123".to_string()),
            MigrationErrorType::TransformationLogicError,
            "Custom transformation function returned invalid result",
            vec![
                "Review transformation function logic".to_string(),
                "Check input data format".to_string(),
                "Validate transformation parameters".to_string(),
                "Consider rollback if issues persist".to_string(),
            ]
        );
        
        let formatted = complex_error.to_string();
        
        // Verify all components are present
        assert!(formatted.contains("🚨"));
        assert!(formatted.contains("multi_table_transformation"));
        assert!(formatted.contains("complex_table"));
        assert!(formatted.contains("complex_column"));
        assert!(formatted.contains("complex_record_123"));
        assert!(formatted.contains("❌ Error:"));
        assert!(formatted.contains("💡 Suggestion:"));
        assert!(formatted.contains("🔧 Recovery actions:"));
        assert!(formatted.contains("1. Review transformation function logic"));
        assert!(formatted.contains("4. Consider rollback if issues persist"));
    }

    #[test]
    fn test_error_chaining_and_context() {
        // Test that errors can be properly chained and provide useful context
        
        let base_error = D1RsError::data_migration(
            "initial_operation",
            MigrationErrorType::MemoryLimitExceeded,
            "Reduce batch size for large dataset operations"
        );
        
        // In real usage, this would be part of a Result chain
        let error_string = base_error.to_string();
        assert!(error_string.contains("Memory limit exceeded"));
        assert!(error_string.contains("Reduce batch size"));
    }

    #[test]
    fn test_error_message_helpfulness() {
        // Test that error messages are genuinely helpful and actionable
        
        let helpful_error = D1RsError::schema_change(
            "add_foreign_key_constraint",
            "posts",
            "Cannot add foreign key constraint because orphaned records exist",
            vec![
                "Run query: SELECT * FROM posts WHERE user_id NOT IN (SELECT id FROM users)".to_string(),
                "Either delete orphaned records or update them with valid user_id".to_string(),
                "After cleanup, re-run the migration".to_string(),
            ]
        );
        
        let formatted = helpful_error.to_string();
        assert!(formatted.contains("SELECT * FROM posts WHERE user_id NOT IN"));
        assert!(formatted.contains("delete orphaned records"));
        assert!(formatted.contains("re-run the migration"));
        
        // Verify the error provides concrete, actionable steps
        assert!(formatted.chars().filter(|&c| c == '.').count() >= 3); // Multiple sentences
        assert!(formatted.contains("SELECT")); // Contains actual SQL
    }

    #[test]
    fn test_error_consistency_across_modules() {
        // Test that all error types follow consistent formatting patterns
        
        let migration_error = D1RsError::data_migration(
            "test_op",
            MigrationErrorType::TypeConversionFailed,
            "test suggestion"
        );
        
        let schema_error = D1RsError::schema_change(
            "test_op",
            "test_table",
            "test reason",
            vec!["test recovery".to_string()]
        );
        
        let validation_error = D1RsError::migration_validation(
            "test_validation",
            vec!["test issue".to_string()],
            vec!["test suggestion".to_string()],
            true
        );
        
        // All should contain emoji indicators
        assert!(migration_error.to_string().contains("🚨"));
        assert!(schema_error.to_string().contains("🚨"));
        assert!(validation_error.to_string().contains("🔥"));
        
        // All should have structured formatting
        for error in [&migration_error, &schema_error, &validation_error] {
            let formatted = error.to_string();
            assert!(formatted.len() > 50, "Error should be descriptive: {}", formatted);
            assert!(formatted.contains('\n'), "Error should be multi-line for readability");
        }
    }

    #[test]
    fn test_revolutionary_error_integration() {
        // Test that demonstrates the complete Phase 4.2 error handling system
        // showing how migration operations create proper standardized errors
        
        // Simulate a complex migration scenario with multiple error types
        let mut errors = Vec::new();
        
        // 1. Data migration error during type conversion
        errors.push(D1RsError::data_migration_detailed(
            "convert_user_scores_to_integer",
            Some("user_stats".to_string()),
            Some("score".to_string()),
            Some("user_12345".to_string()),
            MigrationErrorType::TypeConversionFailed,
            "Cannot convert 'high' to integer. Use a mapping function or provide default values.",
            vec![
                "UPDATE user_stats SET score = 0 WHERE score NOT LIKE '%[0-9]%'".to_string(),
                "Add a pre-migration cleanup step".to_string(),
                "Consider using an enum type instead".to_string(),
            ]
        ));
        
        // 2. Schema change error during table restructuring  
        errors.push(D1RsError::schema_change_with_affected(
            "add_foreign_key_constraints",
            "posts",
            "Cannot add foreign key constraint due to orphaned records",
            vec![
                "Identify orphaned records: SELECT * FROM posts WHERE user_id NOT IN (SELECT id FROM users)".to_string(),
                "Either delete orphaned records or assign them to a default user".to_string(),
                "Re-run the migration after cleanup".to_string(),
            ],
            vec!["posts".to_string(), "users".to_string(), "user_posts_junction".to_string()]
        ));
        
        // 3. Migration validation warning
        errors.push(D1RsError::migration_validation(
            "performance_impact_analysis",
            vec![
                "Large table 'analytics_events' (10M+ rows) will take significant time to migrate".to_string(),
                "Index creation on 'user_id' column may cause temporary performance degradation".to_string(),
            ],
            vec![
                "Consider running migration during maintenance window".to_string(),
                "Enable progress monitoring with --verbose flag".to_string(),
                "Ensure adequate disk space for temporary tables".to_string(),
            ],
            false  // This is a warning, not critical
        ));
        
        // Test that all errors are properly formatted and contain actionable information
        for (i, error) in errors.iter().enumerate() {
            let formatted = error.to_string();
            
            println!("Error {}: {}", i + 1, formatted);
            
            // All errors should be helpful and actionable
            assert!(formatted.contains("🚨") || formatted.contains("⚠️") || formatted.contains("🔥"), "Error should have severity indicator");
            assert!(formatted.contains("💡") || formatted.contains("Suggestions:") || formatted.contains("🔧"), "Error should have suggestions or recovery actions");
            assert!(formatted.len() > 100, "Error should be comprehensive");
            
            // Should contain concrete actions the user can take
            assert!(
                formatted.contains("SELECT") || 
                formatted.contains("UPDATE") || 
                formatted.contains("--") ||
                formatted.contains("Consider") ||
                formatted.contains("Re-run"),
                "Error should contain actionable advice"
            );
        }
        
        // Test that errors can be serialized/deserialized for storage
        let json = serde_json::to_string(&errors[0]).expect("Should serialize error");
        let deserialized: D1RsError = serde_json::from_str(&json).expect("Should deserialize error");
        
        // Should maintain all information after round-trip
        assert_eq!(format!("{:?}", errors[0]), format!("{:?}", deserialized));
    }

    #[test]
    fn test_migration_error_types_comprehensive() {
        // Test that all MigrationErrorType variants are properly handled
        let all_error_types = vec![
            // Data migration errors
            (MigrationErrorType::TypeConversionFailed, "Type conversion failed"),
            (MigrationErrorType::ForeignKeyViolation, "Foreign key constraint violation"),
            (MigrationErrorType::DataIntegrityViolation, "Data integrity violation"), 
            (MigrationErrorType::TransformationLogicError, "Transformation logic error"),
            (MigrationErrorType::DuplicateKeyError, "Duplicate key error"),
            (MigrationErrorType::ConstraintViolation, "Database constraint violation"),
            (MigrationErrorType::InsufficientPermissions, "Insufficient permissions"),
            (MigrationErrorType::TimeoutError, "Operation timeout"),
            
            // Schema change errors
            (MigrationErrorType::TableRestructuringFailed, "Table restructuring failed"),
            (MigrationErrorType::ColumnModificationFailed, "Column modification failed"),
            (MigrationErrorType::IndexCreationFailed, "Index creation failed"),
            (MigrationErrorType::RelationshipEvolutionFailed, "Relationship evolution failed"),
            (MigrationErrorType::JunctionTableCreationFailed, "Junction table creation failed"),
            
            // Validation errors
            (MigrationErrorType::SchemaValidationFailed, "Schema validation failed"),
            (MigrationErrorType::BackupCreationFailed, "Backup creation failed"),
            (MigrationErrorType::RollbackFailed, "Rollback operation failed"),
            (MigrationErrorType::IntegrityCheckFailed, "Integrity check failed"),
            
            // Performance errors
            (MigrationErrorType::MemoryLimitExceeded, "Memory limit exceeded"),
            (MigrationErrorType::ProcessingTimeoutError, "Processing timeout"),
            (MigrationErrorType::ResourceExhausted, "System resources exhausted"),
        ];
        
        for (error_type, expected_display) in all_error_types {
            // Test Display implementation
            assert_eq!(error_type.to_string(), expected_display);
            
            // Test that errors can be created with this type
            let error = D1RsError::data_migration(
                "test_operation",
                error_type.clone(),
                "test suggestion"
            );
            
            let formatted = error.to_string();
            assert!(formatted.contains(expected_display));
            assert!(formatted.contains("test_operation"));
            assert!(formatted.contains("test suggestion"));
            
            // Test serialization works for all error types
            let _json = serde_json::to_string(&error_type).expect("Should serialize error type");
        }
    }
}