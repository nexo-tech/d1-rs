mod common;

use common::*;
use d1orm::*;
use chrono::Utc;

#[tokio::test]
async fn test_create_and_find() {
    let db = setup_test_db().await;
    
    // Create a new user
    let user = TestUser::create()
        .set_email("test@example.com".to_string())
        .set_name("Test User".to_string())
        .set_is_active(true)
        .set_score(Some(75))
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create user");
    
    assert_eq!(user.email, "test@example.com");
    assert_eq!(user.name, "Test User");
    assert_eq!(user.is_active, true);
    assert_eq!(user.score, Some(75));
    
    // Find the user by ID
    let found_user = TestUser::find(&db, user.id)
        .await
        .expect("Failed to find user")
        .expect("User not found");
    
    assert_eq!(found_user.id, user.id);
    assert_eq!(found_user.email, user.email);
    assert_eq!(found_user.is_active, true);
}

#[tokio::test]
async fn test_update() {
    let db = setup_test_db().await;
    seed_test_data(&db).await;
    
    // Find Alice
    let alice = TestUser::query()
        .where_email_eq("alice@example.com".to_string())
        .first(&db)
        .await
        .expect("Failed to query")
        .expect("Alice not found");
    
    // Update Alice's score
    let updated_alice = TestUser::update(alice.id)
        .set_score(Some(200))
        .set_is_active(false)
        .save(&db)
        .await
        .expect("Failed to update");
    
    assert_eq!(updated_alice.score, Some(200));
    assert_eq!(updated_alice.is_active, false);
    
    // Verify the update persisted
    let found_alice = TestUser::find(&db, alice.id)
        .await
        .expect("Failed to find")
        .expect("Alice not found");
    
    assert_eq!(found_alice.score, Some(200));
    assert_eq!(found_alice.is_active, false);
}

#[tokio::test]
async fn test_delete() {
    let db = setup_test_db().await;
    
    // Create a user
    let user = TestUser::create()
        .set_email("delete_me@example.com".to_string())
        .set_name("Delete Me".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create user");
    
    let user_id = user.id;
    
    // Delete the user
    TestUser::delete(&db, user_id)
        .await
        .expect("Failed to delete user");
    
    // Verify the user is deleted
    let found = TestUser::find(&db, user_id)
        .await
        .expect("Failed to query");
    
    assert!(found.is_none(), "User should have been deleted");
}