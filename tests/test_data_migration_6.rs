use d1_rs::auto_migration::{
    DataMigrationConfig, DataMigrator, FailureStrategy, MigrationContext, RollbackInfo,
    TransformationFunction,
};
use d1_rs::*;
use d1_rs::backends::QueryResult;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

async fn setup_test_db() -> D1Client {
    D1Client::new_in_memory()
        .await
        .expect("Failed to create in-memory database")
}

// Test custom transformation function implementations
struct TestTransformationFunction {
    name: String,
}

impl TestTransformationFunction {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl TransformationFunction for TestTransformationFunction {
    fn transform(
        &self,
        input_data: &HashMap<String, serde_json::Value>,
        _context: &MigrationContext,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let mut output = HashMap::new();

        // Simple transformation: uppercase name field, double age values, keep other fields unchanged
        for (key, value) in input_data {
            let transformed_value = match value {
                serde_json::Value::String(s) if key == "name" => {
                    serde_json::Value::String(s.to_uppercase())
                }
                serde_json::Value::Number(n) if key == "age" => {
                    // Only double the age field
                    if let Some(int_val) = n.as_i64() {
                        serde_json::Value::Number((int_val * 2).into())
                    } else if let Some(float_val) = n.as_f64() {
                        serde_json::Value::Number(
                            serde_json::Number::from_f64(float_val * 2.0)
                                .unwrap_or_else(|| 0.into()),
                        )
                    } else {
                        value.clone()
                    }
                }
                _ => value.clone(),
            };
            output.insert(key.clone(), transformed_value);

            // Add computed field
            if key == "name" {
                output.insert(
                    "computed_field".to_string(),
                    serde_json::Value::String("computed_value".to_string()),
                );
            }
        }

        Ok(output)
    }

    fn validate_config(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(D1RsError::ValidationError(
                "Transformation name cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

struct FailingTransformationFunction;

impl TransformationFunction for FailingTransformationFunction {
    fn transform(
        &self,
        _input_data: &HashMap<String, serde_json::Value>,
        _context: &MigrationContext,
    ) -> Result<HashMap<String, serde_json::Value>> {
        Err(D1RsError::ValidationError(
            "Intentional transformation failure".to_string(),
        ))
    }

    fn validate_config(&self) -> Result<()> {
        Ok(())
    }

    fn name(&self) -> &str {
        "failing_transformation"
    }
}

async fn create_test_tables_for_custom_transformation(db: &D1Client) {
    // Create source table with test data
    db.execute(
        "CREATE TABLE custom_source_users (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            age INTEGER,
            email TEXT
        )",
        &[],
    )
    .await
    .expect("Failed to create custom_source_users table");

    // Insert test data
    db.execute(
        "INSERT INTO custom_source_users (id, name, age, email) VALUES 
        (1, 'alice', 25, 'alice@example.com'),
        (2, 'bob', 30, 'bob@example.com'),
        (3, 'charlie', 35, 'charlie@example.com')",
        &[],
    )
    .await
    .expect("Failed to insert test data");

    // Create target table
    db.execute(
        "CREATE TABLE custom_target_users (
            id INTEGER PRIMARY KEY,
            transformed_name TEXT NOT NULL,
            doubled_age INTEGER,
            original_email TEXT,
            computed_field TEXT
        )",
        &[],
    )
    .await
    .expect("Failed to create custom_target_users table");
}

#[tokio::test]
async fn test_execute_custom_transformation_validation_errors() {
    let db = setup_test_db().await;
    create_test_tables_for_custom_transformation(&db).await;

    let mut custom_transformations = HashMap::new();
    custom_transformations.insert(
        "test_transform".to_string(),
        Box::new(TestTransformationFunction::new("test_transform"))
            as Box<dyn TransformationFunction>,
    );

    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations,
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    // Test empty migration name
    let result = data_migration
        .execute_custom_transformation(
            "",
            "SELECT * FROM custom_source_users",
            "test_transform",
            &[
                "INSERT INTO custom_target_users (id, transformed_name) VALUES ($id, $name)"
                    .to_string(),
            ],
        )
        .await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Migration name cannot be empty"));

    // Test empty source query
    let result = data_migration
        .execute_custom_transformation(
            "test_migration",
            "",
            "test_transform",
            &[
                "INSERT INTO custom_target_users (id, transformed_name) VALUES ($id, $name)"
                    .to_string(),
            ],
        )
        .await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Source query cannot be empty"));

    // Test empty transformation logic
    let result = data_migration
        .execute_custom_transformation(
            "test_migration",
            "SELECT * FROM custom_source_users",
            "",
            &[
                "INSERT INTO custom_target_users (id, transformed_name) VALUES ($id, $name)"
                    .to_string(),
            ],
        )
        .await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Transformation logic cannot be empty"));

    // Test empty target operations
    let result = data_migration
        .execute_custom_transformation(
            "test_migration",
            "SELECT * FROM custom_source_users",
            "test_transform",
            &[],
        )
        .await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("At least one target operation must be specified"));

    // Test non-existent transformation function
    let result = data_migration
        .execute_custom_transformation(
            "test_migration",
            "SELECT * FROM custom_source_users",
            "non_existent_transform",
            &[
                "INSERT INTO custom_target_users (id, transformed_name) VALUES ($id, $name)"
                    .to_string(),
            ],
        )
        .await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Custom transformation 'non_existent_transform' not found"));
}

#[tokio::test]
async fn test_execute_custom_transformation_success() {
    let db = setup_test_db().await;
    create_test_tables_for_custom_transformation(&db).await;

    let mut custom_transformations = HashMap::new();
    custom_transformations.insert(
        "test_transform".to_string(),
        Box::new(TestTransformationFunction::new("test_transform"))
            as Box<dyn TransformationFunction>,
    );

    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations,
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    let target_operations = vec![
        "INSERT INTO custom_target_users (id, transformed_name, doubled_age, original_email, computed_field) VALUES ($id, $name, $age, $email, $computed_field)".to_string()
    ];

    let result = data_migration
        .execute_custom_transformation(
            "user_transformation",
            "SELECT * FROM custom_source_users",
            "test_transform",
            &target_operations,
        )
        .await
        .expect("Should execute custom transformation successfully");

    assert!(result.success);
    assert_eq!(result.records_processed, 3);
    assert_eq!(result.records_failed, 0);
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Starting custom migration: user_transformation")));
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Using transformation function: test_transform")));

    // Verify transformed data was inserted correctly
    let verification_result = db.execute(
        "SELECT id, transformed_name, doubled_age, original_email, computed_field FROM custom_target_users ORDER BY id",
        &[]
    ).await.expect("Failed to verify transformed data");

    assert_eq!(verification_result.rows().len(), 3);

    // Verify first row transformation
    if let Value::Object(row) = &verification_result.rows()[0] {
        assert_eq!(row.get("id").unwrap(), &Value::Number(1.into()));
        assert_eq!(
            row.get("transformed_name").unwrap(),
            &Value::String("ALICE".to_string())
        );
        assert_eq!(row.get("doubled_age").unwrap(), &Value::Number(50.into()));
        assert_eq!(
            row.get("original_email").unwrap(),
            &Value::String("alice@example.com".to_string())
        );
        assert_eq!(
            row.get("computed_field").unwrap(),
            &Value::String("computed_value".to_string())
        );
    }
}

#[tokio::test]
async fn test_execute_custom_transformation_failure() {
    let db = setup_test_db().await;
    create_test_tables_for_custom_transformation(&db).await;

    let mut custom_transformations = HashMap::new();
    custom_transformations.insert(
        "failing_transform".to_string(),
        Box::new(FailingTransformationFunction) as Box<dyn TransformationFunction>,
    );

    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations,
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    let target_operations = vec![
        "INSERT INTO custom_target_users (id, transformed_name) VALUES ($id, $name)".to_string(),
    ];

    let result = data_migration
        .execute_custom_transformation(
            "failing_migration",
            "SELECT * FROM custom_source_users",
            "failing_transform",
            &target_operations,
        )
        .await
        .expect("Should handle transformation failures gracefully");

    assert!(!result.success);
    assert_eq!(result.records_processed, 3);
    assert_eq!(result.records_failed, 3);
    assert!(result
        .errors
        .iter()
        .any(|e| e.to_string().contains("Custom transformation failed")));

    // Verify no data was inserted due to transformation failures
    let verification_result = db
        .execute("SELECT COUNT(*) as count FROM custom_target_users", &[])
        .await
        .expect("Failed to verify no data inserted");

    if let Value::Object(row) = &verification_result.rows()[0] {
        if let Some(Value::Number(count)) = row.get("count") {
            assert_eq!(count.as_u64().unwrap(), 0);
        }
    }
}

#[tokio::test]
async fn test_execute_custom_transformation_target_operation_failure() {
    let db = setup_test_db().await;
    create_test_tables_for_custom_transformation(&db).await;

    let mut custom_transformations = HashMap::new();
    custom_transformations.insert(
        "test_transform".to_string(),
        Box::new(TestTransformationFunction::new("test_transform"))
            as Box<dyn TransformationFunction>,
    );

    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations,
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    // Use invalid SQL for target operation
    let target_operations =
        vec!["INSERT INTO non_existent_table (id, name) VALUES ($id, $name)".to_string()];

    let result = data_migration
        .execute_custom_transformation(
            "failing_target_migration",
            "SELECT * FROM custom_source_users",
            "test_transform",
            &target_operations,
        )
        .await
        .expect("Should handle target operation failures gracefully");

    assert!(!result.success);
    assert_eq!(result.records_processed, 3);
    assert_eq!(result.records_failed, 3);
    assert!(result
        .errors
        .iter()
        .any(|e| e.to_string().contains("Target operation")));
}

#[tokio::test]
async fn test_execute_custom_transformation_invalid_source_query() {
    let db = setup_test_db().await;
    create_test_tables_for_custom_transformation(&db).await;

    let mut custom_transformations = HashMap::new();
    custom_transformations.insert(
        "test_transform".to_string(),
        Box::new(TestTransformationFunction::new("test_transform"))
            as Box<dyn TransformationFunction>,
    );

    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations,
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    let target_operations = vec![
        "INSERT INTO custom_target_users (id, transformed_name) VALUES ($id, $name)".to_string(),
    ];

    let result = data_migration
        .execute_custom_transformation(
            "invalid_source_migration",
            "SELECT * FROM non_existent_source_table",
            "test_transform",
            &target_operations,
        )
        .await
        .expect("Should handle invalid source query gracefully");

    assert!(!result.success);
    assert_eq!(result.records_processed, 0);
    assert_eq!(result.records_failed, 1);
    assert!(result
        .errors
        .iter()
        .any(|e| e.to_string().contains("Failed to execute source query")));
}

#[tokio::test]
async fn test_execute_custom_transformation_batch_processing() {
    let db = setup_test_db().await;
    create_test_tables_for_custom_transformation(&db).await;

    // Insert more test data for batch testing
    for i in 4..=15 {
        db.execute(
            "INSERT INTO custom_source_users (id, name, age, email) VALUES (?, ?, ?, ?)",
            &[
                serde_json::Value::Number(i.into()),
                serde_json::Value::String(format!("user{}", i)),
                serde_json::Value::Number((20 + i).into()),
                serde_json::Value::String(format!("user{}@example.com", i)),
            ],
        )
        .await
        .expect("Failed to insert additional test data");
    }

    let mut custom_transformations = HashMap::new();
    custom_transformations.insert(
        "test_transform".to_string(),
        Box::new(TestTransformationFunction::new("test_transform"))
            as Box<dyn TransformationFunction>,
    );

    let config = DataMigrationConfig {
        batch_size: 5, // Small batch size to test batching
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations,
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    let target_operations = vec![
        "INSERT INTO custom_target_users (id, transformed_name, doubled_age, original_email, computed_field) VALUES ($id, $name, $age, $email, $computed_field)".to_string()
    ];

    let result = data_migration
        .execute_custom_transformation(
            "batch_processing_migration",
            "SELECT * FROM custom_source_users ORDER BY id",
            "test_transform",
            &target_operations,
        )
        .await
        .expect("Should handle batch processing successfully");

    assert!(result.success);
    assert_eq!(result.records_processed, 15); // Total records including original 3 + 12 new
    assert_eq!(result.records_failed, 0);

    // Should have batch processing warnings
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Processing batch")));

    // Verify all records were transformed and inserted
    let verification_result = db
        .execute("SELECT COUNT(*) as count FROM custom_target_users", &[])
        .await
        .expect("Failed to verify batch processing results");

    if let Value::Object(row) = &verification_result.rows()[0] {
        if let Some(Value::Number(count)) = row.get("count") {
            assert_eq!(count.as_u64().unwrap(), 15);
        }
    }
}

#[tokio::test]
async fn test_execute_custom_transformation_placeholder_replacement() {
    let db = setup_test_db().await;
    create_test_tables_for_custom_transformation(&db).await;

    let mut custom_transformations = HashMap::new();
    custom_transformations.insert(
        "test_transform".to_string(),
        Box::new(TestTransformationFunction::new("test_transform"))
            as Box<dyn TransformationFunction>,
    );

    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations,
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    // Test multiple placeholders and special value handling
    let target_operations = vec![
        "INSERT INTO custom_target_users (id, transformed_name, doubled_age, original_email, computed_field) VALUES ($id, $name, $age, $email, $computed_field)".to_string()
    ];

    let result = data_migration
        .execute_custom_transformation(
            "placeholder_test_migration",
            "SELECT * FROM custom_source_users WHERE id = 1",
            "test_transform",
            &target_operations,
        )
        .await
        .expect("Should handle placeholder replacement successfully");

    assert!(result.success);
    assert_eq!(result.records_processed, 1);
    assert_eq!(result.records_failed, 0);

    // Verify placeholder replacement worked correctly by checking the inserted data
    let verification_result = db.execute(
        "SELECT id, transformed_name, doubled_age, original_email, computed_field FROM custom_target_users WHERE id = 1",
        &[]
    ).await.expect("Failed to verify placeholder replacement");

    assert_eq!(verification_result.rows().len(), 1);

    if let Value::Object(row) = &verification_result.rows()[0] {
        // ID should be unchanged
        assert_eq!(row.get("id").unwrap(), &Value::Number(1.into()));
        // Name should be uppercased
        assert_eq!(
            row.get("transformed_name").unwrap(),
            &Value::String("ALICE".to_string())
        );
        // Age should be doubled
        assert_eq!(row.get("doubled_age").unwrap(), &Value::Number(50.into()));
        // Email should be unchanged
        assert_eq!(
            row.get("original_email").unwrap(),
            &Value::String("alice@example.com".to_string())
        );
        // Computed field should be added
        assert_eq!(
            row.get("computed_field").unwrap(),
            &Value::String("computed_value".to_string())
        );
    }
}

async fn create_test_tables_for_built_in_custom_migration(db: &D1Client) {
    // Create source table with test data for built-in transformations
    db.execute(
        "CREATE TABLE builtin_source_users (
            id INTEGER PRIMARY KEY,
            full_name TEXT NOT NULL,
            age_str TEXT,
            status_code INTEGER,
            display_name TEXT
        )",
        &[],
    )
    .await
    .expect("Failed to create builtin_source_users table");

    // Insert test data
    db.execute(
        "INSERT INTO builtin_source_users (id, full_name, age_str, status_code, display_name) VALUES 
        (1, 'Alice Johnson', '25', 1, 'ALICE_J'),
        (2, 'Bob Smith', '30', 2, 'BOB_S'),
        (3, 'Charlie Brown', '35', 1, 'CHARLIE_B')",
        &[]
    ).await.expect("Failed to insert test data");

    // Create target table
    db.execute(
        "CREATE TABLE builtin_target_users (
            id INTEGER PRIMARY KEY,
            first_name TEXT,
            last_name TEXT,
            age INTEGER,
            status_desc TEXT,
            normalized_name TEXT,
            aggregated_info TEXT
        )",
        &[],
    )
    .await
    .expect("Failed to create builtin_target_users table");
}

#[tokio::test]
async fn test_execute_built_in_custom_migration_validation_errors() {
    let db = setup_test_db().await;
    create_test_tables_for_built_in_custom_migration(&db).await;

    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    // Test empty migration name
    let result = data_migration.execute_built_in_custom_migration(
        "",
        "SELECT * FROM builtin_source_users",
        r#"{"type": "type_conversion", "from_type": "TEXT", "to_type": "INTEGER", "source_column": "age_str"}"#,
        &["INSERT INTO builtin_target_users (id, age) VALUES ($id, $age_str)".to_string()]
    ).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Migration name cannot be empty"));

    // Test empty source query
    let result = data_migration.execute_built_in_custom_migration(
        "test_migration",
        "",
        r#"{"type": "type_conversion", "from_type": "TEXT", "to_type": "INTEGER", "source_column": "age_str"}"#,
        &["INSERT INTO builtin_target_users (id, age) VALUES ($id, $age_str)".to_string()]
    ).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Source query cannot be empty"));

    // Test empty transformation logic
    let result = data_migration
        .execute_built_in_custom_migration(
            "test_migration",
            "SELECT * FROM builtin_source_users",
            "",
            &["INSERT INTO builtin_target_users (id, age) VALUES ($id, $age_str)".to_string()],
        )
        .await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Transformation logic cannot be empty"));

    // Test empty target operations
    let result = data_migration.execute_built_in_custom_migration(
        "test_migration",
        "SELECT * FROM builtin_source_users",
        r#"{"type": "type_conversion", "from_type": "TEXT", "to_type": "INTEGER", "source_column": "age_str"}"#,
        &[]
    ).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("At least one target operation must be specified"));

    // Test invalid JSON transformation logic
    let result = data_migration
        .execute_built_in_custom_migration(
            "test_migration",
            "SELECT * FROM builtin_source_users",
            "invalid_json",
            &["INSERT INTO builtin_target_users (id, age) VALUES ($id, $age_str)".to_string()],
        )
        .await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid transformation logic JSON"));

    // Test missing transformation type
    let result = data_migration
        .execute_built_in_custom_migration(
            "test_migration",
            "SELECT * FROM builtin_source_users",
            r#"{"from_type": "TEXT", "to_type": "INTEGER", "source_column": "age_str"}"#,
            &["INSERT INTO builtin_target_users (id, age) VALUES ($id, $age_str)".to_string()],
        )
        .await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("must specify a 'type' field"));

    // Test unsupported transformation type
    let result = data_migration
        .execute_built_in_custom_migration(
            "test_migration",
            "SELECT * FROM builtin_source_users",
            r#"{"type": "unsupported_type", "source_column": "age_str"}"#,
            &["INSERT INTO builtin_target_users (id, age) VALUES ($id, $age_str)".to_string()],
        )
        .await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Unsupported built-in transformation type"));
}

#[tokio::test]
async fn test_execute_built_in_custom_migration_type_conversion() {
    let db = setup_test_db().await;
    create_test_tables_for_built_in_custom_migration(&db).await;

    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    let transformation_logic = r#"{
        "type": "type_conversion",
        "from_type": "TEXT",
        "to_type": "INTEGER",
        "source_column": "age_str",
        "target_column": "age"
    }"#;

    let target_operations =
        vec!["INSERT INTO builtin_target_users (id, age) VALUES ($id, $age)".to_string()];

    let result = data_migration
        .execute_built_in_custom_migration(
            "type_conversion_migration",
            "SELECT * FROM builtin_source_users",
            transformation_logic,
            &target_operations,
        )
        .await
        .expect("Should execute type conversion successfully");

    assert!(result.success);
    assert_eq!(result.records_processed, 3);
    assert_eq!(result.records_failed, 0);
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Using built-in transformation type: type_conversion")));

    // Verify converted data was inserted correctly
    let verification_result = db
        .execute("SELECT id, age FROM builtin_target_users ORDER BY id", &[])
        .await
        .expect("Failed to verify converted data");

    assert_eq!(verification_result.rows().len(), 3);

    // Verify first row conversion (TEXT "25" -> INTEGER 25)
    if let Value::Object(row) = &verification_result.rows()[0] {
        assert_eq!(row.get("id").unwrap(), &Value::Number(1.into()));
        assert_eq!(row.get("age").unwrap(), &Value::Number(25.into()));
    }
}

#[tokio::test]
async fn test_execute_built_in_custom_migration_value_mapping() {
    let db = setup_test_db().await;
    create_test_tables_for_built_in_custom_migration(&db).await;

    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    let transformation_logic = r#"{
        "type": "value_mapping",
        "source_column": "status_code",
        "target_column": "status_desc",
        "mapping": {
            "1": "Active",
            "2": "Inactive"
        },
        "default_value": "Unknown"
    }"#;

    let target_operations = vec![
        "INSERT INTO builtin_target_users (id, status_desc) VALUES ($id, $status_desc)".to_string(),
    ];

    let result = data_migration
        .execute_built_in_custom_migration(
            "value_mapping_migration",
            "SELECT * FROM builtin_source_users",
            transformation_logic,
            &target_operations,
        )
        .await
        .expect("Should execute value mapping successfully");

    assert!(result.success);
    assert_eq!(result.records_processed, 3);
    assert_eq!(result.records_failed, 0);
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Using built-in transformation type: value_mapping")));

    // Verify mapped data was inserted correctly
    let verification_result = db
        .execute(
            "SELECT id, status_desc FROM builtin_target_users ORDER BY id",
            &[],
        )
        .await
        .expect("Failed to verify mapped data");

    assert_eq!(verification_result.rows().len(), 3);

    // Verify mappings: 1 -> "Active", 2 -> "Inactive"
    if let Value::Object(row) = &verification_result.rows()[0] {
        assert_eq!(
            row.get("status_desc").unwrap(),
            &Value::String("Active".to_string())
        );
    }
    if let Value::Object(row) = &verification_result.rows()[1] {
        assert_eq!(
            row.get("status_desc").unwrap(),
            &Value::String("Inactive".to_string())
        );
    }
}

#[tokio::test]
async fn test_execute_built_in_custom_migration_normalization() {
    let db = setup_test_db().await;
    create_test_tables_for_built_in_custom_migration(&db).await;

    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    let transformation_logic = r#"{
        "type": "normalization",
        "source_column": "full_name",
        "target_fields": ["first_name", "last_name"]
    }"#;

    let target_operations = vec![
        "INSERT INTO builtin_target_users (id, first_name, last_name) VALUES ($id, $first_name, $last_name)".to_string()
    ];

    let result = data_migration
        .execute_built_in_custom_migration(
            "normalization_migration",
            "SELECT * FROM builtin_source_users",
            transformation_logic,
            &target_operations,
        )
        .await
        .expect("Should execute normalization successfully");

    assert!(result.success);
    assert_eq!(result.records_processed, 3);
    assert_eq!(result.records_failed, 0);
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Using built-in transformation type: normalization")));

    // Verify normalized data was inserted correctly
    let verification_result = db
        .execute(
            "SELECT id, first_name, last_name FROM builtin_target_users ORDER BY id",
            &[],
        )
        .await
        .expect("Failed to verify normalized data");

    assert_eq!(verification_result.rows().len(), 3);

    // Verify normalization: "Alice Johnson" -> first_name="Alice", last_name="Johnson"
    if let Value::Object(row) = &verification_result.rows()[0] {
        assert_eq!(
            row.get("first_name").unwrap(),
            &Value::String("Alice".to_string())
        );
        assert_eq!(
            row.get("last_name").unwrap(),
            &Value::String("Johnson".to_string())
        );
    }
}

#[tokio::test]
async fn test_execute_built_in_custom_migration_aggregation() {
    let db = setup_test_db().await;
    create_test_tables_for_built_in_custom_migration(&db).await;

    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    let transformation_logic = r#"{
        "type": "aggregation",
        "source_fields": ["full_name", "display_name"],
        "target_field": "aggregated_info",
        "separator": " | "
    }"#;

    let target_operations = vec![
        "INSERT INTO builtin_target_users (id, aggregated_info) VALUES ($id, $aggregated_info)"
            .to_string(),
    ];

    let result = data_migration
        .execute_built_in_custom_migration(
            "aggregation_migration",
            "SELECT * FROM builtin_source_users",
            transformation_logic,
            &target_operations,
        )
        .await
        .expect("Should execute aggregation successfully");

    assert!(result.success);
    assert_eq!(result.records_processed, 3);
    assert_eq!(result.records_failed, 0);
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Using built-in transformation type: aggregation")));

    // Verify aggregated data was inserted correctly
    let verification_result = db
        .execute(
            "SELECT id, aggregated_info FROM builtin_target_users ORDER BY id",
            &[],
        )
        .await
        .expect("Failed to verify aggregated data");

    assert_eq!(verification_result.rows().len(), 3);

    // Verify aggregation: "Alice Johnson" + "ALICE_J" -> "Alice Johnson | ALICE_J"
    if let Value::Object(row) = &verification_result.rows()[0] {
        assert_eq!(
            row.get("aggregated_info").unwrap(),
            &Value::String("Alice Johnson | ALICE_J".to_string())
        );
    }
}

#[tokio::test]
async fn test_execute_built_in_custom_migration_format_transformation() {
    let db = setup_test_db().await;
    create_test_tables_for_built_in_custom_migration(&db).await;

    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    let transformation_logic = r#"{
        "type": "format_transformation",
        "source_column": "display_name",
        "target_column": "normalized_name",
        "source_format": "uppercase",
        "target_format": "lowercase"
    }"#;

    let target_operations = vec![
        "INSERT INTO builtin_target_users (id, normalized_name) VALUES ($id, $normalized_name)"
            .to_string(),
    ];

    let result = data_migration
        .execute_built_in_custom_migration(
            "format_transformation_migration",
            "SELECT * FROM builtin_source_users",
            transformation_logic,
            &target_operations,
        )
        .await
        .expect("Should execute format transformation successfully");

    assert!(result.success);
    assert_eq!(result.records_processed, 3);
    assert_eq!(result.records_failed, 0);
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Using built-in transformation type: format_transformation")));

    // Verify format transformed data was inserted correctly
    let verification_result = db
        .execute(
            "SELECT id, normalized_name FROM builtin_target_users ORDER BY id",
            &[],
        )
        .await
        .expect("Failed to verify format transformed data");

    assert_eq!(verification_result.rows().len(), 3);

    // Verify format transformation: "ALICE_J" -> "alice_j"
    if let Value::Object(row) = &verification_result.rows()[0] {
        assert_eq!(
            row.get("normalized_name").unwrap(),
            &Value::String("alice_j".to_string())
        );
    }
}

#[tokio::test]
async fn test_execute_built_in_custom_migration_invalid_source_query() {
    let db = setup_test_db().await;
    create_test_tables_for_built_in_custom_migration(&db).await;

    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    let transformation_logic = r#"{
        "type": "type_conversion",
        "from_type": "TEXT",
        "to_type": "INTEGER",
        "source_column": "age_str"
    }"#;

    let target_operations =
        vec!["INSERT INTO builtin_target_users (id, age) VALUES ($id, $age_str)".to_string()];

    let result = data_migration
        .execute_built_in_custom_migration(
            "invalid_source_migration",
            "SELECT * FROM non_existent_table",
            transformation_logic,
            &target_operations,
        )
        .await
        .expect("Should handle invalid source query gracefully");

    assert!(!result.success);
    assert_eq!(result.records_processed, 0);
    assert_eq!(result.records_failed, 1);
    assert!(result
        .errors
        .iter()
        .any(|e| e.to_string().contains("Failed to execute source query")));
}

#[tokio::test]
async fn test_execute_built_in_custom_migration_target_operation_failure() {
    let db = setup_test_db().await;
    create_test_tables_for_built_in_custom_migration(&db).await;

    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    let transformation_logic = r#"{
        "type": "type_conversion",
        "from_type": "TEXT",
        "to_type": "INTEGER",
        "source_column": "age_str",
        "target_column": "age"
    }"#;

    // Use invalid SQL for target operation
    let target_operations =
        vec!["INSERT INTO non_existent_table (id, age) VALUES ($id, $age)".to_string()];

    let result = data_migration
        .execute_built_in_custom_migration(
            "failing_target_migration",
            "SELECT * FROM builtin_source_users",
            transformation_logic,
            &target_operations,
        )
        .await
        .expect("Should handle target operation failures gracefully");

    assert!(!result.success);
    assert_eq!(result.records_processed, 3);
    assert_eq!(result.records_failed, 3);
    assert!(result
        .errors
        .iter()
        .any(|e| e.to_string().contains("Target operation")));
}

#[tokio::test]
async fn test_execute_built_in_custom_migration_transformation_config_errors() {
    let db = setup_test_db().await;
    create_test_tables_for_built_in_custom_migration(&db).await;

    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    // Test type_conversion missing required fields
    let result = data_migration
        .execute_built_in_custom_migration(
            "incomplete_config_migration",
            "SELECT * FROM builtin_source_users",
            r#"{"type": "type_conversion"}"#,
            &["INSERT INTO builtin_target_users (id) VALUES ($id)".to_string()],
        )
        .await
        .expect("Should handle config errors gracefully");

    assert!(!result.success);
    assert!(result.errors.iter().any(|e| e
        .to_string()
        .contains("Built-in transformation 'type_conversion' failed")));

    // Test value_mapping missing required fields
    let result = data_migration
        .execute_built_in_custom_migration(
            "incomplete_mapping_migration",
            "SELECT * FROM builtin_source_users",
            r#"{"type": "value_mapping"}"#,
            &["INSERT INTO builtin_target_users (id) VALUES ($id)".to_string()],
        )
        .await
        .expect("Should handle config errors gracefully");

    assert!(!result.success);
    assert!(result.errors.iter().any(|e| e
        .to_string()
        .contains("Built-in transformation 'value_mapping' failed")));
}

#[tokio::test]
async fn test_execute_built_in_custom_migration_batch_processing() {
    let db = setup_test_db().await;
    create_test_tables_for_built_in_custom_migration(&db).await;

    // Insert more test data for batch testing
    for i in 4..=15 {
        db.execute(
            "INSERT INTO builtin_source_users (id, full_name, age_str, status_code, display_name) VALUES (?, ?, ?, ?, ?)",
            &[
                serde_json::Value::Number(i.into()),
                serde_json::Value::String(format!("User {}", i)),
                serde_json::Value::String((20 + i).to_string()),
                serde_json::Value::Number(1.into()),
                serde_json::Value::String(format!("USER_{}", i))
            ]
        ).await.expect("Failed to insert additional test data");
    }

    let config = DataMigrationConfig {
        batch_size: 5, // Small batch size to test batching
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    let transformation_logic = r#"{
        "type": "type_conversion",
        "from_type": "TEXT",
        "to_type": "INTEGER",
        "source_column": "age_str",
        "target_column": "age"
    }"#;

    let target_operations =
        vec!["INSERT INTO builtin_target_users (id, age) VALUES ($id, $age)".to_string()];

    let result = data_migration
        .execute_built_in_custom_migration(
            "batch_processing_builtin_migration",
            "SELECT * FROM builtin_source_users ORDER BY id",
            transformation_logic,
            &target_operations,
        )
        .await
        .expect("Should handle batch processing successfully");

    assert!(result.success);
    assert_eq!(result.records_processed, 15); // Total records including original 3 + 12 new
    assert_eq!(result.records_failed, 0);

    // Should have batch processing warnings
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Processing batch")));

    // Verify all records were transformed and inserted
    let verification_result = db
        .execute("SELECT COUNT(*) as count FROM builtin_target_users", &[])
        .await
        .expect("Failed to verify batch processing results");

    if let Value::Object(row) = &verification_result.rows()[0] {
        if let Some(Value::Number(count)) = row.get("count") {
            assert_eq!(count.as_u64().unwrap(), 15);
        }
    }
}
// Helper function for restore_from_backup tests
async fn create_test_tables_for_restore_from_backup(db: &D1Client) {
    // Create main test table
    let create_table_sql = r#"
        CREATE TABLE restore_test_users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            email TEXT UNIQUE,
            age INTEGER,
            is_active INTEGER DEFAULT 1
        )
    "#;

    db.execute(create_table_sql, &[])
        .await
        .expect("Failed to create restore test table");

    // Insert test data
    let test_data = vec![
        ("Alice Smith", "alice@example.com", 30),
        ("Bob Johnson", "bob@example.com", 25),
        ("Carol Brown", "carol@example.com", 35),
    ];

    for (name, email, age) in test_data {
        db.execute(
            "INSERT INTO restore_test_users (name, email, age) VALUES (?, ?, ?)",
            &[
                Value::String(name.to_string()),
                Value::String(email.to_string()),
                Value::Number(age.into()),
            ],
        )
        .await
        .expect("Failed to insert test data");
    }

    // Create backup table with modified data
    let create_backup_sql = r#"
        CREATE TABLE restore_test_users_backup_20240101_120000 (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT UNIQUE,
            age INTEGER,
            is_active INTEGER DEFAULT 1
        )
    "#;

    db.execute(create_backup_sql, &[])
        .await
        .expect("Failed to create backup table");

    // Insert backup data (original state before modifications)
    let backup_data = vec![
        (1, "Alice Smith Original", "alice.original@example.com", 29),
        (2, "Bob Johnson Original", "bob.original@example.com", 24),
        (3, "Carol Brown Original", "carol.original@example.com", 34),
    ];

    for (id, name, email, age) in backup_data {
        db.execute(
            "INSERT INTO restore_test_users_backup_20240101_120000 (id, name, email, age) VALUES (?, ?, ?, ?)",
            &[
                Value::Number(id.into()),
                Value::String(name.to_string()),
                Value::String(email.to_string()),
                Value::Number(age.into()),
            ]
        ).await.expect("Failed to insert backup data");
    }
}

#[tokio::test]
async fn test_restore_from_backup_validation_errors() {
    let db = setup_test_db().await;
    create_test_tables_for_restore_from_backup(&db).await;

    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    // Test with empty backup tables
    let empty_rollback_info = RollbackInfo {
        backup_tables: HashMap::new(),
        data_snapshots: HashMap::new(),
        temporary_tables: Vec::new(),
        reverse_operations: Vec::new(),
    };

    let result = data_migration
        .restore_from_backup(&empty_rollback_info)
        .await
        .expect("Should handle empty backup gracefully");

    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("No backup tables or data snapshots to restore from")));
}

#[tokio::test]
async fn test_restore_from_backup_successful_backup_table_restoration() {
    let db = setup_test_db().await;
    create_test_tables_for_restore_from_backup(&db).await;

    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    // Create backup tables mapping
    let mut backup_tables = HashMap::new();
    backup_tables.insert(
        "restore_test_users".to_string(),
        "restore_test_users_backup_20240101_120000".to_string(),
    );

    let rollback_info = RollbackInfo {
        backup_tables,
        data_snapshots: HashMap::new(),
        temporary_tables: Vec::new(),
        reverse_operations: Vec::new(),
    };

    let result = data_migration
        .restore_from_backup(&rollback_info)
        .await
        .expect("Should restore from backup table successfully");

    assert!(result.success);
    assert_eq!(result.records_processed, 3);
    assert_eq!(result.records_failed, 0);

    // Verify data was restored
    let verification_result = db
        .execute("SELECT name FROM restore_test_users WHERE id = 1", &[])
        .await
        .expect("Failed to verify restoration");

    if let Value::Object(row) = &verification_result.rows()[0] {
        if let Some(Value::String(name)) = row.get("name") {
            assert_eq!(name, "Alice Smith Original");
        }
    }
}

#[tokio::test]
async fn test_restore_from_backup_successful_snapshot_restoration() {
    let db = setup_test_db().await;

    // Create a test table for snapshot restoration
    let create_table_sql = r#"
        CREATE TABLE snapshot_test_products (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            price REAL,
            category TEXT
        )
    "#;

    db.execute(create_table_sql, &[])
        .await
        .expect("Failed to create snapshot test table");

    // Insert current data that will be overwritten
    db.execute(
        "INSERT INTO snapshot_test_products (name, price, category) VALUES (?, ?, ?)",
        &[
            Value::String("Modified Product".to_string()),
            Value::Number(serde_json::Number::from_f64(99.99).unwrap()),
            Value::String("Modified Category".to_string()),
        ],
    )
    .await
    .expect("Failed to insert current data");

    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    // Create snapshot data (JSON format)
    let snapshot_data = r#"[
        {"id": 1, "name": "Original Product", "price": 49.99, "category": "Original Category"}
    ]"#;

    let mut data_snapshots = HashMap::new();
    data_snapshots.insert(
        "snapshot_test_products".to_string(),
        snapshot_data.to_string(),
    );

    let rollback_info = RollbackInfo {
        backup_tables: HashMap::new(),
        data_snapshots,
        temporary_tables: Vec::new(),
        reverse_operations: Vec::new(),
    };

    let result = data_migration
        .restore_from_backup(&rollback_info)
        .await
        .expect("Should restore from snapshot successfully");

    assert!(result.success);
    assert_eq!(result.records_processed, 1);
    assert_eq!(result.records_failed, 0);

    // Verify data was restored from snapshot
    let verification_result = db
        .execute(
            "SELECT name, price FROM snapshot_test_products WHERE id = 1",
            &[],
        )
        .await
        .expect("Failed to verify snapshot restoration");

    if let Value::Object(row) = &verification_result.rows()[0] {
        if let Some(Value::String(name)) = row.get("name") {
            assert_eq!(name, "Original Product");
        }
        if let Some(Value::Number(price)) = row.get("price") {
            assert_eq!(price.as_f64().unwrap(), 49.99);
        }
    }
}

#[tokio::test]
async fn test_restore_from_backup_missing_backup_table() {
    let db = setup_test_db().await;
    create_test_tables_for_restore_from_backup(&db).await;

    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    // Reference non-existent backup table
    let mut backup_tables = HashMap::new();
    backup_tables.insert(
        "restore_test_users".to_string(),
        "nonexistent_backup_table".to_string(),
    );

    let rollback_info = RollbackInfo {
        backup_tables,
        data_snapshots: HashMap::new(),
        temporary_tables: Vec::new(),
        reverse_operations: Vec::new(),
    };

    let result = data_migration
        .restore_from_backup(&rollback_info)
        .await
        .expect("Should handle missing backup table gracefully");

    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Backup table nonexistent_backup_table not found")));
}

#[tokio::test]
async fn test_restore_from_backup_invalid_snapshot_data() {
    let db = setup_test_db().await;
    create_test_tables_for_restore_from_backup(&db).await;

    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    // Create invalid JSON snapshot data
    let invalid_snapshot_data = "{ invalid json syntax";

    let mut data_snapshots = HashMap::new();
    data_snapshots.insert(
        "restore_test_users".to_string(),
        invalid_snapshot_data.to_string(),
    );

    let rollback_info = RollbackInfo {
        backup_tables: HashMap::new(),
        data_snapshots,
        temporary_tables: Vec::new(),
        reverse_operations: Vec::new(),
    };

    let result = data_migration
        .restore_from_backup(&rollback_info)
        .await
        .expect("Should handle invalid snapshot gracefully");

    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Failed to parse snapshot data")));
}

#[tokio::test]
async fn test_restore_from_backup_mixed_operations() {
    let db = setup_test_db().await;
    create_test_tables_for_restore_from_backup(&db).await;

    // Create another table for mixed operations
    let create_mixed_table_sql = r#"
        CREATE TABLE mixed_restore_test (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            data TEXT,
            status INTEGER
        )
    "#;

    db.execute(create_mixed_table_sql, &[])
        .await
        .expect("Failed to create mixed test table");

    db.execute(
        "INSERT INTO mixed_restore_test (data, status) VALUES (?, ?)",
        &[
            Value::String("Modified Data".to_string()),
            Value::Number(1.into()),
        ],
    )
    .await
    .expect("Failed to insert mixed test data");

    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    // Set up both backup table and snapshot restoration
    let mut backup_tables = HashMap::new();
    backup_tables.insert(
        "restore_test_users".to_string(),
        "restore_test_users_backup_20240101_120000".to_string(),
    );

    let snapshot_data = r#"[
        {"id": 1, "data": "Original Data", "status": 0}
    ]"#;

    let mut data_snapshots = HashMap::new();
    data_snapshots.insert("mixed_restore_test".to_string(), snapshot_data.to_string());

    let rollback_info = RollbackInfo {
        backup_tables,
        data_snapshots,
        temporary_tables: Vec::new(),
        reverse_operations: Vec::new(),
    };

    let result = data_migration
        .restore_from_backup(&rollback_info)
        .await
        .expect("Should handle mixed operations successfully");

    assert!(result.success);
    assert_eq!(result.records_processed, 4); // 3 from backup table + 1 from snapshot
    assert_eq!(result.records_failed, 0);

    // Verify both restorations worked
    let users_result = db
        .execute("SELECT name FROM restore_test_users WHERE id = 1", &[])
        .await
        .expect("Failed to verify users restoration");

    if let Value::Object(row) = &users_result.rows()[0] {
        if let Some(Value::String(name)) = row.get("name") {
            assert_eq!(name, "Alice Smith Original");
        }
    }

    let mixed_result = db
        .execute("SELECT data FROM mixed_restore_test WHERE id = 1", &[])
        .await
        .expect("Failed to verify mixed restoration");

    if let Value::Object(row) = &mixed_result.rows()[0] {
        if let Some(Value::String(data)) = row.get("data") {
            assert_eq!(data, "Original Data");
        }
    }
}

#[tokio::test]
async fn test_restore_from_backup_batch_processing() {
    let db = setup_test_db().await;

    // Create a larger test table for batch processing
    let create_table_sql = r#"
        CREATE TABLE batch_restore_test (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            value INTEGER
        )
    "#;

    db.execute(create_table_sql, &[])
        .await
        .expect("Failed to create batch test table");

    // Insert many records for batch testing
    for i in 1..=25 {
        db.execute(
            "INSERT INTO batch_restore_test (name, value) VALUES (?, ?)",
            &[
                Value::String(format!("Record {}", i)),
                Value::Number(i.into()),
            ],
        )
        .await
        .expect("Failed to insert batch test data");
    }

    // Create backup table with original data
    let create_backup_sql = r#"
        CREATE TABLE batch_restore_test_backup (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            value INTEGER
        )
    "#;

    db.execute(create_backup_sql, &[])
        .await
        .expect("Failed to create batch backup table");

    // Insert backup data
    for i in 1..=25 {
        db.execute(
            "INSERT INTO batch_restore_test_backup (id, name, value) VALUES (?, ?, ?)",
            &[
                Value::Number(i.into()),
                Value::String(format!("Original Record {}", i)),
                Value::Number((i * 10).into()),
            ],
        )
        .await
        .expect("Failed to insert batch backup data");
    }

    let config = DataMigrationConfig {
        batch_size: 5, // Small batch size to test batching
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    let mut backup_tables = HashMap::new();
    backup_tables.insert(
        "batch_restore_test".to_string(),
        "batch_restore_test_backup".to_string(),
    );

    let rollback_info = RollbackInfo {
        backup_tables,
        data_snapshots: HashMap::new(),
        temporary_tables: Vec::new(),
        reverse_operations: Vec::new(),
    };

    let result = data_migration
        .restore_from_backup(&rollback_info)
        .await
        .expect("Should handle batch processing successfully");

    assert!(result.success);
    assert_eq!(result.records_processed, 25);
    assert_eq!(result.records_failed, 0);

    // Should have batch processing warnings
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Processing batch")));

    // Verify batch restoration worked
    let verification_result = db
        .execute(
            "SELECT COUNT(*) as count FROM batch_restore_test WHERE name LIKE 'Original Record%'",
            &[],
        )
        .await
        .expect("Failed to verify batch restoration");

    if let Value::Object(row) = &verification_result.rows()[0] {
        if let Some(Value::Number(count)) = row.get("count") {
            assert_eq!(count.as_u64().unwrap(), 25);
        }
    }
}

#[tokio::test]
async fn test_restore_from_backup_empty_backup_table() {
    let db = setup_test_db().await;
    create_test_tables_for_restore_from_backup(&db).await;

    // Create empty backup table
    let create_empty_backup_sql = r#"
        CREATE TABLE empty_backup_table (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT UNIQUE,
            age INTEGER,
            is_active INTEGER DEFAULT 1
        )
    "#;

    db.execute(create_empty_backup_sql, &[])
        .await
        .expect("Failed to create empty backup table");

    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    let mut backup_tables = HashMap::new();
    backup_tables.insert(
        "restore_test_users".to_string(),
        "empty_backup_table".to_string(),
    );

    let rollback_info = RollbackInfo {
        backup_tables,
        data_snapshots: HashMap::new(),
        temporary_tables: Vec::new(),
        reverse_operations: Vec::new(),
    };

    let result = data_migration
        .restore_from_backup(&rollback_info)
        .await
        .expect("Should handle empty backup table gracefully");

    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("No records found in backup table")));
}
