mod common;

use common::*;
use d1orm::*;
use serde_json::Value;

#[tokio::test]
async fn test_boolean_to_integer_conversion_on_insert() {
    let db = setup_test_db().await;
    
    // Insert with boolean values - ORM should convert to integers for SQLite
    let sql = "INSERT INTO test_users (email, name, is_active) VALUES (?, ?, ?)";
    let result = db.execute(sql, &[
        Value::String("bool_test@example.com".to_string()),
        Value::String("Bool Test".to_string()),
        Value::Bool(true),  // This should be converted to 1
    ]).await;
    
    assert!(result.is_ok(), "Failed to insert with boolean: {:?}", result);
    
    // Debug: check if user was inserted at all
    let raw_result = db.execute("SELECT * FROM test_users WHERE email = ?", &[
        Value::String("bool_test@example.com".to_string())
    ]).await.expect("Failed to query raw");
    
    println!("Raw query result: {:?}", raw_result.rows);
    assert!(!raw_result.rows.is_empty(), "No user was inserted!");
    
    // Debug: Check TestUser boolean metadata  
    println!("TestUser boolean fields: {:?}", TestUser::boolean_fields());
    
    // Verify the data was inserted
    let user_result = TestUser::query()
        .where_email_eq("bool_test@example.com".to_string())
        .first(&db)
        .await;
        
    println!("Query result: {:?}", user_result);
    
    let user = user_result
        .expect("Failed to query")
        .expect("User not found");
    
    assert_eq!(user.is_active, true);
}

#[tokio::test]
async fn test_integer_to_boolean_conversion_on_read() {
    let db = setup_test_db().await;
    
    // Insert directly with integers (as SQLite stores them)
    let sql = "INSERT INTO test_users (email, name, is_active) VALUES (?, ?, ?)";
    db.execute(sql, &[
        Value::String("int_test@example.com".to_string()),
        Value::String("Int Test".to_string()),
        Value::Number(0.into()),  // SQLite stores false as 0
    ]).await.expect("Failed to insert");
    
    // Query should convert integer back to boolean
    let user = TestUser::query()
        .where_email_eq("int_test@example.com".to_string())
        .first(&db)
        .await
        .expect("Failed to query")
        .expect("User not found");
    
    assert_eq!(user.is_active, false);
}

#[tokio::test]
async fn test_boolean_conversion_with_entity_methods() {
    let db = setup_test_db().await;
    
    // Create users with different boolean values
    let active_user = TestUser::create()
        .set_email("active@example.com".to_string())
        .set_name("Active User".to_string())
        .set_is_active(true)
        .set_created_at(chrono::Utc::now())
        .save(&db)
        .await
        .expect("Failed to create active user");
    
    let inactive_user = TestUser::create()
        .set_email("inactive@example.com".to_string())
        .set_name("Inactive User".to_string())
        .set_is_active(false)
        .set_created_at(chrono::Utc::now())
        .save(&db)
        .await
        .expect("Failed to create inactive user");
    
    assert!(active_user.is_active);
    assert!(!inactive_user.is_active);
    
    // Find and verify boolean values are preserved
    let found_active = TestUser::find(&db, active_user.id)
        .await
        .expect("Failed to find")
        .expect("Active user not found");
    
    let found_inactive = TestUser::find(&db, inactive_user.id)
        .await
        .expect("Failed to find")
        .expect("Inactive user not found");
    
    assert!(found_active.is_active);
    assert!(!found_inactive.is_active);
}

#[tokio::test]
async fn test_boolean_conversion_in_queries() {
    let db = setup_test_db().await;
    seed_test_data(&db).await;
    
    // Query with boolean condition
    let active_users = TestUser::query()
        .where_is_active_eq(true)
        .all(&db)
        .await
        .expect("Failed to query active users");
    
    let inactive_users = TestUser::query()
        .where_is_active_eq(false)
        .all(&db)
        .await
        .expect("Failed to query inactive users");
    
    // Verify counts
    assert_eq!(active_users.len(), 2);
    assert_eq!(inactive_users.len(), 1);
    
    // Verify all active users have is_active = true
    for user in &active_users {
        assert!(user.is_active);
    }
    
    // Verify all inactive users have is_active = false
    for user in &inactive_users {
        assert!(!user.is_active);
    }
}

#[tokio::test]
async fn test_boolean_conversion_in_updates() {
    let db = setup_test_db().await;
    seed_test_data(&db).await;
    
    // Find an active user
    let user = TestUser::query()
        .where_is_active_eq(true)
        .first(&db)
        .await
        .expect("Failed to query")
        .expect("No active user found");
    
    // Update to inactive
    let updated = TestUser::update(user.id)
        .set_is_active(false)
        .save(&db)
        .await
        .expect("Failed to update");
    
    assert!(!updated.is_active);
    
    // Verify the update persisted
    let found = TestUser::find(&db, user.id)
        .await
        .expect("Failed to find")
        .expect("User not found");
    
    assert!(!found.is_active);
}

#[tokio::test]
async fn test_multiple_boolean_fields() {
    let db = setup_test_db().await;
    seed_test_data(&db).await;
    
    // Create a post with boolean field
    let post = TestPost::create()
        .set_user_id(1)
        .set_title("Test Post".to_string())
        .set_content("Test content".to_string())
        .set_is_published(true)
        .set_views(0)
        .set_created_at(chrono::Utc::now())
        .save(&db)
        .await
        .expect("Failed to create post");
    
    assert!(post.is_published);
    
    // Find and verify
    let found_post = TestPost::find(&db, post.id)
        .await
        .expect("Failed to find")
        .expect("Post not found");
    
    assert!(found_post.is_published);
    
    // Update to unpublished
    let updated_post = TestPost::update(post.id)
        .set_is_published(false)
        .save(&db)
        .await
        .expect("Failed to update");
    
    assert!(!updated_post.is_published);
}