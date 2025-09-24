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
        (
            "Getting Started with Rust",
            Some("programming,rust,tutorial"),
            "tech,development",
        ),
        (
            "Advanced Database Patterns",
            Some("database,sql,patterns"),
            "tech,data",
        ),
        (
            "Web Development Guide",
            Some("web,javascript,frontend"),
            "web,tutorial",
        ),
        (
            "Data Migration Strategies",
            Some("migration,database,automation"),
            "data,enterprise",
        ),
        ("Article with Empty Tags", Some(""), "misc"), // Empty tags
        ("Article with Null Tags", None, "misc"),      // Null tags
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
            ],
        )
        .await
        .expect("Failed to insert article data");
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
    let result = data_migration
        .execute_denormalized_column_population(
            "", // Empty junction table
            "articles",
            "tags",
            ",",
            "article_id",
            "tag_name",
        )
        .await
        .expect("Should handle empty junction table gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Junction table cannot be empty"));

    // Test empty source table
    let result = data_migration
        .execute_denormalized_column_population(
            "article_tags",
            "", // Empty source table
            "tags",
            ",",
            "article_id",
            "tag_name",
        )
        .await
        .expect("Should handle empty source table gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Source table cannot be empty"));

    // Test empty source column
    let result = data_migration
        .execute_denormalized_column_population(
            "article_tags",
            "articles",
            "", // Empty source column
            ",",
            "article_id",
            "tag_name",
        )
        .await
        .expect("Should handle empty source column gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Source column cannot be empty"));

    // Test empty delimiter
    let result = data_migration
        .execute_denormalized_column_population(
            "article_tags",
            "articles",
            "tags",
            "", // Empty delimiter
            "article_id",
            "tag_name",
        )
        .await
        .expect("Should handle empty delimiter gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Delimiter cannot be empty"));

    // Test empty foreign key columns
    let result = data_migration
        .execute_denormalized_column_population(
            "article_tags",
            "articles",
            "tags",
            ",",
            "", // Empty source FK
            "tag_name",
        )
        .await
        .expect("Should handle empty source FK gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Source and target foreign key columns cannot be empty"));

    let result = data_migration
        .execute_denormalized_column_population(
            "article_tags",
            "articles",
            "tags",
            ",",
            "article_id",
            "", // Empty target FK
        )
        .await
        .expect("Should handle empty target FK gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Source and target foreign key columns cannot be empty"));

    // Test same source and target foreign key columns
    let result = data_migration
        .execute_denormalized_column_population(
            "article_tags",
            "articles",
            "tags",
            ",",
            "article_id",
            "article_id", // Same column
        )
        .await
        .expect("Should handle same FK columns gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Source and target foreign key columns cannot be the same"));
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
    let result = data_migration
        .execute_denormalized_column_population(
            "article_tags",
            "non_existent_articles", // Non-existent source table
            "tags",
            ",",
            "article_id",
            "tag_name",
        )
        .await
        .expect("Should handle missing source table gracefully");

    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Source table non_existent_articles not accessible")));

    // Test non-existent junction table
    let result = data_migration
        .execute_denormalized_column_population(
            "non_existent_junction", // Non-existent junction table
            "articles",
            "tags",
            ",",
            "article_id",
            "tag_name",
        )
        .await
        .expect("Should handle missing junction table gracefully");

    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Junction table non_existent_junction not accessible")));
}

#[tokio::test]
async fn test_execute_denormalized_column_population_successful_operation() {
    let db = setup_test_db().await;
    create_test_tables_for_denormalized_population(&db).await;

    let config = DataMigrationConfig {
        batch_size: 2, // Small batch size to test batch processing
        max_transformation_time: Duration::from_secs(30),
        create_backups: false,
        failure_strategy: FailureStrategy::StopOnFailure,
        verify_integrity: false,
        custom_transformations: HashMap::new(),
    };

    let data_migration = DataMigrator::new(db.clone(), config);

    // Execute denormalized column population for tags
    let result = data_migration
        .execute_denormalized_column_population(
            "article_tags",
            "articles",
            "tags",
            ",",
            "article_id",
            "tag_name",
        )
        .await
        .expect("Denormalized column population should succeed");

    assert!(result.success);
    assert_eq!(result.records_processed, 4); // 4 articles with non-empty tags
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());

    // Should have summary of created junction records
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Created") && w.contains("junction records")));

    // Verify the junction records were created correctly
    let junction_result = db
        .execute(
            "SELECT article_id, tag_name FROM article_tags ORDER BY article_id, tag_name",
            &[],
        )
        .await
        .expect("Failed to query junction table");

    // Expected records:
    // Article 1: programming, rust, tutorial (3 tags)
    // Article 2: database, sql, patterns (3 tags)
    // Article 3: web, javascript, frontend (3 tags)
    // Article 4: migration, database, automation (3 tags)
    assert_eq!(junction_result.rows().len(), 12); // Total tag entries

    // Check specific tag entries
    let tags_for_article_1: Vec<String> = junction_result
        .rows()
        .iter()
        .filter_map(|row| {
            if let Value::Object(row_map) = row {
                if let (Some(Value::Number(article_id)), Some(Value::String(tag_name))) =
                    (row_map.get("article_id"), row_map.get("tag_name"))
                {
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
    let result = data_migration
        .execute_denormalized_column_population(
            "empty_article_tags",
            "empty_articles",
            "tags",
            ",",
            "article_id",
            "tag_name",
        )
        .await
        .expect("Should handle empty source table gracefully");

    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert_eq!(result.records_failed, 0);
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("No records with denormalized data found")));
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
        ("Article with commas only", ",,,,"),          // Only commas
        ("Article with spaces and commas", " , , , "), // Spaces and commas
        ("Article with valid and empty", "valid,,empty,   ,another"), // Mixed valid and empty
    ];

    for (title, tags) in test_articles {
        db.execute(
            "INSERT INTO test_articles (title, tags) VALUES (?, ?)",
            &[
                Value::String(title.to_string()),
                Value::String(tags.to_string()),
            ],
        )
        .await
        .expect("Failed to insert test article");
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
    let result = data_migration
        .execute_denormalized_column_population(
            "test_article_tags",
            "test_articles",
            "tags",
            ",",
            "article_id",
            "tag_name",
        )
        .await
        .expect("Should handle empty values gracefully");

    assert!(result.success);
    assert_eq!(result.records_processed, 3);
    assert_eq!(result.records_failed, 0);

    // Verify only valid tags were inserted (should filter out empty strings)
    let junction_result = db
        .execute(
            "SELECT tag_name FROM test_article_tags ORDER BY tag_name",
            &[],
        )
        .await
        .expect("Failed to query junction table");

    // Should have only valid tags: "another", "empty", "valid"
    assert_eq!(junction_result.rows().len(), 3);

    let tag_names: Vec<String> = junction_result
        .rows()
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
            ],
        )
        .await
        .expect("Failed to insert junction record");
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

    let result = data_migration
        .execute_existing_junction_table_population(
            "", // Empty new junction table
            "old_user_roles",
            &column_mapping,
        )
        .await
        .expect("Should handle empty new junction table gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("New junction table cannot be empty"));

    // Test empty old junction table
    let result = data_migration
        .execute_existing_junction_table_population(
            "new_user_permissions",
            "", // Empty old junction table
            &column_mapping,
        )
        .await
        .expect("Should handle empty old junction table gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Old junction table cannot be empty"));

    // Test same table names
    let result = data_migration
        .execute_existing_junction_table_population(
            "old_user_roles",
            "old_user_roles", // Same as new table
            &column_mapping,
        )
        .await
        .expect("Should handle same table names gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("New and old junction tables cannot be the same"));

    // Test empty column mapping
    let empty_mapping = HashMap::new();
    let result = data_migration
        .execute_existing_junction_table_population(
            "new_user_permissions",
            "old_user_roles",
            &empty_mapping,
        )
        .await
        .expect("Should handle empty column mapping gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Column mapping cannot be empty"));

    // Test empty column names in mapping
    let mut invalid_mapping = HashMap::new();
    invalid_mapping.insert("".to_string(), "person_id".to_string()); // Empty old column
    invalid_mapping.insert("role_id".to_string(), "permission_id".to_string());

    let result = data_migration
        .execute_existing_junction_table_population(
            "new_user_permissions",
            "old_user_roles",
            &invalid_mapping,
        )
        .await
        .expect("Should handle empty column names gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Column names in mapping cannot be empty"));

    // Test empty new column name in mapping
    let mut invalid_mapping = HashMap::new();
    invalid_mapping.insert("user_id".to_string(), "".to_string()); // Empty new column
    invalid_mapping.insert("role_id".to_string(), "permission_id".to_string());

    let result = data_migration
        .execute_existing_junction_table_population(
            "new_user_permissions",
            "old_user_roles",
            &invalid_mapping,
        )
        .await
        .expect("Should handle empty column names gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Column names in mapping cannot be empty"));

    // Test duplicate target columns in mapping
    let mut duplicate_mapping = HashMap::new();
    duplicate_mapping.insert("user_id".to_string(), "person_id".to_string());
    duplicate_mapping.insert("role_id".to_string(), "person_id".to_string()); // Duplicate target

    let result = data_migration
        .execute_existing_junction_table_population(
            "new_user_permissions",
            "old_user_roles",
            &duplicate_mapping,
        )
        .await
        .expect("Should handle duplicate target columns gracefully");

    assert!(!result.success);
    assert!(result.errors[0]
        .to_string()
        .contains("Duplicate target column 'person_id'"));
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
        ],
    )
    .await
    .expect("Failed to insert test data");

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
    self_mapping.insert("user_id".to_string(), "user_id".to_string()); // Maps to itself - this should cause warning
    self_mapping.insert("role_id".to_string(), "permission_id".to_string()); // Valid mapping

    let result = data_migration
        .execute_existing_junction_table_population(
            "target_junction",
            "source_junction",
            &self_mapping,
        )
        .await
        .expect("Should handle self mapping gracefully");

    assert!(result.success);
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Column mapping maps 'user_id' to itself")));
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
    let result = data_migration
        .execute_existing_junction_table_population(
            "new_user_permissions",
            "non_existent_old_table", // Non-existent old table
            &column_mapping,
        )
        .await
        .expect("Should handle missing old table gracefully");

    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Old junction table non_existent_old_table not accessible")));

    // Test non-existent new junction table
    let result = data_migration
        .execute_existing_junction_table_population(
            "non_existent_new_table", // Non-existent new table
            "old_user_roles",
            &column_mapping,
        )
        .await
        .expect("Should handle missing new table gracefully");

    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("New junction table non_existent_new_table not accessible")));
}

#[tokio::test]
async fn test_execute_existing_junction_table_population_successful_operation() {
    let db = setup_test_db().await;
    create_test_tables_for_existing_junction_population(&db).await;

    let config = DataMigrationConfig {
        batch_size: 2, // Small batch size to test batch processing
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

    let result = data_migration
        .execute_existing_junction_table_population(
            "new_user_permissions",
            "old_user_roles",
            &column_mapping,
        )
        .await
        .expect("Existing junction table population should succeed");

    assert!(result.success);
    assert_eq!(result.records_processed, 5); // 5 records in old table
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());

    // Should have summary of copied records
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Copied 5 records from old_user_roles to new_user_permissions")));

    // Should have column mapping summary
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Applied column mappings:")));

    // Verify the records were copied correctly with column mapping
    let new_records_result = db.execute("SELECT person_id, permission_id, state FROM new_user_permissions ORDER BY person_id, permission_id", &[])
        .await.expect("Failed to query new junction table");

    assert_eq!(new_records_result.rows().len(), 5);

    // Check specific records were mapped correctly
    if let Value::Object(row) = &new_records_result.rows()[0] {
        if let (
            Some(Value::Number(person_id)),
            Some(Value::Number(permission_id)),
            Some(Value::String(state)),
        ) = (
            row.get("person_id"),
            row.get("permission_id"),
            row.get("state"),
        ) {
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

    let result = data_migration
        .execute_existing_junction_table_population(
            "empty_new_junction",
            "empty_old_junction",
            &column_mapping,
        )
        .await
        .expect("Should handle empty old table gracefully");

    assert!(result.success);
    assert_eq!(result.records_processed, 0);
    assert_eq!(result.records_failed, 0);
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("No records found in old junction table")));
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

    let result = data_migration
        .execute_existing_junction_table_population(
            "new_user_permissions",
            "old_user_roles",
            &partial_mapping,
        )
        .await
        .expect("Should handle partial column mapping gracefully");

    assert!(result.success);
    assert_eq!(result.records_processed, 5); // All records are processed
    assert_eq!(result.records_failed, 0);

    // Should have warnings about missing columns - but the implementation doesn't require all columns
    // Instead it only selects the columns specified in the mapping

    // Verify records were copied with only the mapped columns
    let new_records_result = db.execute("SELECT person_id, permission_id FROM new_user_permissions ORDER BY person_id, permission_id", &[])
        .await.expect("Failed to query new junction table");

    assert_eq!(new_records_result.rows().len(), 5);
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
                &[Value::Number(i.into()), Value::Number(j.into())],
            )
            .await
            .expect("Failed to insert batch test record");
        }
    }

    let config = DataMigrationConfig {
        batch_size: 3, // Very small batch size to test multiple batches
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

    let result = data_migration
        .execute_existing_junction_table_population(
            "batch_new_junction",
            "batch_old_junction",
            &column_mapping,
        )
        .await
        .expect("Batch processing should succeed");

    assert!(result.success);
    assert_eq!(result.records_processed, 30); // 10 * 3 = 30 records
    assert_eq!(result.records_failed, 0);
    assert!(result.errors.is_empty());

    // Verify all records were copied
    let count_result = db
        .execute("SELECT COUNT(*) as count FROM batch_new_junction", &[])
        .await
        .expect("Failed to count new records");

    if let Value::Object(row) = &count_result.rows()[0] {
        if let Some(Value::Number(count)) = row.get("count") {
            assert_eq!(count.as_u64().unwrap(), 30);
        }
    }
}
