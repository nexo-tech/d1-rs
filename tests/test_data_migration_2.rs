use d1_rs::auto_migration::{
    DataMigrationConfig, DataMigrator, FailureStrategy, MigrationContext, RollbackInfo,
    TransformationFunction,
};
use d1_rs::*;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

async fn setup_test_db() -> D1Client {
    D1Client::new_in_memory()
        .await
        .expect("Failed to create in-memory database")
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
        (
            "alice",
            "12/25/2023",
            "2:30 PM",
            "1,234.56",
            "(555) 123-4567",
            "$1,234.56",
            "hello world",
        ),
        (
            "bob",
            "01/15/2024",
            "9:45 AM",
            "5,678.90",
            "(555) 987-6543",
            "$5,678.90",
            "JOHN DOE",
        ),
        (
            "charlie",
            "03/08/2023",
            "11:30 PM",
            "999.99",
            "(555) 111-2222",
            "$999.99",
            "tEsT CaSe",
        ),
        (
            "diana",
            "07/04/2024",
            "12:00 AM",
            "12,345.67",
            "(555) 444-5555",
            "$12,345.67",
            "api endpoint",
        ),
        (
            "eve",
            "09/30/2023",
            "6:15 PM",
            "87.50",
            "(555) 777-8888",
            "$87.50",
            "database connection",
        ),
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
    let result = data_migration
        .execute_format_transformation(
            "test_formats",
            "", // Empty old column
            "new_field",
            "source_format",
            "target_format",
            "DATE_FORMAT",
        )
        .await
        .expect("Should handle empty column gracefully");

    assert!(!result.success);
    assert!(!result.errors.is_empty());

    // Test unsupported format function
    let result = data_migration
        .execute_format_transformation(
            "test_formats",
            "old_date",
            "new_date",
            "MM/DD/YYYY",
            "YYYY-MM-DD",
            "INVALID_FUNCTION",
        )
        .await
        .expect("Should handle unsupported function gracefully");

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
    let result = data_migration
        .execute_format_transformation(
            "non_existent_table",
            "old_field",
            "new_field",
            "MM/DD/YYYY",
            "YYYY-MM-DD",
            "DATE_FORMAT",
        )
        .await
        .expect("Should handle missing table gracefully");

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

    let result = data_migration
        .execute_format_transformation(
            "test_formats",
            "mixed_case_text",
            "formatted_text",
            "",
            "",
            "LOWER",
        )
        .await
        .expect("Batch processing failed");

    assert!(result.success);
    assert_eq!(result.records_processed, 5);
    assert_eq!(result.records_failed, 0);

    // Verify all records were processed correctly despite batching
    let rows = db
        .execute(
            "SELECT COUNT(*) as count FROM test_formats WHERE formatted_text IS NOT NULL",
            &[],
        )
        .await
        .expect("Failed to count results");

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
            &[Value::String(text.to_string())],
        )
        .await
        .expect("Failed to insert test data");
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
    let result = data_migration
        .execute_format_transformation("test_trim", "padded_text", "trimmed_text", "", "", "TRIM")
        .await
        .expect("TRIM transformation failed");

    assert!(result.success);
    assert_eq!(result.records_processed, 5);
    assert_eq!(result.records_failed, 0);

    // Verify trimming worked
    let rows = db
        .execute(
            "SELECT padded_text, trimmed_text FROM test_trim ORDER BY id",
            &[],
        )
        .await
        .expect("Failed to query results");

    if let Value::Object(row) = &rows.rows[0] {
        assert_eq!(
            row.get("padded_text"),
            Some(&Value::String("  hello world  ".to_string()))
        );
        assert_eq!(
            row.get("trimmed_text"),
            Some(&Value::String("hello world".to_string()))
        );
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
    let result = data_migration
        .execute_direct_fk_copy(
            "", // Empty table
            "old_product_id",
            "new_product_id",
        )
        .await
        .expect("Should handle empty table gracefully");

    assert!(!result.success);
    assert_eq!(result.records_processed, 0);
    assert!(!result.errors.is_empty());
    assert!(result.errors[0]
        .to_string()
        .contains("Source table cannot be empty"));

    // Test empty old column
    let result = data_migration
        .execute_direct_fk_copy(
            "test_orders",
            "", // Empty old column
            "new_product_id",
        )
        .await
        .expect("Should handle empty column gracefully");

    assert!(!result.success);
    assert_eq!(result.records_processed, 0);
    assert!(!result.errors.is_empty());
    assert!(result.errors[0]
        .to_string()
        .contains("Source and target FK columns cannot be empty"));

    // Test empty new column
    let result = data_migration
        .execute_direct_fk_copy(
            "test_orders",
            "old_product_id",
            "", // Empty new column
        )
        .await
        .expect("Should handle empty column gracefully");

    assert!(!result.success);
    assert_eq!(result.records_processed, 0);
    assert!(!result.errors.is_empty());
    assert!(result.errors[0]
        .to_string()
        .contains("Source and target FK columns cannot be empty"));
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
    let result = data_migration
        .execute_direct_fk_copy(
            "test_orders",
            "old_product_id",
            "old_product_id", // Same column
        )
        .await
        .expect("Should handle same column gracefully");

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
    let result = data_migration
        .execute_direct_fk_copy("non_existent_table", "old_product_id", "new_product_id")
        .await
        .expect("Should handle missing table gracefully");

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
    let result = data_migration
        .execute_direct_fk_copy("test_orders", "old_product_id", "new_product_id")
        .await
        .expect("FK copy should succeed");

    assert!(result.success);
    assert_eq!(result.records_processed, 5);
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());

    // Verify the copy worked - check that new_product_id matches old_product_id
    let rows = db
        .execute(
            "SELECT old_product_id, new_product_id FROM test_orders ORDER BY id",
            &[],
        )
        .await
        .expect("Failed to query results");

    assert_eq!(rows.rows.len(), 5);

    for (i, expected_product_id) in [201, 202, 203, 204, 205].iter().enumerate() {
        if let Value::Object(row) = &rows.rows[i] {
            assert_eq!(
                row.get("old_product_id"),
                Some(&Value::Number((*expected_product_id).into()))
            );
            assert_eq!(
                row.get("new_product_id"),
                Some(&Value::Number((*expected_product_id).into()))
            );
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
            ],
        )
        .await
        .expect("Failed to insert test data");
    }

    let config = DataMigrationConfig {
        batch_size: 3, // Small batch size to test batching
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    // Execute FK copy
    let result = data_migration
        .execute_direct_fk_copy("test_large_orders", "old_supplier_id", "new_supplier_id")
        .await
        .expect("FK copy should succeed");

    assert!(result.success);
    assert_eq!(result.records_processed, 25);
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());

    // Verify all records were processed correctly
    let count_result = db
        .execute(
            "SELECT COUNT(*) as count FROM test_large_orders WHERE new_supplier_id IS NOT NULL",
            &[],
        )
        .await
        .expect("Failed to count results");

    if let Value::Object(row) = &count_result.rows[0] {
        if let Some(Value::Number(count)) = row.get("count") {
            assert_eq!(count.as_u64().unwrap(), 25);
        }
    }

    // Verify values are correctly copied
    let sample_result = db
        .execute(
            "SELECT old_supplier_id, new_supplier_id FROM test_large_orders WHERE id = 1",
            &[],
        )
        .await
        .expect("Failed to query sample");

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
            ],
        )
        .await
        .expect("Failed to insert department data");
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
            &[Value::Number(old_id.into()), Value::Number(new_id.into())],
        )
        .await
        .expect("Failed to insert mapping data");
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
            ],
        )
        .await
        .expect("Failed to insert user data");
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
    let result = data_migration
        .execute_id_mapping_migration(
            "", // Empty source table
            "departments",
            "department_id_mapping",
            "old_department_id",
            "new_department_id",
        )
        .await
        .expect("Should handle empty source table gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Source table cannot be empty"));

    // Test empty target table
    let result = data_migration
        .execute_id_mapping_migration(
            "test_users",
            "", // Empty target table
            "department_id_mapping",
            "old_department_id",
            "new_department_id",
        )
        .await
        .expect("Should handle empty target table gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Target table cannot be empty"));

    // Test empty mapping table
    let result = data_migration
        .execute_id_mapping_migration(
            "test_users",
            "departments",
            "", // Empty mapping table
            "old_department_id",
            "new_department_id",
        )
        .await
        .expect("Should handle empty mapping table gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Mapping table cannot be empty"));

    // Test empty columns
    let result = data_migration
        .execute_id_mapping_migration(
            "test_users",
            "departments",
            "department_id_mapping",
            "", // Empty old column
            "new_department_id",
        )
        .await
        .expect("Should handle empty column gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Old and new ID columns cannot be empty"));
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
    let result = data_migration
        .execute_id_mapping_migration(
            "test_users",
            "departments",
            "department_id_mapping",
            "old_department_id",
            "old_department_id", // Same column
        )
        .await
        .expect("Should handle same column gracefully");

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
    let result = data_migration
        .execute_id_mapping_migration(
            "test_users",
            "departments",
            "non_existent_mapping",
            "old_department_id",
            "new_department_id",
        )
        .await
        .expect("Should handle missing mapping table gracefully");

    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(result.warnings[0].contains("not accessible"));

    // Test non-existent source table
    let result = data_migration
        .execute_id_mapping_migration(
            "non_existent_users",
            "departments",
            "department_id_mapping",
            "old_department_id",
            "new_department_id",
        )
        .await
        .expect("Should handle missing source table gracefully");

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

    let result = data_migration
        .execute_id_mapping_migration(
            "test_users",
            "departments",
            "department_id_mapping",
            "old_department_id",
            "new_department_id",
        )
        .await
        .expect("Should handle empty mapping table gracefully");

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
    let result = data_migration
        .execute_id_mapping_migration(
            "test_users",
            "departments",
            "department_id_mapping",
            "old_department_id",
            "new_department_id",
        )
        .await
        .expect("ID mapping migration should succeed");

    assert!(result.success);
    assert_eq!(result.records_processed, 5);
    assert_eq!(result.records_failed, 0);

    // Should have warning about unmapped ID (user Eve with old_department_id = 5)
    assert!(!result.warnings.is_empty());
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("could not be mapped")));

    // Verify the mapping worked for users with valid mappings
    let rows = db
        .execute(
            "SELECT name, old_department_id, new_department_id FROM test_users ORDER BY id",
            &[],
        )
        .await
        .expect("Failed to query results");

    assert_eq!(rows.rows.len(), 5);

    // Check mapped users (Alice, Bob, Charlie, Diana)
    let expected_mappings = vec![
        ("Alice", 1, Some(100)),
        ("Bob", 2, Some(200)),
        ("Charlie", 3, Some(300)),
        ("Diana", 4, Some(400)),
        ("Eve", 5, None), // No mapping for old_department_id = 5
    ];

    for (i, (expected_name, expected_old_id, expected_new_id)) in
        expected_mappings.iter().enumerate()
    {
        if let Value::Object(row) = &rows.rows[i] {
            assert_eq!(
                row.get("name"),
                Some(&Value::String(expected_name.to_string()))
            );
            assert_eq!(
                row.get("old_department_id"),
                Some(&Value::Number((*expected_old_id).into()))
            );

            if let Some(new_id) = expected_new_id {
                assert_eq!(
                    row.get("new_department_id"),
                    Some(&Value::Number((*new_id).into()))
                );
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
            ],
        )
        .await
        .expect("Failed to insert product data");
    }

    // Insert mapping data for categories 1-4 (category 5 will be unmapped)
    for i in 1..=4 {
        db.execute(
            "INSERT INTO category_id_mapping (old_id, new_id) VALUES (?, ?)",
            &[
                Value::Number(i.into()),
                Value::Number((i * 100).into()), // 1->100, 2->200, 3->300, 4->400
            ],
        )
        .await
        .expect("Failed to insert mapping data");
    }

    let config = DataMigrationConfig {
        batch_size: 3, // Small batch size to test batching
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    // Execute ID mapping migration
    let result = data_migration
        .execute_id_mapping_migration(
            "test_products",
            "categories",
            "category_id_mapping",
            "old_category_id",
            "new_category_id",
        )
        .await
        .expect("ID mapping migration should succeed");

    assert!(result.success);
    assert_eq!(result.records_processed, 15);
    assert_eq!(result.records_failed, 0);

    // Should have warning about unmapped IDs (products with old_category_id = 5)
    assert!(!result.warnings.is_empty());
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("could not be mapped")));

    // Verify correct number of mapped vs unmapped records
    let mapped_count = db
        .execute(
            "SELECT COUNT(*) as count FROM test_products WHERE new_category_id IS NOT NULL",
            &[],
        )
        .await
        .expect("Failed to count mapped records");

    if let Value::Object(row) = &mapped_count.rows[0] {
        if let Some(Value::Number(count)) = row.get("count") {
            assert_eq!(count.as_u64().unwrap(), 12); // 15 products - 3 with unmapped category 5
        }
    }

    let unmapped_count = db
        .execute(
            "SELECT COUNT(*) as count FROM test_products WHERE new_category_id IS NULL",
            &[],
        )
        .await
        .expect("Failed to count unmapped records");

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
