use d1_rs::auto_migration::*;

#[tokio::test]
async fn test_column_rename_detection_high_similarity() {
    let strategies = SmartMigrationStrategies::new()
        .with_rename_threshold(0.7);
    
    let column_changes = vec![
        ColumnChange {
            column_name: "user_name".to_string(),
            change_type: ChangeType::Remove,
            old_definition: Some(ColumnSchema {
                name: "user_name".to_string(),
                column_type: "TEXT".to_string(),
                nullable: false,
                default_value: None,
                primary_key: false,
                auto_increment: false,
                unique: true,
                constraints: vec![],
            }),
            new_definition: None,
            safety_warnings: vec![],
        },
        ColumnChange {
            column_name: "username".to_string(),
            change_type: ChangeType::Add,
            old_definition: None,
            new_definition: Some(ColumnSchema {
                name: "username".to_string(),
                column_type: "TEXT".to_string(),
                nullable: false,
                default_value: None,
                primary_key: false,
                auto_increment: false,
                unique: true,
                constraints: vec![],
            }),
            safety_warnings: vec![],
        },
    ];
    
    let rename_operations = strategies.detect_column_renames(&column_changes).unwrap();
    
    // Should detect rename due to high similarity
    assert_eq!(rename_operations.len(), 1);
    match &rename_operations[0] {
        MigrationOperation::RenameColumn { old_name, new_name, .. } => {
            assert_eq!(old_name, "user_name");
            assert_eq!(new_name, "username");
        }
        _ => panic!("Expected RenameColumn operation"),
    }
}

#[tokio::test]
async fn test_column_rename_detection_low_similarity() {
    let strategies = SmartMigrationStrategies::new()
        .with_rename_threshold(0.7);
    
    let column_changes = vec![
        ColumnChange {
            column_name: "age".to_string(),
            change_type: ChangeType::Remove,
            old_definition: Some(ColumnSchema {
                name: "age".to_string(),
                column_type: "INTEGER".to_string(),
                nullable: false,
                default_value: None,
                primary_key: false,
                auto_increment: false,
                unique: false,
                constraints: vec![],
            }),
            new_definition: None,
            safety_warnings: vec![],
        },
        ColumnChange {
            column_name: "description".to_string(),
            change_type: ChangeType::Add,
            old_definition: None,
            new_definition: Some(ColumnSchema {
                name: "description".to_string(),
                column_type: "TEXT".to_string(),
                nullable: true,
                default_value: None,
                primary_key: false,
                auto_increment: false,
                unique: false,
                constraints: vec![],
            }),
            safety_warnings: vec![],
        },
    ];
    
    let rename_operations = strategies.detect_column_renames(&column_changes).unwrap();
    
    // Should not detect rename due to low similarity
    assert_eq!(rename_operations.len(), 0);
}

#[tokio::test]
async fn test_column_rename_detection_type_compatibility() {
    let strategies = SmartMigrationStrategies::new()
        .with_rename_threshold(0.5);
    
    let column_changes = vec![
        ColumnChange {
            column_name: "count".to_string(),
            change_type: ChangeType::Remove,
            old_definition: Some(ColumnSchema {
                name: "count".to_string(),
                column_type: "INTEGER".to_string(),
                nullable: false,
                default_value: None,
                primary_key: false,
                auto_increment: false,
                unique: false,
                constraints: vec![],
            }),
            new_definition: None,
            safety_warnings: vec![],
        },
        ColumnChange {
            column_name: "total_count".to_string(),
            change_type: ChangeType::Add,
            old_definition: None,
            new_definition: Some(ColumnSchema {
                name: "total_count".to_string(),
                column_type: "BIGINT".to_string(), // Compatible integer type
                nullable: false,
                default_value: None,
                primary_key: false,
                auto_increment: false,
                unique: false,
                constraints: vec![],
            }),
            safety_warnings: vec![],
        },
    ];
    
    let rename_operations = strategies.detect_column_renames(&column_changes).unwrap();
    
    // Should detect rename due to type compatibility and name similarity
    assert_eq!(rename_operations.len(), 1);
}

#[tokio::test]
async fn test_column_rename_detection_multiple_candidates() {
    let strategies = SmartMigrationStrategies::new()
        .with_rename_threshold(0.6);
    
    let column_changes = vec![
        ColumnChange {
            column_name: "name".to_string(),
            change_type: ChangeType::Remove,
            old_definition: Some(ColumnSchema {
                name: "name".to_string(),
                column_type: "TEXT".to_string(),
                nullable: false,
                default_value: None,
                primary_key: false,
                auto_increment: false,
                unique: false,
                constraints: vec![],
            }),
            new_definition: None,
            safety_warnings: vec![],
        },
        ColumnChange {
            column_name: "user_name".to_string(),
            change_type: ChangeType::Add,
            old_definition: None,
            new_definition: Some(ColumnSchema {
                name: "user_name".to_string(),
                column_type: "TEXT".to_string(),
                nullable: false,
                default_value: None,
                primary_key: false,
                auto_increment: false,
                unique: false,
                constraints: vec![],
            }),
            safety_warnings: vec![],
        },
        ColumnChange {
            column_name: "full_name".to_string(),
            change_type: ChangeType::Add,
            old_definition: None,
            new_definition: Some(ColumnSchema {
                name: "full_name".to_string(),
                column_type: "TEXT".to_string(),
                nullable: false,
                default_value: None,
                primary_key: false,
                auto_increment: false,
                unique: false,
                constraints: vec![],
            }),
            safety_warnings: vec![],
        },
    ];
    
    let rename_operations = strategies.detect_column_renames(&column_changes).unwrap();
    
    // Should pick the best match (highest similarity)
    assert_eq!(rename_operations.len(), 1);
    match &rename_operations[0] {
        MigrationOperation::RenameColumn { old_name, new_name, .. } => {
            assert_eq!(old_name, "name");
            // Should pick the one with higher similarity
            assert!(new_name == "user_name" || new_name == "full_name");
        }
        _ => panic!("Expected RenameColumn operation"),
    }
}

#[tokio::test]
async fn test_string_similarity_calculation() {
    let strategies = SmartMigrationStrategies::new();
    
    // Identical strings
    let similarity = strategies.calculate_string_similarity("username", "username");
    assert_eq!(similarity, 1.0);
    
    // Very similar strings
    let similarity = strategies.calculate_string_similarity("user_name", "username");
    assert!(similarity > 0.7);
    
    // Somewhat similar strings
    let similarity = strategies.calculate_string_similarity("name", "user_name");
    assert!(similarity > 0.4 && similarity < 0.8);
    
    // Completely different strings
    let similarity = strategies.calculate_string_similarity("age", "description");
    assert!(similarity < 0.3);
    
    // Empty strings
    let similarity = strategies.calculate_string_similarity("", "test");
    assert_eq!(similarity, 0.0);
    
    let similarity = strategies.calculate_string_similarity("", "");
    assert_eq!(similarity, 1.0);
}

#[tokio::test]
async fn test_type_compatibility_calculation() {
    let strategies = SmartMigrationStrategies::new();
    
    // Identical types
    let compatibility = strategies.calculate_type_compatibility("TEXT", "TEXT");
    assert_eq!(compatibility, 1.0);
    
    // Equivalent types
    let compatibility = strategies.calculate_type_compatibility("VARCHAR", "TEXT");
    assert_eq!(compatibility, 0.9);
    
    let compatibility = strategies.calculate_type_compatibility("BOOLEAN", "INTEGER");
    assert_eq!(compatibility, 0.9);
    
    // Same family types
    let compatibility = strategies.calculate_type_compatibility("INTEGER", "BIGINT");
    assert_eq!(compatibility, 0.7);
    
    let compatibility = strategies.calculate_type_compatibility("REAL", "DECIMAL");
    assert_eq!(compatibility, 0.7);
    
    // Compatible families
    let compatibility = strategies.calculate_type_compatibility("INTEGER", "REAL");
    assert_eq!(compatibility, 0.5);
    
    // Incompatible types
    let compatibility = strategies.calculate_type_compatibility("TEXT", "BLOB");
    assert_eq!(compatibility, 0.0);
}

#[tokio::test]
async fn test_type_normalization() {
    let strategies = SmartMigrationStrategies::new();
    
    assert_eq!(strategies.normalize_column_type("VARCHAR(255)"), "TEXT");
    assert_eq!(strategies.normalize_column_type("char(10)"), "TEXT");
    assert_eq!(strategies.normalize_column_type("BOOLEAN"), "INTEGER");
    assert_eq!(strategies.normalize_column_type("bool"), "INTEGER");
    assert_eq!(strategies.normalize_column_type("INTEGER"), "INTEGER");
    assert_eq!(strategies.normalize_column_type("DECIMAL(10,2)"), "DECIMAL");
}

#[tokio::test]
async fn test_requires_table_restructuring() {
    let strategies = SmartMigrationStrategies::new();
    
    // Test column removal (requires restructuring in SQLite)
    let table_change = TableChange {
        table_name: "users".to_string(),
        change_type: ChangeType::Modify,
        old_schema: None,
        new_schema: None,
        column_changes: vec![
            ColumnChange {
                column_name: "old_field".to_string(),
                change_type: ChangeType::Remove,
                old_definition: Some(ColumnSchema {
                    name: "old_field".to_string(),
                    column_type: "TEXT".to_string(),
                    nullable: true,
                    default_value: None,
                    primary_key: false,
                    auto_increment: false,
                    unique: false,
                    constraints: vec![],
                }),
                new_definition: None,
                safety_warnings: vec![],
            }
        ],
        index_changes: vec![],
        foreign_key_changes: vec![],
        safety_warnings: vec![],
    };
    
    assert!(strategies.requires_table_restructuring(&table_change));
    
    // Test type change (requires restructuring)
    let table_change = TableChange {
        table_name: "users".to_string(),
        change_type: ChangeType::Modify,
        old_schema: None,
        new_schema: None,
        column_changes: vec![
            ColumnChange {
                column_name: "age".to_string(),
                change_type: ChangeType::Modify,
                old_definition: Some(ColumnSchema {
                    name: "age".to_string(),
                    column_type: "TEXT".to_string(),
                    nullable: false,
                    default_value: None,
                    primary_key: false,
                    auto_increment: false,
                    unique: false,
                    constraints: vec![],
                }),
                new_definition: Some(ColumnSchema {
                    name: "age".to_string(),
                    column_type: "INTEGER".to_string(),
                    nullable: false,
                    default_value: None,
                    primary_key: false,
                    auto_increment: false,
                    unique: false,
                    constraints: vec![],
                }),
                safety_warnings: vec![],
            }
        ],
        index_changes: vec![],
        foreign_key_changes: vec![],
        safety_warnings: vec![],
    };
    
    assert!(strategies.requires_table_restructuring(&table_change));
    
    // Test primary key change (requires restructuring)
    let table_change = TableChange {
        table_name: "users".to_string(),
        change_type: ChangeType::Modify,
        old_schema: None,
        new_schema: None,
        column_changes: vec![
            ColumnChange {
                column_name: "id".to_string(),
                change_type: ChangeType::Modify,
                old_definition: Some(ColumnSchema {
                    name: "id".to_string(),
                    column_type: "INTEGER".to_string(),
                    nullable: false,
                    default_value: None,
                    primary_key: false,
                    auto_increment: false,
                    unique: false,
                    constraints: vec![],
                }),
                new_definition: Some(ColumnSchema {
                    name: "id".to_string(),
                    column_type: "INTEGER".to_string(),
                    nullable: false,
                    default_value: None,
                    primary_key: true,
                    auto_increment: false,
                    unique: false,
                    constraints: vec![],
                }),
                safety_warnings: vec![],
            }
        ],
        index_changes: vec![],
        foreign_key_changes: vec![],
        safety_warnings: vec![],
    };
    
    assert!(strategies.requires_table_restructuring(&table_change));
    
    // Test simple column addition (no restructuring needed)
    let table_change = TableChange {
        table_name: "users".to_string(),
        change_type: ChangeType::Modify,
        old_schema: None,
        new_schema: None,
        column_changes: vec![
            ColumnChange {
                column_name: "email".to_string(),
                change_type: ChangeType::Add,
                old_definition: None,
                new_definition: Some(ColumnSchema {
                    name: "email".to_string(),
                    column_type: "TEXT".to_string(),
                    nullable: true,
                    default_value: None,
                    primary_key: false,
                    auto_increment: false,
                    unique: false,
                    constraints: vec![],
                }),
                safety_warnings: vec![],
            }
        ],
        index_changes: vec![],
        foreign_key_changes: vec![],
        safety_warnings: vec![],
    };
    
    assert!(!strategies.requires_table_restructuring(&table_change));
}

#[tokio::test]
async fn test_build_target_schema() {
    let strategies = SmartMigrationStrategies::new();
    
    let old_schema = TableSchema {
        name: "users".to_string(),
        columns: vec![
            ColumnSchema {
                name: "id".to_string(),
                column_type: "INTEGER".to_string(),
                nullable: false,
                default_value: None,
                primary_key: true,
                auto_increment: false,
                unique: false,
                constraints: vec![],
            },
            ColumnSchema {
                name: "old_name".to_string(),
                column_type: "TEXT".to_string(),
                nullable: true,
                default_value: None,
                primary_key: false,
                auto_increment: false,
                unique: false,
                constraints: vec![],
            },
            ColumnSchema {
                name: "age".to_string(),
                column_type: "INTEGER".to_string(),
                nullable: true,
                default_value: None,
                primary_key: false,
                auto_increment: false,
                unique: false,
                constraints: vec![],
            },
        ],
        indexes: vec![],
        foreign_keys: vec![],
        constraints: vec![],
    };
    
    let table_change = TableChange {
        table_name: "users".to_string(),
        change_type: ChangeType::Modify,
        old_schema: Some(old_schema),
        new_schema: None,
        column_changes: vec![
            // Remove old_name column
            ColumnChange {
                column_name: "old_name".to_string(),
                change_type: ChangeType::Remove,
                old_definition: Some(ColumnSchema {
                    name: "old_name".to_string(),
                    column_type: "TEXT".to_string(),
                    nullable: true,
                    default_value: None,
                    primary_key: false,
                    auto_increment: false,
                    unique: false,
                    constraints: vec![],
                }),
                new_definition: None,
                safety_warnings: vec![],
            },
            // Add email column
            ColumnChange {
                column_name: "email".to_string(),
                change_type: ChangeType::Add,
                old_definition: None,
                new_definition: Some(ColumnSchema {
                    name: "email".to_string(),
                    column_type: "TEXT".to_string(),
                    nullable: false,
                    default_value: None,
                    primary_key: false,
                    auto_increment: false,
                    unique: true,
                    constraints: vec![],
                }),
                safety_warnings: vec![],
            },
            // Modify age column
            ColumnChange {
                column_name: "age".to_string(),
                change_type: ChangeType::Modify,
                old_definition: Some(ColumnSchema {
                    name: "age".to_string(),
                    column_type: "INTEGER".to_string(),
                    nullable: true,
                    default_value: None,
                    primary_key: false,
                    auto_increment: false,
                    unique: false,
                    constraints: vec![],
                }),
                new_definition: Some(ColumnSchema {
                    name: "age".to_string(),
                    column_type: "INTEGER".to_string(),
                    nullable: false,
                    default_value: Some("0".to_string()),
                    primary_key: false,
                    auto_increment: false,
                    unique: false,
                    constraints: vec![],
                }),
                safety_warnings: vec![],
            },
        ],
        index_changes: vec![],
        foreign_key_changes: vec![],
        safety_warnings: vec![],
    };
    
    let target_schema = strategies.build_target_schema(&table_change).unwrap();
    
    // Should have temp table name
    assert_eq!(target_schema.name, "users_temp_migration");
    
    // Should have id, email, and modified age columns (old_name removed)
    assert_eq!(target_schema.columns.len(), 3);
    
    // Check id column is preserved
    let id_column = target_schema.columns.iter().find(|col| col.name == "id").unwrap();
    assert!(id_column.primary_key);
    
    // Check email column is added
    let email_column = target_schema.columns.iter().find(|col| col.name == "email").unwrap();
    assert!(email_column.unique);
    assert!(!email_column.nullable);
    
    // Check age column is modified
    let age_column = target_schema.columns.iter().find(|col| col.name == "age").unwrap();
    assert!(!age_column.nullable);
    assert_eq!(age_column.default_value, Some("0".to_string()));
    
    // Check old_name column is removed
    assert!(target_schema.columns.iter().find(|col| col.name == "old_name").is_none());
}

#[tokio::test]
async fn test_data_migration_type_conversion() {
    let strategies = SmartMigrationStrategies::new();
    
    let old_def = ColumnSchema {
        name: "age".to_string(),
        column_type: "TEXT".to_string(),
        nullable: false,
        default_value: None,
        primary_key: false,
        auto_increment: false,
        unique: false,
        constraints: vec![],
    };
    
    let new_def = ColumnSchema {
        name: "age".to_string(),
        column_type: "INTEGER".to_string(),
        nullable: false,
        default_value: None,
        primary_key: false,
        auto_increment: false,
        unique: false,
        constraints: vec![],
    };
    
    let data_migration = strategies.plan_column_type_migration("users", &old_def, &new_def).unwrap();
    
    assert_eq!(data_migration.migration_type, DataMigrationType::TypeConversion);
    assert_eq!(data_migration.table_name, "users");
    assert_eq!(data_migration.source_columns, vec!["age"]);
    assert_eq!(data_migration.target_columns, vec!["age"]);
    
    match data_migration.transformation {
        TransformationStrategy::TypeConversion { conversion_type, .. } => {
            assert_eq!(conversion_type, ConversionType::TextToInteger);
        }
        _ => panic!("Expected TypeConversion transformation"),
    }
}

#[tokio::test]
async fn test_type_conversion_strategies() {
    let strategies = SmartMigrationStrategies::new();
    
    // Integer to Text
    let transformation = strategies.determine_type_conversion("INTEGER", "TEXT").unwrap();
    match transformation {
        TransformationStrategy::TypeConversion { conversion_type, .. } => {
            assert_eq!(conversion_type, ConversionType::IntegerToText);
        }
        _ => panic!("Expected TypeConversion"),
    }
    
    // Text to Integer
    let transformation = strategies.determine_type_conversion("TEXT", "INTEGER").unwrap();
    match transformation {
        TransformationStrategy::TypeConversion { conversion_type, validation_rules } => {
            assert_eq!(conversion_type, ConversionType::TextToInteger);
            assert!(!validation_rules.is_empty());
        }
        _ => panic!("Expected TypeConversion"),
    }
    
    // Generic conversion
    let transformation = strategies.determine_type_conversion("BLOB", "REAL").unwrap();
    match transformation {
        TransformationStrategy::TypeConversion { conversion_type, .. } => {
            assert_eq!(conversion_type, ConversionType::Generic);
        }
        _ => panic!("Expected TypeConversion"),
    }
}

#[tokio::test]
async fn test_enhanced_migration_plan_creation() {
    let strategies = SmartMigrationStrategies::new();
    
    let original_plan = MigrationPlan {
        operations: vec![
            MigrationOperation::AddColumn {
                table: "users".to_string(),
                column: ColumnSchema {
                    name: "email".to_string(),
                    column_type: "TEXT".to_string(),
                    nullable: false,
                    default_value: None,
                    primary_key: false,
                    auto_increment: false,
                    unique: true,
                    constraints: vec![],
                },
            }
        ],
        estimated_duration: std::time::Duration::from_secs(5),
        safety_warnings: vec![],
        rollback_plan: vec![],
    };
    
    let diff = SchemaDiff {
        table_changes: vec![
            TableChange {
                table_name: "users".to_string(),
                change_type: ChangeType::Modify,
                old_schema: None,
                new_schema: None,
                column_changes: vec![
                    ColumnChange {
                        column_name: "email".to_string(),
                        change_type: ChangeType::Add,
                        old_definition: None,
                        new_definition: Some(ColumnSchema {
                            name: "email".to_string(),
                            column_type: "TEXT".to_string(),
                            nullable: false,
                            default_value: None,
                            primary_key: false,
                            auto_increment: false,
                            unique: true,
                            constraints: vec![],
                        }),
                        safety_warnings: vec![],
                    }
                ],
                index_changes: vec![],
                foreign_key_changes: vec![],
                safety_warnings: vec![],
            }
        ],
    };
    
    let enhanced_plan = strategies.enhance_migration_plan(original_plan, &diff).unwrap();
    
    // Should have operations
    assert!(!enhanced_plan.operations.is_empty());
    
    // Should have estimated duration
    assert!(enhanced_plan.estimated_duration > std::time::Duration::from_secs(0));
    
    // Data migrations should be empty for simple addition
    assert!(enhanced_plan.data_migrations.is_empty());
}

#[tokio::test]
async fn test_levenshtein_distance() {
    use d1_rs::auto_migration::smart_strategies::levenshtein_distance;
    
    // Identical strings
    assert_eq!(levenshtein_distance("hello", "hello"), 0);
    
    // Single character difference
    assert_eq!(levenshtein_distance("hello", "hallo"), 1);
    
    // Single insertion
    assert_eq!(levenshtein_distance("hello", "helllo"), 1);
    
    // Single deletion
    assert_eq!(levenshtein_distance("hello", "helo"), 1);
    
    // Multiple operations
    assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    
    // Empty strings
    assert_eq!(levenshtein_distance("", ""), 0);
    assert_eq!(levenshtein_distance("hello", ""), 5);
    assert_eq!(levenshtein_distance("", "hello"), 5);
    
    // Completely different strings
    assert_eq!(levenshtein_distance("abc", "xyz"), 3);
}

#[tokio::test]
async fn test_strategies_configuration() {
    let strategies = SmartMigrationStrategies::new()
        .with_rename_threshold(0.8)
        .with_table_restructuring(false)
        .with_data_migration(false);
    
    // Test that configuration affects behavior
    let table_change = TableChange {
        table_name: "users".to_string(),
        change_type: ChangeType::Modify,
        old_schema: None,
        new_schema: None,
        column_changes: vec![
            ColumnChange {
                column_name: "old_field".to_string(),
                change_type: ChangeType::Remove,
                old_definition: Some(ColumnSchema {
                    name: "old_field".to_string(),
                    column_type: "TEXT".to_string(),
                    nullable: true,
                    default_value: None,
                    primary_key: false,
                    auto_increment: false,
                    unique: false,
                    constraints: vec![],
                }),
                new_definition: None,
                safety_warnings: vec![],
            }
        ],
        index_changes: vec![],
        foreign_key_changes: vec![],
        safety_warnings: vec![],
    };
    
    // With table restructuring disabled, should not require restructuring
    assert!(!strategies.requires_table_restructuring(&table_change));
}

#[tokio::test]
async fn test_constraint_similarity_calculation() {
    let strategies = SmartMigrationStrategies::new();
    
    let col1 = ColumnSchema {
        name: "test".to_string(),
        column_type: "TEXT".to_string(),
        nullable: false,
        default_value: None,
        primary_key: true,
        auto_increment: false,
        unique: true,
        constraints: vec![],
    };
    
    let col2 = ColumnSchema {
        name: "test".to_string(),
        column_type: "TEXT".to_string(),
        nullable: false,
        default_value: None,
        primary_key: true,
        auto_increment: false,
        unique: true,
        constraints: vec![],
    };
    
    // Identical constraints
    let similarity = strategies.calculate_constraint_similarity(&col1, &col2);
    assert_eq!(similarity, 1.0);
    
    let col3 = ColumnSchema {
        name: "test".to_string(),
        column_type: "TEXT".to_string(),
        nullable: false,
        default_value: None,
        primary_key: false, // Different
        auto_increment: false,
        unique: false, // Different
        constraints: vec![],
    };
    
    // Partially different constraints
    let similarity = strategies.calculate_constraint_similarity(&col1, &col3);
    assert!(similarity < 1.0 && similarity > 0.0);
}

#[tokio::test]
async fn test_edge_case_empty_column_changes() {
    let strategies = SmartMigrationStrategies::new();
    
    let empty_changes = vec![];
    let rename_operations = strategies.detect_column_renames(&empty_changes).unwrap();
    
    assert!(rename_operations.is_empty());
}

#[tokio::test]
async fn test_edge_case_only_additions() {
    let strategies = SmartMigrationStrategies::new();
    
    let column_changes = vec![
        ColumnChange {
            column_name: "new_field1".to_string(),
            change_type: ChangeType::Add,
            old_definition: None,
            new_definition: Some(ColumnSchema {
                name: "new_field1".to_string(),
                column_type: "TEXT".to_string(),
                nullable: true,
                default_value: None,
                primary_key: false,
                auto_increment: false,
                unique: false,
                constraints: vec![],
            }),
            safety_warnings: vec![],
        },
        ColumnChange {
            column_name: "new_field2".to_string(),
            change_type: ChangeType::Add,
            old_definition: None,
            new_definition: Some(ColumnSchema {
                name: "new_field2".to_string(),
                column_type: "INTEGER".to_string(),
                nullable: true,
                default_value: None,
                primary_key: false,
                auto_increment: false,
                unique: false,
                constraints: vec![],
            }),
            safety_warnings: vec![],
        },
    ];
    
    let rename_operations = strategies.detect_column_renames(&column_changes).unwrap();
    
    // No renames possible with only additions
    assert!(rename_operations.is_empty());
}

#[tokio::test]
async fn test_edge_case_only_removals() {
    let strategies = SmartMigrationStrategies::new();
    
    let column_changes = vec![
        ColumnChange {
            column_name: "old_field1".to_string(),
            change_type: ChangeType::Remove,
            old_definition: Some(ColumnSchema {
                name: "old_field1".to_string(),
                column_type: "TEXT".to_string(),
                nullable: true,
                default_value: None,
                primary_key: false,
                auto_increment: false,
                unique: false,
                constraints: vec![],
            }),
            new_definition: None,
            safety_warnings: vec![],
        },
        ColumnChange {
            column_name: "old_field2".to_string(),
            change_type: ChangeType::Remove,
            old_definition: Some(ColumnSchema {
                name: "old_field2".to_string(),
                column_type: "INTEGER".to_string(),
                nullable: true,
                default_value: None,
                primary_key: false,
                auto_increment: false,
                unique: false,
                constraints: vec![],
            }),
            new_definition: None,
            safety_warnings: vec![],
        },
    ];
    
    let rename_operations = strategies.detect_column_renames(&column_changes).unwrap();
    
    // No renames possible with only removals
    assert!(rename_operations.is_empty());
}