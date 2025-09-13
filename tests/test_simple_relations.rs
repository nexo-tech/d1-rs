/// Simple test to verify our type-safe relations API works
use d1_rs::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct User {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct Post {
    #[primary_key]
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

// Our ONLY relations API - fully type-safe!
relations! {
    User {
        has_many posts: Post via user_id,
    }
    
    Post {
        belongs_to user: User via user_id,
    }
}

async fn setup_simple_db() -> D1Client {
    let db = D1Client::new_in_memory().await.expect("Failed to create database");
    
    // Simple manual table creation
    db.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT, created_at TEXT)", &[]).await.expect("Failed to create users table");
    db.execute("CREATE TABLE posts (id INTEGER PRIMARY KEY, user_id INTEGER, title TEXT, content TEXT, created_at TEXT)", &[]).await.expect("Failed to create posts table");
    
    db
}

#[tokio::test]
async fn test_type_safe_relations_api() {
    let db = setup_simple_db().await;
    
    // Create a user manually
    let now_str = "2024-01-01T00:00:00Z";
    db.execute("INSERT INTO users (id, name, email, created_at) VALUES (1, 'Test User', 'test@example.com', ?)", &[serde_json::Value::String(now_str.to_string())]).await.expect("Failed to create user");
    
    // Create posts manually
    db.execute("INSERT INTO posts (id, user_id, title, content, created_at) VALUES (1, 1, 'First Post', 'Content 1', ?)", &[serde_json::Value::String(now_str.to_string())]).await.expect("Failed to create post 1");
    db.execute("INSERT INTO posts (id, user_id, title, content, created_at) VALUES (2, 1, 'Second Post', 'Content 2', ?)", &[serde_json::Value::String(now_str.to_string())]).await.expect("Failed to create post 2");
    
    // Get the user
    let users = User::query().all(&db).await.expect("Failed to get users");
    assert_eq!(users.len(), 1);
    let user = &users[0];
    
    // TEST: Type-safe association API - NO STRING LITERALS!
    let user_posts = user.posts().all(&db).await.expect("Failed to get user posts");
    assert_eq!(user_posts.len(), 2);
    assert_eq!(user_posts[0].title, "First Post");
    assert_eq!(user_posts[1].title, "Second Post");
    
    // TEST: Count posts
    let post_count = user.posts().count(&db).await.expect("Failed to count posts");
    assert_eq!(post_count, 2);
    
    // TEST: Get first post
    let first_post = user.posts().first(&db).await.expect("Failed to get first post");
    assert!(first_post.is_some());
    assert_eq!(first_post.unwrap().title, "First Post");
    
    println!("✅ Type-safe relations API working perfectly!");
}