use d1orm::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

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

pub async fn setup_test_db() -> D1Client {
    let db = D1Client::new_in_memory().await.expect("Failed to create in-memory database");
    
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
    
    db.execute(create_users, &[]).await.expect("Failed to create users table");
    db.execute(create_posts, &[]).await.expect("Failed to create posts table");
    
    db
}

pub async fn seed_test_data(db: &D1Client) {
    use serde_json::Value;
    
    // Insert test users
    let insert_user = "INSERT INTO test_users (email, name, is_active, score) VALUES (?, ?, ?, ?)";
    
    db.execute(insert_user, &[
        Value::String("alice@example.com".to_string()),
        Value::String("Alice".to_string()),
        Value::Bool(true),
        Value::Number(100.into()),
    ]).await.expect("Failed to insert alice");
    
    db.execute(insert_user, &[
        Value::String("bob@example.com".to_string()),
        Value::String("Bob".to_string()),
        Value::Bool(false),
        Value::Number(50.into()),
    ]).await.expect("Failed to insert bob");
    
    db.execute(insert_user, &[
        Value::String("charlie@example.com".to_string()),
        Value::String("Charlie".to_string()),
        Value::Bool(true),
        Value::Null,
    ]).await.expect("Failed to insert charlie");
    
    // Insert test posts
    let insert_post = "INSERT INTO test_posts (user_id, title, content, is_published, views) VALUES (?, ?, ?, ?, ?)";
    
    db.execute(insert_post, &[
        Value::Number(1.into()),
        Value::String("First Post".to_string()),
        Value::String("Content of first post".to_string()),
        Value::Bool(true),
        Value::Number(100.into()),
    ]).await.expect("Failed to insert first post");
    
    db.execute(insert_post, &[
        Value::Number(1.into()),
        Value::String("Second Post".to_string()),
        Value::String("Content of second post".to_string()),
        Value::Bool(false),
        Value::Number(0.into()),
    ]).await.expect("Failed to insert second post");
    
    db.execute(insert_post, &[
        Value::Number(2.into()),
        Value::String("Bob's Post".to_string()),
        Value::String("Bob's content".to_string()),
        Value::Bool(true),
        Value::Number(50.into()),
    ]).await.expect("Failed to insert Bob's post");
}