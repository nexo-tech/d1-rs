use d1_rs::auto_migration::SchemaIntrospector;
use d1_rs::*;

mod common;
use common::query_helpers::{
    build_create_table_query_with_columns, build_create_index_query, build_create_unique_index_query,
    build_pragma_query, build_add_foreign_key_query, build_create_table_query_with_composite_pk,
    build_create_table_query_with_foreign_keys
};

#[tokio::test]
async fn test_pragma_compatibility_indexes() {
    // Create a test database with multiple indexes
    let db = D1Client::new_in_memory().await.unwrap();
    
    // Create table using database-agnostic helper
    let (sql, params) = build_create_table_query_with_columns(
        "pragma_index_test",
        vec![
            ("id", "INTEGER", true, Some("AUTOINCREMENT"), false),
            ("email", "TEXT", false, None, false), // UNIQUE constraint handled separately
            ("name", "TEXT", false, None, false),
            ("age", "INTEGER", false, None, true),
            ("is_active", "INTEGER", false, Some("1"), true)
        ],
        db.dialect()
    );
    db.execute(&sql, &params).await.unwrap();
    
    // Create regular index using database-agnostic helper
    let (index_sql, index_params) = build_create_index_query(
        "idx_name_age",
        "pragma_index_test",
        vec!["name", "age"],
        false,
        db.dialect()
    );
    db.execute(&index_sql, &index_params).await.unwrap();
    
    // Create unique index using database-agnostic helper
    let (unique_index_sql, unique_index_params) = build_create_unique_index_query(
        "idx_email_unique",
        "pragma_index_test",
        vec!["email"],
        db.dialect()
    );
    db.execute(&unique_index_sql, &unique_index_params).await.unwrap();
    
    // Test pragma_index_list function
    let introspector = SchemaIntrospector::new(&db);
    
    match introspector.introspect_indexes("pragma_index_test").await {
        Ok(indexes) => {
            println!("Indexes found: {:?}", indexes);
            
            // Should find our custom indexes (not auto-generated ones)
            let name_age_index = indexes.iter().find(|i| i.name == "idx_name_age");
            assert!(name_age_index.is_some(), "Should find idx_name_age index");
            
            let name_age_index = name_age_index.unwrap();
            assert_eq!(name_age_index.columns, vec!["name", "age"]);
            assert!(!name_age_index.unique);
            
            // Check unique index
            let email_index = indexes.iter().find(|i| i.name == "idx_email_unique");
            if let Some(email_index) = email_index {
                assert_eq!(email_index.columns, vec!["email"]);
                assert!(email_index.unique);
            }
        },
        Err(e) => {
            panic!("PRAGMA index introspection failed: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_pragma_compatibility_foreign_keys() {
    // Create a test database with foreign keys
    let db = D1Client::new_in_memory().await.unwrap();
    
    // Enable foreign keys using database-agnostic helper
    let (pragma_sql, pragma_params) = build_pragma_query("foreign_keys", "ON", db.dialect());
    db.execute(&pragma_sql, &pragma_params).await.unwrap();
    
    // Create parent table using database-agnostic helper
    let (users_sql, users_params) = build_create_table_query_with_columns(
        "pragma_users",
        vec![
            ("id", "INTEGER", true, Some("AUTOINCREMENT"), false),
            ("name", "TEXT", false, None, false)
        ],
        db.dialect()
    );
    db.execute(&users_sql, &users_params).await.unwrap();
    
    // Create child table with foreign keys included in CREATE TABLE statement
    let (posts_sql, posts_params) = build_create_table_query_with_foreign_keys(
        "pragma_posts",
        vec![
            ("id", "INTEGER", true, Some("AUTOINCREMENT"), false),
            ("title", "TEXT", false, None, false),
            ("user_id", "INTEGER", false, None, false),
            ("category_id", "INTEGER", false, None, true)
        ],
        vec![
            ("user_id", "pragma_users", Some("CASCADE"), Some("SET NULL")),
            ("category_id", "pragma_users", Some("SET NULL"), None)
        ],
        db.dialect()
    );
    db.execute(&posts_sql, &posts_params).await.unwrap();
    
    // Test pragma_foreign_key_list function
    let introspector = SchemaIntrospector::new(&db);
    
    match introspector.introspect_foreign_keys("pragma_posts").await {
        Ok(foreign_keys) => {
            println!("Foreign keys found: {:?}", foreign_keys);
            assert_eq!(foreign_keys.len(), 2);
            
            // Find user_id foreign key
            let user_fk = foreign_keys.iter().find(|fk| fk.columns.contains(&"user_id".to_string()));
            assert!(user_fk.is_some(), "Should find user_id foreign key");
            
            let user_fk = user_fk.unwrap();
            assert_eq!(user_fk.referenced_table, "pragma_users");
            assert_eq!(user_fk.referenced_columns, vec!["id"]);
            assert_eq!(user_fk.on_delete, Some("CASCADE".to_string()));
            assert_eq!(user_fk.on_update, Some("SET NULL".to_string()));
            
            // Find category_id foreign key
            let category_fk = foreign_keys.iter().find(|fk| fk.columns.contains(&"category_id".to_string()));
            assert!(category_fk.is_some(), "Should find category_id foreign key");
            
            let category_fk = category_fk.unwrap();
            assert_eq!(category_fk.referenced_table, "pragma_users");
            assert_eq!(category_fk.on_delete, Some("SET NULL".to_string()));
        },
        Err(e) => {
            panic!("PRAGMA foreign key introspection failed: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_pragma_table_info_comprehensive() {
    // Test all column types and constraints with pragma_table_info
    let db = D1Client::new_in_memory().await.unwrap();
    
    let (sql, params) = build_create_table_query_with_columns(
        "pragma_comprehensive",
        vec![
            ("id", "INTEGER", true, Some("AUTOINCREMENT"), false),
            ("name", "TEXT", false, None, false),
            ("email", "TEXT", false, None, true),
            ("age", "INTEGER", false, Some("25"), true),
            ("salary", "REAL", false, None, true),
            ("bio", "BLOB", false, None, true),
            ("is_active", "BOOLEAN", false, Some("1"), true),
            ("has_premium", "BOOLEAN", false, None, true),
            ("created_at", "DATETIME", false, Some("CURRENT_TIMESTAMP"), true),
            ("optional_field", "TEXT", false, None, true)
        ],
        db.dialect()
    );
    db.execute(&sql, &params).await.unwrap();
    
    // Add UNIQUE constraint for email
    let (email_unique_sql, email_unique_params) = build_create_unique_index_query(
        "idx_pragma_comprehensive_email_unique",
        "pragma_comprehensive",
        vec!["email"],
        db.dialect()
    );
    db.execute(&email_unique_sql, &email_unique_params).await.unwrap();
    
    let introspector = SchemaIntrospector::new(&db);
    
    match introspector.introspect_columns("pragma_comprehensive").await {
        Ok(columns) => {
            println!("Comprehensive columns: {:#?}", columns);
            assert_eq!(columns.len(), 10);
            
            // Test INTEGER PRIMARY KEY AUTOINCREMENT
            let id_col = columns.iter().find(|c| c.name == "id").unwrap();
            assert!(id_col.primary_key);
            assert!(id_col.auto_increment);
            assert_eq!(id_col.column_type, "INTEGER");
            
            // Test NOT NULL constraint
            let name_col = columns.iter().find(|c| c.name == "name").unwrap();
            assert!(!name_col.nullable);
            assert_eq!(name_col.column_type, "TEXT");
            
            // Test DEFAULT values
            let age_col = columns.iter().find(|c| c.name == "age").unwrap();
            assert_eq!(age_col.default_value, Some("25".to_string()));
            assert_eq!(age_col.column_type, "INTEGER");
            
            // Test REAL type
            let salary_col = columns.iter().find(|c| c.name == "salary").unwrap();
            assert_eq!(salary_col.column_type, "REAL");
            
            // Test BLOB type
            let bio_col = columns.iter().find(|c| c.name == "bio").unwrap();
            assert_eq!(bio_col.column_type, "BLOB");
            
            // Test boolean detection
            let is_active_col = columns.iter().find(|c| c.name == "is_active").unwrap();
            assert_eq!(is_active_col.column_type, "BOOLEAN");
            assert_eq!(is_active_col.default_value, Some("1".to_string()));
            
            let has_premium_col = columns.iter().find(|c| c.name == "has_premium").unwrap();
            assert_eq!(has_premium_col.column_type, "BOOLEAN");
            
            // Test DATETIME with default
            let created_at_col = columns.iter().find(|c| c.name == "created_at").unwrap();
            assert_eq!(created_at_col.column_type, "DATETIME");
            assert_eq!(created_at_col.default_value, Some("CURRENT_TIMESTAMP".to_string()));
            
            // Test nullable field
            let optional_col = columns.iter().find(|c| c.name == "optional_field").unwrap();
            assert!(optional_col.nullable);
            assert_eq!(optional_col.column_type, "TEXT");
        },
        Err(e) => {
            panic!("PRAGMA table_info comprehensive test failed: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_complete_database_introspection() {
    // Test full database introspection with all features
    let db = D1Client::new_in_memory().await.unwrap();
    
    // Enable foreign keys using database-agnostic helper
    let (pragma_sql, pragma_params) = build_pragma_query("foreign_keys", "ON", db.dialect());
    db.execute(&pragma_sql, &pragma_params).await.unwrap();
    
    // Create complete schema using database-agnostic helpers
    let (users_sql, users_params) = build_create_table_query_with_columns(
        "introspect_users",
        vec![
            ("id", "INTEGER", true, Some("AUTOINCREMENT"), false),
            ("username", "TEXT", false, None, false),
            ("email", "TEXT", false, None, false),
            ("is_admin", "BOOLEAN", false, Some("0"), true),
            ("created_at", "DATETIME", false, Some("CURRENT_TIMESTAMP"), true)
        ],
        db.dialect()
    );
    db.execute(&users_sql, &users_params).await.unwrap();
    
    // Add UNIQUE constraint for username
    let (username_unique_sql, username_unique_params) = build_create_unique_index_query(
        "idx_introspect_users_username_unique",
        "introspect_users",
        vec!["username"],
        db.dialect()
    );
    db.execute(&username_unique_sql, &username_unique_params).await.unwrap();
    
    let (posts_sql, posts_params) = build_create_table_query_with_columns(
        "introspect_posts",
        vec![
            ("id", "INTEGER", true, Some("AUTOINCREMENT"), false),
            ("title", "TEXT", false, None, false),
            ("content", "TEXT", false, None, true),
            ("user_id", "INTEGER", false, None, false),
            ("is_published", "BOOLEAN", false, Some("0"), true),
            ("created_at", "DATETIME", false, Some("CURRENT_TIMESTAMP"), true)
        ],
        db.dialect()
    );
    db.execute(&posts_sql, &posts_params).await.unwrap();
    
    // Add foreign key constraint from posts to users
    let (fk_posts_sql, fk_posts_params) = build_add_foreign_key_query(
        "introspect_posts",
        "fk_introspect_posts_user_id",
        vec!["user_id"],
        "introspect_users",
        vec!["id"],
        Some("CASCADE"),
        None,
        db.dialect()
    );
    db.execute(&fk_posts_sql, &fk_posts_params).await.unwrap();
    
    let (tags_sql, tags_params) = build_create_table_query_with_columns(
        "introspect_tags",
        vec![
            ("id", "INTEGER", true, Some("AUTOINCREMENT"), false),
            ("name", "TEXT", false, None, false),
            ("color", "TEXT", false, Some("#ffffff"), true)
        ],
        db.dialect()
    );
    db.execute(&tags_sql, &tags_params).await.unwrap();
    
    // Add UNIQUE constraint for name (this is already being created as idx_tags_name below)
    
    // Create junction table for many-to-many using database-agnostic helper with composite primary key
    let (post_tags_sql, post_tags_params) = build_create_table_query_with_composite_pk(
        "introspect_post_tags",
        vec![
            ("post_id", "INTEGER", None, false),
            ("tag_id", "INTEGER", None, false),
            ("created_at", "DATETIME", Some("CURRENT_TIMESTAMP"), true)
        ],
        vec!["post_id", "tag_id"],
        db.dialect()
    );
    db.execute(&post_tags_sql, &post_tags_params).await.unwrap();
    
    // Add foreign key constraints for junction table
    let (fk3_sql, fk3_params) = build_add_foreign_key_query(
        "introspect_post_tags",
        "fk_post_tags_post_id",
        vec!["post_id"],
        "introspect_posts",
        vec!["id"],
        Some("CASCADE"),
        None,
        db.dialect()
    );
    db.execute(&fk3_sql, &fk3_params).await.unwrap();
    
    let (fk4_sql, fk4_params) = build_add_foreign_key_query(
        "introspect_post_tags",
        "fk_post_tags_tag_id", 
        vec!["tag_id"],
        "introspect_tags",
        vec!["id"],
        Some("CASCADE"),
        None,
        db.dialect()
    );
    db.execute(&fk4_sql, &fk4_params).await.unwrap();
    
    // Create indexes using database-agnostic helpers
    let (idx1_sql, idx1_params) = build_create_index_query(
        "idx_posts_user_published",
        "introspect_posts",
        vec!["user_id", "is_published"],
        false,
        db.dialect()
    );
    db.execute(&idx1_sql, &idx1_params).await.unwrap();
    
    let (idx2_sql, idx2_params) = build_create_unique_index_query(
        "idx_tags_name",
        "introspect_tags",
        vec!["name"],
        db.dialect()
    );
    db.execute(&idx2_sql, &idx2_params).await.unwrap();
    
    // Test complete database introspection
    let introspector = SchemaIntrospector::new(&db);
    
    match introspector.introspect_database().await {
        Ok(schema) => {
            println!("Complete database schema: {:#?}", schema);
            
            // Verify all tables are found
            assert_eq!(schema.tables.len(), 4);
            assert!(schema.get_table("introspect_users").is_some());
            assert!(schema.get_table("introspect_posts").is_some());
            assert!(schema.get_table("introspect_tags").is_some());
            assert!(schema.get_table("introspect_post_tags").is_some());
            
            // Verify users table structure
            let users_table = schema.get_table("introspect_users").unwrap();
            assert_eq!(users_table.columns.len(), 5);
            assert!(users_table.get_column("is_admin").unwrap().column_type == "BOOLEAN");
            
            // Verify posts table with foreign key
            let posts_table = schema.get_table("introspect_posts").unwrap();
            assert_eq!(posts_table.columns.len(), 6);
            assert_eq!(posts_table.foreign_keys.len(), 1);
            assert!(!posts_table.indexes.is_empty());
            
            // Verify junction table with composite primary key
            let post_tags_table = schema.get_table("introspect_post_tags").unwrap();
            assert_eq!(post_tags_table.columns.len(), 3);
            assert_eq!(post_tags_table.foreign_keys.len(), 2);
            
            // Count primary key columns in junction table
            let pk_columns = post_tags_table.primary_key_columns();
            assert_eq!(pk_columns.len(), 2);
            
            println!("✅ Complete database introspection successful!");
        },
        Err(e) => {
            panic!("Complete database introspection failed: {:?}", e);
        }
    }
}