use d1_rs::*;
use d1_rs::auto_migration::{DataMigrator, DataMigrationConfig, FailureStrategy};
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