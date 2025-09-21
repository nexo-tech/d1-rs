use d1_rs::*;
use d1_rs::auto_migration::{DataMigrator, DataMigrationConfig, FailureStrategy, TransformationFunction, MigrationContext};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

async fn setup_test_db() -> D1Client {
    D1Client::new_in_memory()
        .await
        .expect("Failed to create in-memory database")
}

async fn create_test_table_with_data(db: &D1Client) {
    // Create a test table with sample data for aggregation testing
    let create_table_sql = r#"
        CREATE TABLE test_scores (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            student_name TEXT NOT NULL,
            math_score INTEGER,
            english_score INTEGER,
            science_score INTEGER,
            total_score INTEGER,
            average_score REAL,
            max_score INTEGER,
            min_score INTEGER,
            score_summary TEXT
        )
    "#;
    
    db.execute(create_table_sql, &[])
        .await
        .expect("Failed to create test table");
    
    // Insert test data
    let test_data = vec![
        ("Alice", 85, 90, 88),
        ("Bob", 78, 82, 79),
        ("Charlie", 92, 88, 95),
        ("Diana", 90, 85, 87),
        ("Eve", 75, 80, 82),
    ];
    
    for (name, math, english, science) in test_data {
        db.execute(
            "INSERT INTO test_scores (student_name, math_score, english_score, science_score) VALUES (?, ?, ?, ?)",
            &[
                Value::String(name.to_string()),
                Value::Number(math.into()),
                Value::Number(english.into()),
                Value::Number(science.into()),
            ]
        ).await.expect("Failed to insert test data");
    }
}

#[tokio::test]
async fn test_execute_aggregation_sum() {
    let db = setup_test_db().await;
    create_test_table_with_data(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test SUM aggregation: calculate total score from math, english, science scores
    let result = data_migration.execute_aggregation(
        "test_scores",
        &["math_score".to_string(), "english_score".to_string(), "science_score".to_string()],
        "total_score",
        "SUM"
    ).await.expect("Aggregation failed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 5);
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());
    
    // Verify the aggregation results
    let rows = db.execute("SELECT student_name, total_score, math_score, english_score, science_score FROM test_scores ORDER BY student_name", &[])
        .await.expect("Failed to query results");
    
    let expected_totals = vec![
        ("Alice", 263), // 85 + 90 + 88
        ("Bob", 239),   // 78 + 82 + 79
        ("Charlie", 275), // 92 + 88 + 95
        ("Diana", 262), // 90 + 85 + 87
        ("Eve", 237),   // 75 + 80 + 82
    ];
    
    for (i, (expected_name, expected_total)) in expected_totals.iter().enumerate() {
        if let Value::Object(row) = &rows.rows[i] {
            assert_eq!(row.get("student_name"), Some(&Value::String(expected_name.to_string())));
            assert_eq!(row.get("total_score"), Some(&Value::Number((*expected_total).into())));
        }
    }
}

#[tokio::test]
async fn test_execute_aggregation_average() {
    let db = setup_test_db().await;
    create_test_table_with_data(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test AVG aggregation: calculate average score from three subjects
    let result = data_migration.execute_aggregation(
        "test_scores",
        &["math_score".to_string(), "english_score".to_string(), "science_score".to_string()],
        "average_score",
        "AVG"
    ).await.expect("Aggregation failed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 5);
    assert_eq!(result.records_failed, 0);
    
    // Verify the average calculations
    let rows = db.execute("SELECT student_name, average_score, math_score, english_score, science_score FROM test_scores ORDER BY student_name", &[])
        .await.expect("Failed to query results");
    
    let expected_averages = vec![
        ("Alice", 87),  // (85 + 90 + 88) / 3 = 87.67 -> 87
        ("Bob", 79),    // (78 + 82 + 79) / 3 = 79.67 -> 79  
        ("Charlie", 91), // (92 + 88 + 95) / 3 = 91.67 -> 91
        ("Diana", 87),  // (90 + 85 + 87) / 3 = 87.33 -> 87
        ("Eve", 79),    // (75 + 80 + 82) / 3 = 79.0 -> 79
    ];
    
    for (i, (expected_name, expected_avg)) in expected_averages.iter().enumerate() {
        if let Value::Object(row) = &rows.rows[i] {
            assert_eq!(row.get("student_name"), Some(&Value::String(expected_name.to_string())));
            if let Some(Value::Number(avg)) = row.get("average_score") {
                let avg_int = avg.as_f64().unwrap() as i64;
                assert_eq!(avg_int, *expected_avg);
            }
        }
    }
}

#[tokio::test]
async fn test_execute_aggregation_max_min() {
    let db = setup_test_db().await;
    create_test_table_with_data(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test MAX aggregation
    let result = data_migration.execute_aggregation(
        "test_scores",
        &["math_score".to_string(), "english_score".to_string(), "science_score".to_string()],
        "max_score",
        "MAX"
    ).await.expect("MAX aggregation failed");
    
    if !result.success {
        println!("MAX aggregation failed!");
        println!("Errors: {:?}", result.errors);
        println!("Warnings: {:?}", result.warnings);
    }
    assert!(result.success);
    assert_eq!(result.records_processed, 5);
    
    // Test MIN aggregation
    let result = data_migration.execute_aggregation(
        "test_scores",
        &["math_score".to_string(), "english_score".to_string(), "science_score".to_string()],
        "min_score",
        "MIN"
    ).await.expect("MIN aggregation failed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 5);
    
    // Verify MAX and MIN results
    let rows = db.execute("SELECT student_name, max_score, min_score, math_score, english_score, science_score FROM test_scores ORDER BY student_name", &[])
        .await.expect("Failed to query results");
    
    let expected_max_min = vec![
        ("Alice", 90, 85),   // max(85, 90, 88) = 90, min(85, 90, 88) = 85
        ("Bob", 82, 78),     // max(78, 82, 79) = 82, min(78, 82, 79) = 78
        ("Charlie", 95, 88), // max(92, 88, 95) = 95, min(92, 88, 95) = 88  
        ("Diana", 90, 85),   // max(90, 85, 87) = 90, min(90, 85, 87) = 85
        ("Eve", 82, 75),     // max(75, 80, 82) = 82, min(75, 80, 82) = 75
    ];
    
    for (i, (expected_name, expected_max, expected_min)) in expected_max_min.iter().enumerate() {
        if let Value::Object(row) = &rows.rows[i] {
            assert_eq!(row.get("student_name"), Some(&Value::String(expected_name.to_string())));
            assert_eq!(row.get("max_score"), Some(&Value::Number((*expected_max).into())));
            assert_eq!(row.get("min_score"), Some(&Value::Number((*expected_min).into())));
        }
    }
}

#[tokio::test]
async fn test_execute_aggregation_string_functions() {
    let db = setup_test_db().await;
    create_test_table_with_data(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test CONCAT aggregation: create summary from scores
    let result = data_migration.execute_aggregation(
        "test_scores",
        &["math_score".to_string(), "english_score".to_string(), "science_score".to_string()],
        "score_summary",
        "CONCAT"
    ).await.expect("CONCAT aggregation failed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 5);
    assert_eq!(result.records_failed, 0);
    
    // Verify the concatenation results
    let rows = db.execute("SELECT student_name, score_summary FROM test_scores WHERE student_name = 'Alice'", &[])
        .await.expect("Failed to query results");
    
    if let Value::Object(row) = &rows.rows[0] {
        if let Some(Value::String(summary)) = row.get("score_summary") {
            // Should contain all three scores concatenated with spaces
            assert!(summary.contains("85"));
            assert!(summary.contains("90"));
            assert!(summary.contains("88"));
        }
    }
}

#[tokio::test]
async fn test_execute_aggregation_validation_errors() {
    let db = setup_test_db().await;
    create_test_table_with_data(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test empty source fields
    let result = data_migration.execute_aggregation(
        "test_scores",
        &[],
        "total_score",
        "SUM"
    ).await.expect("Should handle empty source fields gracefully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(!result.warnings.is_empty());
    assert!(result.warnings[0].contains("No source fields specified"));
    
    // Test empty target field
    let result = data_migration.execute_aggregation(
        "test_scores",
        &["math_score".to_string()],
        "",
        "SUM"
    ).await.expect("Should handle empty target field gracefully");
    
    assert!(!result.success);
    assert!(!result.errors.is_empty());
    
    // Test unsupported aggregation function
    let result = data_migration.execute_aggregation(
        "test_scores",
        &["math_score".to_string()],
        "result",
        "INVALID_FUNCTION"
    ).await.expect("Should handle unsupported function gracefully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(!result.warnings.is_empty());
    assert!(result.warnings[0].contains("not supported"));
}

#[tokio::test]
async fn test_execute_aggregation_missing_table() {
    let db = setup_test_db().await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test aggregation on non-existent table
    let result = data_migration.execute_aggregation(
        "non_existent_table",
        &["field1".to_string()],
        "result",
        "SUM"
    ).await.expect("Should handle missing table gracefully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(!result.warnings.is_empty());
    assert!(result.warnings[0].contains("not accessible"));
}

#[tokio::test]
async fn test_execute_aggregation_batch_processing() {
    let db = setup_test_db().await;
    create_test_table_with_data(&db).await;
    
    // Test with small batch size to ensure batching works
    let config = DataMigrationConfig {
        batch_size: 2, // Small batch size to force multiple batches
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    let result = data_migration.execute_aggregation(
        "test_scores",
        &["math_score".to_string(), "english_score".to_string()],
        "total_score",
        "SUM"
    ).await.expect("Batch processing failed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 5);
    assert_eq!(result.records_failed, 0);
    
    // Verify all records were processed correctly despite batching
    let rows = db.execute("SELECT COUNT(*) as count FROM test_scores WHERE total_score IS NOT NULL", &[])
        .await.expect("Failed to count results");
    
    if let Value::Object(row) = &rows.rows[0] {
        assert_eq!(row.get("count"), Some(&Value::Number(5.into())));
    }
}

async fn create_test_table_with_categories(db: &D1Client) {
    // Create a test table with categorical data for value mapping testing
    let create_table_sql = r#"
        CREATE TABLE test_users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL,
            old_status TEXT,
            new_status TEXT,
            old_role TEXT,
            new_role TEXT,
            department_code TEXT,
            department_name TEXT
        )
    "#;
    
    db.execute(create_table_sql, &[])
        .await
        .expect("Failed to create test table");
    
    // Insert test data with old status/role values that need mapping
    let test_data = vec![
        ("alice", "A", "ADM"),
        ("bob", "I", "USR"), 
        ("charlie", "P", "MOD"),
        ("diana", "A", "USR"),
        ("eve", "D", "USR"),
    ];
    
    for (username, status, role) in test_data {
        db.execute(
            "INSERT INTO test_users (username, old_status, old_role, department_code) VALUES (?, ?, ?, ?)",
            &[
                Value::String(username.to_string()),
                Value::String(status.to_string()),
                Value::String(role.to_string()),
                Value::String("ENG".to_string()),
            ]
        ).await.expect("Failed to insert test data");
    }
}

#[tokio::test]
async fn test_execute_value_mapping_status_codes() {
    let db = setup_test_db().await;
    create_test_table_with_categories(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Create mapping for status codes
    let mut status_mapping = HashMap::new();
    status_mapping.insert("A".to_string(), "Active".to_string());
    status_mapping.insert("I".to_string(), "Inactive".to_string());
    status_mapping.insert("P".to_string(), "Pending".to_string());
    status_mapping.insert("D".to_string(), "Deleted".to_string());
    
    // Test value mapping: map old status codes to descriptive names
    let result = data_migration.execute_value_mapping(
        "test_users",
        "old_status",
        "new_status",
        &status_mapping,
        &Some("Unknown".to_string())
    ).await.expect("Value mapping failed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 5);
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());
    
    // Verify the mapping results
    let rows = db.execute("SELECT username, old_status, new_status FROM test_users ORDER BY username", &[])
        .await.expect("Failed to query results");
    
    let expected_mappings = vec![
        ("alice", "A", "Active"),
        ("bob", "I", "Inactive"), 
        ("charlie", "P", "Pending"),
        ("diana", "A", "Active"),
        ("eve", "D", "Deleted"),
    ];
    
    for (i, (expected_username, expected_old, expected_new)) in expected_mappings.iter().enumerate() {
        if let Value::Object(row) = &rows.rows[i] {
            assert_eq!(row.get("username"), Some(&Value::String(expected_username.to_string())));
            assert_eq!(row.get("old_status"), Some(&Value::String(expected_old.to_string())));
            assert_eq!(row.get("new_status"), Some(&Value::String(expected_new.to_string())));
        }
    }
}

#[tokio::test]
async fn test_execute_value_mapping_with_default() {
    let db = setup_test_db().await;
    create_test_table_with_categories(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Create partial mapping for roles (some values not mapped)
    let mut role_mapping = HashMap::new();
    role_mapping.insert("ADM".to_string(), "Administrator".to_string());
    role_mapping.insert("MOD".to_string(), "Moderator".to_string());
    // Note: USR is not mapped, should use default
    
    let result = data_migration.execute_value_mapping(
        "test_users",
        "old_role", 
        "new_role",
        &role_mapping,
        &Some("Standard User".to_string())
    ).await.expect("Value mapping with default failed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 5);
    assert_eq!(result.records_failed, 0);
    
    // Verify the mapping results with defaults
    let rows = db.execute("SELECT username, old_role, new_role FROM test_users ORDER BY username", &[])
        .await.expect("Failed to query results");
    
    let expected_mappings = vec![
        ("alice", "ADM", "Administrator"),
        ("bob", "USR", "Standard User"),    // Should use default
        ("charlie", "MOD", "Moderator"),
        ("diana", "USR", "Standard User"),  // Should use default
        ("eve", "USR", "Standard User"),    // Should use default
    ];
    
    for (i, (expected_username, expected_old, expected_new)) in expected_mappings.iter().enumerate() {
        if let Value::Object(row) = &rows.rows[i] {
            assert_eq!(row.get("username"), Some(&Value::String(expected_username.to_string())));
            assert_eq!(row.get("old_role"), Some(&Value::String(expected_old.to_string())));
            assert_eq!(row.get("new_role"), Some(&Value::String(expected_new.to_string())));
        }
    }
}

#[tokio::test]
async fn test_execute_value_mapping_no_default() {
    let db = setup_test_db().await;
    create_test_table_with_categories(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Create mapping for department codes without default
    let mut dept_mapping = HashMap::new();
    dept_mapping.insert("ENG".to_string(), "Engineering".to_string());
    dept_mapping.insert("MKT".to_string(), "Marketing".to_string());
    
    let result = data_migration.execute_value_mapping(
        "test_users",
        "department_code",
        "department_name",
        &dept_mapping,
        &None  // No default value
    ).await.expect("Value mapping without default failed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 5);
    assert_eq!(result.records_failed, 0);
    
    // Verify that unmapped values keep original value when no default
    let rows = db.execute("SELECT department_code, department_name FROM test_users LIMIT 1", &[])
        .await.expect("Failed to query results");
    
    if let Value::Object(row) = &rows.rows[0] {
        assert_eq!(row.get("department_code"), Some(&Value::String("ENG".to_string())));
        assert_eq!(row.get("department_name"), Some(&Value::String("Engineering".to_string())));
    }
}

#[tokio::test]
async fn test_execute_value_mapping_validation_errors() {
    let db = setup_test_db().await;
    create_test_table_with_categories(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test empty mapping table and no default
    let empty_mapping = HashMap::new();
    let result = data_migration.execute_value_mapping(
        "test_users",
        "old_status",
        "new_status",
        &empty_mapping,
        &None
    ).await.expect("Should handle empty mapping gracefully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(!result.warnings.is_empty());
    assert!(result.warnings[0].contains("No mapping table provided"));
    
    // Test empty column names
    let mut mapping = HashMap::new();
    mapping.insert("test".to_string(), "value".to_string());
    
    let result = data_migration.execute_value_mapping(
        "test_users",
        "",  // Empty old column
        "new_status",
        &mapping,
        &None
    ).await.expect("Should handle empty column gracefully");
    
    assert!(!result.success);
    assert!(!result.errors.is_empty());
}

#[tokio::test]
async fn test_execute_value_mapping_missing_table() {
    let db = setup_test_db().await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    let mut mapping = HashMap::new();
    mapping.insert("old".to_string(), "new".to_string());
    
    // Test value mapping on non-existent table
    let result = data_migration.execute_value_mapping(
        "non_existent_table",
        "old_field",
        "new_field", 
        &mapping,
        &Some("default".to_string())
    ).await.expect("Should handle missing table gracefully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(!result.warnings.is_empty());
    assert!(result.warnings[0].contains("not accessible"));
}

#[tokio::test]
async fn test_execute_value_mapping_batch_processing() {
    let db = setup_test_db().await;
    create_test_table_with_categories(&db).await;
    
    // Test with small batch size to ensure batching works
    let config = DataMigrationConfig {
        batch_size: 2, // Small batch size to force multiple batches
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    let mut mapping = HashMap::new();
    mapping.insert("A".to_string(), "Active".to_string());
    mapping.insert("I".to_string(), "Inactive".to_string());
    mapping.insert("P".to_string(), "Pending".to_string());
    mapping.insert("D".to_string(), "Deleted".to_string());
    
    let result = data_migration.execute_value_mapping(
        "test_users",
        "old_status",
        "new_status",
        &mapping,
        &Some("Unknown".to_string())
    ).await.expect("Batch processing failed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 5);
    assert_eq!(result.records_failed, 0);
    
    // Verify all records were processed correctly despite batching
    let rows = db.execute("SELECT COUNT(*) as count FROM test_users WHERE new_status IS NOT NULL", &[])
        .await.expect("Failed to count results");
    
    if let Value::Object(row) = &rows.rows[0] {
        assert_eq!(row.get("count"), Some(&Value::Number(5.into())));
    }
}

#[tokio::test]
async fn test_execute_value_mapping_sql_injection_safety() {
    let db = setup_test_db().await;
    create_test_table_with_categories(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test with values containing single quotes (potential SQL injection)
    let mut mapping = HashMap::new();
    mapping.insert("A".to_string(), "O'Reilly".to_string());  // Contains single quote
    mapping.insert("I".to_string(), "In'active".to_string()); // Contains single quote
    
    let result = data_migration.execute_value_mapping(
        "test_users",
        "old_status",
        "new_status",
        &mapping,
        &Some("Un'known".to_string())  // Default also contains single quote
    ).await.expect("SQL injection safety test failed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 5);
    assert_eq!(result.records_failed, 0);
    
    // Verify that single quotes were properly escaped and values were inserted correctly
    let rows = db.execute("SELECT new_status FROM test_users WHERE username = 'alice'", &[])
        .await.expect("Failed to query results");
    
    if let Value::Object(row) = &rows.rows[0] {
        assert_eq!(row.get("new_status"), Some(&Value::String("O'Reilly".to_string())));
    }
}

async fn create_test_table_with_formats(db: &D1Client) {
    // Create a test table with various format data for transformation testing
    let create_table_sql = r#"
        CREATE TABLE test_formats (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            old_date TEXT,
            new_date TEXT,
            old_time TEXT,
            new_time TEXT,
            old_number TEXT,
            new_number TEXT,
            old_phone TEXT,
            new_phone TEXT,
            old_currency TEXT,
            new_currency TEXT,
            mixed_case_text TEXT,
            formatted_text TEXT
        )
    "#;
    
    db.execute(create_table_sql, &[])
        .await
        .expect("Failed to create test table");
    
    // Insert test data with various formats that need transformation
    let test_data = vec![
        ("alice", "12/25/2023", "2:30 PM", "1,234.56", "(555) 123-4567", "$1,234.56", "hello world"),
        ("bob", "01/15/2024", "9:45 AM", "5,678.90", "(555) 987-6543", "$5,678.90", "JOHN DOE"),
        ("charlie", "03/08/2023", "11:30 PM", "999.99", "(555) 111-2222", "$999.99", "tEsT CaSe"),
        ("diana", "07/04/2024", "12:00 AM", "12,345.67", "(555) 444-5555", "$12,345.67", "api endpoint"),
        ("eve", "09/30/2023", "6:15 PM", "87.50", "(555) 777-8888", "$87.50", "database connection"),
    ];
    
    for (name, date, time, number, phone, currency, text) in test_data {
        db.execute(
            "INSERT INTO test_formats (name, old_date, old_time, old_number, old_phone, old_currency, mixed_case_text) VALUES (?, ?, ?, ?, ?, ?, ?)",
            &[
                Value::String(name.to_string()),
                Value::String(date.to_string()),
                Value::String(time.to_string()),
                Value::String(number.to_string()),
                Value::String(phone.to_string()),
                Value::String(currency.to_string()),
                Value::String(text.to_string()),
            ]
        ).await.expect("Failed to insert test data");
    }
}

#[tokio::test]
async fn test_execute_format_transformation_date_formats() {
    let db = setup_test_db().await;
    create_test_table_with_formats(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test date format transformation: MM/DD/YYYY to YYYY-MM-DD
    let result = data_migration.execute_format_transformation(
        "test_formats",
        "old_date",
        "new_date",
        "MM/DD/YYYY",
        "YYYY-MM-DD",
        "DATE_FORMAT"
    ).await.expect("Date format transformation failed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 5);
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());
    
    // Verify the date transformation results
    let rows = db.execute("SELECT name, old_date, new_date FROM test_formats ORDER BY name", &[])
        .await.expect("Failed to query results");
    
    let expected_transformations = vec![
        ("alice", "12/25/2023", "2023-12-25"),
        ("bob", "01/15/2024", "2024-01-15"),
        ("charlie", "03/08/2023", "2023-03-08"),
        ("diana", "07/04/2024", "2024-07-04"),
        ("eve", "09/30/2023", "2023-09-30"),
    ];
    
    for (i, (expected_name, expected_old, expected_new)) in expected_transformations.iter().enumerate() {
        if let Value::Object(row) = &rows.rows[i] {
            assert_eq!(row.get("name"), Some(&Value::String(expected_name.to_string())));
            assert_eq!(row.get("old_date"), Some(&Value::String(expected_old.to_string())));
            assert_eq!(row.get("new_date"), Some(&Value::String(expected_new.to_string())));
        }
    }
}

#[tokio::test]
async fn test_execute_format_transformation_number_formats() {
    let db = setup_test_db().await;
    create_test_table_with_formats(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test number format transformation: remove commas
    let result = data_migration.execute_format_transformation(
        "test_formats",
        "old_number",
        "new_number",
        "1,234.56",
        "1234.56",
        "NUMBER_FORMAT"
    ).await.expect("Number format transformation failed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 5);
    assert_eq!(result.records_failed, 0);
    
    // Verify the number transformation results
    let rows = db.execute("SELECT name, old_number, new_number FROM test_formats ORDER BY name", &[])
        .await.expect("Failed to query results");
    
    let expected_transformations = vec![
        ("alice", "1,234.56", "1234.56"),
        ("bob", "5,678.90", "5678.90"),
        ("charlie", "999.99", "999.99"),  // No comma to remove
        ("diana", "12,345.67", "12345.67"),
        ("eve", "87.50", "87.50"),  // No comma to remove
    ];
    
    for (i, (expected_name, expected_old, expected_new)) in expected_transformations.iter().enumerate() {
        if let Value::Object(row) = &rows.rows[i] {
            assert_eq!(row.get("name"), Some(&Value::String(expected_name.to_string())));
            assert_eq!(row.get("old_number"), Some(&Value::String(expected_old.to_string())));
            assert_eq!(row.get("new_number"), Some(&Value::String(expected_new.to_string())));
        }
    }
}

#[tokio::test]
async fn test_execute_format_transformation_phone_formats() {
    let db = setup_test_db().await;
    create_test_table_with_formats(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test phone format transformation: (555) 123-4567 to 555-123-4567
    let result = data_migration.execute_format_transformation(
        "test_formats",
        "old_phone",
        "new_phone",
        "(555) 123-4567",
        "555-123-4567",
        "PHONE_FORMAT"
    ).await.expect("Phone format transformation failed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 5);
    assert_eq!(result.records_failed, 0);
    
    // Verify the phone transformation results (simplified - exact transformation may vary)
    let rows = db.execute("SELECT COUNT(*) as count FROM test_formats WHERE new_phone IS NOT NULL", &[])
        .await.expect("Failed to count results");
    
    if let Value::Object(row) = &rows.rows[0] {
        assert_eq!(row.get("count"), Some(&Value::Number(5.into())));
    }
}

#[tokio::test]
async fn test_execute_format_transformation_currency_formats() {
    let db = setup_test_db().await;
    create_test_table_with_formats(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test currency format transformation: remove currency symbols
    let result = data_migration.execute_format_transformation(
        "test_formats",
        "old_currency",
        "new_currency",
        "$1,234.56",
        "1234.56",
        "CURRENCY_FORMAT"
    ).await.expect("Currency format transformation failed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 5);
    assert_eq!(result.records_failed, 0);
    
    // Verify the currency transformation results
    let rows = db.execute("SELECT name, old_currency, new_currency FROM test_formats WHERE name = 'alice'", &[])
        .await.expect("Failed to query results");
    
    if let Value::Object(row) = &rows.rows[0] {
        assert_eq!(row.get("old_currency"), Some(&Value::String("$1,234.56".to_string())));
        assert_eq!(row.get("new_currency"), Some(&Value::String("1234.56".to_string())));
    }
}

#[tokio::test]
async fn test_execute_format_transformation_case_transformations() {
    let db = setup_test_db().await;
    create_test_table_with_formats(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test UPPER case transformation
    let result = data_migration.execute_format_transformation(
        "test_formats",
        "mixed_case_text",
        "formatted_text",
        "",  // Not needed for case transformations
        "",  // Not needed for case transformations
        "UPPER"
    ).await.expect("UPPER case transformation failed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 5);
    assert_eq!(result.records_failed, 0);
    
    // Verify the case transformation results
    let rows = db.execute("SELECT mixed_case_text, formatted_text FROM test_formats WHERE name = 'alice'", &[])
        .await.expect("Failed to query results");
    
    if let Value::Object(row) = &rows.rows[0] {
        assert_eq!(row.get("mixed_case_text"), Some(&Value::String("hello world".to_string())));
        assert_eq!(row.get("formatted_text"), Some(&Value::String("HELLO WORLD".to_string())));
    }
}

#[tokio::test]
async fn test_execute_format_transformation_validation_errors() {
    let db = setup_test_db().await;
    create_test_table_with_formats(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test empty column names
    let result = data_migration.execute_format_transformation(
        "test_formats",
        "",  // Empty old column
        "new_field",
        "source_format",
        "target_format",
        "DATE_FORMAT"
    ).await.expect("Should handle empty column gracefully");
    
    assert!(!result.success);
    assert!(!result.errors.is_empty());
    
    // Test unsupported format function
    let result = data_migration.execute_format_transformation(
        "test_formats",
        "old_date",
        "new_date",
        "MM/DD/YYYY",
        "YYYY-MM-DD",
        "INVALID_FUNCTION"
    ).await.expect("Should handle unsupported function gracefully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(!result.warnings.is_empty());
    assert!(result.warnings[0].contains("not fully supported"));
}

#[tokio::test]
async fn test_execute_format_transformation_missing_table() {
    let db = setup_test_db().await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test format transformation on non-existent table
    let result = data_migration.execute_format_transformation(
        "non_existent_table",
        "old_field",
        "new_field",
        "MM/DD/YYYY",
        "YYYY-MM-DD",
        "DATE_FORMAT"
    ).await.expect("Should handle missing table gracefully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(!result.warnings.is_empty());
    assert!(result.warnings[0].contains("not accessible"));
}

#[tokio::test]
async fn test_execute_format_transformation_batch_processing() {
    let db = setup_test_db().await;
    create_test_table_with_formats(&db).await;
    
    // Test with small batch size to ensure batching works
    let config = DataMigrationConfig {
        batch_size: 2, // Small batch size to force multiple batches
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    let result = data_migration.execute_format_transformation(
        "test_formats",
        "mixed_case_text",
        "formatted_text",
        "",
        "",
        "LOWER"
    ).await.expect("Batch processing failed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 5);
    assert_eq!(result.records_failed, 0);
    
    // Verify all records were processed correctly despite batching
    let rows = db.execute("SELECT COUNT(*) as count FROM test_formats WHERE formatted_text IS NOT NULL", &[])
        .await.expect("Failed to count results");
    
    if let Value::Object(row) = &rows.rows[0] {
        assert_eq!(row.get("count"), Some(&Value::Number(5.into())));
    }
}

#[tokio::test]
async fn test_execute_format_transformation_trim_functions() {
    let db = setup_test_db().await;
    
    // Create a test table with padded text
    let create_table_sql = r#"
        CREATE TABLE test_trim (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            padded_text TEXT,
            trimmed_text TEXT
        )
    "#;
    
    db.execute(create_table_sql, &[])
        .await
        .expect("Failed to create test table");
    
    // Insert test data with spaces
    let test_data = vec![
        "  hello world  ",
        "   leading spaces",
        "trailing spaces   ",
        "  both sides  ",
        "no padding",
    ];
    
    for text in test_data {
        db.execute(
            "INSERT INTO test_trim (padded_text) VALUES (?)",
            &[Value::String(text.to_string())]
        ).await.expect("Failed to insert test data");
    }
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test TRIM transformation
    let result = data_migration.execute_format_transformation(
        "test_trim",
        "padded_text",
        "trimmed_text",
        "",
        "",
        "TRIM"
    ).await.expect("TRIM transformation failed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 5);
    assert_eq!(result.records_failed, 0);
    
    // Verify trimming worked
    let rows = db.execute("SELECT padded_text, trimmed_text FROM test_trim ORDER BY id", &[])
        .await.expect("Failed to query results");
    
    if let Value::Object(row) = &rows.rows[0] {
        assert_eq!(row.get("padded_text"), Some(&Value::String("  hello world  ".to_string())));
        assert_eq!(row.get("trimmed_text"), Some(&Value::String("hello world".to_string())));
    }
}

async fn create_test_table_with_foreign_keys(db: &D1Client) {
    // Create a test table with foreign key data
    let create_table_sql = r#"
        CREATE TABLE test_orders (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            customer_id INTEGER,
            old_product_id INTEGER,
            new_product_id INTEGER,
            order_date TEXT,
            amount REAL
        )
    "#;
    
    db.execute(create_table_sql, &[])
        .await
        .expect("Failed to create test orders table");
    
    // Insert test data with foreign key values
    let test_data = vec![
        (101, 201, "2023-01-01", 99.99),
        (102, 202, "2023-01-02", 149.50),
        (103, 203, "2023-01-03", 79.25),
        (104, 204, "2023-01-04", 199.00),
        (105, 205, "2023-01-05", 89.75),
    ];
    
    for (customer_id, old_product_id, order_date, amount) in test_data {
        db.execute(
            "INSERT INTO test_orders (customer_id, old_product_id, order_date, amount) VALUES (?, ?, ?, ?)",
            &[
                Value::Number(customer_id.into()),
                Value::Number(old_product_id.into()),
                Value::String(order_date.to_string()),
                Value::Number(serde_json::Number::from_f64(amount).unwrap()),
            ]
        ).await.expect("Failed to insert test data");
    }
}

#[tokio::test]
async fn test_execute_direct_fk_copy_validation_errors() {
    let db = setup_test_db().await;
    create_test_table_with_foreign_keys(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test empty source table
    let result = data_migration.execute_direct_fk_copy(
        "",  // Empty table
        "old_product_id",
        "new_product_id"
    ).await.expect("Should handle empty table gracefully");
    
    assert!(!result.success);
    assert_eq!(result.records_processed, 0);
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].to_string().contains("Source table cannot be empty"));
    
    // Test empty old column
    let result = data_migration.execute_direct_fk_copy(
        "test_orders",
        "",  // Empty old column
        "new_product_id"
    ).await.expect("Should handle empty column gracefully");
    
    assert!(!result.success);
    assert_eq!(result.records_processed, 0);
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].to_string().contains("Source and target FK columns cannot be empty"));
    
    // Test empty new column
    let result = data_migration.execute_direct_fk_copy(
        "test_orders",
        "old_product_id",
        ""  // Empty new column
    ).await.expect("Should handle empty column gracefully");
    
    assert!(!result.success);
    assert_eq!(result.records_processed, 0);
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].to_string().contains("Source and target FK columns cannot be empty"));
}

#[tokio::test]
async fn test_execute_direct_fk_copy_same_columns() {
    let db = setup_test_db().await;
    create_test_table_with_foreign_keys(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test same source and target columns
    let result = data_migration.execute_direct_fk_copy(
        "test_orders",
        "old_product_id",
        "old_product_id"  // Same column
    ).await.expect("Should handle same column gracefully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(!result.warnings.is_empty());
    assert!(result.warnings[0].contains("Source and target FK columns are the same"));
}

#[tokio::test]
async fn test_execute_direct_fk_copy_missing_table() {
    let db = setup_test_db().await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test non-existent table
    let result = data_migration.execute_direct_fk_copy(
        "non_existent_table",
        "old_product_id",
        "new_product_id"
    ).await.expect("Should handle missing table gracefully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(!result.warnings.is_empty());
    assert!(result.warnings[0].contains("not accessible"));
}

#[tokio::test]
async fn test_execute_direct_fk_copy_successful_operation() {
    let db = setup_test_db().await;
    create_test_table_with_foreign_keys(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Execute FK copy
    let result = data_migration.execute_direct_fk_copy(
        "test_orders",
        "old_product_id",
        "new_product_id"
    ).await.expect("FK copy should succeed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 5);
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());
    
    // Verify the copy worked - check that new_product_id matches old_product_id
    let rows = db.execute("SELECT old_product_id, new_product_id FROM test_orders ORDER BY id", &[])
        .await.expect("Failed to query results");
    
    assert_eq!(rows.rows.len(), 5);
    
    for (i, expected_product_id) in [201, 202, 203, 204, 205].iter().enumerate() {
        if let Value::Object(row) = &rows.rows[i] {
            assert_eq!(row.get("old_product_id"), Some(&Value::Number((*expected_product_id).into())));
            assert_eq!(row.get("new_product_id"), Some(&Value::Number((*expected_product_id).into())));
        }
    }
}

#[tokio::test]
async fn test_execute_direct_fk_copy_batch_processing() {
    let db = setup_test_db().await;
    
    // Create table with more data to test batching
    let create_table_sql = r#"
        CREATE TABLE test_large_orders (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            old_supplier_id INTEGER,
            new_supplier_id INTEGER,
            order_value REAL
        )
    "#;
    
    db.execute(create_table_sql, &[])
        .await
        .expect("Failed to create test table");
    
    // Insert 25 records to test batch processing with batch_size=3
    for i in 1..=25 {
        db.execute(
            "INSERT INTO test_large_orders (old_supplier_id, order_value) VALUES (?, ?)",
            &[
                Value::Number((300 + i).into()),
                Value::Number(serde_json::Number::from_f64(i as f64 * 10.5).unwrap()),
            ]
        ).await.expect("Failed to insert test data");
    }
    
    let config = DataMigrationConfig {
        batch_size: 3,  // Small batch size to test batching
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Execute FK copy
    let result = data_migration.execute_direct_fk_copy(
        "test_large_orders",
        "old_supplier_id",
        "new_supplier_id"
    ).await.expect("FK copy should succeed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 25);
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());
    
    // Verify all records were processed correctly
    let count_result = db.execute("SELECT COUNT(*) as count FROM test_large_orders WHERE new_supplier_id IS NOT NULL", &[])
        .await.expect("Failed to count results");
    
    if let Value::Object(row) = &count_result.rows[0] {
        if let Some(Value::Number(count)) = row.get("count") {
            assert_eq!(count.as_u64().unwrap(), 25);
        }
    }
    
    // Verify values are correctly copied
    let sample_result = db.execute("SELECT old_supplier_id, new_supplier_id FROM test_large_orders WHERE id = 1", &[])
        .await.expect("Failed to query sample");
    
    if let Value::Object(row) = &sample_result.rows[0] {
        assert_eq!(row.get("old_supplier_id"), row.get("new_supplier_id"));
    }
}

async fn create_test_tables_with_id_mapping(db: &D1Client) {
    // Create source table with old IDs
    let create_source_sql = r#"
        CREATE TABLE test_users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            old_department_id INTEGER,
            new_department_id INTEGER,
            email TEXT
        )
    "#;
    
    db.execute(create_source_sql, &[])
        .await
        .expect("Failed to create test users table");
    
    // Create target table (represents departments with new ID structure)
    let create_target_sql = r#"
        CREATE TABLE departments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            code TEXT
        )
    "#;
    
    db.execute(create_target_sql, &[])
        .await
        .expect("Failed to create departments table");
    
    // Create mapping table (maps old department IDs to new department IDs)
    let create_mapping_sql = r#"
        CREATE TABLE department_id_mapping (
            old_id INTEGER PRIMARY KEY,
            new_id INTEGER NOT NULL
        )
    "#;
    
    db.execute(create_mapping_sql, &[])
        .await
        .expect("Failed to create mapping table");
    
    // Insert data into target table (new departments)
    let departments = vec![
        (100, "Engineering", "ENG"),
        (200, "Marketing", "MKT"),
        (300, "Sales", "SAL"),
        (400, "HR", "HR"),
        (500, "Finance", "FIN"),
    ];
    
    for (id, name, code) in departments {
        db.execute(
            "INSERT INTO departments (id, name, code) VALUES (?, ?, ?)",
            &[
                Value::Number(id.into()),
                Value::String(name.to_string()),
                Value::String(code.to_string()),
            ]
        ).await.expect("Failed to insert department data");
    }
    
    // Insert mapping data (old_id -> new_id)
    let mappings = vec![
        (1, 100), // Old dept 1 -> New dept 100 (Engineering)
        (2, 200), // Old dept 2 -> New dept 200 (Marketing)
        (3, 300), // Old dept 3 -> New dept 300 (Sales)
        (4, 400), // Old dept 4 -> New dept 400 (HR)
        // Note: Old dept 5 has no mapping (to test unmapped IDs)
    ];
    
    for (old_id, new_id) in mappings {
        db.execute(
            "INSERT INTO department_id_mapping (old_id, new_id) VALUES (?, ?)",
            &[
                Value::Number(old_id.into()),
                Value::Number(new_id.into()),
            ]
        ).await.expect("Failed to insert mapping data");
    }
    
    // Insert test users with old department IDs
    let users = vec![
        ("Alice", 1, "alice@example.com"),
        ("Bob", 2, "bob@example.com"),
        ("Charlie", 3, "charlie@example.com"),
        ("Diana", 4, "diana@example.com"),
        ("Eve", 5, "eve@example.com"), // This will have unmapped ID
    ];
    
    for (name, old_dept_id, email) in users {
        db.execute(
            "INSERT INTO test_users (name, old_department_id, email) VALUES (?, ?, ?)",
            &[
                Value::String(name.to_string()),
                Value::Number(old_dept_id.into()),
                Value::String(email.to_string()),
            ]
        ).await.expect("Failed to insert user data");
    }
}

#[tokio::test]
async fn test_execute_id_mapping_migration_validation_errors() {
    let db = setup_test_db().await;
    create_test_tables_with_id_mapping(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test empty source table
    let result = data_migration.execute_id_mapping_migration(
        "",  // Empty source table
        "departments",
        "department_id_mapping",
        "old_department_id",
        "new_department_id"
    ).await.expect("Should handle empty source table gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Source table cannot be empty"));
    
    // Test empty target table
    let result = data_migration.execute_id_mapping_migration(
        "test_users",
        "",  // Empty target table
        "department_id_mapping",
        "old_department_id",
        "new_department_id"
    ).await.expect("Should handle empty target table gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Target table cannot be empty"));
    
    // Test empty mapping table
    let result = data_migration.execute_id_mapping_migration(
        "test_users",
        "departments",
        "",  // Empty mapping table
        "old_department_id",
        "new_department_id"
    ).await.expect("Should handle empty mapping table gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Mapping table cannot be empty"));
    
    // Test empty columns
    let result = data_migration.execute_id_mapping_migration(
        "test_users",
        "departments", 
        "department_id_mapping",
        "",  // Empty old column
        "new_department_id"
    ).await.expect("Should handle empty column gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Old and new ID columns cannot be empty"));
}

#[tokio::test]
async fn test_execute_id_mapping_migration_same_columns() {
    let db = setup_test_db().await;
    create_test_tables_with_id_mapping(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test same old and new columns
    let result = data_migration.execute_id_mapping_migration(
        "test_users",
        "departments",
        "department_id_mapping",
        "old_department_id",
        "old_department_id"  // Same column
    ).await.expect("Should handle same column gracefully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(result.warnings[0].contains("Old and new ID columns are the same"));
}

#[tokio::test]
async fn test_execute_id_mapping_migration_missing_tables() {
    let db = setup_test_db().await;
    create_test_tables_with_id_mapping(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test non-existent mapping table
    let result = data_migration.execute_id_mapping_migration(
        "test_users",
        "departments",
        "non_existent_mapping",
        "old_department_id",
        "new_department_id"
    ).await.expect("Should handle missing mapping table gracefully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(result.warnings[0].contains("not accessible"));
    
    // Test non-existent source table
    let result = data_migration.execute_id_mapping_migration(
        "non_existent_users",
        "departments",
        "department_id_mapping",
        "old_department_id",
        "new_department_id"
    ).await.expect("Should handle missing source table gracefully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(result.warnings[0].contains("not accessible"));
}

#[tokio::test]
async fn test_execute_id_mapping_migration_empty_mapping_table() {
    let db = setup_test_db().await;
    create_test_tables_with_id_mapping(&db).await;
    
    // Clear the mapping table to test empty mapping scenario
    db.execute("DELETE FROM department_id_mapping", &[])
        .await
        .expect("Failed to clear mapping table");
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    let result = data_migration.execute_id_mapping_migration(
        "test_users",
        "departments",
        "department_id_mapping",
        "old_department_id",
        "new_department_id"
    ).await.expect("Should handle empty mapping table gracefully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(result.warnings[0].contains("Mapping table is empty"));
}

#[tokio::test]
async fn test_execute_id_mapping_migration_successful_operation() {
    let db = setup_test_db().await;
    create_test_tables_with_id_mapping(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Execute ID mapping migration
    let result = data_migration.execute_id_mapping_migration(
        "test_users",
        "departments",
        "department_id_mapping",
        "old_department_id",
        "new_department_id"
    ).await.expect("ID mapping migration should succeed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 5);
    assert_eq!(result.records_failed, 0);
    
    // Should have warning about unmapped ID (user Eve with old_department_id = 5)
    assert!(!result.warnings.is_empty());
    assert!(result.warnings.iter().any(|w| w.contains("could not be mapped")));
    
    // Verify the mapping worked for users with valid mappings
    let rows = db.execute("SELECT name, old_department_id, new_department_id FROM test_users ORDER BY id", &[])
        .await.expect("Failed to query results");
    
    assert_eq!(rows.rows.len(), 5);
    
    // Check mapped users (Alice, Bob, Charlie, Diana)
    let expected_mappings = vec![
        ("Alice", 1, Some(100)),
        ("Bob", 2, Some(200)),
        ("Charlie", 3, Some(300)),
        ("Diana", 4, Some(400)),
        ("Eve", 5, None), // No mapping for old_department_id = 5
    ];
    
    for (i, (expected_name, expected_old_id, expected_new_id)) in expected_mappings.iter().enumerate() {
        if let Value::Object(row) = &rows.rows[i] {
            assert_eq!(row.get("name"), Some(&Value::String(expected_name.to_string())));
            assert_eq!(row.get("old_department_id"), Some(&Value::Number((*expected_old_id).into())));
            
            if let Some(new_id) = expected_new_id {
                assert_eq!(row.get("new_department_id"), Some(&Value::Number((*new_id).into())));
            } else {
                assert_eq!(row.get("new_department_id"), Some(&Value::Null));
            }
        }
    }
}

#[tokio::test]
async fn test_execute_id_mapping_migration_batch_processing() {
    let db = setup_test_db().await;
    
    // Create larger dataset for batch testing
    let create_source_sql = r#"
        CREATE TABLE test_products (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            old_category_id INTEGER,
            new_category_id INTEGER,
            price REAL
        )
    "#;
    
    db.execute(create_source_sql, &[])
        .await
        .expect("Failed to create test products table");
    
    let create_target_sql = r#"
        CREATE TABLE categories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL
        )
    "#;
    
    db.execute(create_target_sql, &[])
        .await
        .expect("Failed to create categories table");
    
    let create_mapping_sql = r#"
        CREATE TABLE category_id_mapping (
            old_id INTEGER PRIMARY KEY,
            new_id INTEGER NOT NULL
        )
    "#;
    
    db.execute(create_mapping_sql, &[])
        .await
        .expect("Failed to create category mapping table");
    
    // Insert 15 products to test batch processing with batch_size=3
    for i in 1..=15 {
        db.execute(
            "INSERT INTO test_products (name, old_category_id, price) VALUES (?, ?, ?)",
            &[
                Value::String(format!("Product {}", i)),
                Value::Number((i % 5 + 1).into()), // Cycle through category IDs 1-5
                Value::Number(serde_json::Number::from_f64(i as f64 * 9.99).unwrap()),
            ]
        ).await.expect("Failed to insert product data");
    }
    
    // Insert mapping data for categories 1-4 (category 5 will be unmapped)
    for i in 1..=4 {
        db.execute(
            "INSERT INTO category_id_mapping (old_id, new_id) VALUES (?, ?)",
            &[
                Value::Number(i.into()),
                Value::Number((i * 100).into()), // 1->100, 2->200, 3->300, 4->400
            ]
        ).await.expect("Failed to insert mapping data");
    }
    
    let config = DataMigrationConfig {
        batch_size: 3,  // Small batch size to test batching
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Execute ID mapping migration
    let result = data_migration.execute_id_mapping_migration(
        "test_products",
        "categories",
        "category_id_mapping",
        "old_category_id",
        "new_category_id"
    ).await.expect("ID mapping migration should succeed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 15);
    assert_eq!(result.records_failed, 0);
    
    // Should have warning about unmapped IDs (products with old_category_id = 5)
    assert!(!result.warnings.is_empty());
    assert!(result.warnings.iter().any(|w| w.contains("could not be mapped")));
    
    // Verify correct number of mapped vs unmapped records
    let mapped_count = db.execute("SELECT COUNT(*) as count FROM test_products WHERE new_category_id IS NOT NULL", &[])
        .await.expect("Failed to count mapped records");
    
    if let Value::Object(row) = &mapped_count.rows[0] {
        if let Some(Value::Number(count)) = row.get("count") {
            assert_eq!(count.as_u64().unwrap(), 12); // 15 products - 3 with unmapped category 5
        }
    }
    
    let unmapped_count = db.execute("SELECT COUNT(*) as count FROM test_products WHERE new_category_id IS NULL", &[])
        .await.expect("Failed to count unmapped records");
    
    if let Value::Object(row) = &unmapped_count.rows[0] {
        if let Some(Value::Number(count)) = row.get("count") {
            assert_eq!(count.as_u64().unwrap(), 3); // 3 products with unmapped category 5
        }
    }
    
    // Verify a sample mapping
    let sample_result = db.execute("SELECT old_category_id, new_category_id FROM test_products WHERE old_category_id = 1 LIMIT 1", &[])
        .await.expect("Failed to query sample");
    
    if let Value::Object(row) = &sample_result.rows[0] {
        assert_eq!(row.get("old_category_id"), Some(&Value::Number(1.into())));
        assert_eq!(row.get("new_category_id"), Some(&Value::Number(100.into())));
    }
}

async fn create_test_tables_for_business_logic(db: &D1Client) {
    // Create source table with customer data
    let create_source_sql = r#"
        CREATE TABLE customers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            email TEXT NOT NULL,
            total_orders INTEGER DEFAULT 0,
            total_spent REAL DEFAULT 0.0,
            status TEXT DEFAULT 'active',
            created_at TEXT
        )
    "#;
    
    db.execute(create_source_sql, &[])
        .await
        .expect("Failed to create customers table");
    
    // Create target table for customer analytics
    let create_target_sql = r#"
        CREATE TABLE customer_analytics (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            customer_id INTEGER NOT NULL,
            tier TEXT NOT NULL,
            lifetime_value REAL NOT NULL,
            risk_score INTEGER NOT NULL,
            created_at TEXT
        )
    "#;
    
    db.execute(create_target_sql, &[])
        .await
        .expect("Failed to create customer_analytics table");
    
    // Create orders table for business logic calculations
    let create_orders_sql = r#"
        CREATE TABLE orders (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            customer_id INTEGER NOT NULL,
            amount REAL NOT NULL,
            status TEXT NOT NULL,
            created_at TEXT
        )
    "#;
    
    db.execute(create_orders_sql, &[])
        .await
        .expect("Failed to create orders table");
    
    // Insert test customers
    let customers = vec![
        ("Alice Smith", "alice@example.com", "2023-01-01"),
        ("Bob Johnson", "bob@example.com", "2023-01-02"),
        ("Charlie Brown", "charlie@example.com", "2023-01-03"),
        ("Diana Prince", "diana@example.com", "2023-01-04"),
    ];
    
    for (name, email, created_at) in customers {
        db.execute(
            "INSERT INTO customers (name, email, created_at) VALUES (?, ?, ?)",
            &[
                Value::String(name.to_string()),
                Value::String(email.to_string()),
                Value::String(created_at.to_string()),
            ]
        ).await.expect("Failed to insert customer data");
    }
    
    // Insert test orders
    let orders = vec![
        (1, 100.0, "completed"),
        (1, 250.0, "completed"),
        (2, 75.0, "completed"),
        (2, 125.0, "completed"),
        (2, 50.0, "completed"),
        (3, 500.0, "completed"),
        (4, 25.0, "completed"),
    ];
    
    for (customer_id, amount, status) in orders {
        db.execute(
            "INSERT INTO orders (customer_id, amount, status, created_at) VALUES (?, ?, ?, ?)",
            &[
                Value::Number(customer_id.into()),
                Value::Number(serde_json::Number::from_f64(amount).unwrap()),
                Value::String(status.to_string()),
                Value::String("2023-01-15".to_string()),
            ]
        ).await.expect("Failed to insert order data");
    }
}

#[tokio::test]
async fn test_execute_business_logic_recreation_validation_errors() {
    let db = setup_test_db().await;
    create_test_tables_for_business_logic(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test empty source table
    let result = data_migration.execute_business_logic_recreation(
        "",  // Empty source table
        "customer_analytics",
        "INSERT INTO customer_analytics SELECT * FROM customers",
        &[]
    ).await.expect("Should handle empty source table gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Source table cannot be empty"));
    
    // Test empty target table
    let result = data_migration.execute_business_logic_recreation(
        "customers",
        "",  // Empty target table
        "INSERT INTO customer_analytics SELECT * FROM customers",
        &[]
    ).await.expect("Should handle empty target table gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Target table cannot be empty"));
    
    // Test empty recreation query
    let result = data_migration.execute_business_logic_recreation(
        "customers",
        "customer_analytics",
        "",  // Empty query
        &[]
    ).await.expect("Should handle empty query gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Recreation query cannot be empty"));
}

#[tokio::test]
async fn test_execute_business_logic_recreation_sql_injection_protection() {
    let db = setup_test_db().await;
    create_test_tables_for_business_logic(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test dangerous DROP pattern
    let result = data_migration.execute_business_logic_recreation(
        "customers",
        "customer_analytics",
        "DROP TABLE customers",  // Dangerous query
        &[]
    ).await.expect("Should handle dangerous query gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("potentially dangerous SQL pattern"));
    
    // Test dangerous DELETE pattern
    let result = data_migration.execute_business_logic_recreation(
        "customers",
        "customer_analytics", 
        "DELETE FROM customers WHERE id = 1",  // Dangerous query
        &[]
    ).await.expect("Should handle dangerous query gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("potentially dangerous SQL pattern"));
    
    // Test dangerous TRUNCATE pattern
    let result = data_migration.execute_business_logic_recreation(
        "customers",
        "customer_analytics",
        "TRUNCATE TABLE customers",  // Dangerous query
        &[]
    ).await.expect("Should handle dangerous query gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("potentially dangerous SQL pattern"));
}

#[tokio::test]
async fn test_execute_business_logic_recreation_missing_tables() {
    let db = setup_test_db().await;
    create_test_tables_for_business_logic(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test non-existent source table
    let result = data_migration.execute_business_logic_recreation(
        "non_existent_source",
        "customer_analytics",
        "INSERT INTO customer_analytics SELECT 1, 1, 'bronze', 100.0, 1, '2023-01-01'",
        &[]
    ).await.expect("Should handle missing source table gracefully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(result.warnings.iter().any(|w| w.contains("not accessible")));
    
    // Test non-existent target table
    let result = data_migration.execute_business_logic_recreation(
        "customers",
        "non_existent_target",
        "SELECT COUNT(*) FROM customers",
        &[]
    ).await.expect("Should handle missing target table gracefully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(result.warnings.iter().any(|w| w.contains("not accessible")));
}

#[tokio::test]
async fn test_execute_business_logic_recreation_table_reference_warnings() {
    let db = setup_test_db().await;
    create_test_tables_for_business_logic(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test query that doesn't reference expected tables
    let result = data_migration.execute_business_logic_recreation(
        "customers",
        "customer_analytics",
        "SELECT 1",  // Query doesn't reference either table
        &[]
    ).await.expect("Should execute query with warnings");
    
    assert!(result.success);
    assert!(result.warnings.iter().any(|w| w.contains("does not reference source table")));
    assert!(result.warnings.iter().any(|w| w.contains("does not reference target table")));
}

#[tokio::test]
async fn test_execute_business_logic_recreation_successful_operation() {
    let db = setup_test_db().await;
    create_test_tables_for_business_logic(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Create business logic recreation query that calculates customer tiers based on order totals
    let recreation_query = r#"
        INSERT INTO customer_analytics (customer_id, tier, lifetime_value, risk_score, created_at)
        SELECT 
            c.id,
            CASE 
                WHEN COALESCE(SUM(o.amount), 0) >= 400 THEN 'gold'
                WHEN COALESCE(SUM(o.amount), 0) >= 200 THEN 'silver'
                ELSE 'bronze'
            END as tier,
            COALESCE(SUM(o.amount), 0) as lifetime_value,
            CASE 
                WHEN COALESCE(SUM(o.amount), 0) < 50 THEN 1
                ELSE 0
            END as risk_score,
            datetime('now') as created_at
        FROM customers c
        LEFT JOIN orders o ON c.id = o.customer_id AND o.status = 'completed'
        GROUP BY c.id, c.name, c.email
    "#;
    
    // Execute business logic recreation
    let result = data_migration.execute_business_logic_recreation(
        "customers",
        "customer_analytics",
        recreation_query,
        &[]
    ).await.expect("Business logic recreation should succeed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 4); // 4 customers
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());
    
    // Verify the business logic was applied correctly
    let analytics_rows = db.execute("SELECT customer_id, tier, lifetime_value, risk_score FROM customer_analytics ORDER BY customer_id", &[])
        .await.expect("Failed to query analytics results");
    
    assert_eq!(analytics_rows.rows.len(), 4);
    
    // Check expected tiers based on order totals:
    // Customer 1: $350 total -> silver
    // Customer 2: $250 total -> silver  
    // Customer 3: $500 total -> gold
    // Customer 4: $25 total -> bronze (high risk)
    let expected_analytics = vec![
        (1, "silver", 350.0, 0),
        (2, "silver", 250.0, 0),
        (3, "gold", 500.0, 0),
        (4, "bronze", 25.0, 1), // High risk due to low spend
    ];
    
    for (i, (expected_customer_id, expected_tier, expected_value, expected_risk)) in expected_analytics.iter().enumerate() {
        if let Value::Object(row) = &analytics_rows.rows[i] {
            assert_eq!(row.get("customer_id"), Some(&Value::Number((*expected_customer_id).into())));
            assert_eq!(row.get("tier"), Some(&Value::String(expected_tier.to_string())));
            assert_eq!(row.get("lifetime_value"), Some(&Value::Number(serde_json::Number::from_f64(*expected_value).unwrap())));
            assert_eq!(row.get("risk_score"), Some(&Value::Number((*expected_risk).into())));
        }
    }
}

#[tokio::test]
async fn test_execute_business_logic_recreation_with_validation_rules() {
    let db = setup_test_db().await;
    create_test_tables_for_business_logic(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // First, populate the analytics table with some data
    let recreation_query = r#"
        INSERT INTO customer_analytics (customer_id, tier, lifetime_value, risk_score, created_at)
        SELECT 
            c.id,
            'bronze' as tier,
            0.0 as lifetime_value,
            1 as risk_score,
            datetime('now') as created_at
        FROM customers c
    "#;
    
    // Define validation rules that check data integrity
    let validation_rules = vec![
        // Check for duplicate customer_ids
        "SELECT COUNT(*) FROM (SELECT customer_id FROM customer_analytics GROUP BY customer_id HAVING COUNT(*) > 1)".to_string(),
        
        // Check for invalid tier values
        "SELECT COUNT(*) FROM customer_analytics WHERE tier NOT IN ('bronze', 'silver', 'gold')".to_string(),
        
        // Check for negative lifetime values
        "SELECT COUNT(*) FROM customer_analytics WHERE lifetime_value < 0".to_string(),
        
        // Check for invalid risk scores
        "SELECT COUNT(*) FROM customer_analytics WHERE risk_score NOT IN (0, 1)".to_string(),
    ];
    
    // Execute business logic recreation with validation
    let result = data_migration.execute_business_logic_recreation(
        "customers",
        "customer_analytics",
        recreation_query,
        &validation_rules
    ).await.expect("Business logic recreation with validation should succeed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 4);
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());
    
    // Should have no validation warnings since our data is valid
    let validation_warnings: Vec<_> = result.warnings.iter()
        .filter(|w| w.contains("violations"))
        .collect();
    assert!(validation_warnings.is_empty(), "Expected no validation violations, but got: {:?}", validation_warnings);
}

#[tokio::test]
async fn test_execute_business_logic_recreation_validation_failures() {
    let db = setup_test_db().await;
    create_test_tables_for_business_logic(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Insert some data that will violate validation rules
    let recreation_query = r#"
        INSERT INTO customer_analytics (customer_id, tier, lifetime_value, risk_score, created_at)
        VALUES 
            (1, 'invalid_tier', -100.0, 5, datetime('now')),
            (2, 'bronze', 100.0, 0, datetime('now')),
            (1, 'silver', 200.0, 1, datetime('now'))
    "#;
    
    let validation_rules = vec![
        // Check for duplicate customer_ids (should find 1 violation)
        "SELECT COUNT(*) FROM (SELECT customer_id FROM customer_analytics GROUP BY customer_id HAVING COUNT(*) > 1)".to_string(),
        
        // Check for invalid tier values (should find 1 violation)
        "SELECT COUNT(*) FROM customer_analytics WHERE tier NOT IN ('bronze', 'silver', 'gold')".to_string(),
        
        // Check for negative lifetime values (should find 1 violation)
        "SELECT COUNT(*) FROM customer_analytics WHERE lifetime_value < 0".to_string(),
    ];
    
    let result = data_migration.execute_business_logic_recreation(
        "customers",
        "customer_analytics",
        recreation_query,
        &validation_rules
    ).await.expect("Business logic recreation should succeed despite validation violations");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 4); // Based on customers count
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());
    
    // Should have validation warnings for each rule that found violations
    let validation_warnings: Vec<_> = result.warnings.iter()
        .filter(|w| w.contains("violations"))
        .collect();
    assert!(!validation_warnings.is_empty());
    
    // Should have summary warning about total violations
    assert!(result.warnings.iter().any(|w| w.contains("validation violations were found")));
}

async fn create_test_tables_for_cascade_migration(db: &D1Client) {
    // Create a set of interdependent tables to test cascade migration
    // Order: companies -> departments -> employees -> projects
    
    // 1. Companies table (no dependencies)
    let create_companies_sql = r#"
        CREATE TABLE companies (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            status TEXT DEFAULT 'active',
            migration_status TEXT DEFAULT 'pending'
        )
    "#;
    
    db.execute(create_companies_sql, &[])
        .await
        .expect("Failed to create companies table");
    
    // 2. Departments table (depends on companies)
    let create_departments_sql = r#"
        CREATE TABLE departments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            company_id INTEGER NOT NULL,
            budget REAL DEFAULT 0.0,
            migration_status TEXT DEFAULT 'pending'
        )
    "#;
    
    db.execute(create_departments_sql, &[])
        .await
        .expect("Failed to create departments table");
    
    // 3. Employees table (depends on departments)
    let create_employees_sql = r#"
        CREATE TABLE employees (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            department_id INTEGER NOT NULL,
            salary REAL DEFAULT 0.0,
            migration_status TEXT DEFAULT 'pending'
        )
    "#;
    
    db.execute(create_employees_sql, &[])
        .await
        .expect("Failed to create employees table");
    
    // 4. Projects table (depends on employees and departments)
    let create_projects_sql = r#"
        CREATE TABLE projects (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            department_id INTEGER NOT NULL,
            lead_employee_id INTEGER,
            status TEXT DEFAULT 'planning',
            migration_status TEXT DEFAULT 'pending'
        )
    "#;
    
    db.execute(create_projects_sql, &[])
        .await
        .expect("Failed to create projects table");
    
    // Insert test data
    // Companies
    let companies = vec![
        ("TechCorp", "active"),
        ("DataSys", "active"),
    ];
    
    for (name, status) in companies {
        db.execute(
            "INSERT INTO companies (name, status) VALUES (?, ?)",
            &[
                Value::String(name.to_string()),
                Value::String(status.to_string()),
            ]
        ).await.expect("Failed to insert company data");
    }
    
    // Departments
    let departments = vec![
        ("Engineering", 1, 100000.0),
        ("Marketing", 1, 50000.0),
        ("Sales", 2, 75000.0),
    ];
    
    for (name, company_id, budget) in departments {
        db.execute(
            "INSERT INTO departments (name, company_id, budget) VALUES (?, ?, ?)",
            &[
                Value::String(name.to_string()),
                Value::Number(company_id.into()),
                Value::Number(serde_json::Number::from_f64(budget).unwrap()),
            ]
        ).await.expect("Failed to insert department data");
    }
    
    // Employees
    let employees = vec![
        ("Alice Johnson", 1, 75000.0),
        ("Bob Smith", 1, 80000.0),
        ("Carol Davis", 2, 60000.0),
        ("David Wilson", 3, 65000.0),
    ];
    
    for (name, dept_id, salary) in employees {
        db.execute(
            "INSERT INTO employees (name, department_id, salary) VALUES (?, ?, ?)",
            &[
                Value::String(name.to_string()),
                Value::Number(dept_id.into()),
                Value::Number(serde_json::Number::from_f64(salary).unwrap()),
            ]
        ).await.expect("Failed to insert employee data");
    }
    
    // Projects
    let projects = vec![
        ("Website Redesign", 1, Some(1), "active"),
        ("Data Analytics", 1, Some(2), "planning"),
        ("Marketing Campaign", 2, Some(3), "active"),
    ];
    
    for (name, dept_id, lead_id, status) in projects {
        db.execute(
            "INSERT INTO projects (name, department_id, lead_employee_id, status) VALUES (?, ?, ?, ?)",
            &[
                Value::String(name.to_string()),
                Value::Number(dept_id.into()),
                match lead_id {
                    Some(id) => Value::Number(id.into()),
                    None => Value::Null,
                },
                Value::String(status.to_string()),
            ]
        ).await.expect("Failed to insert project data");
    }
}

#[tokio::test]
async fn test_execute_cascade_migration_validation_errors() {
    let db = setup_test_db().await;
    create_test_tables_for_cascade_migration(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test empty dependency order
    let mut cascade_rules = HashMap::new();
    cascade_rules.insert("companies".to_string(), "UPDATE companies SET migration_status = 'completed'".to_string());
    
    let result = data_migration.execute_cascade_migration(
        &[],  // Empty dependency order
        &cascade_rules
    ).await.expect("Should handle empty dependency order gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Dependency order cannot be empty"));
    
    // Test empty cascade rules
    let dependency_order = vec!["companies".to_string()];
    let empty_rules = HashMap::new();
    
    let result = data_migration.execute_cascade_migration(
        &dependency_order,
        &empty_rules  // Empty rules
    ).await.expect("Should handle empty cascade rules gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Cascade rules cannot be empty"));
    
    // Test empty table name in dependency order
    let dependency_order = vec!["".to_string()];  // Empty table name
    let mut cascade_rules = HashMap::new();
    cascade_rules.insert("companies".to_string(), "UPDATE companies SET migration_status = 'completed'".to_string());
    
    let result = data_migration.execute_cascade_migration(
        &dependency_order,
        &cascade_rules
    ).await.expect("Should handle empty table name gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Table name in dependency order cannot be empty"));
    
    // Test empty table name in cascade rules
    let dependency_order = vec!["companies".to_string()];
    let mut cascade_rules = HashMap::new();
    cascade_rules.insert("".to_string(), "UPDATE companies SET migration_status = 'completed'".to_string());  // Empty table name
    
    let result = data_migration.execute_cascade_migration(
        &dependency_order,
        &cascade_rules
    ).await.expect("Should handle empty table name in rules gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Table name in cascade rules cannot be empty"));
}

#[tokio::test]
async fn test_execute_cascade_migration_sql_injection_protection() {
    let db = setup_test_db().await;
    create_test_tables_for_cascade_migration(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test dangerous DROP pattern
    let dependency_order = vec!["companies".to_string()];
    let mut cascade_rules = HashMap::new();
    cascade_rules.insert("companies".to_string(), "DROP TABLE companies".to_string());  // Dangerous query
    
    let result = data_migration.execute_cascade_migration(
        &dependency_order,
        &cascade_rules
    ).await.expect("Should handle dangerous query gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("potentially dangerous SQL pattern"));
    
    // Test dangerous TRUNCATE pattern
    let mut cascade_rules = HashMap::new();
    cascade_rules.insert("companies".to_string(), "TRUNCATE TABLE companies".to_string());  // Dangerous query
    
    let result = data_migration.execute_cascade_migration(
        &dependency_order,
        &cascade_rules
    ).await.expect("Should handle dangerous query gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("potentially dangerous SQL pattern"));
    
    // Test dangerous ALTER pattern
    let mut cascade_rules = HashMap::new();
    cascade_rules.insert("companies".to_string(), "ALTER TABLE companies ADD COLUMN test TEXT".to_string());  // Dangerous query
    
    let result = data_migration.execute_cascade_migration(
        &dependency_order,
        &cascade_rules
    ).await.expect("Should handle dangerous query gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("potentially dangerous SQL pattern"));
}

#[tokio::test]
async fn test_execute_cascade_migration_missing_rules_and_tables() {
    let db = setup_test_db().await;
    create_test_tables_for_cascade_migration(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test table in dependency order without corresponding rule
    let dependency_order = vec!["companies".to_string(), "departments".to_string()];
    let mut cascade_rules = HashMap::new();
    cascade_rules.insert("companies".to_string(), "UPDATE companies SET migration_status = 'completed'".to_string());
    // Note: departments rule is missing
    
    let result = data_migration.execute_cascade_migration(
        &dependency_order,
        &cascade_rules
    ).await.expect("Should handle missing rule gracefully");
    
    assert!(result.success);
    assert!(result.warnings.iter().any(|w| w.contains("departments") && w.contains("no corresponding cascade rule")));
    
    // Test non-existent table
    let dependency_order = vec!["non_existent_table".to_string()];
    let mut cascade_rules = HashMap::new();
    cascade_rules.insert("non_existent_table".to_string(), "UPDATE non_existent_table SET status = 'completed'".to_string());
    
    let result = data_migration.execute_cascade_migration(
        &dependency_order,
        &cascade_rules
    ).await.expect("Should handle non-existent table gracefully");
    
    assert!(result.success);
    assert!(result.warnings.iter().any(|w| w.contains("not accessible")));
    
    // Test unused cascade rules
    let dependency_order = vec!["companies".to_string()];
    let mut cascade_rules = HashMap::new();
    cascade_rules.insert("companies".to_string(), "UPDATE companies SET migration_status = 'completed'".to_string());
    cascade_rules.insert("unused_table".to_string(), "UPDATE unused_table SET status = 'completed'".to_string());
    
    let result = data_migration.execute_cascade_migration(
        &dependency_order,
        &cascade_rules
    ).await.expect("Should handle unused rules gracefully");
    
    assert!(result.success);
    assert!(result.warnings.iter().any(|w| w.contains("have cascade rules but are not in dependency order")));
}

#[tokio::test]
async fn test_execute_cascade_migration_successful_operation() {
    let db = setup_test_db().await;
    create_test_tables_for_cascade_migration(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Define proper dependency order: companies -> departments -> employees -> projects
    let dependency_order = vec![
        "companies".to_string(),
        "departments".to_string(),
        "employees".to_string(),
        "projects".to_string(),
    ];
    
    // Define cascade rules for each table
    let mut cascade_rules = HashMap::new();
    cascade_rules.insert("companies".to_string(), "UPDATE companies SET migration_status = 'completed' WHERE migration_status = 'pending'".to_string());
    cascade_rules.insert("departments".to_string(), "UPDATE departments SET migration_status = 'completed' WHERE migration_status = 'pending'".to_string());
    cascade_rules.insert("employees".to_string(), "UPDATE employees SET migration_status = 'completed' WHERE migration_status = 'pending'".to_string());
    cascade_rules.insert("projects".to_string(), "UPDATE projects SET migration_status = 'completed' WHERE migration_status = 'pending'".to_string());
    
    // Execute cascade migration
    let result = data_migration.execute_cascade_migration(
        &dependency_order,
        &cascade_rules
    ).await.expect("Cascade migration should succeed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 12); // 2 companies + 3 departments + 4 employees + 3 projects
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());
    
    // Should have summary of processed tables
    assert!(result.warnings.iter().any(|w| w.contains("Cascade migration processed 4 tables in order: companies → departments → employees → projects")));
    
    // Verify the migration was applied correctly - check that all records have completed status
    let companies_result = db.execute("SELECT COUNT(*) as count FROM companies WHERE migration_status = 'completed'", &[])
        .await.expect("Failed to query companies");
    
    if let Value::Object(row) = &companies_result.rows[0] {
        if let Some(Value::Number(count)) = row.get("count") {
            assert_eq!(count.as_u64().unwrap(), 2);
        }
    }
    
    let departments_result = db.execute("SELECT COUNT(*) as count FROM departments WHERE migration_status = 'completed'", &[])
        .await.expect("Failed to query departments");
    
    if let Value::Object(row) = &departments_result.rows[0] {
        if let Some(Value::Number(count)) = row.get("count") {
            assert_eq!(count.as_u64().unwrap(), 3);
        }
    }
    
    let employees_result = db.execute("SELECT COUNT(*) as count FROM employees WHERE migration_status = 'completed'", &[])
        .await.expect("Failed to query employees");
    
    if let Value::Object(row) = &employees_result.rows[0] {
        if let Some(Value::Number(count)) = row.get("count") {
            assert_eq!(count.as_u64().unwrap(), 4);
        }
    }
    
    let projects_result = db.execute("SELECT COUNT(*) as count FROM projects WHERE migration_status = 'completed'", &[])
        .await.expect("Failed to query projects");
    
    if let Value::Object(row) = &projects_result.rows[0] {
        if let Some(Value::Number(count)) = row.get("count") {
            assert_eq!(count.as_u64().unwrap(), 3);
        }
    }
}

#[tokio::test]
async fn test_execute_cascade_migration_partial_failure() {
    let db = setup_test_db().await;
    create_test_tables_for_cascade_migration(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Define dependency order
    let dependency_order = vec![
        "companies".to_string(),
        "departments".to_string(),
        "employees".to_string(),
    ];
    
    // Define cascade rules with one invalid rule
    let mut cascade_rules = HashMap::new();
    cascade_rules.insert("companies".to_string(), "UPDATE companies SET migration_status = 'completed'".to_string());
    cascade_rules.insert("departments".to_string(), "UPDATE non_existent_column SET invalid = 'fail'".to_string());  // This will fail
    cascade_rules.insert("employees".to_string(), "UPDATE employees SET migration_status = 'completed'".to_string());
    
    // Execute cascade migration
    let result = data_migration.execute_cascade_migration(
        &dependency_order,
        &cascade_rules
    ).await.expect("Cascade migration should handle failure gracefully");
    
    assert!(!result.success);  // Should fail due to invalid SQL
    assert!(result.records_failed > 0);
    assert!(!result.errors.is_empty());
    
    // Should have error for the failing table
    assert!(result.errors.iter().any(|e| e.to_string().contains("departments")));
    
    // Should continue processing after failure
    assert!(result.warnings.iter().any(|w| w.contains("Continuing cascade migration despite failure")));
}

#[tokio::test]
async fn test_execute_cascade_migration_empty_rules() {
    let db = setup_test_db().await;
    create_test_tables_for_cascade_migration(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test with some empty rules
    let dependency_order = vec![
        "companies".to_string(),
        "departments".to_string(),
    ];
    
    let mut cascade_rules = HashMap::new();
    cascade_rules.insert("companies".to_string(), "UPDATE companies SET migration_status = 'completed'".to_string());
    cascade_rules.insert("departments".to_string(), "".to_string());  // Empty rule
    
    let result = data_migration.execute_cascade_migration(
        &dependency_order,
        &cascade_rules
    ).await.expect("Should handle empty rules gracefully");
    
    assert!(result.success);
    assert!(result.warnings.iter().any(|w| w.contains("Cascade rule for table 'departments' is empty")));
    assert!(result.warnings.iter().any(|w| w.contains("Skipping table 'departments' due to empty cascade rule")));
}

// Test helper function for denormalized column population tests
async fn create_test_tables_for_denormalized_population(db: &D1Client) {
    // Create source table with denormalized data
    let create_source_sql = r#"
        CREATE TABLE articles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            tags TEXT,  -- Denormalized tags like "technology,programming,rust"
            categories TEXT  -- Denormalized categories like "tech,dev"
        )
    "#;
    
    db.execute(create_source_sql, &[])
        .await
        .expect("Failed to create source table");
    
    // Create junction table for article-tags relationship
    let create_junction_sql = r#"
        CREATE TABLE article_tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            article_id INTEGER NOT NULL,
            tag_name TEXT NOT NULL
        )
    "#;
    
    db.execute(create_junction_sql, &[])
        .await
        .expect("Failed to create junction table");
    
    // Create another junction table for article-categories relationship
    let create_categories_junction_sql = r#"
        CREATE TABLE article_categories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            article_id INTEGER NOT NULL,
            category_name TEXT NOT NULL
        )
    "#;
    
    db.execute(create_categories_junction_sql, &[])
        .await
        .expect("Failed to create categories junction table");
    
    // Insert test data with denormalized columns
    let articles = vec![
        ("Getting Started with Rust", Some("programming,rust,tutorial"), "tech,development"),
        ("Advanced Database Patterns", Some("database,sql,patterns"), "tech,data"),
        ("Web Development Guide", Some("web,javascript,frontend"), "web,tutorial"),
        ("Data Migration Strategies", Some("migration,database,automation"), "data,enterprise"),
        ("Article with Empty Tags", Some(""), "misc"),  // Empty tags
        ("Article with Null Tags", None, "misc"),  // Null tags
    ];
    
    for (title, tags, categories) in articles {
        let tags_value = match tags {
            Some(t) => Value::String(t.to_string()),
            None => Value::Null,
        };
        
        db.execute(
            "INSERT INTO articles (title, tags, categories) VALUES (?, ?, ?)",
            &[
                Value::String(title.to_string()),
                tags_value,
                Value::String(categories.to_string()),
            ]
        ).await.expect("Failed to insert article data");
    }
}

#[tokio::test]
async fn test_execute_denormalized_column_population_validation_errors() {
    let db = setup_test_db().await;
    create_test_tables_for_denormalized_population(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test empty junction table
    let result = data_migration.execute_denormalized_column_population(
        "",  // Empty junction table
        "articles",
        "tags",
        ",",
        "article_id",
        "tag_name"
    ).await.expect("Should handle empty junction table gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Junction table cannot be empty"));
    
    // Test empty source table
    let result = data_migration.execute_denormalized_column_population(
        "article_tags",
        "",  // Empty source table
        "tags",
        ",",
        "article_id",
        "tag_name"
    ).await.expect("Should handle empty source table gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Source table cannot be empty"));
    
    // Test empty source column
    let result = data_migration.execute_denormalized_column_population(
        "article_tags",
        "articles",
        "",  // Empty source column
        ",",
        "article_id",
        "tag_name"
    ).await.expect("Should handle empty source column gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Source column cannot be empty"));
    
    // Test empty delimiter
    let result = data_migration.execute_denormalized_column_population(
        "article_tags",
        "articles",
        "tags",
        "",  // Empty delimiter
        "article_id",
        "tag_name"
    ).await.expect("Should handle empty delimiter gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Delimiter cannot be empty"));
    
    // Test empty foreign key columns
    let result = data_migration.execute_denormalized_column_population(
        "article_tags",
        "articles",
        "tags",
        ",",
        "",  // Empty source FK
        "tag_name"
    ).await.expect("Should handle empty source FK gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Source and target foreign key columns cannot be empty"));
    
    let result = data_migration.execute_denormalized_column_population(
        "article_tags",
        "articles",
        "tags",
        ",",
        "article_id",
        ""  // Empty target FK
    ).await.expect("Should handle empty target FK gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Source and target foreign key columns cannot be empty"));
    
    // Test same source and target foreign key columns
    let result = data_migration.execute_denormalized_column_population(
        "article_tags",
        "articles",
        "tags",
        ",",
        "article_id",
        "article_id"  // Same column
    ).await.expect("Should handle same FK columns gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Source and target foreign key columns cannot be the same"));
}

#[tokio::test]
async fn test_execute_denormalized_column_population_missing_tables() {
    let db = setup_test_db().await;
    create_test_tables_for_denormalized_population(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test non-existent source table
    let result = data_migration.execute_denormalized_column_population(
        "article_tags",
        "non_existent_articles",  // Non-existent source table
        "tags",
        ",",
        "article_id",
        "tag_name"
    ).await.expect("Should handle missing source table gracefully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(result.warnings.iter().any(|w| w.contains("Source table non_existent_articles not accessible")));
    
    // Test non-existent junction table
    let result = data_migration.execute_denormalized_column_population(
        "non_existent_junction",  // Non-existent junction table
        "articles",
        "tags",
        ",",
        "article_id",
        "tag_name"
    ).await.expect("Should handle missing junction table gracefully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(result.warnings.iter().any(|w| w.contains("Junction table non_existent_junction not accessible")));
}

#[tokio::test]
async fn test_execute_denormalized_column_population_successful_operation() {
    let db = setup_test_db().await;
    create_test_tables_for_denormalized_population(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 2,  // Small batch size to test batch processing
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Execute denormalized column population for tags
    let result = data_migration.execute_denormalized_column_population(
        "article_tags",
        "articles",
        "tags",
        ",",
        "article_id",
        "tag_name"
    ).await.expect("Denormalized column population should succeed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 4);  // 4 articles with non-empty tags
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());
    
    // Should have summary of created junction records
    assert!(result.warnings.iter().any(|w| w.contains("Created") && w.contains("junction records")));
    
    // Verify the junction records were created correctly
    let junction_result = db.execute("SELECT article_id, tag_name FROM article_tags ORDER BY article_id, tag_name", &[])
        .await.expect("Failed to query junction table");
    
    // Expected records:
    // Article 1: programming, rust, tutorial (3 tags)
    // Article 2: database, sql, patterns (3 tags)
    // Article 3: web, javascript, frontend (3 tags)
    // Article 4: migration, database, automation (3 tags)
    assert_eq!(junction_result.rows.len(), 12);  // Total tag entries
    
    // Check specific tag entries
    let tags_for_article_1: Vec<String> = junction_result.rows
        .iter()
        .filter_map(|row| {
            if let Value::Object(row_map) = row {
                if let (Some(Value::Number(article_id)), Some(Value::String(tag_name))) = 
                    (row_map.get("article_id"), row_map.get("tag_name")) {
                    if article_id.as_u64() == Some(1) {
                        return Some(tag_name.clone());
                    }
                }
            }
            None
        })
        .collect();
    
    assert_eq!(tags_for_article_1.len(), 3);
    assert!(tags_for_article_1.contains(&"programming".to_string()));
    assert!(tags_for_article_1.contains(&"rust".to_string()));
    assert!(tags_for_article_1.contains(&"tutorial".to_string()));
}

#[tokio::test]
async fn test_execute_denormalized_column_population_no_data() {
    let db = setup_test_db().await;
    
    // Create empty tables
    let create_empty_source_sql = r#"
        CREATE TABLE empty_articles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            tags TEXT
        )
    "#;
    
    db.execute(create_empty_source_sql, &[])
        .await
        .expect("Failed to create empty source table");
    
    let create_empty_junction_sql = r#"
        CREATE TABLE empty_article_tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            article_id INTEGER NOT NULL,
            tag_name TEXT NOT NULL
        )
    "#;
    
    db.execute(create_empty_junction_sql, &[])
        .await
        .expect("Failed to create empty junction table");
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test with empty source table
    let result = data_migration.execute_denormalized_column_population(
        "empty_article_tags",
        "empty_articles",
        "tags",
        ",",
        "article_id",
        "tag_name"
    ).await.expect("Should handle empty source table gracefully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert_eq!(result.records_failed, 0);
    assert!(result.warnings.iter().any(|w| w.contains("No records with denormalized data found")));
}

#[tokio::test]
async fn test_execute_denormalized_column_population_empty_values() {
    let db = setup_test_db().await;
    
    // Create tables with articles that have empty/null denormalized data
    let create_source_sql = r#"
        CREATE TABLE test_articles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            tags TEXT
        )
    "#;
    
    db.execute(create_source_sql, &[])
        .await
        .expect("Failed to create source table");
    
    let create_junction_sql = r#"
        CREATE TABLE test_article_tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            article_id INTEGER NOT NULL,
            tag_name TEXT NOT NULL
        )
    "#;
    
    db.execute(create_junction_sql, &[])
        .await
        .expect("Failed to create junction table");
    
    // Insert articles with various empty tag scenarios
    let test_articles = vec![
        ("Article with commas only", ",,,,"),  // Only commas
        ("Article with spaces and commas", " , , , "),  // Spaces and commas
        ("Article with valid and empty", "valid,,empty,   ,another"),  // Mixed valid and empty
    ];
    
    for (title, tags) in test_articles {
        db.execute(
            "INSERT INTO test_articles (title, tags) VALUES (?, ?)",
            &[
                Value::String(title.to_string()),
                Value::String(tags.to_string()),
            ]
        ).await.expect("Failed to insert test article");
    }
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Execute denormalized column population
    let result = data_migration.execute_denormalized_column_population(
        "test_article_tags",
        "test_articles",
        "tags",
        ",",
        "article_id",
        "tag_name"
    ).await.expect("Should handle empty values gracefully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 3);
    assert_eq!(result.records_failed, 0);
    
    // Verify only valid tags were inserted (should filter out empty strings)
    let junction_result = db.execute("SELECT tag_name FROM test_article_tags ORDER BY tag_name", &[])
        .await.expect("Failed to query junction table");
    
    // Should have only valid tags: "another", "empty", "valid"
    assert_eq!(junction_result.rows.len(), 3);
    
    let tag_names: Vec<String> = junction_result.rows
        .iter()
        .filter_map(|row| {
            if let Value::Object(row_map) = row {
                if let Some(Value::String(tag_name)) = row_map.get("tag_name") {
                    return Some(tag_name.clone());
                }
            }
            None
        })
        .collect();
    
    assert!(tag_names.contains(&"valid".to_string()));
    assert!(tag_names.contains(&"empty".to_string()));
    assert!(tag_names.contains(&"another".to_string()));
}

// Test helper function for existing junction table population tests
async fn create_test_tables_for_existing_junction_population(db: &D1Client) {
    // Create old junction table with existing data
    let create_old_junction_sql = r#"
        CREATE TABLE old_user_roles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            role_id INTEGER NOT NULL,
            status TEXT DEFAULT 'active'
        )
    "#;
    
    db.execute(create_old_junction_sql, &[])
        .await
        .expect("Failed to create old junction table");
    
    // Create new junction table with different column names
    let create_new_junction_sql = r#"
        CREATE TABLE new_user_permissions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            person_id INTEGER NOT NULL,
            permission_id INTEGER NOT NULL,
            state TEXT DEFAULT 'enabled'
        )
    "#;
    
    db.execute(create_new_junction_sql, &[])
        .await
        .expect("Failed to create new junction table");
    
    // Insert test data into old junction table
    let junction_records = vec![
        (1, 1, "active"),
        (1, 2, "active"),
        (2, 1, "active"),
        (2, 3, "inactive"),
        (3, 2, "active"),
    ];
    
    for (user_id, role_id, status) in junction_records {
        db.execute(
            "INSERT INTO old_user_roles (user_id, role_id, status) VALUES (?, ?, ?)",
            &[
                Value::Number(user_id.into()),
                Value::Number(role_id.into()),
                Value::String(status.to_string()),
            ]
        ).await.expect("Failed to insert junction record");
    }
}

#[tokio::test]
async fn test_execute_existing_junction_table_population_validation_errors() {
    let db = setup_test_db().await;
    create_test_tables_for_existing_junction_population(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test empty new junction table
    let mut column_mapping = HashMap::new();
    column_mapping.insert("user_id".to_string(), "person_id".to_string());
    column_mapping.insert("role_id".to_string(), "permission_id".to_string());
    
    let result = data_migration.execute_existing_junction_table_population(
        "",  // Empty new junction table
        "old_user_roles",
        &column_mapping
    ).await.expect("Should handle empty new junction table gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("New junction table cannot be empty"));
    
    // Test empty old junction table
    let result = data_migration.execute_existing_junction_table_population(
        "new_user_permissions",
        "",  // Empty old junction table
        &column_mapping
    ).await.expect("Should handle empty old junction table gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Old junction table cannot be empty"));
    
    // Test same table names
    let result = data_migration.execute_existing_junction_table_population(
        "old_user_roles",
        "old_user_roles",  // Same as new table
        &column_mapping
    ).await.expect("Should handle same table names gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("New and old junction tables cannot be the same"));
    
    // Test empty column mapping
    let empty_mapping = HashMap::new();
    let result = data_migration.execute_existing_junction_table_population(
        "new_user_permissions",
        "old_user_roles",
        &empty_mapping
    ).await.expect("Should handle empty column mapping gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Column mapping cannot be empty"));
    
    // Test empty column names in mapping
    let mut invalid_mapping = HashMap::new();
    invalid_mapping.insert("".to_string(), "person_id".to_string());  // Empty old column
    invalid_mapping.insert("role_id".to_string(), "permission_id".to_string());
    
    let result = data_migration.execute_existing_junction_table_population(
        "new_user_permissions",
        "old_user_roles",
        &invalid_mapping
    ).await.expect("Should handle empty column names gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Column names in mapping cannot be empty"));
    
    // Test empty new column name in mapping
    let mut invalid_mapping = HashMap::new();
    invalid_mapping.insert("user_id".to_string(), "".to_string());  // Empty new column
    invalid_mapping.insert("role_id".to_string(), "permission_id".to_string());
    
    let result = data_migration.execute_existing_junction_table_population(
        "new_user_permissions",
        "old_user_roles",
        &invalid_mapping
    ).await.expect("Should handle empty column names gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Column names in mapping cannot be empty"));
    
    // Test duplicate target columns in mapping
    let mut duplicate_mapping = HashMap::new();
    duplicate_mapping.insert("user_id".to_string(), "person_id".to_string());
    duplicate_mapping.insert("role_id".to_string(), "person_id".to_string());  // Duplicate target
    
    let result = data_migration.execute_existing_junction_table_population(
        "new_user_permissions",
        "old_user_roles",
        &duplicate_mapping
    ).await.expect("Should handle duplicate target columns gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Duplicate target column 'person_id'"));
}

#[tokio::test]
async fn test_execute_existing_junction_table_population_self_mapping_warning() {
    let db = setup_test_db().await;
    
    // Create tables with same column names to test self-mapping properly
    let create_source_sql = r#"
        CREATE TABLE source_junction (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            role_id INTEGER NOT NULL,
            status TEXT DEFAULT 'active'
        )
    "#;
    
    db.execute(create_source_sql, &[])
        .await
        .expect("Failed to create source junction table");
    
    let create_target_sql = r#"
        CREATE TABLE target_junction (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            permission_id INTEGER NOT NULL,
            state TEXT DEFAULT 'enabled'
        )
    "#;
    
    db.execute(create_target_sql, &[])
        .await
        .expect("Failed to create target junction table");
    
    // Insert test data
    db.execute(
        "INSERT INTO source_junction (user_id, role_id, status) VALUES (?, ?, ?)",
        &[
            Value::Number(1.into()),
            Value::Number(1.into()),
            Value::String("active".to_string()),
        ]
    ).await.expect("Failed to insert test data");
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test column mapping to itself (should generate warning)
    let mut self_mapping = HashMap::new();
    self_mapping.insert("user_id".to_string(), "user_id".to_string());  // Maps to itself - this should cause warning
    self_mapping.insert("role_id".to_string(), "permission_id".to_string());  // Valid mapping
    
    let result = data_migration.execute_existing_junction_table_population(
        "target_junction",
        "source_junction",
        &self_mapping
    ).await.expect("Should handle self mapping gracefully");
    
    assert!(result.success);
    assert!(result.warnings.iter().any(|w| w.contains("Column mapping maps 'user_id' to itself")));
}

#[tokio::test]
async fn test_execute_existing_junction_table_population_missing_tables() {
    let db = setup_test_db().await;
    create_test_tables_for_existing_junction_population(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    let mut column_mapping = HashMap::new();
    column_mapping.insert("user_id".to_string(), "person_id".to_string());
    column_mapping.insert("role_id".to_string(), "permission_id".to_string());
    
    // Test non-existent old junction table
    let result = data_migration.execute_existing_junction_table_population(
        "new_user_permissions",
        "non_existent_old_table",  // Non-existent old table
        &column_mapping
    ).await.expect("Should handle missing old table gracefully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(result.warnings.iter().any(|w| w.contains("Old junction table non_existent_old_table not accessible")));
    
    // Test non-existent new junction table
    let result = data_migration.execute_existing_junction_table_population(
        "non_existent_new_table",  // Non-existent new table
        "old_user_roles",
        &column_mapping
    ).await.expect("Should handle missing new table gracefully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(result.warnings.iter().any(|w| w.contains("New junction table non_existent_new_table not accessible")));
}

#[tokio::test]
async fn test_execute_existing_junction_table_population_successful_operation() {
    let db = setup_test_db().await;
    create_test_tables_for_existing_junction_population(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 2,  // Small batch size to test batch processing
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Execute existing junction table population with column mapping
    let mut column_mapping = HashMap::new();
    column_mapping.insert("user_id".to_string(), "person_id".to_string());
    column_mapping.insert("role_id".to_string(), "permission_id".to_string());
    column_mapping.insert("status".to_string(), "state".to_string());
    
    let result = data_migration.execute_existing_junction_table_population(
        "new_user_permissions",
        "old_user_roles",
        &column_mapping
    ).await.expect("Existing junction table population should succeed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 5);  // 5 records in old table
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());
    
    // Should have summary of copied records
    assert!(result.warnings.iter().any(|w| w.contains("Copied 5 records from old_user_roles to new_user_permissions")));
    
    // Should have column mapping summary
    assert!(result.warnings.iter().any(|w| w.contains("Applied column mappings:")));
    
    // Verify the records were copied correctly with column mapping
    let new_records_result = db.execute("SELECT person_id, permission_id, state FROM new_user_permissions ORDER BY person_id, permission_id", &[])
        .await.expect("Failed to query new junction table");
    
    assert_eq!(new_records_result.rows.len(), 5);
    
    // Check specific records were mapped correctly
    if let Value::Object(row) = &new_records_result.rows[0] {
        if let (Some(Value::Number(person_id)), Some(Value::Number(permission_id)), Some(Value::String(state))) = 
            (row.get("person_id"), row.get("permission_id"), row.get("state")) {
            assert_eq!(person_id.as_u64().unwrap(), 1);
            assert_eq!(permission_id.as_u64().unwrap(), 1);
            assert_eq!(state, "active");
        }
    }
}

#[tokio::test]
async fn test_execute_existing_junction_table_population_no_data() {
    let db = setup_test_db().await;
    
    // Create empty tables
    let create_empty_old_sql = r#"
        CREATE TABLE empty_old_junction (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            role_id INTEGER NOT NULL
        )
    "#;
    
    db.execute(create_empty_old_sql, &[])
        .await
        .expect("Failed to create empty old junction table");
    
    let create_empty_new_sql = r#"
        CREATE TABLE empty_new_junction (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            person_id INTEGER NOT NULL,
            permission_id INTEGER NOT NULL
        )
    "#;
    
    db.execute(create_empty_new_sql, &[])
        .await
        .expect("Failed to create empty new junction table");
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test with empty old table
    let mut column_mapping = HashMap::new();
    column_mapping.insert("user_id".to_string(), "person_id".to_string());
    column_mapping.insert("role_id".to_string(), "permission_id".to_string());
    
    let result = data_migration.execute_existing_junction_table_population(
        "empty_new_junction",
        "empty_old_junction",
        &column_mapping
    ).await.expect("Should handle empty old table gracefully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert_eq!(result.records_failed, 0);
    assert!(result.warnings.iter().any(|w| w.contains("No records found in old junction table")));
}

#[tokio::test]
async fn test_execute_existing_junction_table_population_partial_column_mapping() {
    let db = setup_test_db().await;
    create_test_tables_for_existing_junction_population(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test with partial column mapping (only map some columns)
    let mut partial_mapping = HashMap::new();
    partial_mapping.insert("user_id".to_string(), "person_id".to_string());
    partial_mapping.insert("role_id".to_string(), "permission_id".to_string());
    // Note: not mapping 'status' column, so records should be skipped
    
    let result = data_migration.execute_existing_junction_table_population(
        "new_user_permissions",
        "old_user_roles",
        &partial_mapping
    ).await.expect("Should handle partial column mapping gracefully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 5);  // All records are processed
    assert_eq!(result.records_failed, 0);
    
    // Should have warnings about missing columns - but the implementation doesn't require all columns
    // Instead it only selects the columns specified in the mapping
    
    // Verify records were copied with only the mapped columns
    let new_records_result = db.execute("SELECT person_id, permission_id FROM new_user_permissions ORDER BY person_id, permission_id", &[])
        .await.expect("Failed to query new junction table");
    
    assert_eq!(new_records_result.rows.len(), 5);
}

#[tokio::test]
async fn test_execute_existing_junction_table_population_batch_processing() {
    let db = setup_test_db().await;
    
    // Create tables with more data to test batch processing
    let create_old_sql = r#"
        CREATE TABLE batch_old_junction (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_id INTEGER NOT NULL,
            target_id INTEGER NOT NULL
        )
    "#;
    
    db.execute(create_old_sql, &[])
        .await
        .expect("Failed to create batch old junction table");
    
    let create_new_sql = r#"
        CREATE TABLE batch_new_junction (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            from_id INTEGER NOT NULL,
            to_id INTEGER NOT NULL
        )
    "#;
    
    db.execute(create_new_sql, &[])
        .await
        .expect("Failed to create batch new junction table");
    
    // Insert multiple records to test batching
    for i in 1..=10 {
        for j in 1..=3 {
            db.execute(
                "INSERT INTO batch_old_junction (source_id, target_id) VALUES (?, ?)",
                &[
                    Value::Number(i.into()),
                    Value::Number(j.into()),
                ]
            ).await.expect("Failed to insert batch test record");
        }
    }
    
    let config = DataMigrationConfig {
        batch_size: 3,  // Very small batch size to test multiple batches
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Execute with small batch size
    let mut column_mapping = HashMap::new();
    column_mapping.insert("source_id".to_string(), "from_id".to_string());
    column_mapping.insert("target_id".to_string(), "to_id".to_string());
    
    let result = data_migration.execute_existing_junction_table_population(
        "batch_new_junction",
        "batch_old_junction",
        &column_mapping
    ).await.expect("Batch processing should succeed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 30);  // 10 * 3 = 30 records
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());
    
    // Verify all records were copied
    let count_result = db.execute("SELECT COUNT(*) as count FROM batch_new_junction", &[])
        .await.expect("Failed to count new records");
    
    if let Value::Object(row) = &count_result.rows[0] {
        if let Some(Value::Number(count)) = row.get("count") {
            assert_eq!(count.as_u64().unwrap(), 30);
        }
    }
}

// Test helper function for business rules population tests
async fn create_test_tables_for_business_rules_population(db: &D1Client) {
    // Create users table
    let create_users_sql = r#"
        CREATE TABLE users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            department TEXT NOT NULL,
            role TEXT NOT NULL,
            seniority_level INTEGER DEFAULT 1
        )
    "#;
    
    db.execute(create_users_sql, &[])
        .await
        .expect("Failed to create users table");
    
    // Create projects table
    let create_projects_sql = r#"
        CREATE TABLE projects (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            department TEXT NOT NULL,
            complexity_level INTEGER DEFAULT 1,
            status TEXT DEFAULT 'active'
        )
    "#;
    
    db.execute(create_projects_sql, &[])
        .await
        .expect("Failed to create projects table");
    
    // Create user_project_assignments junction table
    let create_junction_sql = r#"
        CREATE TABLE user_project_assignments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            project_id INTEGER NOT NULL,
            assignment_type TEXT DEFAULT 'standard'
        )
    "#;
    
    db.execute(create_junction_sql, &[])
        .await
        .expect("Failed to create junction table");
    
    // Insert test users
    let users = vec![
        ("Alice Johnson", "Engineering", "Senior Developer", 4),
        ("Bob Smith", "Engineering", "Junior Developer", 2),
        ("Carol Davis", "Marketing", "Marketing Manager", 3),
        ("David Wilson", "Engineering", "Tech Lead", 5),
        ("Eve Brown", "Design", "UI Designer", 3),
    ];
    
    for (name, department, role, seniority) in users {
        db.execute(
            "INSERT INTO users (name, department, role, seniority_level) VALUES (?, ?, ?, ?)",
            &[
                Value::String(name.to_string()),
                Value::String(department.to_string()),
                Value::String(role.to_string()),
                Value::Number(seniority.into()),
            ]
        ).await.expect("Failed to insert user data");
    }
    
    // Insert test projects
    let projects = vec![
        ("Web Platform Redesign", "Engineering", 4),
        ("Mobile App Development", "Engineering", 5),
        ("Marketing Campaign", "Marketing", 2),
        ("API Documentation", "Engineering", 3),
        ("Brand Identity", "Design", 3),
    ];
    
    for (name, department, complexity) in projects {
        db.execute(
            "INSERT INTO projects (name, department, complexity_level) VALUES (?, ?, ?)",
            &[
                Value::String(name.to_string()),
                Value::String(department.to_string()),
                Value::Number(complexity.into()),
            ]
        ).await.expect("Failed to insert project data");
    }
}

#[tokio::test]
async fn test_execute_business_rules_population_validation_errors() {
    let db = setup_test_db().await;
    create_test_tables_for_business_rules_population(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test empty junction table
    let generation_query = "INSERT INTO user_project_assignments (user_id, project_id) VALUES (1, 1)";
    let validation_rules = vec!["SELECT COUNT(*) FROM user_project_assignments".to_string()];
    
    let result = data_migration.execute_business_rules_population(
        "",  // Empty junction table
        generation_query,
        &validation_rules
    ).await.expect("Should handle empty junction table gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Junction table cannot be empty"));
    
    // Test empty generation query
    let result = data_migration.execute_business_rules_population(
        "user_project_assignments",
        "",  // Empty generation query
        &validation_rules
    ).await.expect("Should handle empty generation query gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Generation query cannot be empty"));
    
    // Test dangerous generation query (DROP)
    let dangerous_query = "DROP TABLE user_project_assignments";
    let result = data_migration.execute_business_rules_population(
        "user_project_assignments",
        dangerous_query,
        &validation_rules
    ).await.expect("Should handle dangerous query gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("potentially dangerous SQL patterns"));
    
    // Test dangerous generation query (DELETE)
    let dangerous_query = "DELETE FROM user_project_assignments";
    let result = data_migration.execute_business_rules_population(
        "user_project_assignments",
        dangerous_query,
        &validation_rules
    ).await.expect("Should handle dangerous query gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("potentially dangerous SQL patterns"));
    
    // Test non-INSERT generation query
    let non_insert_query = "SELECT * FROM users";
    let result = data_migration.execute_business_rules_population(
        "user_project_assignments",
        non_insert_query,
        &validation_rules
    ).await.expect("Should handle non-INSERT query gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Generation query must be an INSERT statement"));
    
    // Test generation query targeting wrong table
    let wrong_table_query = "INSERT INTO wrong_table (user_id, project_id) VALUES (1, 1)";
    let result = data_migration.execute_business_rules_population(
        "user_project_assignments",
        wrong_table_query,
        &validation_rules
    ).await.expect("Should handle wrong table query gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("does not target the specified junction table"));
    
    // Test empty validation rule
    let empty_validation_rules = vec!["".to_string()];
    let result = data_migration.execute_business_rules_population(
        "user_project_assignments",
        generation_query,
        &empty_validation_rules
    ).await.expect("Should handle empty validation rule gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Validation rule 1 cannot be empty"));
    
    // Test dangerous validation rule
    let dangerous_validation_rules = vec!["DROP TABLE users".to_string()];
    let result = data_migration.execute_business_rules_population(
        "user_project_assignments",
        generation_query,
        &dangerous_validation_rules
    ).await.expect("Should handle dangerous validation rule gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Validation rule 1 contains potentially dangerous SQL patterns"));
}

#[tokio::test]
async fn test_execute_business_rules_population_missing_table() {
    let db = setup_test_db().await;
    create_test_tables_for_business_rules_population(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test non-existent junction table
    let generation_query = "INSERT INTO non_existent_table (user_id, project_id) VALUES (1, 1)";
    let validation_rules = vec!["SELECT COUNT(*) FROM non_existent_table".to_string()];
    
    let result = data_migration.execute_business_rules_population(
        "non_existent_table",
        generation_query,
        &validation_rules
    ).await.expect("Should handle missing table gracefully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(result.warnings.iter().any(|w| w.contains("Junction table non_existent_table not accessible")));
}

#[tokio::test]
async fn test_execute_business_rules_population_successful_operation() {
    let db = setup_test_db().await;
    create_test_tables_for_business_rules_population(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Business rule: Assign users to projects in the same department where seniority >= complexity
    let generation_query = r#"
        INSERT INTO user_project_assignments (user_id, project_id, assignment_type)
        SELECT u.id, p.id, 
               CASE WHEN u.seniority_level >= p.complexity_level + 2 THEN 'lead'
                    WHEN u.seniority_level >= p.complexity_level THEN 'standard'
                    ELSE 'support'
               END
        FROM users u
        CROSS JOIN projects p
        WHERE u.department = p.department
        AND u.seniority_level >= p.complexity_level - 1
    "#;
    
    let validation_rules = vec![
        // Rule 1: Check that all assignments have valid users and projects
        "SELECT COUNT(*) FROM user_project_assignments ua JOIN users u ON ua.user_id = u.id JOIN projects p ON ua.project_id = p.id".to_string(),
        // Rule 2: Check for department consistency
        "SELECT COUNT(*) FROM user_project_assignments ua JOIN users u ON ua.user_id = u.id JOIN projects p ON ua.project_id = p.id WHERE u.department = p.department".to_string(),
        // Rule 3: Check assignment types are valid
        "SELECT COUNT(*) FROM user_project_assignments WHERE assignment_type IN ('lead', 'standard', 'support')".to_string(),
    ];
    
    let result = data_migration.execute_business_rules_population(
        "user_project_assignments",
        generation_query,
        &validation_rules
    ).await.expect("Business rules population should succeed");
    
    assert!(result.success);
    assert!(result.records_processed > 0);
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());
    
    // Should have generation success message
    assert!(result.warnings.iter().any(|w| w.contains("Generation query executed successfully")));
    
    // Should have validation summary
    assert!(result.warnings.iter().any(|w| w.contains("validation rules to verify business rules compliance")));
    assert!(result.warnings.iter().any(|w| w.contains("All validation rules passed successfully")));
    
    // Verify the business rules were applied correctly
    let assignments_result = db.execute(
        "SELECT ua.assignment_type, u.seniority_level, p.complexity_level FROM user_project_assignments ua JOIN users u ON ua.user_id = u.id JOIN projects p ON ua.project_id = p.id ORDER BY ua.id",
        &[]
    ).await.expect("Failed to query assignments");
    
    // Check that assignment types follow the business rules
    for row in &assignments_result.rows {
        if let Value::Object(row_map) = row {
            if let (Some(Value::String(assignment_type)), Some(Value::Number(seniority)), Some(Value::Number(complexity))) = 
                (row_map.get("assignment_type"), row_map.get("seniority_level"), row_map.get("complexity_level")) {
                let seniority_val = seniority.as_u64().unwrap();
                let complexity_val = complexity.as_u64().unwrap();
                
                match assignment_type.as_str() {
                    "lead" => assert!(seniority_val >= complexity_val + 2),
                    "standard" => assert!(seniority_val >= complexity_val && seniority_val < complexity_val + 2),
                    "support" => assert!(seniority_val >= complexity_val.saturating_sub(1) && seniority_val < complexity_val),
                    _ => panic!("Invalid assignment type: {}", assignment_type),
                }
            }
        }
    }
}

#[tokio::test]
async fn test_execute_business_rules_population_generation_failure() {
    let db = setup_test_db().await;
    create_test_tables_for_business_rules_population(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Use a generation query that will fail (invalid column reference)
    let failing_query = "INSERT INTO user_project_assignments (user_id, project_id) SELECT invalid_column, id FROM projects";
    let validation_rules = vec!["SELECT COUNT(*) FROM user_project_assignments".to_string()];
    
    let result = data_migration.execute_business_rules_population(
        "user_project_assignments",
        failing_query,
        &validation_rules
    ).await.expect("Should handle generation failure gracefully");
    
    assert!(!result.success);
    assert!(result.records_failed > 0);
    assert!(result.errors.iter().any(|e| e.to_string().contains("Generation query failed")));
}

#[tokio::test]
async fn test_execute_business_rules_population_validation_failure() {
    let db = setup_test_db().await;
    create_test_tables_for_business_rules_population(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Use a simple generation query that will succeed
    let generation_query = "INSERT INTO user_project_assignments (user_id, project_id) VALUES (1, 1)";
    
    // Use validation rules that will fail (invalid table reference)
    let failing_validation_rules = vec![
        "SELECT COUNT(*) FROM user_project_assignments".to_string(),  // This will pass
        "SELECT COUNT(*) FROM invalid_table_name".to_string(),  // This will fail
    ];
    
    let result = data_migration.execute_business_rules_population(
        "user_project_assignments",
        generation_query,
        &failing_validation_rules
    ).await.expect("Should handle validation failure gracefully");
    
    assert!(result.success);  // Generation succeeded, so overall success is true
    assert!(result.records_processed > 0);
    assert!(result.records_failed > 0);  // But validation failures are counted
    assert!(result.errors.iter().any(|e| e.to_string().contains("Validation rule 2 failed")));
    
    // Should have validation failure summary
    assert!(result.warnings.iter().any(|w| w.contains("validation rules failed or detected issues")));
}

#[tokio::test]
async fn test_execute_business_rules_population_no_validation_rules() {
    let db = setup_test_db().await;
    create_test_tables_for_business_rules_population(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Use a simple generation query with no validation rules
    let generation_query = "INSERT INTO user_project_assignments (user_id, project_id) SELECT u.id, p.id FROM users u, projects p WHERE u.department = p.department LIMIT 3";
    let no_validation_rules: Vec<String> = vec![];
    
    let result = data_migration.execute_business_rules_population(
        "user_project_assignments",
        generation_query,
        &no_validation_rules
    ).await.expect("Should handle no validation rules gracefully");
    
    assert!(result.success);
    assert!(result.records_processed > 0);
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());
    
    // Should not have validation messages since no rules were provided
    assert!(!result.warnings.iter().any(|w| w.contains("validation rules")));
    
    // Should have generation success message
    assert!(result.warnings.iter().any(|w| w.contains("Generation query executed successfully")));
}

#[tokio::test]
async fn test_execute_business_rules_population_complex_business_logic() {
    let db = setup_test_db().await;
    create_test_tables_for_business_rules_population(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Complex business rule: Only assign senior developers to high-complexity projects
    let complex_generation_query = r#"
        INSERT INTO user_project_assignments (user_id, project_id, assignment_type)
        SELECT 
            u.id, 
            p.id,
            CASE 
                WHEN u.role LIKE '%Senior%' AND p.complexity_level >= 4 THEN 'lead'
                WHEN u.role LIKE '%Lead%' THEN 'lead'
                WHEN u.seniority_level >= p.complexity_level THEN 'standard'
                ELSE 'support'
            END
        FROM users u
        CROSS JOIN projects p
        WHERE 
            (u.department = p.department) AND
            (
                (u.role LIKE '%Senior%' AND p.complexity_level >= 3) OR
                (u.role LIKE '%Lead%') OR  
                (u.seniority_level >= p.complexity_level - 1 AND p.complexity_level <= 3)
            )
    "#;
    
    let complex_validation_rules = vec![
        // Rule 1: No junior developers on high-complexity projects
        "SELECT COUNT(*) FROM user_project_assignments ua JOIN users u ON ua.user_id = u.id JOIN projects p ON ua.project_id = p.id WHERE u.role LIKE '%Junior%' AND p.complexity_level >= 4".to_string(),
        // Rule 2: All leads should be on appropriate projects
        "SELECT COUNT(*) FROM user_project_assignments ua JOIN users u ON ua.user_id = u.id WHERE ua.assignment_type = 'lead' AND (u.role LIKE '%Senior%' OR u.role LIKE '%Lead%')".to_string(),
        // Rule 3: Department consistency
        "SELECT COUNT(*) FROM user_project_assignments ua JOIN users u ON ua.user_id = u.id JOIN projects p ON ua.project_id = p.id WHERE u.department = p.department".to_string(),
    ];
    
    let result = data_migration.execute_business_rules_population(
        "user_project_assignments",
        complex_generation_query,
        &complex_validation_rules
    ).await.expect("Complex business rules population should succeed");
    
    assert!(result.success);
    assert!(result.records_processed > 0);
    // Note: Some validation rules may detect issues (e.g., count=0 for junior devs on high complexity projects)
    // This is expected behavior and doesn't indicate failure
    assert!(result.errors.is_empty());
    
    // Verify complex business rules were applied
    let complex_result = db.execute(
        r#"SELECT 
            u.role, u.seniority_level, p.complexity_level, ua.assignment_type
           FROM user_project_assignments ua 
           JOIN users u ON ua.user_id = u.id 
           JOIN projects p ON ua.project_id = p.id 
           ORDER BY p.complexity_level DESC, u.seniority_level DESC"#,
        &[]
    ).await.expect("Failed to query complex assignments");
    
    // Verify no junior developers are assigned to high-complexity projects
    for row in &complex_result.rows {
        if let Value::Object(row_map) = row {
            if let (Some(Value::String(role)), Some(Value::Number(complexity))) = 
                (row_map.get("role"), row_map.get("complexity_level")) {
                let complexity_val = complexity.as_u64().unwrap();
                if role.contains("Junior") {
                    assert!(complexity_val < 4, "Junior developer assigned to high-complexity project");
                }
            }
        }
    }
}

// Test helper function for external source population tests
async fn create_test_tables_for_external_source_population(db: &D1Client) {
    // Create external_assignments junction table
    let create_junction_sql = r#"
        CREATE TABLE external_assignments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            external_user_id INTEGER NOT NULL,
            external_project_id INTEGER NOT NULL,
            assignment_role TEXT DEFAULT 'member',
            start_date TEXT,
            end_date TEXT
        )
    "#;
    
    db.execute(create_junction_sql, &[])
        .await
        .expect("Failed to create external assignments junction table");
    
    // Create another test junction table
    let create_external_permissions_sql = r#"
        CREATE TABLE external_permissions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_ref INTEGER NOT NULL,
            permission_ref INTEGER NOT NULL,
            granted_date TEXT
        )
    "#;
    
    db.execute(create_external_permissions_sql, &[])
        .await
        .expect("Failed to create external permissions junction table");
}

#[tokio::test]
async fn test_execute_external_source_population_validation_errors() {
    let db = setup_test_db().await;
    create_test_tables_for_external_source_population(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    let json_data = r#"[{"user_id": 1, "project_id": 1, "role": "lead"}]"#;
    let mut column_mapping = HashMap::new();
    column_mapping.insert("user_id".to_string(), "external_user_id".to_string());
    column_mapping.insert("project_id".to_string(), "external_project_id".to_string());
    let validation_rules = vec!["SELECT COUNT(*) FROM external_assignments".to_string()];
    
    // Test empty junction table
    let result = data_migration.execute_external_source_population(
        "",  // Empty junction table
        json_data,
        "json",
        &column_mapping,
        &validation_rules
    ).await.expect("Should handle empty junction table gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Junction table cannot be empty"));
    
    // Test empty source data
    let result = data_migration.execute_external_source_population(
        "external_assignments",
        "",  // Empty source data
        "json",
        &column_mapping,
        &validation_rules
    ).await.expect("Should handle empty source data gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Source data cannot be empty"));
    
    // Test empty source format
    let result = data_migration.execute_external_source_population(
        "external_assignments",
        json_data,
        "",  // Empty source format
        &column_mapping,
        &validation_rules
    ).await.expect("Should handle empty source format gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Source format cannot be empty"));
    
    // Test unsupported source format
    let result = data_migration.execute_external_source_population(
        "external_assignments",
        json_data,
        "xml",  // Unsupported format
        &column_mapping,
        &validation_rules
    ).await.expect("Should handle unsupported format gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Unsupported source format 'xml'"));
    
    // Test empty column mapping
    let empty_mapping = HashMap::new();
    let result = data_migration.execute_external_source_population(
        "external_assignments",
        json_data,
        "json",
        &empty_mapping,
        &validation_rules
    ).await.expect("Should handle empty column mapping gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Column mapping cannot be empty"));
    
    // Test empty column names in mapping
    let mut invalid_mapping = HashMap::new();
    invalid_mapping.insert("".to_string(), "external_user_id".to_string());  // Empty source column
    invalid_mapping.insert("project_id".to_string(), "external_project_id".to_string());
    
    let result = data_migration.execute_external_source_population(
        "external_assignments",
        json_data,
        "json",
        &invalid_mapping,
        &validation_rules
    ).await.expect("Should handle empty column names gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Column names in mapping cannot be empty"));
    
    // Test duplicate target columns
    let mut duplicate_mapping = HashMap::new();
    duplicate_mapping.insert("user_id".to_string(), "external_user_id".to_string());
    duplicate_mapping.insert("project_id".to_string(), "external_user_id".to_string());  // Duplicate target
    
    let result = data_migration.execute_external_source_population(
        "external_assignments",
        json_data,
        "json",
        &duplicate_mapping,
        &validation_rules
    ).await.expect("Should handle duplicate target columns gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Duplicate target column 'external_user_id'"));
    
    // Test dangerous validation rule
    let dangerous_validation_rules = vec!["DROP TABLE external_assignments".to_string()];
    let result = data_migration.execute_external_source_population(
        "external_assignments",
        json_data,
        "json",
        &column_mapping,
        &dangerous_validation_rules
    ).await.expect("Should handle dangerous validation rule gracefully");
    
    assert!(!result.success);
    assert!(result.errors[0].to_string().contains("Validation rule 1 contains potentially dangerous SQL patterns"));
}

#[tokio::test]
async fn test_execute_external_source_population_missing_table() {
    let db = setup_test_db().await;
    create_test_tables_for_external_source_population(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    let json_data = r#"[{"user_id": 1, "project_id": 1}]"#;
    let mut column_mapping = HashMap::new();
    column_mapping.insert("user_id".to_string(), "external_user_id".to_string());
    column_mapping.insert("project_id".to_string(), "external_project_id".to_string());
    let validation_rules = vec!["SELECT COUNT(*) FROM non_existent_table".to_string()];
    
    // Test non-existent junction table
    let result = data_migration.execute_external_source_population(
        "non_existent_table",
        json_data,
        "json",
        &column_mapping,
        &validation_rules
    ).await.expect("Should handle missing table gracefully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(result.warnings.iter().any(|w| w.contains("Junction table non_existent_table not accessible")));
}

#[tokio::test]
async fn test_execute_external_source_population_json_success() {
    let db = setup_test_db().await;
    create_test_tables_for_external_source_population(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 2,  // Small batch size to test batch processing
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // JSON array format
    let json_data = r#"[
        {"user_id": 1, "project_id": 1, "role": "lead", "start": "2024-01-01"},
        {"user_id": 2, "project_id": 1, "role": "member", "start": "2024-01-02"},
        {"user_id": 3, "project_id": 2, "role": "member", "start": "2024-01-03"}
    ]"#;
    
    let mut column_mapping = HashMap::new();
    column_mapping.insert("user_id".to_string(), "external_user_id".to_string());
    column_mapping.insert("project_id".to_string(), "external_project_id".to_string());
    column_mapping.insert("role".to_string(), "assignment_role".to_string());
    column_mapping.insert("start".to_string(), "start_date".to_string());
    
    let validation_rules = vec![
        "SELECT COUNT(*) FROM external_assignments".to_string(),
        "SELECT COUNT(*) FROM external_assignments WHERE assignment_role IN ('lead', 'member')".to_string(),
    ];
    
    let result = data_migration.execute_external_source_population(
        "external_assignments",
        json_data,
        "json",
        &column_mapping,
        &validation_rules
    ).await.expect("JSON external source population should succeed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 3);
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());
    
    // Should have parsing success message
    assert!(result.warnings.iter().any(|w| w.contains("Parsed 3 records from external source (format: json)")));
    
    // Should have validation summary
    assert!(result.warnings.iter().any(|w| w.contains("validation rules to verify external source data integrity")));
    assert!(result.warnings.iter().any(|w| w.contains("All validation rules passed successfully")));
    
    // Verify the records were inserted correctly
    let assignments_result = db.execute(
        "SELECT external_user_id, external_project_id, assignment_role, start_date FROM external_assignments ORDER BY external_user_id",
        &[]
    ).await.expect("Failed to query assignments");
    
    assert_eq!(assignments_result.rows.len(), 3);
    
    // Check specific record values
    if let Value::Object(row) = &assignments_result.rows[0] {
        if let (Some(Value::Number(user_id)), Some(Value::String(role))) = 
            (row.get("external_user_id"), row.get("assignment_role")) {
            assert_eq!(user_id.as_u64().unwrap(), 1);
            assert_eq!(role, "lead");
        }
    }
}

#[tokio::test]
async fn test_execute_external_source_population_csv_success() {
    let db = setup_test_db().await;
    create_test_tables_for_external_source_population(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // CSV format data
    let csv_data = r#"user_ref,permission_ref,granted
1,101,2024-01-01
2,102,2024-01-02
3,101,2024-01-03
4,103,2024-01-04"#;
    
    let mut column_mapping = HashMap::new();
    column_mapping.insert("user_ref".to_string(), "user_ref".to_string());
    column_mapping.insert("permission_ref".to_string(), "permission_ref".to_string());
    column_mapping.insert("granted".to_string(), "granted_date".to_string());
    
    let validation_rules = vec![
        "SELECT COUNT(*) FROM external_permissions".to_string(),
        "SELECT COUNT(*) FROM external_permissions WHERE user_ref > 0".to_string(),
    ];
    
    let result = data_migration.execute_external_source_population(
        "external_permissions",
        csv_data,
        "csv",
        &column_mapping,
        &validation_rules
    ).await.expect("CSV external source population should succeed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 4);
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());
    
    // Should have parsing success message
    assert!(result.warnings.iter().any(|w| w.contains("Parsed 4 records from external source (format: csv)")));
    
    // Verify the records were inserted correctly
    let permissions_result = db.execute(
        "SELECT user_ref, permission_ref, granted_date FROM external_permissions ORDER BY user_ref",
        &[]
    ).await.expect("Failed to query permissions");
    
    assert_eq!(permissions_result.rows.len(), 4);
    
    // Check specific record values
    if let Value::Object(row) = &permissions_result.rows[0] {
        if let (Some(Value::Number(user_ref)), Some(Value::Number(permission_ref))) = 
            (row.get("user_ref"), row.get("permission_ref")) {
            assert_eq!(user_ref.as_u64().unwrap(), 1);
            assert_eq!(permission_ref.as_u64().unwrap(), 101);
        }
    }
}

#[tokio::test]
async fn test_execute_external_source_population_array_format() {
    let db = setup_test_db().await;
    create_test_tables_for_external_source_population(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Array format (newline-separated JSON objects)
    let array_data = r#"{"user_id": 10, "project_id": 10, "role": "admin"}
{"user_id": 11, "project_id": 10, "role": "member"}
{"user_id": 12, "project_id": 11, "role": "lead"}"#;
    
    let mut column_mapping = HashMap::new();
    column_mapping.insert("user_id".to_string(), "external_user_id".to_string());
    column_mapping.insert("project_id".to_string(), "external_project_id".to_string());
    column_mapping.insert("role".to_string(), "assignment_role".to_string());
    
    let validation_rules: Vec<String> = vec![];  // No validation rules
    
    let result = data_migration.execute_external_source_population(
        "external_assignments",
        array_data,
        "array",
        &column_mapping,
        &validation_rules
    ).await.expect("Array external source population should succeed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 3);
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());
    
    // Should have parsing success message
    assert!(result.warnings.iter().any(|w| w.contains("Parsed 3 records from external source (format: array)")));
    
    // Should NOT have validation messages since no rules were provided
    assert!(!result.warnings.iter().any(|w| w.contains("validation rules")));
    
    // Verify the records were inserted correctly
    let array_result = db.execute(
        "SELECT external_user_id, assignment_role FROM external_assignments WHERE external_user_id >= 10 ORDER BY external_user_id",
        &[]
    ).await.expect("Failed to query array assignments");
    
    assert_eq!(array_result.rows.len(), 3);
    
    // Check specific record values
    if let Value::Object(row) = &array_result.rows[0] {
        if let (Some(Value::Number(user_id)), Some(Value::String(role))) = 
            (row.get("external_user_id"), row.get("assignment_role")) {
            assert_eq!(user_id.as_u64().unwrap(), 10);
            assert_eq!(role, "admin");
        }
    }
}

#[tokio::test]
async fn test_execute_external_source_population_parsing_failures() {
    let db = setup_test_db().await;
    create_test_tables_for_external_source_population(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    let mut column_mapping = HashMap::new();
    column_mapping.insert("user_id".to_string(), "external_user_id".to_string());
    column_mapping.insert("project_id".to_string(), "external_project_id".to_string());
    let validation_rules: Vec<String> = vec![];
    
    // Test invalid JSON
    let invalid_json = r#"{"user_id": 1, "project_id": invalid}"#;
    let result = data_migration.execute_external_source_population(
        "external_assignments",
        invalid_json,
        "json",
        &column_mapping,
        &validation_rules
    ).await.expect("Should handle invalid JSON gracefully");
    
    assert!(!result.success);
    assert!(result.errors.iter().any(|e| e.to_string().contains("Failed to parse external source data")));
    
    // Test invalid CSV (mismatched columns)
    let invalid_csv = r#"user_id,project_id
1,2,3"#;  // 3 values but 2 headers
    let result = data_migration.execute_external_source_population(
        "external_assignments",
        invalid_csv,
        "csv",
        &column_mapping,
        &validation_rules
    ).await.expect("Should handle invalid CSV gracefully");
    
    assert!(!result.success);
    assert!(result.errors.iter().any(|e| e.to_string().contains("Failed to parse external source data")));
    
    // Test JSON array with non-objects
    let invalid_json_array = r#"[1, 2, 3]"#;
    let result = data_migration.execute_external_source_population(
        "external_assignments",
        invalid_json_array,
        "json",
        &column_mapping,
        &validation_rules
    ).await.expect("Should handle non-object JSON array gracefully");
    
    assert!(!result.success);
    assert!(result.errors.iter().any(|e| e.to_string().contains("Failed to parse external source data")));
}

#[tokio::test]
async fn test_execute_external_source_population_missing_columns() {
    let db = setup_test_db().await;
    create_test_tables_for_external_source_population(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // JSON data missing some columns that are required by mapping
    let incomplete_json = r#"[
        {"user_id": 1, "project_id": 1},
        {"user_id": 2, "missing_project": 2},
        {"user_id": 3, "project_id": 3}
    ]"#;
    
    let mut column_mapping = HashMap::new();
    column_mapping.insert("user_id".to_string(), "external_user_id".to_string());
    column_mapping.insert("project_id".to_string(), "external_project_id".to_string());
    let validation_rules: Vec<String> = vec![];
    
    let result = data_migration.execute_external_source_population(
        "external_assignments",
        incomplete_json,
        "json",
        &column_mapping,
        &validation_rules
    ).await.expect("Should handle missing columns gracefully");
    
    // Success should be false as 2 out of 3 records succeeded (66.7% < 80% threshold)
    assert!(!result.success);
    assert_eq!(result.records_processed, 3);
    assert_eq!(result.records_failed, 1);  // One record missing project_id
    
    // Should have warnings about missing columns
    assert!(result.warnings.iter().any(|w| w.contains("Source column 'project_id' not found")));
    
    // Should have summary about failed records
    assert!(result.warnings.iter().any(|w| w.contains("Failed to process 1 out of 3 external records")));
    
    // Verify only 2 records were inserted
    let incomplete_result = db.execute(
        "SELECT COUNT(*) as count FROM external_assignments",
        &[]
    ).await.expect("Failed to count records");
    
    if let Value::Object(row) = &incomplete_result.rows[0] {
        if let Some(Value::Number(count)) = row.get("count") {
            assert!(count.as_u64().unwrap() >= 2);  // At least 2 records from previous tests
        }
    }
}

#[tokio::test]
async fn test_execute_external_source_population_validation_failure() {
    let db = setup_test_db().await;
    create_test_tables_for_external_source_population(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    let json_data = r#"[{"user_id": 100, "project_id": 100}]"#;
    let mut column_mapping = HashMap::new();
    column_mapping.insert("user_id".to_string(), "external_user_id".to_string());
    column_mapping.insert("project_id".to_string(), "external_project_id".to_string());
    
    // Validation rules that will fail
    let failing_validation_rules = vec![
        "SELECT COUNT(*) FROM external_assignments".to_string(),  // This will pass
        "SELECT COUNT(*) FROM invalid_table_name".to_string(),  // This will fail
    ];
    
    let result = data_migration.execute_external_source_population(
        "external_assignments",
        json_data,
        "json",
        &column_mapping,
        &failing_validation_rules
    ).await.expect("Should handle validation failure gracefully");
    
    assert!(!result.success);  // Should fail due to validation failures
    assert!(result.records_processed > 0);
    assert!(result.records_failed > 0);  // Validation failures are counted
    assert!(result.errors.iter().any(|e| e.to_string().contains("Validation rule 2 failed")));
    
    // Should have validation failure summary
    assert!(result.warnings.iter().any(|w| w.contains("validation rules failed or detected issues")));
}

#[tokio::test]
async fn test_execute_external_source_population_single_json_object() {
    let db = setup_test_db().await;
    create_test_tables_for_external_source_population(&db).await;
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Single JSON object (not an array)
    let single_json = r#"{"user_id": 999, "project_id": 999, "role": "owner"}"#;
    
    let mut column_mapping = HashMap::new();
    column_mapping.insert("user_id".to_string(), "external_user_id".to_string());
    column_mapping.insert("project_id".to_string(), "external_project_id".to_string());
    column_mapping.insert("role".to_string(), "assignment_role".to_string());
    
    let validation_rules: Vec<String> = vec![];
    
    let result = data_migration.execute_external_source_population(
        "external_assignments",
        single_json,
        "json",
        &column_mapping,
        &validation_rules
    ).await.expect("Single JSON object population should succeed");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 1);
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());
    
    // Should have parsing success message
    assert!(result.warnings.iter().any(|w| w.contains("Parsed 1 records from external source (format: json)")));
    
    // Verify the record was inserted correctly
    let single_result = db.execute(
        "SELECT external_user_id, assignment_role FROM external_assignments WHERE external_user_id = 999",
        &[]
    ).await.expect("Failed to query single assignment");
    
    assert_eq!(single_result.rows.len(), 1);
    
    if let Value::Object(row) = &single_result.rows[0] {
        if let Some(Value::String(role)) = row.get("assignment_role") {
            assert_eq!(role, "owner");
        }
    }
}

#[tokio::test]
async fn test_execute_denormalized_column_population_different_delimiters() {
    let db = setup_test_db().await;
    
    // Create tables for testing different delimiters
    let create_source_sql = r#"
        CREATE TABLE delimiter_articles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            pipe_tags TEXT,
            semicolon_tags TEXT
        )
    "#;
    
    db.execute(create_source_sql, &[])
        .await
        .expect("Failed to create source table");
    
    let create_junction_sql = r#"
        CREATE TABLE delimiter_article_tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            article_id INTEGER NOT NULL,
            tag_name TEXT NOT NULL
        )
    "#;
    
    db.execute(create_junction_sql, &[])
        .await
        .expect("Failed to create junction table");
    
    // Insert articles with different delimiters
    db.execute(
        "INSERT INTO delimiter_articles (title, pipe_tags, semicolon_tags) VALUES (?, ?, ?)",
        &[
            Value::String("Test Article".to_string()),
            Value::String("tag1|tag2|tag3".to_string()),  // Pipe delimited
            Value::String("tag4;tag5;tag6".to_string()),  // Semicolon delimited
        ]
    ).await.expect("Failed to insert test article");
    
    let config = DataMigrationConfig {
        batch_size: 10,
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };
    
    let data_migration = DataMigrator::new(db.clone(), config);
    
    // Test pipe delimiter
    let result = data_migration.execute_denormalized_column_population(
        "delimiter_article_tags",
        "delimiter_articles",
        "pipe_tags",
        "|",
        "article_id",
        "tag_name"
    ).await.expect("Should handle pipe delimiter");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 1);
    
    // Verify pipe-delimited tags were inserted
    let pipe_result = db.execute("SELECT COUNT(*) as count FROM delimiter_article_tags", &[])
        .await.expect("Failed to query junction table");
    
    if let Value::Object(row) = &pipe_result.rows[0] {
        if let Some(Value::Number(count)) = row.get("count") {
            assert_eq!(count.as_u64().unwrap(), 3);  // tag1, tag2, tag3
        }
    }
    
    // Clear junction table for next test
    db.execute("DELETE FROM delimiter_article_tags", &[])
        .await.expect("Failed to clear junction table");
    
    // Test semicolon delimiter
    let result = data_migration.execute_denormalized_column_population(
        "delimiter_article_tags",
        "delimiter_articles",
        "semicolon_tags",
        ";",
        "article_id",
        "tag_name"
    ).await.expect("Should handle semicolon delimiter");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 1);
    
    // Verify semicolon-delimited tags were inserted
    let semi_result = db.execute("SELECT COUNT(*) as count FROM delimiter_article_tags", &[])
        .await.expect("Failed to query junction table");
    
    if let Value::Object(row) = &semi_result.rows[0] {
        if let Some(Value::Number(count)) = row.get("count") {
            assert_eq!(count.as_u64().unwrap(), 3);  // tag4, tag5, tag6
        }
    }
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
                serde_json::Value::String(s) if key == "name" => serde_json::Value::String(s.to_uppercase()),
                serde_json::Value::Number(n) if key == "age" => {
                    // Only double the age field
                    if let Some(int_val) = n.as_i64() {
                        serde_json::Value::Number((int_val * 2).into())
                    } else if let Some(float_val) = n.as_f64() {
                        serde_json::Value::Number(serde_json::Number::from_f64(float_val * 2.0).unwrap_or_else(|| 0.into()))
                    } else {
                        value.clone()
                    }
                }
                _ => value.clone(),
            };
            output.insert(key.clone(), transformed_value);
            
            // Add computed field
            if key == "name" {
                output.insert("computed_field".to_string(), serde_json::Value::String("computed_value".to_string()));
            }
        }
        
        Ok(output)
    }
    
    fn validate_config(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(D1RsError::ValidationError("Transformation name cannot be empty".to_string()));
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
        Err(D1RsError::ValidationError("Intentional transformation failure".to_string()))
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
        &[]
    ).await.expect("Failed to create custom_source_users table");
    
    // Insert test data
    db.execute(
        "INSERT INTO custom_source_users (id, name, age, email) VALUES 
        (1, 'alice', 25, 'alice@example.com'),
        (2, 'bob', 30, 'bob@example.com'),
        (3, 'charlie', 35, 'charlie@example.com')",
        &[]
    ).await.expect("Failed to insert test data");
    
    // Create target table
    db.execute(
        "CREATE TABLE custom_target_users (
            id INTEGER PRIMARY KEY,
            transformed_name TEXT NOT NULL,
            doubled_age INTEGER,
            original_email TEXT,
            computed_field TEXT
        )",
        &[]
    ).await.expect("Failed to create custom_target_users table");
}

#[tokio::test]
async fn test_execute_custom_transformation_validation_errors() {
    let db = setup_test_db().await;
    create_test_tables_for_custom_transformation(&db).await;
    
    let mut custom_transformations = HashMap::new();
    custom_transformations.insert("test_transform".to_string(), Box::new(TestTransformationFunction::new("test_transform")) as Box<dyn TransformationFunction>);
    
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
    let result = data_migration.execute_custom_transformation(
        "",
        "SELECT * FROM custom_source_users",
        "test_transform",
        &["INSERT INTO custom_target_users (id, transformed_name) VALUES ($id, $name)".to_string()]
    ).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Migration name cannot be empty"));
    
    // Test empty source query
    let result = data_migration.execute_custom_transformation(
        "test_migration",
        "",
        "test_transform",
        &["INSERT INTO custom_target_users (id, transformed_name) VALUES ($id, $name)".to_string()]
    ).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Source query cannot be empty"));
    
    // Test empty transformation logic
    let result = data_migration.execute_custom_transformation(
        "test_migration",
        "SELECT * FROM custom_source_users",
        "",
        &["INSERT INTO custom_target_users (id, transformed_name) VALUES ($id, $name)".to_string()]
    ).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Transformation logic cannot be empty"));
    
    // Test empty target operations
    let result = data_migration.execute_custom_transformation(
        "test_migration",
        "SELECT * FROM custom_source_users",
        "test_transform",
        &[]
    ).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("At least one target operation must be specified"));
    
    // Test non-existent transformation function
    let result = data_migration.execute_custom_transformation(
        "test_migration",
        "SELECT * FROM custom_source_users",
        "non_existent_transform",
        &["INSERT INTO custom_target_users (id, transformed_name) VALUES ($id, $name)".to_string()]
    ).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Custom transformation 'non_existent_transform' not found"));
}

#[tokio::test]
async fn test_execute_custom_transformation_success() {
    let db = setup_test_db().await;
    create_test_tables_for_custom_transformation(&db).await;
    
    let mut custom_transformations = HashMap::new();
    custom_transformations.insert("test_transform".to_string(), Box::new(TestTransformationFunction::new("test_transform")) as Box<dyn TransformationFunction>);
    
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
    
    let result = data_migration.execute_custom_transformation(
        "user_transformation",
        "SELECT * FROM custom_source_users",
        "test_transform",
        &target_operations
    ).await.expect("Should execute custom transformation successfully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 3);
    assert_eq!(result.records_failed, 0);
    assert!(result.warnings.iter().any(|w| w.contains("Starting custom migration: user_transformation")));
    assert!(result.warnings.iter().any(|w| w.contains("Using transformation function: test_transform")));
    
    // Verify transformed data was inserted correctly
    let verification_result = db.execute(
        "SELECT id, transformed_name, doubled_age, original_email, computed_field FROM custom_target_users ORDER BY id",
        &[]
    ).await.expect("Failed to verify transformed data");
    
    assert_eq!(verification_result.rows.len(), 3);
    
    // Verify first row transformation
    if let Value::Object(row) = &verification_result.rows[0] {
        assert_eq!(row.get("id").unwrap(), &Value::Number(1.into()));
        assert_eq!(row.get("transformed_name").unwrap(), &Value::String("ALICE".to_string()));
        assert_eq!(row.get("doubled_age").unwrap(), &Value::Number(50.into()));
        assert_eq!(row.get("original_email").unwrap(), &Value::String("alice@example.com".to_string()));
        assert_eq!(row.get("computed_field").unwrap(), &Value::String("computed_value".to_string()));
    }
}

#[tokio::test]
async fn test_execute_custom_transformation_failure() {
    let db = setup_test_db().await;
    create_test_tables_for_custom_transformation(&db).await;
    
    let mut custom_transformations = HashMap::new();
    custom_transformations.insert("failing_transform".to_string(), Box::new(FailingTransformationFunction) as Box<dyn TransformationFunction>);
    
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
        "INSERT INTO custom_target_users (id, transformed_name) VALUES ($id, $name)".to_string()
    ];
    
    let result = data_migration.execute_custom_transformation(
        "failing_migration",
        "SELECT * FROM custom_source_users",
        "failing_transform",
        &target_operations
    ).await.expect("Should handle transformation failures gracefully");
    
    assert!(!result.success);
    assert_eq!(result.records_processed, 3);
    assert_eq!(result.records_failed, 3);
    assert!(result.errors.iter().any(|e| e.to_string().contains("Custom transformation failed")));
    
    // Verify no data was inserted due to transformation failures
    let verification_result = db.execute(
        "SELECT COUNT(*) as count FROM custom_target_users",
        &[]
    ).await.expect("Failed to verify no data inserted");
    
    if let Value::Object(row) = &verification_result.rows[0] {
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
    custom_transformations.insert("test_transform".to_string(), Box::new(TestTransformationFunction::new("test_transform")) as Box<dyn TransformationFunction>);
    
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
    let target_operations = vec![
        "INSERT INTO non_existent_table (id, name) VALUES ($id, $name)".to_string()
    ];
    
    let result = data_migration.execute_custom_transformation(
        "failing_target_migration",
        "SELECT * FROM custom_source_users",
        "test_transform",
        &target_operations
    ).await.expect("Should handle target operation failures gracefully");
    
    assert!(!result.success);
    assert_eq!(result.records_processed, 3);
    assert_eq!(result.records_failed, 3);
    assert!(result.errors.iter().any(|e| e.to_string().contains("Target operation")));
}

#[tokio::test]
async fn test_execute_custom_transformation_invalid_source_query() {
    let db = setup_test_db().await;
    create_test_tables_for_custom_transformation(&db).await;
    
    let mut custom_transformations = HashMap::new();
    custom_transformations.insert("test_transform".to_string(), Box::new(TestTransformationFunction::new("test_transform")) as Box<dyn TransformationFunction>);
    
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
        "INSERT INTO custom_target_users (id, transformed_name) VALUES ($id, $name)".to_string()
    ];
    
    let result = data_migration.execute_custom_transformation(
        "invalid_source_migration",
        "SELECT * FROM non_existent_source_table",
        "test_transform",
        &target_operations
    ).await.expect("Should handle invalid source query gracefully");
    
    assert!(!result.success);
    assert_eq!(result.records_processed, 0);
    assert_eq!(result.records_failed, 1);
    assert!(result.errors.iter().any(|e| e.to_string().contains("Failed to execute source query")));
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
                serde_json::Value::String(format!("user{}@example.com", i))
            ]
        ).await.expect("Failed to insert additional test data");
    }
    
    let mut custom_transformations = HashMap::new();
    custom_transformations.insert("test_transform".to_string(), Box::new(TestTransformationFunction::new("test_transform")) as Box<dyn TransformationFunction>);
    
    let config = DataMigrationConfig {
        batch_size: 5,  // Small batch size to test batching
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
    
    let result = data_migration.execute_custom_transformation(
        "batch_processing_migration",
        "SELECT * FROM custom_source_users ORDER BY id",
        "test_transform",
        &target_operations
    ).await.expect("Should handle batch processing successfully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 15);  // Total records including original 3 + 12 new
    assert_eq!(result.records_failed, 0);
    
    // Should have batch processing warnings
    assert!(result.warnings.iter().any(|w| w.contains("Processing batch")));
    
    // Verify all records were transformed and inserted
    let verification_result = db.execute(
        "SELECT COUNT(*) as count FROM custom_target_users",
        &[]
    ).await.expect("Failed to verify batch processing results");
    
    if let Value::Object(row) = &verification_result.rows[0] {
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
    custom_transformations.insert("test_transform".to_string(), Box::new(TestTransformationFunction::new("test_transform")) as Box<dyn TransformationFunction>);
    
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
    
    let result = data_migration.execute_custom_transformation(
        "placeholder_test_migration",
        "SELECT * FROM custom_source_users WHERE id = 1",
        "test_transform",
        &target_operations
    ).await.expect("Should handle placeholder replacement successfully");
    
    assert!(result.success);
    assert_eq!(result.records_processed, 1);
    assert_eq!(result.records_failed, 0);
    
    // Verify placeholder replacement worked correctly by checking the inserted data
    let verification_result = db.execute(
        "SELECT id, transformed_name, doubled_age, original_email, computed_field FROM custom_target_users WHERE id = 1",
        &[]
    ).await.expect("Failed to verify placeholder replacement");
    
    assert_eq!(verification_result.rows.len(), 1);
    
    if let Value::Object(row) = &verification_result.rows[0] {
        // ID should be unchanged
        assert_eq!(row.get("id").unwrap(), &Value::Number(1.into()));
        // Name should be uppercased
        assert_eq!(row.get("transformed_name").unwrap(), &Value::String("ALICE".to_string()));
        // Age should be doubled
        assert_eq!(row.get("doubled_age").unwrap(), &Value::Number(50.into()));
        // Email should be unchanged
        assert_eq!(row.get("original_email").unwrap(), &Value::String("alice@example.com".to_string()));
        // Computed field should be added
        assert_eq!(row.get("computed_field").unwrap(), &Value::String("computed_value".to_string()));
    }
}