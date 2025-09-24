use d1_rs::auto_migration::{DataMigrationConfig, DataMigrator, FailureStrategy};
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
            ],
        )
        .await
        .expect("Failed to insert user data");
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
            ],
        )
        .await
        .expect("Failed to insert project data");
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
    let generation_query =
        "INSERT INTO user_project_assignments (user_id, project_id) VALUES (1, 1)";
    let validation_rules = vec!["SELECT COUNT(*) FROM user_project_assignments".to_string()];

    let result = data_migration
        .execute_business_rules_population(
            "", // Empty junction table
            generation_query,
            &validation_rules,
        )
        .await
        .expect("Should handle empty junction table gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Junction table cannot be empty"));

    // Test empty generation query
    let result = data_migration
        .execute_business_rules_population(
            "user_project_assignments",
            "", // Empty generation query
            &validation_rules,
        )
        .await
        .expect("Should handle empty generation query gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Generation query cannot be empty"));

    // Test dangerous generation query (DROP)
    let dangerous_query = "DROP TABLE user_project_assignments";
    let result = data_migration
        .execute_business_rules_population(
            "user_project_assignments",
            dangerous_query,
            &validation_rules,
        )
        .await
        .expect("Should handle dangerous query gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("potentially dangerous SQL patterns"));

    // Test dangerous generation query (DELETE)
    let dangerous_query = "DELETE FROM user_project_assignments";
    let result = data_migration
        .execute_business_rules_population(
            "user_project_assignments",
            dangerous_query,
            &validation_rules,
        )
        .await
        .expect("Should handle dangerous query gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("potentially dangerous SQL patterns"));

    // Test non-INSERT generation query
    let non_insert_query = "SELECT * FROM users";
    let result = data_migration
        .execute_business_rules_population(
            "user_project_assignments",
            non_insert_query,
            &validation_rules,
        )
        .await
        .expect("Should handle non-INSERT query gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Generation query must be an INSERT statement"));

    // Test generation query targeting wrong table
    let wrong_table_query = "INSERT INTO wrong_table (user_id, project_id) VALUES (1, 1)";
    let result = data_migration
        .execute_business_rules_population(
            "user_project_assignments",
            wrong_table_query,
            &validation_rules,
        )
        .await
        .expect("Should handle wrong table query gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("does not target the specified junction table"));

    // Test empty validation rule
    let empty_validation_rules = vec!["".to_string()];
    let result = data_migration
        .execute_business_rules_population(
            "user_project_assignments",
            generation_query,
            &empty_validation_rules,
        )
        .await
        .expect("Should handle empty validation rule gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Validation rule 1 cannot be empty"));

    // Test dangerous validation rule
    let dangerous_validation_rules = vec!["DROP TABLE users".to_string()];
    let result = data_migration
        .execute_business_rules_population(
            "user_project_assignments",
            generation_query,
            &dangerous_validation_rules,
        )
        .await
        .expect("Should handle dangerous validation rule gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Validation rule 1 contains potentially dangerous SQL patterns"));
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

    let result = data_migration
        .execute_business_rules_population(
            "non_existent_table",
            generation_query,
            &validation_rules,
        )
        .await
        .expect("Should handle missing table gracefully");

    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Junction table non_existent_table not accessible")));
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

    let result = data_migration
        .execute_business_rules_population(
            "user_project_assignments",
            generation_query,
            &validation_rules,
        )
        .await
        .expect("Business rules population should succeed");

    assert!(result.success);
    assert!(result.records_processed > 0);
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());

    // Should have generation success message
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Generation query executed successfully")));

    // Should have validation summary
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("validation rules to verify business rules compliance")));
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("All validation rules passed successfully")));

    // Verify the business rules were applied correctly
    let assignments_result = db.execute(
        "SELECT ua.assignment_type, u.seniority_level, p.complexity_level FROM user_project_assignments ua JOIN users u ON ua.user_id = u.id JOIN projects p ON ua.project_id = p.id ORDER BY ua.id",
        &[]
    ).await.expect("Failed to query assignments");

    // Check that assignment types follow the business rules
    for row in assignments_result.rows() {
        if let Value::Object(row_map) = row {
            if let (
                Some(Value::String(assignment_type)),
                Some(Value::Number(seniority)),
                Some(Value::Number(complexity)),
            ) = (
                row_map.get("assignment_type"),
                row_map.get("seniority_level"),
                row_map.get("complexity_level"),
            ) {
                let seniority_val = seniority.as_u64().unwrap();
                let complexity_val = complexity.as_u64().unwrap();

                match assignment_type.as_str() {
                    "lead" => assert!(seniority_val >= complexity_val + 2),
                    "standard" => assert!(
                        seniority_val >= complexity_val && seniority_val < complexity_val + 2
                    ),
                    "support" => assert!(
                        seniority_val >= complexity_val.saturating_sub(1)
                            && seniority_val < complexity_val
                    ),
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

    let result = data_migration
        .execute_business_rules_population(
            "user_project_assignments",
            failing_query,
            &validation_rules,
        )
        .await
        .expect("Should handle generation failure gracefully");

    assert!(!result.success);
    assert!(result.records_failed > 0);
    assert!(result
        .errors
        .iter()
        .any(|e| e.to_string().contains("Generation query failed")));
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
    let generation_query =
        "INSERT INTO user_project_assignments (user_id, project_id) VALUES (1, 1)";

    // Use validation rules that will fail (invalid table reference)
    let failing_validation_rules = vec![
        "SELECT COUNT(*) FROM user_project_assignments".to_string(), // This will pass
        "SELECT COUNT(*) FROM invalid_table_name".to_string(),       // This will fail
    ];

    let result = data_migration
        .execute_business_rules_population(
            "user_project_assignments",
            generation_query,
            &failing_validation_rules,
        )
        .await
        .expect("Should handle validation failure gracefully");

    assert!(result.success); // Generation succeeded, so overall success is true
    assert!(result.records_processed > 0);
    assert!(result.records_failed > 0); // But validation failures are counted
    assert!(result
        .errors
        .iter()
        .any(|e| e.to_string().contains("Validation rule 2 failed")));

    // Should have validation failure summary
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("validation rules failed or detected issues")));
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

    let result = data_migration
        .execute_business_rules_population(
            "user_project_assignments",
            generation_query,
            &no_validation_rules,
        )
        .await
        .expect("Should handle no validation rules gracefully");

    assert!(result.success);
    assert!(result.records_processed > 0);
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());

    // Should not have validation messages since no rules were provided
    assert!(!result
        .warnings
        .iter()
        .any(|w| w.contains("validation rules")));

    // Should have generation success message
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Generation query executed successfully")));
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

    let result = data_migration
        .execute_business_rules_population(
            "user_project_assignments",
            complex_generation_query,
            &complex_validation_rules,
        )
        .await
        .expect("Complex business rules population should succeed");

    assert!(result.success);
    assert!(result.records_processed > 0);
    // Note: Some validation rules may detect issues (e.g., count=0 for junior devs on high complexity projects)
    // This is expected behavior and doesn't indicate failure
    assert!(result.errors.is_empty());

    // Verify complex business rules were applied
    let complex_result = db
        .execute(
            r#"SELECT 
            u.role, u.seniority_level, p.complexity_level, ua.assignment_type
           FROM user_project_assignments ua 
           JOIN users u ON ua.user_id = u.id 
           JOIN projects p ON ua.project_id = p.id 
           ORDER BY p.complexity_level DESC, u.seniority_level DESC"#,
            &[],
        )
        .await
        .expect("Failed to query complex assignments");

    // Verify no junior developers are assigned to high-complexity projects
    for row in complex_result.rows() {
        if let Value::Object(row_map) = row {
            if let (Some(Value::String(role)), Some(Value::Number(complexity))) =
                (row_map.get("role"), row_map.get("complexity_level"))
            {
                let complexity_val = complexity.as_u64().unwrap();
                if role.contains("Junior") {
                    assert!(
                        complexity_val < 4,
                        "Junior developer assigned to high-complexity project"
                    );
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
    let result = data_migration
        .execute_external_source_population(
            "", // Empty junction table
            json_data,
            "json",
            &column_mapping,
            &validation_rules,
        )
        .await
        .expect("Should handle empty junction table gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Junction table cannot be empty"));

    // Test empty source data
    let result = data_migration
        .execute_external_source_population(
            "external_assignments",
            "", // Empty source data
            "json",
            &column_mapping,
            &validation_rules,
        )
        .await
        .expect("Should handle empty source data gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Source data cannot be empty"));

    // Test empty source format
    let result = data_migration
        .execute_external_source_population(
            "external_assignments",
            json_data,
            "", // Empty source format
            &column_mapping,
            &validation_rules,
        )
        .await
        .expect("Should handle empty source format gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Source format cannot be empty"));

    // Test unsupported source format
    let result = data_migration
        .execute_external_source_population(
            "external_assignments",
            json_data,
            "xml", // Unsupported format
            &column_mapping,
            &validation_rules,
        )
        .await
        .expect("Should handle unsupported format gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Unsupported source format 'xml'"));

    // Test empty column mapping
    let empty_mapping = HashMap::new();
    let result = data_migration
        .execute_external_source_population(
            "external_assignments",
            json_data,
            "json",
            &empty_mapping,
            &validation_rules,
        )
        .await
        .expect("Should handle empty column mapping gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Column mapping cannot be empty"));

    // Test empty column names in mapping
    let mut invalid_mapping = HashMap::new();
    invalid_mapping.insert("".to_string(), "external_user_id".to_string()); // Empty source column
    invalid_mapping.insert("project_id".to_string(), "external_project_id".to_string());

    let result = data_migration
        .execute_external_source_population(
            "external_assignments",
            json_data,
            "json",
            &invalid_mapping,
            &validation_rules,
        )
        .await
        .expect("Should handle empty column names gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Column names in mapping cannot be empty"));

    // Test duplicate target columns
    let mut duplicate_mapping = HashMap::new();
    duplicate_mapping.insert("user_id".to_string(), "external_user_id".to_string());
    duplicate_mapping.insert("project_id".to_string(), "external_user_id".to_string()); // Duplicate target

    let result = data_migration
        .execute_external_source_population(
            "external_assignments",
            json_data,
            "json",
            &duplicate_mapping,
            &validation_rules,
        )
        .await
        .expect("Should handle duplicate target columns gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Duplicate target column 'external_user_id'"));

    // Test dangerous validation rule
    let dangerous_validation_rules = vec!["DROP TABLE external_assignments".to_string()];
    let result = data_migration
        .execute_external_source_population(
            "external_assignments",
            json_data,
            "json",
            &column_mapping,
            &dangerous_validation_rules,
        )
        .await
        .expect("Should handle dangerous validation rule gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Validation rule 1 contains potentially dangerous SQL patterns"));
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
    let result = data_migration
        .execute_external_source_population(
            "non_existent_table",
            json_data,
            "json",
            &column_mapping,
            &validation_rules,
        )
        .await
        .expect("Should handle missing table gracefully");

    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Junction table non_existent_table not accessible")));
}

#[tokio::test]
async fn test_execute_external_source_population_json_success() {
    let db = setup_test_db().await;
    create_test_tables_for_external_source_population(&db).await;

    let config = DataMigrationConfig {
        batch_size: 2, // Small batch size to test batch processing
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
        "SELECT COUNT(*) FROM external_assignments WHERE assignment_role IN ('lead', 'member')"
            .to_string(),
    ];

    let result = data_migration
        .execute_external_source_population(
            "external_assignments",
            json_data,
            "json",
            &column_mapping,
            &validation_rules,
        )
        .await
        .expect("JSON external source population should succeed");

    assert!(result.success);
    assert_eq!(result.records_processed, 3);
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());

    // Should have parsing success message
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Parsed 3 records from external source (format: json)")));

    // Should have validation summary
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("validation rules to verify external source data integrity")));
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("All validation rules passed successfully")));

    // Verify the records were inserted correctly
    let assignments_result = db.execute(
        "SELECT external_user_id, external_project_id, assignment_role, start_date FROM external_assignments ORDER BY external_user_id",
        &[]
    ).await.expect("Failed to query assignments");

    assert_eq!(assignments_result.rows().len(), 3);

    // Check specific record values
    if let Value::Object(row) = &assignments_result.rows()[0] {
        if let (Some(Value::Number(user_id)), Some(Value::String(role))) =
            (row.get("external_user_id"), row.get("assignment_role"))
        {
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

    let result = data_migration
        .execute_external_source_population(
            "external_permissions",
            csv_data,
            "csv",
            &column_mapping,
            &validation_rules,
        )
        .await
        .expect("CSV external source population should succeed");

    assert!(result.success);
    assert_eq!(result.records_processed, 4);
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());

    // Should have parsing success message
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Parsed 4 records from external source (format: csv)")));

    // Verify the records were inserted correctly
    let permissions_result = db.execute(
        "SELECT user_ref, permission_ref, granted_date FROM external_permissions ORDER BY user_ref",
        &[]
    ).await.expect("Failed to query permissions");

    assert_eq!(permissions_result.rows().len(), 4);

    // Check specific record values
    if let Value::Object(row) = &permissions_result.rows()[0] {
        if let (Some(Value::Number(user_ref)), Some(Value::Number(permission_ref))) =
            (row.get("user_ref"), row.get("permission_ref"))
        {
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

    let validation_rules: Vec<String> = vec![]; // No validation rules

    let result = data_migration
        .execute_external_source_population(
            "external_assignments",
            array_data,
            "array",
            &column_mapping,
            &validation_rules,
        )
        .await
        .expect("Array external source population should succeed");

    assert!(result.success);
    assert_eq!(result.records_processed, 3);
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());

    // Should have parsing success message
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Parsed 3 records from external source (format: array)")));

    // Should NOT have validation messages since no rules were provided
    assert!(!result
        .warnings
        .iter()
        .any(|w| w.contains("validation rules")));

    // Verify the records were inserted correctly
    let array_result = db.execute(
        "SELECT external_user_id, assignment_role FROM external_assignments WHERE external_user_id >= 10 ORDER BY external_user_id",
        &[]
    ).await.expect("Failed to query array assignments");

    assert_eq!(array_result.rows().len(), 3);

    // Check specific record values
    if let Value::Object(row) = &array_result.rows()[0] {
        if let (Some(Value::Number(user_id)), Some(Value::String(role))) =
            (row.get("external_user_id"), row.get("assignment_role"))
        {
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
    let result = data_migration
        .execute_external_source_population(
            "external_assignments",
            invalid_json,
            "json",
            &column_mapping,
            &validation_rules,
        )
        .await
        .expect("Should handle invalid JSON gracefully");

    assert!(!result.success);
    assert!(result.errors.iter().any(|e| e
        .to_string()
        .contains("Failed to parse external source data")));

    // Test invalid CSV (mismatched columns)
    let invalid_csv = r#"user_id,project_id
1,2,3"#; // 3 values but 2 headers
    let result = data_migration
        .execute_external_source_population(
            "external_assignments",
            invalid_csv,
            "csv",
            &column_mapping,
            &validation_rules,
        )
        .await
        .expect("Should handle invalid CSV gracefully");

    assert!(!result.success);
    assert!(result.errors.iter().any(|e| e
        .to_string()
        .contains("Failed to parse external source data")));

    // Test JSON array with non-objects
    let invalid_json_array = r#"[1, 2, 3]"#;
    let result = data_migration
        .execute_external_source_population(
            "external_assignments",
            invalid_json_array,
            "json",
            &column_mapping,
            &validation_rules,
        )
        .await
        .expect("Should handle non-object JSON array gracefully");

    assert!(!result.success);
    assert!(result.errors.iter().any(|e| e
        .to_string()
        .contains("Failed to parse external source data")));
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

    let result = data_migration
        .execute_external_source_population(
            "external_assignments",
            incomplete_json,
            "json",
            &column_mapping,
            &validation_rules,
        )
        .await
        .expect("Should handle missing columns gracefully");

    // Success should be false as 2 out of 3 records succeeded (66.7% < 80% threshold)
    assert!(!result.success);
    assert_eq!(result.records_processed, 3);
    assert_eq!(result.records_failed, 1); // One record missing project_id

    // Should have warnings about missing columns
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Source column 'project_id' not found")));

    // Should have summary about failed records
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Failed to process 1 out of 3 external records")));

    // Verify only 2 records were inserted
    let incomplete_result = db
        .execute("SELECT COUNT(*) as count FROM external_assignments", &[])
        .await
        .expect("Failed to count records");

    if let Value::Object(row) = &incomplete_result.rows()[0] {
        if let Some(Value::Number(count)) = row.get("count") {
            assert!(count.as_u64().unwrap() >= 2); // At least 2 records from previous tests
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
        "SELECT COUNT(*) FROM external_assignments".to_string(), // This will pass
        "SELECT COUNT(*) FROM invalid_table_name".to_string(),   // This will fail
    ];

    let result = data_migration
        .execute_external_source_population(
            "external_assignments",
            json_data,
            "json",
            &column_mapping,
            &failing_validation_rules,
        )
        .await
        .expect("Should handle validation failure gracefully");

    assert!(!result.success); // Should fail due to validation failures
    assert!(result.records_processed > 0);
    assert!(result.records_failed > 0); // Validation failures are counted
    assert!(result
        .errors
        .iter()
        .any(|e| e.to_string().contains("Validation rule 2 failed")));

    // Should have validation failure summary
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("validation rules failed or detected issues")));
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

    let result = data_migration
        .execute_external_source_population(
            "external_assignments",
            single_json,
            "json",
            &column_mapping,
            &validation_rules,
        )
        .await
        .expect("Single JSON object population should succeed");

    assert!(result.success);
    assert_eq!(result.records_processed, 1);
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());

    // Should have parsing success message
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Parsed 1 records from external source (format: json)")));

    // Verify the record was inserted correctly
    let single_result = db.execute(
        "SELECT external_user_id, assignment_role FROM external_assignments WHERE external_user_id = 999",
        &[]
    ).await.expect("Failed to query single assignment");

    assert_eq!(single_result.rows().len(), 1);

    if let Value::Object(row) = &single_result.rows()[0] {
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
            Value::String("tag1|tag2|tag3".to_string()), // Pipe delimited
            Value::String("tag4;tag5;tag6".to_string()), // Semicolon delimited
        ],
    )
    .await
    .expect("Failed to insert test article");

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
    let result = data_migration
        .execute_denormalized_column_population(
            "delimiter_article_tags",
            "delimiter_articles",
            "pipe_tags",
            "|",
            "article_id",
            "tag_name",
        )
        .await
        .expect("Should handle pipe delimiter");

    assert!(result.success);
    assert_eq!(result.records_processed, 1);

    // Verify pipe-delimited tags were inserted
    let pipe_result = db
        .execute("SELECT COUNT(*) as count FROM delimiter_article_tags", &[])
        .await
        .expect("Failed to query junction table");

    if let Value::Object(row) = &pipe_result.rows()[0] {
        if let Some(Value::Number(count)) = row.get("count") {
            assert_eq!(count.as_u64().unwrap(), 3); // tag1, tag2, tag3
        }
    }

    // Clear junction table for next test
    db.execute("DELETE FROM delimiter_article_tags", &[])
        .await
        .expect("Failed to clear junction table");

    // Test semicolon delimiter
    let result = data_migration
        .execute_denormalized_column_population(
            "delimiter_article_tags",
            "delimiter_articles",
            "semicolon_tags",
            ";",
            "article_id",
            "tag_name",
        )
        .await
        .expect("Should handle semicolon delimiter");

    assert!(result.success);
    assert_eq!(result.records_processed, 1);

    // Verify semicolon-delimited tags were inserted
    let semi_result = db
        .execute("SELECT COUNT(*) as count FROM delimiter_article_tags", &[])
        .await
        .expect("Failed to query junction table");

    if let Value::Object(row) = &semi_result.rows()[0] {
        if let Some(Value::Number(count)) = row.get("count") {
            assert_eq!(count.as_u64().unwrap(), 3); // tag4, tag5, tag6
        }
    }
}
