use d1_rs::auto_migration::SchemaIntrospector;
use d1_rs::*;

#[tokio::test]
async fn test_pragma_compatibility_indexes() {
    // Create a test database with multiple indexes
    let db = D1Client::new_in_memory().await.unwrap();
    
    // Create table
    let sql = r#"
        CREATE TABLE pragma_index_test (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            email TEXT UNIQUE NOT NULL,
            name TEXT NOT NULL,
            age INTEGER,
            is_active INTEGER DEFAULT 1
        )
    "#;
    db.execute(sql, &[]).await.unwrap();
    
    // Create regular index
    let index_sql = "CREATE INDEX idx_name_age ON pragma_index_test(name, age)";
    db.execute(index_sql, &[]).await.unwrap();
    
    // Create unique index
    let unique_index_sql = "CREATE UNIQUE INDEX idx_email_unique ON pragma_index_test(email)";
    db.execute(unique_index_sql, &[]).await.unwrap();
    
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
    
    // Enable foreign keys
    db.execute("PRAGMA foreign_keys = ON", &[]).await.unwrap();
    
    // Create parent table
    let users_sql = r#"
        CREATE TABLE pragma_users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL
        )
    "#;
    db.execute(users_sql, &[]).await.unwrap();
    
    // Create child table with foreign key
    let posts_sql = r#"
        CREATE TABLE pragma_posts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            user_id INTEGER NOT NULL,
            category_id INTEGER,
            FOREIGN KEY (user_id) REFERENCES pragma_users(id) ON DELETE CASCADE ON UPDATE SET NULL,
            FOREIGN KEY (category_id) REFERENCES pragma_users(id) ON DELETE SET NULL
        )
    "#;
    db.execute(posts_sql, &[]).await.unwrap();
    
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
    
    let sql = r#"
        CREATE TABLE pragma_comprehensive (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            email TEXT UNIQUE,
            age INTEGER DEFAULT 25,
            salary REAL,
            bio BLOB,
            is_active BOOLEAN DEFAULT 1,
            has_premium BOOLEAN,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            optional_field TEXT
        )
    "#;
    db.execute(sql, &[]).await.unwrap();
    
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
    
    // Enable foreign keys
    db.execute("PRAGMA foreign_keys = ON", &[]).await.unwrap();
    
    // Create complete schema
    let users_sql = r#"
        CREATE TABLE introspect_users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT UNIQUE NOT NULL,
            email TEXT NOT NULL,
            is_admin BOOLEAN DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )
    "#;
    db.execute(users_sql, &[]).await.unwrap();
    
    let posts_sql = r#"
        CREATE TABLE introspect_posts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            content TEXT,
            user_id INTEGER NOT NULL,
            is_published BOOLEAN DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES introspect_users(id) ON DELETE CASCADE
        )
    "#;
    db.execute(posts_sql, &[]).await.unwrap();
    
    let tags_sql = r#"
        CREATE TABLE introspect_tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT UNIQUE NOT NULL,
            color TEXT DEFAULT '#ffffff'
        )
    "#;
    db.execute(tags_sql, &[]).await.unwrap();
    
    // Create junction table for many-to-many
    let post_tags_sql = r#"
        CREATE TABLE introspect_post_tags (
            post_id INTEGER NOT NULL,
            tag_id INTEGER NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (post_id, tag_id),
            FOREIGN KEY (post_id) REFERENCES introspect_posts(id) ON DELETE CASCADE,
            FOREIGN KEY (tag_id) REFERENCES introspect_tags(id) ON DELETE CASCADE
        )
    "#;
    db.execute(post_tags_sql, &[]).await.unwrap();
    
    // Create indexes
    db.execute("CREATE INDEX idx_posts_user_published ON introspect_posts(user_id, is_published)", &[]).await.unwrap();
    db.execute("CREATE UNIQUE INDEX idx_tags_name ON introspect_tags(name)", &[]).await.unwrap();
    
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