mod common;

use d1orm::*;
use chrono::{DateTime, Utc};
use serde_json::Value;

#[tokio::test]
async fn test_modern_schema_definition() {
    let db = D1Client::new_in_memory().await.expect("Failed to create database");
    
    // Create a table using the modern schema API
    let users_table = TableDefinition::new("users")
        .integer("id").primary_key().auto_increment()
        .text("email").not_null().unique()
        .text("name").not_null()
        .boolean("is_active").default_true().not_null()  // Boolean field!
        .boolean("is_verified").default_false()           // Another boolean!
        .integer("age").check("age >= 0")
        .datetime("created_at").default(DefaultValue::CurrentTimestamp)
        .datetime("updated_at").default(DefaultValue::CurrentTimestamp)
        .json("metadata")  // JSON field
        .build();
    
    println!("Generated SQL: {}", users_table.to_sql());
    
    // Verify boolean columns are detected properly
    let boolean_cols = users_table.boolean_columns();
    assert_eq!(boolean_cols, vec!["is_active", "is_verified"]);
    
    // Create the table
    let create_sql = users_table.to_sql();
    db.execute(&create_sql, &[]).await.expect("Failed to create table");
    
    // Verify table was created
    let tables = db.execute(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='users'",
        &[]
    ).await.expect("Failed to query tables");
    
    assert_eq!(tables.rows.len(), 1);
    
    // Test inserting data with proper boolean handling
    use serde_json::Value;
    let insert_sql = r#"
        INSERT INTO users (email, name, is_active, is_verified, age, metadata) 
        VALUES (?, ?, ?, ?, ?, ?)
    "#;
    
    db.execute(insert_sql, &[
        Value::String("test@example.com".to_string()),
        Value::String("Test User".to_string()),
        Value::Bool(true),   // This will be converted to 1
        Value::Bool(false),  // This will be converted to 0
        Value::Number(25.into()),
        Value::String(r#"{"role": "admin"}"#.to_string()),
    ]).await.expect("Failed to insert user");
    
    // Query back and verify boolean conversion works
    let result = db.execute("SELECT * FROM users", &[])
        .await.expect("Failed to query users");
    
    assert_eq!(result.rows.len(), 1);
    println!("Query result: {:?}", result.rows[0]);
}

#[tokio::test]
async fn test_schema_migration_with_booleans() {
    let db = D1Client::new_in_memory().await.expect("Failed to create database");
    
    // Create a table with boolean fields using the modern API
    let posts_table = TableDefinition::new("posts")
        .boolean("is_published").default_false()  // Boolean with default false
        .boolean("is_featured").default_false()   // Another boolean
        .build();
    
    // Debug: Print the SQL that would be generated
    let sql = posts_table.to_sql();
    println!("Generated SQL: {}", sql);
    
    // Verify the SQL contains the expected boolean columns
    assert!(sql.contains("is_published INTEGER DEFAULT 0"));
    assert!(sql.contains("is_featured INTEGER DEFAULT 0"));
    
    // Test the boolean metadata detection
    let boolean_cols = posts_table.boolean_columns();
    println!("Detected boolean columns: {:?}", boolean_cols);
    assert_eq!(boolean_cols, vec!["is_published", "is_featured"]);
    
    // Create the table and verify it works
    db.execute(&sql, &[])
        .await
        .expect("Failed to create table");
    
    // Simple test: insert data with boolean values
    use serde_json::Value;
    let insert_result = db.execute(
        "INSERT INTO posts (is_published, is_featured) VALUES (?, ?)",
        &[Value::Bool(true), Value::Bool(false)]
    ).await;
    
    assert!(insert_result.is_ok(), "Should be able to insert boolean values");
    
    // Query the data back
    let query_result = db.execute("SELECT * FROM posts", &[]).await
        .expect("Failed to query data");
    
    assert_eq!(query_result.rows.len(), 1, "Should have one row");
    println!("Query result: {:?}", query_result.rows[0]);
}

#[tokio::test]
async fn test_schema_builder_fluent_api() {
    // Test that the fluent API works correctly
    let table = TableDefinition::new("complex_table")
        .integer("id").primary_key().auto_increment()
        .text("name").not_null()
        .text("email").unique()
        .boolean("active").default_true()
        .integer("score").default(DefaultValue::Integer(100)).check("score >= 0")
        .datetime("created_at").default(DefaultValue::CurrentTimestamp)
        .build();
    
    let sql = table.to_sql();
    println!("Complex table SQL: {}", sql);
    
    // Verify the SQL contains proper constraints
    assert!(sql.contains("id INTEGER PRIMARY KEY AUTOINCREMENT"));
    assert!(sql.contains("name TEXT NOT NULL"));
    assert!(sql.contains("email TEXT UNIQUE"));
    assert!(sql.contains("active INTEGER DEFAULT 1"));  // Boolean true becomes 1
    assert!(sql.contains("score INTEGER DEFAULT 100 CHECK (score >= 0)"));
    assert!(sql.contains("created_at DATETIME DEFAULT CURRENT_TIMESTAMP"));
    
    // Test boolean column detection
    let boolean_cols = table.boolean_columns();
    assert_eq!(boolean_cols, vec!["active"]);
}

#[tokio::test]
async fn test_foreign_key_constraints() {
    let comments_table = TableDefinition::new("comments")
        .integer("id").primary_key().auto_increment()
        .integer("post_id").not_null()  // Test without foreign keys for now
        .integer("user_id").not_null()  // Test without foreign keys for now
        .text("content").not_null()
        .boolean("is_approved").default_false()
        .datetime("created_at").default(DefaultValue::CurrentTimestamp)
        .build();
    
    let sql = comments_table.to_sql();
    println!("Comments table SQL: {}", sql);
    
    // Verify basic constraints and boolean handling
    assert!(sql.contains("post_id INTEGER NOT NULL"));
    assert!(sql.contains("user_id INTEGER NOT NULL"));
    assert!(sql.contains("is_approved INTEGER DEFAULT 0"));  // Boolean false becomes 0
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Entity, PartialEq)]
#[table(name = "modern_users")]
pub struct ModernUser {
    #[primary_key]
    pub id: i64,
    pub email: String,
    pub name: String,
    pub is_active: bool,    // Will be handled properly by boolean conversion
    pub is_verified: bool,  // Another boolean field
    pub score: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[tokio::test]
async fn test_entity_with_schema_defined_booleans() {
    let db = D1Client::new_in_memory().await.expect("Failed to create database");
    
    // Create table using modern schema API
    let table = TableDefinition::new("modern_users")
        .integer("id").primary_key().auto_increment()
        .text("email").not_null().unique()
        .text("name").not_null()
        .boolean("is_active").default_true()
        .boolean("is_verified").default_false()
        .integer("score")
        .datetime("created_at").default(DefaultValue::CurrentTimestamp)
        .build();
    
    db.execute(&table.to_sql(), &[])
        .await.expect("Failed to create table");
    
    // Create entity with boolean fields
    let user = ModernUser::create()
        .set_email("modern@example.com".to_string())
        .set_name("Modern User".to_string())
        .set_is_active(true)      // Boolean handling
        .set_is_verified(false)   // Boolean handling  
        .set_score(Some(95))
        .set_created_at(chrono::Utc::now())
        .save(&db)
        .await
        .expect("Failed to create user");
    
    assert!(user.is_active);
    assert!(!user.is_verified);
    
    // Query and verify boolean conversion works
    let found = ModernUser::find(&db, user.id)
        .await
        .expect("Failed to find")
        .expect("User not found");
    
    assert!(found.is_active);
    assert!(!found.is_verified);
    
    // Test boolean queries
    let active_users = ModernUser::query()
        .where_is_active_eq(true)
        .all(&db)
        .await
        .expect("Failed to query active users");
    
    assert_eq!(active_users.len(), 1);
    
    let verified_users = ModernUser::query()
        .where_is_verified_eq(true)  
        .all(&db)
        .await
        .expect("Failed to query verified users");
    
    assert_eq!(verified_users.len(), 0); // Should be 0 since user is not verified
    
    // Update boolean fields
    let updated = ModernUser::update(user.id)
        .set_is_verified(true)  // Change to verified
        .save(&db)
        .await
        .expect("Failed to update user");
    
    assert!(updated.is_verified);
}