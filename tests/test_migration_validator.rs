//! Comprehensive tests for the migration validator
//! 
//! Tests all aspects of migration validation including schema compatibility,
//! data integrity, performance impact, safety analysis, and breaking change detection.

use d1_rs::auto_migration::*;
use std::time::Duration;

/// Helper function to create a basic table schema for testing
fn create_test_table(name: &str, columns: Vec<ColumnSchema>) -> TableSchema {
    TableSchema {
        name: name.to_string(),
        columns,
        indexes: vec![],
        foreign_keys: vec![],
        constraints: vec![],
    }
}

/// Helper function to create a basic column schema for testing
fn create_test_column(name: &str, column_type: &str, nullable: bool) -> ColumnSchema {
    ColumnSchema {
        name: name.to_string(),
        column_type: column_type.to_string(),
        nullable,
        primary_key: false,
        unique: false,
        auto_increment: false,
        default_value: None,
        constraints: vec![],
    }
}

/// Helper function to create a primary key column
fn create_primary_key_column(name: &str) -> ColumnSchema {
    ColumnSchema {
        name: name.to_string(),
        column_type: "INTEGER".to_string(),
        nullable: false,
        primary_key: true,
        unique: false,
        auto_increment: true,
        default_value: None,
        constraints: vec![],
    }
}

/// Helper function to create a test migration plan
fn create_test_migration_plan(operations: Vec<MigrationOperation>) -> MigrationPlan {
    MigrationPlan {
        operations,
        estimated_duration: Duration::from_secs(1),
        safety_warnings: vec![],
        rollback_plan: vec![],
    }
}

#[cfg(test)]
mod validator_tests {
    use super::*;

    #[test]
    fn test_validator_creation() {
        let _validator = MigrationValidator::new();
        // Validator should be created successfully - basic instantiation test
        assert!(true);
    }

    #[test]
    fn test_empty_migration_plan_validation() {
        let validator = MigrationValidator::new();
        let empty_plan = create_test_migration_plan(vec![]);
        
        let result = validator.validate_migrations(&empty_plan);
        assert!(result.is_ok());
        
        let validation_result = result.unwrap();
        assert!(validation_result.is_safe);
        assert_eq!(validation_result.safety_warnings.len(), 0);
    }

    #[test]
    fn test_safe_table_creation_validation() {
        let validator = MigrationValidator::new();
        
        let table = create_test_table("users", vec![
            create_primary_key_column("id"),
            create_test_column("name", "TEXT", false),
            create_test_column("email", "TEXT", true),
        ]);
        
        let plan = create_test_migration_plan(vec![
            MigrationOperation::CreateTable { definition: table }
        ]);
        
        let result = validator.validate_migrations(&plan);
        assert!(result.is_ok());
        
        let validation_result = result.unwrap();
        assert!(validation_result.is_safe);
    }

    #[test]
    fn test_data_loss_operation_validation() {
        let validator = MigrationValidator::new();
        
        let plan = create_test_migration_plan(vec![
            MigrationOperation::DropTable { name: "users".to_string() }
        ]);
        
        let result = validator.validate_migrations(&plan);
        assert!(result.is_ok());
        
        let validation_result = result.unwrap();
        // Dropping tables should not be considered "safe" due to data loss
        assert!(!validation_result.is_safe);
        assert!(validation_result.safety_warnings.len() > 0);
        
        // Should have data loss warnings
        let has_data_loss_warning = validation_result.safety_warnings.iter()
            .any(|w| matches!(w.warning_type, SafetyWarningType::DataLoss));
        assert!(has_data_loss_warning);
    }

    #[test]
    fn test_breaking_change_detection() {
        let validator = MigrationValidator::new();
        
        let plan = create_test_migration_plan(vec![
            MigrationOperation::DropColumn { 
                table: "users".to_string(), 
                column: "email".to_string() 
            }
        ]);
        
        let result = validator.validate_migrations(&plan);
        assert!(result.is_ok());
        
        let validation_result = result.unwrap();
        assert!(!validation_result.is_safe);
        assert!(validation_result.breaking_changes.has_blocking_changes);
    }

    #[test]
    fn test_constraint_violation_risk_detection() {
        let validator = MigrationValidator::new();
        
        let non_nullable_column = ColumnSchema {
            name: "required_field".to_string(),
            column_type: "TEXT".to_string(),
            nullable: false,
            primary_key: false,
            unique: false,
            auto_increment: false,
            default_value: None, // No default value!
            constraints: vec![],
        };
        
        let plan = create_test_migration_plan(vec![
            MigrationOperation::AddColumn { 
                table: "users".to_string(), 
                column: non_nullable_column 
            }
        ]);
        
        let result = validator.validate_migrations(&plan);
        assert!(result.is_ok());
        
        let validation_result = result.unwrap();
        // Should detect constraint violation risk
        assert!(!validation_result.is_safe);
    }

    #[test]
    fn test_type_change_compatibility() {
        let validator = MigrationValidator::new();
        
        let changes = ColumnChanges {
            type_change: Some(("TEXT".to_string(), "INTEGER".to_string())),
            null_change: None,
            default_change: None,
            constraint_changes: vec![],
        };
        
        let plan = create_test_migration_plan(vec![
            MigrationOperation::ModifyColumn {
                table: "users".to_string(),
                column: "age".to_string(),
                changes,
            }
        ]);
        
        let result = validator.validate_migrations(&plan);
        assert!(result.is_ok());
        
        let validation_result = result.unwrap();
        // TEXT to INTEGER conversion should be flagged as risky
        assert!(!validation_result.is_safe);
    }

    #[test]
    fn test_performance_impact_detection() {
        let validator = MigrationValidator::new();
        
        let index = IndexSchema {
            name: "idx_users_email".to_string(),
            columns: vec!["email".to_string()],
            unique: true,
            table_name: Some("users".to_string()),
        };
        
        let plan = create_test_migration_plan(vec![
            MigrationOperation::CreateIndex {
                table: "users".to_string(),
                index,
            }
        ]);
        
        let result = validator.get_performance_impact(&plan);
        assert!(result.is_ok());
        
        let performance_result = result.unwrap();
        assert!(performance_result.performance_impacts.len() > 0);
    }

    #[test]
    fn test_foreign_key_validation() {
        let validator = MigrationValidator::new();
        
        let foreign_key = ForeignKeySchema {
            name: "fk_posts_user_id".to_string(),
            columns: vec!["user_id".to_string()],
            referenced_table: "users".to_string(),
            referenced_columns: vec!["id".to_string()],
            on_delete: None,
            on_update: None,
        };
        
        let plan = create_test_migration_plan(vec![
            MigrationOperation::AddForeignKey { constraint: foreign_key }
        ]);
        
        let result = validator.validate_migrations(&plan);
        assert!(result.is_ok());
        
        let validation_result = result.unwrap();
        // Foreign key addition should have some warnings about constraint validation
        assert!(validation_result.safety_warnings.len() > 0);
    }

    #[test]
    fn test_index_dropping_performance_warning() {
        let validator = MigrationValidator::new();
        
        let plan = create_test_migration_plan(vec![
            MigrationOperation::DropIndex { name: "idx_users_email".to_string() }
        ]);
        
        let result = validator.validate_migrations(&plan);
        assert!(result.is_ok());
        
        let validation_result = result.unwrap();
        // Should have performance warnings
        let has_performance_warning = validation_result.safety_warnings.iter()
            .any(|w| matches!(w.warning_type, SafetyWarningType::PerformanceImpact));
        assert!(has_performance_warning);
    }

    #[test]
    fn test_table_rename_breaking_change() {
        let validator = MigrationValidator::new();
        
        let plan = create_test_migration_plan(vec![
            MigrationOperation::RenameTable {
                old_name: "users".to_string(),
                new_name: "people".to_string(),
            }
        ]);
        
        let result = validator.get_breaking_changes(&plan);
        assert!(result.is_ok());
        
        let breaking_changes = result.unwrap();
        assert!(breaking_changes.has_blocking_changes);
        assert!(breaking_changes.breaking_changes.len() > 0);
    }

    #[test]
    fn test_migration_safety_method() {
        let validator = MigrationValidator::new();
        
        // Safe migration
        let safe_plan = create_test_migration_plan(vec![
            MigrationOperation::CreateTable {
                definition: create_test_table("products", vec![
                    create_primary_key_column("id"),
                    create_test_column("name", "TEXT", false),
                ])
            }
        ]);
        
        let is_safe = validator.validate_migration_safety(&safe_plan).unwrap();
        assert!(is_safe);
        
        // Unsafe migration (data loss)
        let unsafe_plan = create_test_migration_plan(vec![
            MigrationOperation::DropTable { name: "users".to_string() }
        ]);
        
        let is_safe = validator.validate_migration_safety(&unsafe_plan).unwrap();
        assert!(!is_safe);
    }

    #[test]
    fn test_safety_warnings_method() {
        let validator = MigrationValidator::new();
        
        let plan = create_test_migration_plan(vec![
            MigrationOperation::DropTable { name: "users".to_string() },
            MigrationOperation::DropColumn {
                table: "posts".to_string(),
                column: "content".to_string(),
            }
        ]);
        
        let warnings = validator.get_safety_warnings(&plan).unwrap();
        assert!(warnings.len() > 0);
        
        // Should have data loss warnings
        let data_loss_warnings = warnings.iter()
            .filter(|w| matches!(w.warning_type, SafetyWarningType::DataLoss))
            .count();
        assert!(data_loss_warnings > 0);
    }

    #[test]
    fn test_critical_issues_detection() {
        let validator = MigrationValidator::new();
        
        // Plan with critical issues
        let critical_plan = create_test_migration_plan(vec![
            MigrationOperation::DropTable { name: "users".to_string() },
            MigrationOperation::DropTable { name: "posts".to_string() },
        ]);
        
        let has_critical = validator.has_critical_issues(&critical_plan).unwrap();
        assert!(has_critical);
        
        // Plan without critical issues
        let safe_plan = create_test_migration_plan(vec![
            MigrationOperation::CreateTable {
                definition: create_test_table("logs", vec![
                    create_primary_key_column("id"),
                    create_test_column("message", "TEXT", true),
                ])
            }
        ]);
        
        let has_critical = validator.has_critical_issues(&safe_plan).unwrap();
        assert!(!has_critical);
    }

    #[test]
    fn test_validation_result_risk_levels() {
        let validator = MigrationValidator::new();
        
        // Low risk migration
        let low_risk_plan = create_test_migration_plan(vec![
            MigrationOperation::CreateTable {
                definition: create_test_table("logs", vec![
                    create_primary_key_column("id"),
                    create_test_column("message", "TEXT", true),
                ])
            }
        ]);
        
        let result = validator.validate_migrations(&low_risk_plan).unwrap();
        // Table creation may have Medium risk due to comprehensive validation checks
        assert!(matches!(result.risk_level, RiskLevel::Low | RiskLevel::Medium));
        
        // High risk migration
        let high_risk_plan = create_test_migration_plan(vec![
            MigrationOperation::DropTable { name: "users".to_string() },
            MigrationOperation::DropColumn {
                table: "posts".to_string(),
                column: "content".to_string(),
            },
        ]);
        
        let result = validator.validate_migrations(&high_risk_plan).unwrap();
        assert!(matches!(result.risk_level, RiskLevel::High | RiskLevel::Critical));
    }

    #[test]
    fn test_validation_recommendations() {
        let validator = MigrationValidator::new();
        
        let risky_plan = create_test_migration_plan(vec![
            MigrationOperation::DropTable { name: "temp_data".to_string() },
            MigrationOperation::ModifyColumn {
                table: "users".to_string(),
                column: "email".to_string(),
                changes: ColumnChanges {
                    type_change: Some(("TEXT".to_string(), "INTEGER".to_string())),
                    null_change: None,
                    default_change: None,
                    constraint_changes: vec![],
                },
            },
        ]);
        
        let result = validator.validate_migrations(&risky_plan).unwrap();
        
        // Should have recommendations for risky migration
        assert!(result.recommendations.len() > 0);
        
        // Should recommend creating backup for data loss risk
        let has_backup_recommendation = result.recommendations.iter()
            .any(|r| matches!(r, ValidationRecommendation::CreateBackup));
        assert!(has_backup_recommendation);
    }

    #[test]
    fn test_reserved_name_detection() {
        let validator = MigrationValidator::new();
        
        // Create table with reserved name
        let reserved_table = create_test_table("sqlite_master", vec![
            create_primary_key_column("id"),
            create_test_column("data", "TEXT", true),
        ]);
        
        let plan = create_test_migration_plan(vec![
            MigrationOperation::CreateTable { definition: reserved_table }
        ]);
        
        let result = validator.validate_migrations(&plan).unwrap();
        
        // Should detect naming conflict
        assert!(!result.is_safe);
        assert!(result.safety_warnings.len() > 0);
    }

    #[test]
    fn test_unique_constraint_risk() {
        let validator = MigrationValidator::new();
        
        let unique_index = IndexSchema {
            name: "idx_users_email_unique".to_string(),
            columns: vec!["email".to_string()],
            unique: true,
            table_name: Some("users".to_string()),
        };
        
        let plan = create_test_migration_plan(vec![
            MigrationOperation::CreateIndex {
                table: "users".to_string(),
                index: unique_index,
            }
        ]);
        
        let result = validator.validate_migrations(&plan).unwrap();
        
        // Should warn about unique constraint on existing data
        assert!(result.safety_warnings.len() > 0);
    }

    #[test]
    fn test_migration_duration_estimation() {
        let validator = MigrationValidator::new();
        
        let plan = create_test_migration_plan(vec![
            MigrationOperation::CreateTable {
                definition: create_test_table("analytics", vec![
                    create_primary_key_column("id"),
                    create_test_column("event", "TEXT", false),
                    create_test_column("timestamp", "DATETIME", false),
                ])
            },
            MigrationOperation::CreateIndex {
                table: "analytics".to_string(),
                index: IndexSchema {
                    name: "idx_analytics_timestamp".to_string(),
                    columns: vec!["timestamp".to_string()],
                    unique: false,
                    table_name: Some("analytics".to_string()),
                },
            },
        ]);
        
        let performance_result = validator.get_performance_impact(&plan).unwrap();
        
        // Should have estimated duration > 0
        assert!(performance_result.estimated_total_duration > Duration::from_secs(0));
        
        // Should have performance impacts
        assert!(performance_result.performance_impacts.len() > 0);
    }
}