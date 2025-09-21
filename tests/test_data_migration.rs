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