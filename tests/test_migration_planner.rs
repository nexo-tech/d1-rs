use d1_rs::auto_migration::*;
use d1_rs::*;

/// Comprehensive test suite for Phase 2.1 - Migration Plan Generator
/// Tests the conversion of SchemaDiff objects into executable MigrationPlan objects

#[tokio::test]
async fn test_plan_table_creation() {
    let planner = MigrationPlanner::new();
    
    // Create schema diff with new table
    let new_table = TableSchema {
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
                name: "username".to_string(),
                column_type: "TEXT".to_string(),
                nullable: false,
                default_value: None,
                primary_key: false,
                auto_increment: false,
                unique: true,
                constraints: vec![],
            },
            ColumnSchema {
                name: "email".to_string(),
                column_type: "TEXT".to_string(),
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
    
    let diff = SchemaDiff {
        table_changes: vec![
            TableChange {
                table_name: "users".to_string(),
                change_type: ChangeType::Add,
                old_schema: None,
                new_schema: Some(new_table.clone()),
                column_changes: vec![],
                index_changes: vec![],
                foreign_key_changes: vec![],
                safety_warnings: vec![],
            }
        ],
    };
    
    let plan = planner.plan_migrations(diff).unwrap();
    
    // Verify table creation operation
    assert_eq!(plan.operations.len(), 1);
    match &plan.operations[0] {
        MigrationOperation::CreateTable { definition } => {
            assert_eq!(definition.name, "users");
            assert_eq!(definition.columns.len(), 3);
            assert_eq!(definition.columns[0].name, "id");
            assert_eq!(definition.columns[1].name, "username");
            assert_eq!(definition.columns[2].name, "email");
        }
        _ => panic!("Expected CreateTable operation"),
    }
    
    // Verify rollback plan contains DROP TABLE
    assert_eq!(plan.rollback_plan.len(), 1);
    match &plan.rollback_plan[0] {
        MigrationOperation::DropTable { name } => {
            assert_eq!(name, "users");
        }
        _ => panic!("Expected DropTable in rollback plan"),
    }
    
    // Should have no safety warnings for simple table creation
    assert!(plan.safety_warnings.is_empty());
}

#[tokio::test]
async fn test_plan_table_deletion() {
    let planner = MigrationPlanner::new();
    
    let old_table = TableSchema {
        name: "old_table".to_string(),
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
        ],
        indexes: vec![],
        foreign_keys: vec![],
        constraints: vec![],
    };
    
    let diff = SchemaDiff {
        table_changes: vec![
            TableChange {
                table_name: "old_table".to_string(),
                change_type: ChangeType::Remove,
                old_schema: Some(old_table.clone()),
                new_schema: None,
                column_changes: vec![],
                index_changes: vec![],
                foreign_key_changes: vec![],
                safety_warnings: vec!["This will permanently delete all data in table 'old_table'".to_string()],
            }
        ],
    };
    
    let plan = planner.plan_migrations(diff).unwrap();
    
    // Verify table deletion operation
    assert_eq!(plan.operations.len(), 1);
    match &plan.operations[0] {
        MigrationOperation::DropTable { name } => {
            assert_eq!(name, "old_table");
        }
        _ => panic!("Expected DropTable operation"),
    }
    
    // Verify safety warning about data loss
    assert_eq!(plan.safety_warnings.len(), 1);
    assert_eq!(plan.safety_warnings[0].warning_type, SafetyWarningType::DataLoss);
    assert!(plan.safety_warnings[0].message.contains("permanently delete all data"));
    
    // Rollback should recreate the table
    assert_eq!(plan.rollback_plan.len(), 1);
    match &plan.rollback_plan[0] {
        MigrationOperation::CreateTable { definition } => {
            assert_eq!(definition.name, "old_table");
        }
        _ => panic!("Expected CreateTable in rollback plan"),
    }
}

#[tokio::test]
async fn test_plan_column_addition() {
    let planner = MigrationPlanner::new();
    
    let diff = SchemaDiff {
        table_changes: vec![
            TableChange {
                table_name: "users".to_string(),
                change_type: ChangeType::Modify,
                old_schema: None,
                new_schema: None,
                column_changes: vec![
                    ColumnChange {
                        column_name: "age".to_string(),
                        change_type: ChangeType::Add,
                        old_definition: None,
                        new_definition: Some(ColumnSchema {
                            name: "age".to_string(),
                            column_type: "INTEGER".to_string(),
                            nullable: true,
                            default_value: Some("18".to_string()),
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
            }
        ],
    };
    
    let plan = planner.plan_migrations(diff).unwrap();
    
    // Verify column addition operation
    assert_eq!(plan.operations.len(), 1);
    match &plan.operations[0] {
        MigrationOperation::AddColumn { table, column } => {
            assert_eq!(table, "users");
            assert_eq!(column.name, "age");
            assert_eq!(column.column_type, "INTEGER");
            assert_eq!(column.default_value, Some("18".to_string()));
        }
        _ => panic!("Expected AddColumn operation"),
    }
    
    // Rollback should drop the column
    assert_eq!(plan.rollback_plan.len(), 1);
    match &plan.rollback_plan[0] {
        MigrationOperation::DropColumn { table, column } => {
            assert_eq!(table, "users");
            assert_eq!(column, "age");
        }
        _ => panic!("Expected DropColumn in rollback plan"),
    }
}

#[tokio::test]
async fn test_plan_column_deletion() {
    let planner = MigrationPlanner::new();
    
    let diff = SchemaDiff {
        table_changes: vec![
            TableChange {
                table_name: "users".to_string(),
                change_type: ChangeType::Modify,
                old_schema: None,
                new_schema: None,
                column_changes: vec![
                    ColumnChange {
                        column_name: "deprecated_field".to_string(),
                        change_type: ChangeType::Remove,
                        old_definition: Some(ColumnSchema {
                            name: "deprecated_field".to_string(),
                            column_type: "TEXT".to_string(),
                            nullable: true,
                            default_value: None,
                            primary_key: false,
                            auto_increment: false,
                            unique: false,
                            constraints: vec![],
                        }),
                        new_definition: None,
                        safety_warnings: vec!["Dropping column will permanently delete data".to_string()],
                    }
                ],
                index_changes: vec![],
                foreign_key_changes: vec![],
                safety_warnings: vec![],
            }
        ],
    };
    
    let plan = planner.plan_migrations(diff).unwrap();
    
    // Verify column deletion operation
    assert_eq!(plan.operations.len(), 1);
    match &plan.operations[0] {
        MigrationOperation::DropColumn { table, column } => {
            assert_eq!(table, "users");
            assert_eq!(column, "deprecated_field");
        }
        _ => panic!("Expected DropColumn operation"),
    }
    
    // Should have safety warning about data loss
    assert_eq!(plan.safety_warnings.len(), 1);
    assert_eq!(plan.safety_warnings[0].warning_type, SafetyWarningType::DataLoss);
}

#[tokio::test]
async fn test_plan_column_modification() {
    let planner = MigrationPlanner::new();
    
    let old_column = ColumnSchema {
        name: "username".to_string(),
        column_type: "TEXT".to_string(),
        nullable: true,
        default_value: None,
        primary_key: false,
        auto_increment: false,
        unique: false,
        constraints: vec![],
    };
    
    let new_column = ColumnSchema {
        name: "username".to_string(),
        column_type: "TEXT".to_string(),
        nullable: false,
        default_value: Some("'anonymous'".to_string()),
        primary_key: false,
        auto_increment: false,
        unique: true,
        constraints: vec![],
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
                        column_name: "username".to_string(),
                        change_type: ChangeType::Modify,
                        old_definition: Some(old_column),
                        new_definition: Some(new_column),
                        safety_warnings: vec!["Making column NOT NULL may fail if existing data contains NULLs".to_string()],
                    }
                ],
                index_changes: vec![],
                foreign_key_changes: vec![],
                safety_warnings: vec![],
            }
        ],
    };
    
    let plan = planner.plan_migrations(diff).unwrap();
    
    // Verify column modification operation
    assert_eq!(plan.operations.len(), 1);
    match &plan.operations[0] {
        MigrationOperation::ModifyColumn { table, column, changes } => {
            assert_eq!(table, "users");
            assert_eq!(column, "username");
            assert_eq!(changes.null_change, Some((true, false))); // nullable: true -> false
            assert_eq!(changes.default_change, Some((None, Some("'anonymous'".to_string()))));
        }
        _ => panic!("Expected ModifyColumn operation"),
    }
    
    // Should have safety warning about NULL constraint
    assert_eq!(plan.safety_warnings.len(), 1);
    assert_eq!(plan.safety_warnings[0].warning_type, SafetyWarningType::BreakingChange);
}

#[tokio::test]
async fn test_plan_index_creation() {
    let planner = MigrationPlanner::new();
    
    let diff = SchemaDiff {
        table_changes: vec![
            TableChange {
                table_name: "users".to_string(),
                change_type: ChangeType::Modify,
                old_schema: None,
                new_schema: None,
                column_changes: vec![],
                index_changes: vec![
                    IndexChange {
                        index_name: "idx_users_email".to_string(),
                        change_type: ChangeType::Add,
                        old_definition: None,
                        new_definition: Some(IndexSchema {
                            name: "idx_users_email".to_string(),
                            table_name: Some("users".to_string()),
                            columns: vec!["email".to_string()],
                            unique: false,
                        }),
                        safety_warnings: vec![],
                    }
                ],
                foreign_key_changes: vec![],
                safety_warnings: vec![],
            }
        ],
    };
    
    let plan = planner.plan_migrations(diff).unwrap();
    
    // Verify index creation operation
    assert_eq!(plan.operations.len(), 1);
    match &plan.operations[0] {
        MigrationOperation::CreateIndex { table, index } => {
            assert_eq!(table, "users");
            assert_eq!(index.name, "idx_users_email");
            assert_eq!(index.columns, vec!["email"]);
            assert!(!index.unique);
        }
        _ => panic!("Expected CreateIndex operation"),
    }
    
    // Rollback should drop the index
    assert_eq!(plan.rollback_plan.len(), 1);
    match &plan.rollback_plan[0] {
        MigrationOperation::DropIndex { name } => {
            assert_eq!(name, "idx_users_email");
        }
        _ => panic!("Expected DropIndex in rollback plan"),
    }
}

#[tokio::test]
async fn test_plan_foreign_key_creation() {
    let planner = MigrationPlanner::new();
    
    let diff = SchemaDiff {
        table_changes: vec![
            TableChange {
                table_name: "posts".to_string(),
                change_type: ChangeType::Modify,
                old_schema: None,
                new_schema: None,
                column_changes: vec![],
                index_changes: vec![],
                foreign_key_changes: vec![
                    ForeignKeyChange {
                        foreign_key_name: "fk_posts_user_id".to_string(),
                        change_type: ChangeType::Add,
                        old_definition: None,
                        new_definition: Some(ForeignKeySchema {
                            name: "fk_posts_user_id".to_string(),
                            columns: vec!["user_id".to_string()],
                            referenced_table: "users".to_string(),
                            referenced_columns: vec!["id".to_string()],
                            on_delete: Some("CASCADE".to_string()),
                            on_update: Some("CASCADE".to_string()),
                        }),
                        safety_warnings: vec![],
                    }
                ],
                safety_warnings: vec![],
            }
        ],
    };
    
    let plan = planner.plan_migrations(diff).unwrap();
    
    // Verify foreign key creation operation
    assert_eq!(plan.operations.len(), 1);
    match &plan.operations[0] {
        MigrationOperation::AddForeignKey { constraint } => {
            assert_eq!(constraint.name, "fk_posts_user_id");
            assert_eq!(constraint.columns, vec!["user_id"]);
            assert_eq!(constraint.referenced_table, "users");
            assert_eq!(constraint.referenced_columns, vec!["id"]);
            assert_eq!(constraint.on_delete, Some("CASCADE".to_string()));
        }
        _ => panic!("Expected AddForeignKey operation"),
    }
    
    // Rollback should drop the foreign key
    assert_eq!(plan.rollback_plan.len(), 1);
    match &plan.rollback_plan[0] {
        MigrationOperation::DropForeignKey { table, constraint_name } => {
            assert_eq!(table, "posts");
            assert_eq!(constraint_name, "fk_posts_user_id");
        }
        _ => panic!("Expected DropForeignKey in rollback plan"),
    }
}

#[tokio::test]
async fn test_plan_table_rename() {
    let planner = MigrationPlanner::new();
    
    let diff = SchemaDiff {
        table_changes: vec![
            TableChange {
                table_name: "users".to_string(),
                change_type: ChangeType::Rename,
                old_schema: None,
                new_schema: None,
                column_changes: vec![],
                index_changes: vec![],
                foreign_key_changes: vec![],
                safety_warnings: vec!["Table rename detected - please verify this is intended".to_string()],
            }
        ],
    };
    
    let plan = planner.plan_migrations(diff).unwrap();
    
    // For now, rename should be detected but implementation details depend on the differ
    // This test ensures we handle the ChangeType::Rename case
    assert!(!plan.operations.is_empty() || !plan.safety_warnings.is_empty());
}

#[tokio::test]
async fn test_plan_column_rename() {
    let planner = MigrationPlanner::new();
    
    let diff = SchemaDiff {
        table_changes: vec![
            TableChange {
                table_name: "users".to_string(),
                change_type: ChangeType::Modify,
                old_schema: None,
                new_schema: None,
                column_changes: vec![
                    ColumnChange {
                        column_name: "username".to_string(),
                        change_type: ChangeType::Rename,
                        old_definition: Some(ColumnSchema {
                            name: "username".to_string(),
                            column_type: "TEXT".to_string(),
                            nullable: false,
                            default_value: None,
                            primary_key: false,
                            auto_increment: false,
                            unique: false,
                            constraints: vec![],
                        }),
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
                        safety_warnings: vec!["Column rename detected".to_string()],
                    }
                ],
                index_changes: vec![],
                foreign_key_changes: vec![],
                safety_warnings: vec![],
            }
        ],
    };
    
    let plan = planner.plan_migrations(diff).unwrap();
    
    // Should generate RenameColumn operation
    assert_eq!(plan.operations.len(), 1);
    match &plan.operations[0] {
        MigrationOperation::RenameColumn { table, old_name, new_name } => {
            assert_eq!(table, "users");
            assert_eq!(old_name, "username");
            assert_eq!(new_name, "user_name");
        }
        _ => panic!("Expected RenameColumn operation"),
    }
    
    // Rollback should rename back
    assert_eq!(plan.rollback_plan.len(), 1);
    match &plan.rollback_plan[0] {
        MigrationOperation::RenameColumn { table, old_name, new_name } => {
            assert_eq!(table, "users");
            assert_eq!(old_name, "user_name");
            assert_eq!(new_name, "username");
        }
        _ => panic!("Expected RenameColumn in rollback plan"),
    }
}

#[tokio::test]
async fn test_plan_complex_multi_table_changes() {
    let planner = MigrationPlanner::new();
    
    // Complex scenario: Create new table, modify existing table, add relationships
    let new_table = TableSchema {
        name: "profiles".to_string(),
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
                name: "user_id".to_string(),
                column_type: "INTEGER".to_string(),
                nullable: false,
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
    
    let diff = SchemaDiff {
        table_changes: vec![
            // Create new profiles table
            TableChange {
                table_name: "profiles".to_string(),
                change_type: ChangeType::Add,
                old_schema: None,
                new_schema: Some(new_table),
                column_changes: vec![],
                index_changes: vec![],
                foreign_key_changes: vec![],
                safety_warnings: vec![],
            },
            // Modify users table - add column and foreign key
            TableChange {
                table_name: "users".to_string(),
                change_type: ChangeType::Modify,
                old_schema: None,
                new_schema: None,
                column_changes: vec![
                    ColumnChange {
                        column_name: "profile_id".to_string(),
                        change_type: ChangeType::Add,
                        old_definition: None,
                        new_definition: Some(ColumnSchema {
                            name: "profile_id".to_string(),
                            column_type: "INTEGER".to_string(),
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
                foreign_key_changes: vec![
                    ForeignKeyChange {
                        foreign_key_name: "fk_users_profile_id".to_string(),
                        change_type: ChangeType::Add,
                        old_definition: None,
                        new_definition: Some(ForeignKeySchema {
                            name: "fk_users_profile_id".to_string(),
                            columns: vec!["profile_id".to_string()],
                            referenced_table: "profiles".to_string(),
                            referenced_columns: vec!["id".to_string()],
                            on_delete: Some("SET NULL".to_string()),
                            on_update: Some("CASCADE".to_string()),
                        }),
                        safety_warnings: vec![],
                    }
                ],
                safety_warnings: vec![],
            },
        ],
    };
    
    let plan = planner.plan_migrations(diff).unwrap();
    
    // Should have multiple operations in correct order
    assert!(plan.operations.len() >= 3); // At least: CreateTable, AddColumn, AddForeignKey
    
    // Verify operations are in dependency order
    let create_table_pos = plan.operations.iter().position(|op| {
        matches!(op, MigrationOperation::CreateTable { definition } if definition.name == "profiles")
    });
    let add_fk_pos = plan.operations.iter().position(|op| {
        matches!(op, MigrationOperation::AddForeignKey { constraint } if constraint.referenced_table == "profiles")
    });
    
    assert!(create_table_pos.is_some());
    assert!(add_fk_pos.is_some());
    assert!(create_table_pos.unwrap() < add_fk_pos.unwrap(), "CreateTable must come before AddForeignKey");
    
    // Rollback plan should be in reverse order
    assert!(plan.rollback_plan.len() >= 3);
    
    // First rollback operation should remove FK, last should drop table
    assert!(matches!(plan.rollback_plan[0], MigrationOperation::DropForeignKey { .. }));
    assert!(matches!(plan.rollback_plan.last().unwrap(), MigrationOperation::DropTable { .. }));
}

#[tokio::test]
async fn test_analyze_safety_warnings() {
    let planner = MigrationPlanner::new();
    
    // Create diff with multiple dangerous operations
    let diff = SchemaDiff {
        table_changes: vec![
            TableChange {
                table_name: "users".to_string(),
                change_type: ChangeType::Remove,
                old_schema: Some(TableSchema {
                    name: "users".to_string(),
                    columns: vec![],
                    indexes: vec![],
                    foreign_keys: vec![],
                    constraints: vec![],
                }),
                new_schema: None,
                column_changes: vec![],
                index_changes: vec![],
                foreign_key_changes: vec![],
                safety_warnings: vec!["Dropping table will delete all data".to_string()],
            },
            TableChange {
                table_name: "posts".to_string(),
                change_type: ChangeType::Modify,
                old_schema: None,
                new_schema: None,
                column_changes: vec![
                    ColumnChange {
                        column_name: "content".to_string(),
                        change_type: ChangeType::Modify,
                        old_definition: Some(ColumnSchema {
                            name: "content".to_string(),
                            column_type: "TEXT".to_string(),
                            nullable: true,
                            default_value: None,
                            primary_key: false,
                            auto_increment: false,
                            unique: false,
                            constraints: vec![],
                        }),
                        new_definition: Some(ColumnSchema {
                            name: "content".to_string(),
                            column_type: "VARCHAR(100)".to_string(), // Shortening length
                            nullable: false,
                            default_value: None,
                            primary_key: false,
                            auto_increment: false,
                            unique: false,
                            constraints: vec![],
                        }),
                        safety_warnings: vec![
                            "Type change may truncate data".to_string(),
                            "Making column NOT NULL may fail".to_string(),
                        ],
                    }
                ],
                index_changes: vec![],
                foreign_key_changes: vec![],
                safety_warnings: vec![],
            },
        ],
    };
    
    let plan = planner.plan_migrations(diff).unwrap();
    
    // Should have multiple safety warnings
    assert!(plan.safety_warnings.len() >= 3);
    
    // Should have different types of warnings
    let warning_types: Vec<&SafetyWarningType> = plan.safety_warnings.iter()
        .map(|w| &w.warning_type)
        .collect();
    
    assert!(warning_types.contains(&&SafetyWarningType::DataLoss));
    assert!(warning_types.contains(&&SafetyWarningType::BreakingChange));
    
    // Each warning should have a recommendation
    for warning in &plan.safety_warnings {
        assert!(!warning.recommendation.is_empty());
    }
}

#[tokio::test]
async fn test_estimate_migration_duration() {
    let planner = MigrationPlanner::new();
    
    // Create diff with operations of varying complexity
    let diff = SchemaDiff {
        table_changes: vec![
            TableChange {
                table_name: "users".to_string(),
                change_type: ChangeType::Add,
                old_schema: None,
                new_schema: Some(TableSchema {
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
                    ],
                    indexes: vec![],
                    foreign_keys: vec![],
                    constraints: vec![],
                }),
                column_changes: vec![],
                index_changes: vec![],
                foreign_key_changes: vec![],
                safety_warnings: vec![],
            }
        ],
    };
    
    let plan = planner.plan_migrations(diff).unwrap();
    
    // Should estimate some duration (not zero)
    assert!(plan.estimated_duration > std::time::Duration::from_secs(0));
    
    // For simple operations, should be relatively quick
    assert!(plan.estimated_duration < std::time::Duration::from_secs(60));
}

#[tokio::test]
async fn test_empty_diff_produces_empty_plan() {
    let planner = MigrationPlanner::new();
    
    let diff = SchemaDiff {
        table_changes: vec![],
    };
    
    let plan = planner.plan_migrations(diff).unwrap();
    
    // Empty diff should produce empty plan
    assert!(plan.operations.is_empty());
    assert!(plan.rollback_plan.is_empty());
    assert!(plan.safety_warnings.is_empty());
    assert_eq!(plan.estimated_duration, std::time::Duration::from_secs(0));
}

#[tokio::test]
async fn test_plan_preserves_operation_order() {
    let planner = MigrationPlanner::new();
    
    // Create diff that requires specific ordering
    // 1. Create table first
    // 2. Add column 
    // 3. Create index on that column
    // 4. Add foreign key referencing the table
    
    let diff = SchemaDiff {
        table_changes: vec![
            TableChange {
                table_name: "categories".to_string(),
                change_type: ChangeType::Add,
                old_schema: None,
                new_schema: Some(TableSchema {
                    name: "categories".to_string(),
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
                    ],
                    indexes: vec![],
                    foreign_keys: vec![],
                    constraints: vec![],
                }),
                column_changes: vec![],
                index_changes: vec![],
                foreign_key_changes: vec![],
                safety_warnings: vec![],
            },
            TableChange {
                table_name: "posts".to_string(),
                change_type: ChangeType::Modify,
                old_schema: None,
                new_schema: None,
                column_changes: vec![
                    ColumnChange {
                        column_name: "category_id".to_string(),
                        change_type: ChangeType::Add,
                        old_definition: None,
                        new_definition: Some(ColumnSchema {
                            name: "category_id".to_string(),
                            column_type: "INTEGER".to_string(),
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
                index_changes: vec![
                    IndexChange {
                        index_name: "idx_posts_category_id".to_string(),
                        change_type: ChangeType::Add,
                        old_definition: None,
                        new_definition: Some(IndexSchema {
                            name: "idx_posts_category_id".to_string(),
                            table_name: Some("posts".to_string()),
                            columns: vec!["category_id".to_string()],
                            unique: false,
                        }),
                        safety_warnings: vec![],
                    }
                ],
                foreign_key_changes: vec![
                    ForeignKeyChange {
                        foreign_key_name: "fk_posts_category_id".to_string(),
                        change_type: ChangeType::Add,
                        old_definition: None,
                        new_definition: Some(ForeignKeySchema {
                            name: "fk_posts_category_id".to_string(),
                            columns: vec!["category_id".to_string()],
                            referenced_table: "categories".to_string(),
                            referenced_columns: vec!["id".to_string()],
                            on_delete: Some("SET NULL".to_string()),
                            on_update: Some("CASCADE".to_string()),
                        }),
                        safety_warnings: vec![],
                    }
                ],
                safety_warnings: vec![],
            },
        ],
    };
    
    let plan = planner.plan_migrations(diff).unwrap();
    
    // Find positions of different operation types
    let mut create_table_pos = None;
    let mut add_column_pos = None;
    let mut create_index_pos = None;
    let mut add_fk_pos = None;
    
    for (i, op) in plan.operations.iter().enumerate() {
        match op {
            MigrationOperation::CreateTable { definition } if definition.name == "categories" => {
                create_table_pos = Some(i);
            }
            MigrationOperation::AddColumn { table, column } if table == "posts" && column.name == "category_id" => {
                add_column_pos = Some(i);
            }
            MigrationOperation::CreateIndex { table, index } if table == "posts" => {
                create_index_pos = Some(i);
            }
            MigrationOperation::AddForeignKey { constraint } if constraint.referenced_table == "categories" => {
                add_fk_pos = Some(i);
            }
            _ => {}
        }
    }
    
    // Verify correct ordering
    assert!(create_table_pos.is_some());
    assert!(add_column_pos.is_some());
    assert!(create_index_pos.is_some());
    assert!(add_fk_pos.is_some());
    
    let create_table_pos = create_table_pos.unwrap();
    let add_column_pos = add_column_pos.unwrap();
    let create_index_pos = create_index_pos.unwrap();
    let add_fk_pos = add_fk_pos.unwrap();
    
    // CreateTable must come first
    assert!(create_table_pos < add_column_pos);
    assert!(create_table_pos < create_index_pos);
    assert!(create_table_pos < add_fk_pos);
    
    // AddColumn must come before CreateIndex (on that column)
    assert!(add_column_pos < create_index_pos);
    
    // AddColumn must come before AddForeignKey (on that column)  
    assert!(add_column_pos < add_fk_pos);
}