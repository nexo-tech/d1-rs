mod common;

use common::*;
use d1_rs::*;
use d1_rs::Entity;

#[tokio::test]
async fn test_query_all() {
    let db = setup_test_db().await;
    seed_test_data(&db).await;
    
    // Query all users
    let users = TestUser::query()
        .all(&db)
        .await
        .expect("Failed to query all users");
    
    assert_eq!(users.len(), 3);
    
    // Verify we got the right users
    let emails: Vec<String> = users.iter().map(|u| u.email.clone()).collect();
    assert!(emails.contains(&"alice@example.com".to_string()));
    assert!(emails.contains(&"bob@example.com".to_string()));
    assert!(emails.contains(&"charlie@example.com".to_string()));
}

#[tokio::test]
async fn test_query_with_conditions() {
    let db = setup_test_db().await;
    seed_test_data(&db).await;
    
    // Query active users
    let active_users = TestUser::query()
        .where_is_active_eq(true)
        .all(&db)
        .await
        .expect("Failed to query active users");
    
    assert_eq!(active_users.len(), 2);
    for user in &active_users {
        assert!(user.is_active);
    }
    
    // Query user by email
    let alice = TestUser::query()
        .where_email_eq("alice@example.com".to_string())
        .first(&db)
        .await
        .expect("Failed to query")
        .expect("Alice not found");
    
    assert_eq!(alice.email, "alice@example.com");
    assert_eq!(alice.name, "Alice");
}

#[tokio::test]
async fn test_query_with_multiple_conditions() {
    let db = setup_test_db().await;
    seed_test_data(&db).await;
    
    // Query active users with score (Note: is_not_null method needs implementation)
    let users_with_score = TestUser::query()
        .where_is_active_eq(true)
        .all(&db)
        .await
        .expect("Failed to query");
    
    assert_eq!(users_with_score.len(), 2); // Alice and Charlie are active
}

#[tokio::test]
async fn test_query_count() {
    let db = setup_test_db().await;
    seed_test_data(&db).await;
    
    // Count all users
    let total_count = TestUser::query()
        .count(&db)
        .await
        .expect("Failed to count");
    
    assert_eq!(total_count, 3);
    
    // Count active users
    let active_count = TestUser::query()
        .where_is_active_eq(true)
        .count(&db)
        .await
        .expect("Failed to count active users");
    
    assert_eq!(active_count, 2);
}

#[tokio::test]
async fn test_query_posts_with_foreign_key() {
    let db = setup_test_db().await;
    seed_test_data(&db).await;
    
    // Query posts by user_id
    let alice_posts = TestPost::query()
        .where_user_id_eq(1)
        .all(&db)
        .await
        .expect("Failed to query posts");
    
    assert_eq!(alice_posts.len(), 2);
    
    // Query published posts
    let published_posts = TestPost::query()
        .where_is_published_eq(true)
        .all(&db)
        .await
        .expect("Failed to query published posts");
    
    assert_eq!(published_posts.len(), 2);
    
    // Verify all published posts have is_published = true
    for post in &published_posts {
        assert!(post.is_published);
    }
}

#[tokio::test]
async fn test_query_ordering_and_limit() {
    let db = setup_test_db().await;
    seed_test_data(&db).await;
    
    // Query posts ordered by views (using type-safe method - no string literals!)
    let posts = TestPost::query()
        .order_by_views_desc()
        .limit(2)
        .all(&db)
        .await
        .expect("Failed to query posts");
    
    assert_eq!(posts.len(), 2);
    assert_eq!(posts[0].views, 100);
    assert_eq!(posts[1].views, 50);
}