/// Tests for Ent-Go style relation filtering (Has/HasWith predicates)
use chrono::{DateTime, Utc};
use d1_rs::*;
use d1_rs::schema_evolution::SchemaMigration;
use d1_rs::edges::Predicate;
use serde::{Deserialize, Serialize};

// Test entities
#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct User {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub email: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct Post {
    #[primary_key]
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub content: String,
    pub is_published: bool,
    pub view_count: i64,
    pub created_at: DateTime<Utc>,
}

// Relations definitions
relations! {
    User {
        has_many posts: Post via user_id,
    }

    Post {
        belongs_to user: User via user_id,
    }
}

async fn setup_relation_filtering_db() -> D1Client {
    let db = D1Client::new_in_memory()
        .await
        .expect("Failed to create database");

    // Create tables
    let migration = SchemaMigration::new("create_relation_filtering_schema".to_string())
        .create_table("users")
        .integer("id").primary_key().auto_increment().build()
        .text("email").not_null().unique().build()
        .text("name").not_null().build()
        .boolean("is_active").default_value(DefaultValue::Boolean(true)).build()
        .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
        .build()
        .create_table("posts")
        .integer("id").primary_key().auto_increment().build()
        .integer("user_id").not_null().build()
        .text("title").not_null().build()
        .text("content").not_null().build()
        .boolean("is_published").default_value(DefaultValue::Boolean(false)).build()
        .integer("view_count").default_value(DefaultValue::Integer(0)).build()
        .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
        .build()
        .auto_generate_for::<User>()
        .auto_generate_for::<Post>();

    migration
        .execute(&db)
        .await
        .expect("Failed to run migration");

    db
}

#[tokio::test]
async fn test_relation_predicate_construction() {
    // Test that we can construct relation predicates using the enhanced Predicate API
    
    // Simple relation existence check
    let has_posts = Predicate::has("posts");
    match has_posts {
        Predicate::Relation(ref rel_pred) => {
            assert_eq!(rel_pred.relation, "posts");
            assert!(rel_pred.exists);
            assert!(rel_pred.predicate.is_none());
        }
        _ => panic!("Expected relation predicate"),
    }
    
    // Relation with conditions
    let has_published_posts = Predicate::has_with("posts", 
        Predicate::field("is_published", "=", true)
    );
    match has_published_posts {
        Predicate::Relation(ref rel_pred) => {
            assert_eq!(rel_pred.relation, "posts");
            assert!(rel_pred.exists);
            assert!(rel_pred.predicate.is_some());
        }
        _ => panic!("Expected relation predicate"),
    }
}

#[tokio::test]
async fn test_relation_predicate_sql_generation() {
    // Test that relation predicates generate correct SQL
    
    let has_posts = Predicate::has("posts");
    let (sql, params) = has_posts.to_sql("u");
    
    // Should generate an EXISTS subquery
    assert!(sql.contains("EXISTS"));
    assert!(sql.contains("posts"));
    assert!(sql.contains("u.id"));
    assert_eq!(params.len(), 0); // No parameters for simple existence check
}

#[tokio::test] 
async fn test_relation_filtering_integration() {
    let db = setup_relation_filtering_db().await;
    
    // Create users - some with posts, some without
    let user_with_posts = User::create()
        .set_email("author@example.com".to_string())
        .set_name("Author".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create author");

    let user_without_posts = User::create()
        .set_email("reader@example.com".to_string())
        .set_name("Reader".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create reader");

    // Create posts for the author
    Post::create()
        .set_user_id(user_with_posts.id)
        .set_title("Published Post".to_string())
        .set_content("This is published".to_string())
        .set_is_published(true)
        .set_view_count(100)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create published post");

    Post::create()
        .set_user_id(user_with_posts.id)
        .set_title("Draft Post".to_string())
        .set_content("This is a draft".to_string())
        .set_is_published(false)
        .set_view_count(5)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create draft post");

    // TEST: Basic relation existence
    // This test demonstrates the API we want to achieve
    // For now, we test the predicate construction and SQL generation
    
    let has_posts_predicate = Predicate::has("posts");
    let (sql, _params) = has_posts_predicate.to_sql("users");
    
    // Verify the SQL contains the expected EXISTS structure
    assert!(sql.contains("EXISTS"));
    assert!(sql.contains("SELECT 1 FROM posts"));
    assert!(sql.contains("users.id"));
    
    println!("Generated SQL for has_posts: {}", sql);
}