use chrono::{DateTime, Utc};
use d1_rs::*;
use d1_rs::backends::QueryResult;
use serde::{Deserialize, Serialize};

pub mod multi_db;
pub mod database_manager;
pub mod query_helpers;
pub mod backend_verification;

// Add fixtures module path for the tests directory
#[path = "../fixtures/mod.rs"]
pub mod fixtures;

// Test model definitions
#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
#[table(name = "test_users")]
pub struct TestUser {
    #[primary_key]
    pub id: i64,
    pub email: String,
    pub name: String,
    pub is_active: bool,
    pub score: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
#[table(name = "test_posts")]
pub struct TestPost {
    #[primary_key]
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub content: String,
    pub is_published: bool,
    pub views: i32,
    pub created_at: DateTime<Utc>,
}

#[allow(dead_code)]
pub async fn setup_test_db() -> D1Client {
    let db = D1Client::new_in_memory()
        .await
        .expect("Failed to create in-memory database");

    // Create test tables
    let create_users = r#"
        CREATE TABLE test_users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            email TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            score INTEGER,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )
    "#;

    let create_posts = r#"
        CREATE TABLE test_posts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            is_published INTEGER NOT NULL DEFAULT 0,
            views INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES test_users(id)
        )
    "#;

    db.execute(create_users, &[])
        .await
        .expect("Failed to create users table");
    db.execute(create_posts, &[])
        .await
        .expect("Failed to create posts table");

    db
}

#[allow(dead_code)]
pub async fn seed_test_data(db: &D1Client) {
    use serde_json::Value;

    // Insert test users
    let insert_user = "INSERT INTO test_users (email, name, is_active, score) VALUES (?, ?, ?, ?)";

    db.execute(
        insert_user,
        &[
            Value::String("alice@example.com".to_string()),
            Value::String("Alice".to_string()),
            Value::Bool(true),
            Value::Number(100.into()),
        ],
    )
    .await
    .expect("Failed to insert alice");

    db.execute(
        insert_user,
        &[
            Value::String("bob@example.com".to_string()),
            Value::String("Bob".to_string()),
            Value::Bool(false),
            Value::Number(50.into()),
        ],
    )
    .await
    .expect("Failed to insert bob");

    db.execute(
        insert_user,
        &[
            Value::String("charlie@example.com".to_string()),
            Value::String("Charlie".to_string()),
            Value::Bool(true),
            Value::Null,
        ],
    )
    .await
    .expect("Failed to insert charlie");

    // Insert test posts
    let insert_post = "INSERT INTO test_posts (user_id, title, content, is_published, views) VALUES (?, ?, ?, ?, ?)";

    db.execute(
        insert_post,
        &[
            Value::Number(1.into()),
            Value::String("First Post".to_string()),
            Value::String("Content of first post".to_string()),
            Value::Bool(true),
            Value::Number(100.into()),
        ],
    )
    .await
    .expect("Failed to insert first post");

    db.execute(
        insert_post,
        &[
            Value::Number(1.into()),
            Value::String("Second Post".to_string()),
            Value::String("Content of second post".to_string()),
            Value::Bool(false),
            Value::Number(0.into()),
        ],
    )
    .await
    .expect("Failed to insert second post");

    db.execute(
        insert_post,
        &[
            Value::Number(2.into()),
            Value::String("Bob's Post".to_string()),
            Value::String("Bob's content".to_string()),
            Value::Bool(true),
            Value::Number(50.into()),
        ],
    )
    .await
    .expect("Failed to insert Bob's post");
}

// =================================================================================================
// ROLLBACK SYSTEM TEST UTILITIES
// =================================================================================================

use d1_rs::auto_migration::rollback::*;
use d1_rs::auto_migration::{MigrationPlan, MigrationOperation, DatabaseSchema, ColumnSchema, IndexSchema};
use std::time::Duration;

/// Specialized utilities for rollback system testing
#[allow(dead_code)]
pub struct RollbackTestUtilities;

#[allow(dead_code)]
impl RollbackTestUtilities {
    /// Create a minimal test schema for rollback testing
    pub async fn setup_minimal_rollback_schema(db: &D1Client) -> d1_rs::Result<DatabaseSchema> {
        let create_simple_table = r#"
            CREATE TABLE simple_test_table (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                value INTEGER,
                is_active BOOLEAN DEFAULT true
            )
        "#;
        
        db.execute(create_simple_table, &[]).await?;
        
        let introspector = d1_rs::auto_migration::SchemaIntrospector::new(db);
        introspector.introspect_database().await
    }
    
    /// Create a complex test schema with relationships for comprehensive rollback testing
    pub async fn setup_complex_rollback_schema(db: &D1Client) -> d1_rs::Result<DatabaseSchema> {
        let schemas = vec![
            r#"CREATE TABLE rollback_users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username VARCHAR(50) NOT NULL UNIQUE,
                email TEXT NOT NULL UNIQUE,
                full_name TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )"#,
            
            r#"CREATE TABLE rollback_projects (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name VARCHAR(100) NOT NULL,
                description TEXT,
                owner_id INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (owner_id) REFERENCES rollback_users(id) ON DELETE CASCADE
            )"#,
            
            r#"CREATE TABLE rollback_tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title VARCHAR(200) NOT NULL,
                description TEXT,
                project_id INTEGER NOT NULL,
                assignee_id INTEGER,
                status VARCHAR(20) DEFAULT 'pending',
                priority INTEGER DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (project_id) REFERENCES rollback_projects(id) ON DELETE CASCADE,
                FOREIGN KEY (assignee_id) REFERENCES rollback_users(id) ON DELETE SET NULL
            )"#,
        ];
        
        let indexes = vec![
            "CREATE INDEX idx_rollback_users_username ON rollback_users(username)",
            "CREATE INDEX idx_rollback_users_email ON rollback_users(email)",
            "CREATE INDEX idx_rollback_projects_owner ON rollback_projects(owner_id)",
            "CREATE INDEX idx_rollback_tasks_project ON rollback_tasks(project_id)",
            "CREATE INDEX idx_rollback_tasks_assignee ON rollback_tasks(assignee_id)",
            "CREATE INDEX idx_rollback_tasks_status ON rollback_tasks(status)",
        ];
        
        // Execute table creation
        for schema_sql in schemas {
            db.execute(schema_sql, &[]).await?;
        }
        
        // Execute index creation
        for index_sql in indexes {
            db.execute(index_sql, &[]).await?;
        }
        
        let introspector = d1_rs::auto_migration::SchemaIntrospector::new(db);
        introspector.introspect_database().await
    }
    
    /// Seed test data for rollback testing scenarios
    pub async fn seed_rollback_test_data(db: &D1Client) -> d1_rs::Result<()> {
        use serde_json::Value;
        
        // Insert test users
        let insert_user = "INSERT INTO rollback_users (username, email, full_name) VALUES (?, ?, ?)";
        let users = vec![
            ("alice_dev", "alice@example.com", "Alice Developer"),
            ("bob_pm", "bob@example.com", "Bob Project Manager"),
            ("charlie_qa", "charlie@example.com", "Charlie QA Engineer"),
        ];
        
        for (username, email, full_name) in users {
            db.execute(insert_user, &[
                Value::String(username.to_string()),
                Value::String(email.to_string()),
                Value::String(full_name.to_string()),
            ]).await?;
        }
        
        // Insert test projects
        let insert_project = "INSERT INTO rollback_projects (name, description, owner_id) VALUES (?, ?, ?)";
        let projects = vec![
            ("Database Migration System", "Automatic database migration tool", 1),
            ("API Gateway", "Microservices API gateway", 2),
            ("Monitoring Dashboard", "System monitoring and alerting", 1),
        ];
        
        for (name, description, owner_id) in projects {
            db.execute(insert_project, &[
                Value::String(name.to_string()),
                Value::String(description.to_string()),
                Value::Number(owner_id.into()),
            ]).await?;
        }
        
        // Insert test tasks
        let insert_task = "INSERT INTO rollback_tasks (title, description, project_id, assignee_id, status, priority) VALUES (?, ?, ?, ?, ?, ?)";
        let tasks = vec![
            ("Implement rollback system", "Create comprehensive rollback functionality", 1, Some(1), "in_progress", 1),
            ("Add safety validation", "Implement safety checks for rollbacks", 1, Some(3), "pending", 2),
            ("Setup API routes", "Configure gateway routing", 2, Some(2), "completed", 1),
            ("Create monitoring alerts", "Setup alerting system", 3, Some(1), "pending", 2),
        ];
        
        for (title, description, project_id, assignee_id, status, priority) in tasks {
            db.execute(insert_task, &[
                Value::String(title.to_string()),
                Value::String(description.to_string()),
                Value::Number(project_id.into()),
                match assignee_id {
                    Some(id) => Value::Number(id.into()),
                    None => Value::Null,
                },
                Value::String(status.to_string()),
                Value::Number(priority.into()),
            ]).await?;
        }
        
        Ok(())
    }
    
    /// Create a safe test migration plan for rollback testing
    pub fn create_safe_test_migration() -> MigrationPlan {
        MigrationPlan {
            operations: vec![
                MigrationOperation::AddColumn {
                    table: "rollback_users".to_string(),
                    column: ColumnSchema {
                        name: "last_login".to_string(),
                        column_type: "DATETIME".to_string(),
                        nullable: true,
                        default_value: None,
                        primary_key: false,
                        auto_increment: false,
                        unique: false,
                        constraints: vec![],
                    },
                },
                MigrationOperation::CreateIndex {
                    table: "rollback_tasks".to_string(),
                    index: IndexSchema {
                        name: "idx_rollback_tasks_priority".to_string(),
                        table_name: Some("rollback_tasks".to_string()),
                        columns: vec!["priority".to_string(), "status".to_string()],
                        unique: false,
                    },
                },
            ],
            estimated_duration: Duration::from_secs(10),
            safety_warnings: vec![],
            rollback_plan: vec![],
        }
    }
    
    /// Create a risky test migration plan for safety testing
    pub fn create_risky_test_migration() -> MigrationPlan {
        MigrationPlan {
            operations: vec![
                MigrationOperation::DropTable {
                    name: "rollback_tasks".to_string(),
                },
                MigrationOperation::DropColumn {
                    table: "rollback_users".to_string(),
                    column: "email".to_string(),
                },
            ],
            estimated_duration: Duration::from_secs(10),
            safety_warnings: vec![],
            rollback_plan: vec![],
        }
    }
    
    /// Create test rollback configuration with various settings
    pub fn create_test_rollback_config(preserve_data: bool, use_transactions: bool) -> RollbackConfig {
        RollbackConfig {
            preserve_data_on_rollback: preserve_data,
            restore_data_on_rollback: preserve_data,
            create_backup_tables: preserve_data,
            stop_on_first_failure: true,
            operation_timeout: Duration::from_secs(30),
            use_transactions,
            max_parallel_operations: 1,
            validate_schema_compatibility: true,
            perform_dry_run: false,
            backup_table_prefix: "rollback_backup_".to_string(),
            backup_table_suffix: "_temp".to_string(),
        }
    }
    
    /// Verify database state after rollback operation
    pub async fn verify_rollback_state(
        db: &D1Client,
        expected_tables: &[&str],
        expected_missing_tables: &[&str],
    ) -> d1_rs::Result<bool> {
        use serde_json::Value;
        
        let tables_query = "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name";
        let result = db.execute(tables_query, &[]).await?;
        
        let existing_tables: Vec<String> = result.rows()
            .iter()
            .filter_map(|row| {
                if let Value::Object(obj) = row {
                    if let Some(Value::String(name)) = obj.get("name") {
                        Some(name.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        
        // Check expected tables exist
        for expected_table in expected_tables {
            if !existing_tables.contains(&expected_table.to_string()) {
                return Ok(false);
            }
        }
        
        // Check expected missing tables don't exist
        for missing_table in expected_missing_tables {
            if existing_tables.contains(&missing_table.to_string()) {
                return Ok(false);
            }
        }
        
        Ok(true)
    }
    
    /// Verify column exists in table
    pub async fn verify_column_exists(
        db: &D1Client,
        table_name: &str,
        column_name: &str,
    ) -> d1_rs::Result<bool> {
        use serde_json::Value;
        
        let pragma_query = format!("PRAGMA table_info({})", table_name);
        let result = db.execute(&pragma_query, &[]).await?;
        
        let column_exists = result.rows()
            .iter()
            .any(|row| {
                if let Value::Object(obj) = row {
                    if let Some(Value::String(name)) = obj.get("name") {
                        name == column_name
                    } else {
                        false
                    }
                } else {
                    false
                }
            });
        
        Ok(column_exists)
    }
    
    /// Verify index exists
    pub async fn verify_index_exists(
        db: &D1Client,
        index_name: &str,
    ) -> d1_rs::Result<bool> {
        use serde_json::Value;
        
        let index_query = "SELECT name FROM sqlite_master WHERE type='index' AND name = ?";
        let result = db.execute(index_query, &[
            Value::String(index_name.to_string())
        ]).await?;
        
        Ok(!result.rows().is_empty())
    }
    
    /// Count records in a table (for data preservation verification)
    pub async fn count_table_records(
        db: &D1Client,
        table_name: &str,
    ) -> d1_rs::Result<i64> {
        use serde_json::Value;
        
        let count_query = format!("SELECT COUNT(*) as count FROM {}", table_name);
        let result = db.execute(&count_query, &[]).await?;
        
        if let Some(Value::Object(obj)) = result.rows().first() {
            if let Some(Value::Number(count)) = obj.get("count") {
                return Ok(count.as_i64().unwrap_or(0));
            }
        }
        
        Ok(0)
    }
    
    /// Create a mock progress tracker for testing
    pub fn create_mock_progress_tracker() -> RollbackProgressTracker {
        RollbackProgressTracker::new()
    }
    
    /// Validate rollback execution result structure
    pub fn validate_execution_result(result: &RollbackExecutionResult) -> bool {
        // Check required fields are present
        if result.rollback_id.is_empty() {
            return false;
        }
        
        if result.total_duration == Duration::from_secs(0) && !result.completed_operations.is_empty() {
            return false;
        }
        
        // Verify operation counts consistency
        if result.operations_completed != result.completed_operations.len() {
            return false;
        }
        
        if result.operations_failed != result.failed_operations.len() {
            return false;
        }
        
        // Verify success flag consistency
        if result.success && result.operations_failed > 0 {
            return false;
        }
        
        if !result.success && result.operations_failed == 0 && result.operations_completed > 0 {
            return false;
        }
        
        true
    }
    
    /// Validate rollback validation result structure
    pub fn validate_validation_result(result: &RollbackValidationResult) -> bool {
        // If not safe (has blocking issues), should have validation issues or blocking issues
        let is_safe = result.blocking_issues.is_empty();
        if !is_safe && result.validation_issues.is_empty() && result.blocking_issues.is_empty() {
            return false;
        }
        
        // High risk should have validation issues
        if matches!(result.overall_risk, RollbackRiskLevel::High | RollbackRiskLevel::Critical) 
           && result.validation_issues.is_empty() {
            return false;
        }
        
        // Blocking issues should make it unsafe
        let is_safe = result.blocking_issues.is_empty();
        if !result.blocking_issues.is_empty() && is_safe {
            return false;
        }
        
        true
    }
    
    /// Create error simulation database scenarios for testing failure conditions
    pub async fn create_error_simulation_scenario(db: &D1Client, scenario_type: &str) -> d1_rs::Result<()> {
        match scenario_type {
            "missing_table" => {
                // Create references to non-existent tables
                let _ = db.execute(
                    r#"CREATE TABLE orphan_references (
                        id INTEGER PRIMARY KEY,
                        missing_table_id INTEGER,
                        FOREIGN KEY (missing_table_id) REFERENCES nonexistent_table(id)
                    )"#,
                    &[]
                ).await;
            },
            "type_mismatch" => {
                // Create tables with incompatible types
                db.execute(
                    r#"CREATE TABLE type_mismatch_source (
                        id INTEGER PRIMARY KEY,
                        text_as_number TEXT DEFAULT 'not_a_number'
                    )"#,
                    &[]
                ).await?;
                
                db.execute(
                    r#"CREATE TABLE type_mismatch_target (
                        id INTEGER PRIMARY KEY,
                        number_field INTEGER NOT NULL
                    )"#,
                    &[]
                ).await?;
            },
            "constraint_violation" => {
                // Create data that violates constraints
                use serde_json::Value;
                
                db.execute(
                    r#"CREATE TABLE constraint_test (
                        id INTEGER PRIMARY KEY,
                        email TEXT UNIQUE NOT NULL,
                        age INTEGER CHECK (age >= 0 AND age <= 150)
                    )"#,
                    &[]
                ).await?;
                
                // Insert duplicate emails
                db.execute(
                    "INSERT INTO constraint_test (email, age) VALUES (?, ?)",
                    &[Value::String("duplicate@test.com".to_string()), Value::Number(25.into())]
                ).await?;
                
                db.execute(
                    "INSERT INTO constraint_test (email, age) VALUES (?, ?)",
                    &[Value::String("duplicate@test.com".to_string()), Value::Number(30.into())]
                ).await.ok(); // Will fail but that's expected
            },
            "corrupted_backup" => {
                // Create intentionally corrupted backup tables
                db.execute(
                    r#"CREATE TABLE original_data (
                        id INTEGER PRIMARY KEY,
                        name TEXT NOT NULL,
                        data TEXT NOT NULL
                    )"#,
                    &[]
                ).await?;
                
                db.execute(
                    r#"CREATE TABLE original_data_backup (
                        id TEXT,  -- Wrong type
                        wrong_name TEXT,  -- Wrong column name
                        missing_data INTEGER  -- Wrong type and missing data column
                    )"#,
                    &[]
                ).await?;
            },
            _ => {
                return Err(d1_rs::D1RsError::ValidationError(
                    format!("Unknown error scenario type: {}", scenario_type)
                ));
            }
        }
        
        Ok(())
    }
    
    /// Verify error recovery state after failed operations
    pub async fn verify_error_recovery_state(
        db: &D1Client,
        expected_state: &str,
    ) -> d1_rs::Result<bool> {
        match expected_state {
            "partial_completion" => {
                // Check if some operations completed while others failed
                let tables = db.execute(
                    "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
                    &[]
                ).await?;
                
                // Should have at least some tables but possibly missing others
                Ok(!tables.rows().is_empty())
            },
            "rollback_recovery" => {
                // Check if rollback operations can be recovered
                let integrity_check = db.execute("PRAGMA integrity_check", &[]).await?;
                
                use serde_json::Value;
                if let Some(Value::Object(obj)) = integrity_check.rows().first() {
                    if let Some(Value::String(result)) = obj.values().next() {
                        return Ok(result == "ok");
                    }
                }
                Ok(false)
            },
            "disaster_recovery" => {
                // Check if database is in a recoverable state after disaster
                let basic_query = db.execute("SELECT 1", &[]).await;
                Ok(basic_query.is_ok())
            },
            _ => {
                Err(d1_rs::D1RsError::ValidationError(
                    format!("Unknown recovery state: {}", expected_state)
                ))
            }
        }
    }
    
    /// Create mock failure conditions for testing error handling
    pub fn create_mock_failure_condition(failure_type: &str) -> MockFailureCondition {
        match failure_type {
            "connection_timeout" => MockFailureCondition {
                failure_type: "connection_timeout".to_string(),
                should_fail: true,
                error_message: "Database connection timed out".to_string(),
                recovery_possible: true,
            },
            "permission_denied" => MockFailureCondition {
                failure_type: "permission_denied".to_string(),
                should_fail: true,
                error_message: "Insufficient permissions for operation".to_string(),
                recovery_possible: false,
            },
            "storage_exhausted" => MockFailureCondition {
                failure_type: "storage_exhausted".to_string(),
                should_fail: true,
                error_message: "Insufficient storage space".to_string(),
                recovery_possible: true,
            },
            "schema_corruption" => MockFailureCondition {
                failure_type: "schema_corruption".to_string(),
                should_fail: true,
                error_message: "Schema corruption detected".to_string(),
                recovery_possible: false,
            },
            _ => MockFailureCondition {
                failure_type: "unknown".to_string(),
                should_fail: true,
                error_message: "Unknown error occurred".to_string(),
                recovery_possible: false,
            },
        }
    }
    
    /// Validate comprehensive error scenario test results
    pub fn validate_error_scenario_result(
        scenario_type: &str,
        execution_result: &d1_rs::auto_migration::rollback::RollbackExecutionResult,
    ) -> bool {
        match scenario_type {
            "schema_mismatch" => {
                // Should detect schema-related failures
                execution_result.operations_failed > 0 &&
                execution_result.failed_operations.iter().any(|failure| 
                    failure.error.message.contains("schema") ||
                    failure.error.message.contains("table") ||
                    failure.error.message.contains("column")
                )
            },
            "data_consistency" => {
                // Should detect constraint or consistency violations
                execution_result.operations_failed > 0 &&
                execution_result.failed_operations.iter().any(|failure| 
                    failure.error.message.contains("constraint") ||
                    failure.error.message.contains("violation") ||
                    failure.error.message.contains("consistency")
                )
            },
            "execution_failure" => {
                // Should handle execution-level failures gracefully
                !execution_result.recovery_recommendations.is_empty() &&
                !execution_result.audit_trail.is_empty()
            },
            "recovery_scenario" => {
                // Should provide recovery information
                execution_result.operations_completed > 0 || 
                (!execution_result.recovery_recommendations.is_empty() &&
                 !execution_result.final_database_state.created_tables.is_empty())
            },
            _ => false,
        }
    }
}

/// Mock failure condition for testing error handling
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MockFailureCondition {
    pub failure_type: String,
    pub should_fail: bool,
    pub error_message: String,
    pub recovery_possible: bool,
}

