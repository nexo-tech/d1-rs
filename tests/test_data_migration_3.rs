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
            ],
        )
        .await
        .expect("Failed to insert customer data");
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
            ],
        )
        .await
        .expect("Failed to insert order data");
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
    let result = data_migration
        .execute_business_logic_recreation(
            "", // Empty source table
            "customer_analytics",
            "INSERT INTO customer_analytics SELECT * FROM customers",
            &[],
        )
        .await
        .expect("Should handle empty source table gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Source table cannot be empty"));

    // Test empty target table
    let result = data_migration
        .execute_business_logic_recreation(
            "customers",
            "", // Empty target table
            "INSERT INTO customer_analytics SELECT * FROM customers",
            &[],
        )
        .await
        .expect("Should handle empty target table gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Target table cannot be empty"));

    // Test empty recreation query
    let result = data_migration
        .execute_business_logic_recreation(
            "customers",
            "customer_analytics",
            "", // Empty query
            &[],
        )
        .await
        .expect("Should handle empty query gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Recreation query cannot be empty"));
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
    let result = data_migration
        .execute_business_logic_recreation(
            "customers",
            "customer_analytics",
            "DROP TABLE customers", // Dangerous query
            &[],
        )
        .await
        .expect("Should handle dangerous query gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("potentially dangerous SQL pattern"));

    // Test dangerous DELETE pattern
    let result = data_migration
        .execute_business_logic_recreation(
            "customers",
            "customer_analytics",
            "DELETE FROM customers WHERE id = 1", // Dangerous query
            &[],
        )
        .await
        .expect("Should handle dangerous query gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("potentially dangerous SQL pattern"));

    // Test dangerous TRUNCATE pattern
    let result = data_migration
        .execute_business_logic_recreation(
            "customers",
            "customer_analytics",
            "TRUNCATE TABLE customers", // Dangerous query
            &[],
        )
        .await
        .expect("Should handle dangerous query gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("potentially dangerous SQL pattern"));
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
    let result = data_migration
        .execute_business_logic_recreation(
            "non_existent_source",
            "customer_analytics",
            "INSERT INTO customer_analytics SELECT 1, 1, 'bronze', 100.0, 1, '2023-01-01'",
            &[],
        )
        .await
        .expect("Should handle missing source table gracefully");

    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(result.warnings.iter().any(|w| w.contains("not accessible")));

    // Test non-existent target table
    let result = data_migration
        .execute_business_logic_recreation(
            "customers",
            "non_existent_target",
            "SELECT COUNT(*) FROM customers",
            &[],
        )
        .await
        .expect("Should handle missing target table gracefully");

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

