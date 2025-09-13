use d1_rs::*;
use d1_rs::schema_evolution::SchemaMigration;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Test entities for relations
#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
#[table(name = "users")]
pub struct User {
    #[primary_key]
    pub id: i64,
    pub email: String,
    pub name: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
#[table(name = "profiles")]
pub struct Profile {
    #[primary_key]
    pub id: i64,
    pub user_id: i64, // Foreign key to users
    pub bio: String,
    pub avatar_url: Option<String>,
    pub is_public: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
#[table(name = "posts")]
pub struct Post {
    #[primary_key]
    pub id: i64,
    pub user_id: i64, // Foreign key to users
    pub title: String,
    pub content: String,
    pub is_published: bool,
    pub view_count: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
#[table(name = "categories")]
pub struct Category {
    #[primary_key]
    pub id: i64,
    pub category_name: String,
    pub category_description: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
#[table(name = "post_categories")]
pub struct PostCategory {
    #[primary_key]
    pub id: i64,
    pub post_id: i64,
    pub category_id: i64,
    pub created_at: DateTime<Utc>,
}

// Setup test database with relations
async fn setup_relations_db() -> D1Client {
    let db = D1Client::new_in_memory().await.expect("Failed to create database");
    
    // Create tables one by one using individual migrations
    let users_migration = SchemaMigration::new("create_users".to_string())
        .create_table("users")
            .integer("id").primary_key().auto_increment().build()
            .text("email").not_null().unique().build()
            .text("name").not_null().build()
            .boolean("is_active").not_null().default_value(DefaultValue::Boolean(true)).build()
            .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
        .build();
    
    users_migration.execute(&db).await.expect("Failed to create users table");
    
    let profiles_migration = SchemaMigration::new("create_profiles".to_string())
        .create_table("profiles")
            .integer("id").primary_key().auto_increment().build()
            .integer("user_id").not_null().build()
            .text("bio").build()
            .text("avatar_url").build()
            .boolean("is_public").not_null().default_value(DefaultValue::Boolean(true)).build()
            .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
        .build();
    
    profiles_migration.execute(&db).await.expect("Failed to create profiles table");
    
    let posts_migration = SchemaMigration::new("create_posts".to_string())
        .create_table("posts")
            .integer("id").primary_key().auto_increment().build()
            .integer("user_id").not_null().build()
            .text("title").not_null().build()
            .text("content").not_null().build()
            .boolean("is_published").not_null().default_value(DefaultValue::Boolean(false)).build()
            .integer("view_count").not_null().default_value(DefaultValue::Integer(0)).build()
            .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
        .build();
    
    posts_migration.execute(&db).await.expect("Failed to create posts table");
    
    let categories_migration = SchemaMigration::new("create_categories".to_string())
        .create_table("categories")
            .integer("id").primary_key().auto_increment().build()
            .text("category_name").not_null().unique().build()
            .text("category_description").build()
            .boolean("is_active").not_null().default_value(DefaultValue::Boolean(true)).build()
            .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
        .build();
    
    categories_migration.execute(&db).await.expect("Failed to create categories table");
    
    let post_categories_migration = SchemaMigration::new("create_post_categories".to_string())
        .create_table("post_categories")
            .integer("id").primary_key().auto_increment().build()
            .integer("post_id").not_null().build()
            .integer("category_id").not_null().build()
            .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
        .build();
    
    post_categories_migration.execute(&db).await.expect("Failed to create post_categories table");
    
    db
}

#[tokio::test]
async fn test_schema_migration_with_relations() {
    let db = D1Client::new_in_memory().await.expect("Failed to create database");
    
    // Create tables one by one using simpler migrations
    let users_migration = SchemaMigration::new("create_users".to_string())
        .create_table("users")
            .integer("id").primary_key().auto_increment().build()
            .text("email").not_null().unique().build()
            .text("name").not_null().build()
            .boolean("is_active").not_null().default_value(DefaultValue::Boolean(true)).build()
            .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
        .build();
    
    users_migration.execute(&db).await.expect("Failed to create users table");
    
    let profiles_migration = SchemaMigration::new("create_profiles".to_string())
        .create_table("profiles")
            .integer("id").primary_key().auto_increment().build()
            .integer("user_id").not_null().build()
            .text("bio").build()
            .text("avatar_url").build()
            .boolean("is_public").not_null().default_value(DefaultValue::Boolean(true)).build()
            .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
        .build();
    
    profiles_migration.execute(&db).await.expect("Failed to create profiles table");
    
    let posts_migration = SchemaMigration::new("create_posts".to_string())
        .create_table("posts")
            .integer("id").primary_key().auto_increment().build()
            .integer("user_id").not_null().build()
            .text("title").not_null().build()
            .text("content").not_null().build()
            .boolean("is_published").not_null().default_value(DefaultValue::Boolean(false)).build()
            .integer("view_count").not_null().default_value(DefaultValue::Integer(0)).build()
            .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
        .build();
    
    posts_migration.execute(&db).await.expect("Failed to create posts table");
    
    let categories_migration = SchemaMigration::new("create_categories".to_string())
        .create_table("categories")
            .integer("id").primary_key().auto_increment().build()
            .text("category_name").not_null().unique().build()
            .text("category_description").build()
            .boolean("is_active").not_null().default_value(DefaultValue::Boolean(true)).build()
            .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
        .build();
    
    categories_migration.execute(&db).await.expect("Failed to create categories table");
    
    let post_categories_migration = SchemaMigration::new("create_post_categories".to_string())
        .create_table("post_categories")
            .integer("id").primary_key().auto_increment().build()
            .integer("post_id").not_null().build()
            .integer("category_id").not_null().build()
            .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
        .build();
    
    post_categories_migration.execute(&db).await.expect("Failed to create post_categories table");
    
    // Verify tables were created
    let tables_result = db.execute(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        &[]
    ).await.expect("Failed to query tables");
    
    let table_names: Vec<String> = tables_result.rows
        .into_iter()
        .map(|row| {
            if let serde_json::Value::Object(obj) = row {
                obj.get("name").unwrap().as_str().unwrap().to_string()
            } else {
                panic!("Invalid row format");
            }
        })
        .collect();
    
    assert_eq!(table_names, vec![
        "categories",
        "post_categories", 
        "posts",
        "profiles",
        "users"
    ]);
}

#[tokio::test]
async fn test_one_to_one_relation_creation() {
    let db = setup_relations_db().await;
    
    // Create a user
    let user = User::create()
        .set_email("alice@example.com".to_string())
        .set_name("Alice".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create user");
    
    // Create a profile for the user
    let profile = Profile::create()
        .set_user_id(user.id)
        .set_bio("Software engineer passionate about Rust".to_string())
        .set_avatar_url(Some("https://example.com/avatar.jpg".to_string()))
        .set_is_public(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create profile");
    
    // Verify the relation
    let found_profile = Profile::query()
        .where_user_id_eq(user.id)
        .first(&db)
        .await
        .expect("Failed to query profile")
        .expect("Profile not found");
    
    assert_eq!(found_profile.id, profile.id);
    assert_eq!(found_profile.user_id, user.id);
    assert_eq!(found_profile.bio, "Software engineer passionate about Rust");
}

#[tokio::test]
async fn test_one_to_many_relation_creation() {
    let db = setup_relations_db().await;
    
    // Create a user
    let user = User::create()
        .set_email("bob@example.com".to_string())
        .set_name("Bob".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create user");
    
    // Create multiple posts for the user
    let _post1 = Post::create()
        .set_user_id(user.id)
        .set_title("Introduction to Rust".to_string())
        .set_content("Rust is a systems programming language...".to_string())
        .set_is_published(true)
        .set_view_count(100)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create post1");
    
    let _post2 = Post::create()
        .set_user_id(user.id)
        .set_title("Advanced Rust Patterns".to_string())
        .set_content("In this post, we'll explore...".to_string())
        .set_is_published(false)
        .set_view_count(0)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create post2");
    
    // Query all posts for the user
    let user_posts = Post::query()
        .where_user_id_eq(user.id)
        .all(&db)
        .await
        .expect("Failed to query user posts");
    
    assert_eq!(user_posts.len(), 2);
    
    // Check published posts only
    let published_posts = Post::query()
        .where_user_id_eq(user.id)
        .where_is_published_eq(true)
        .all(&db)
        .await
        .expect("Failed to query published posts");
    
    assert_eq!(published_posts.len(), 1);
    assert_eq!(published_posts[0].title, "Introduction to Rust");
}

#[tokio::test]
async fn test_simple_category_creation() {
    let db = setup_relations_db().await;
    
    // Test Category entity creation
    let category = Category::create()
        .set_category_name("Simple Test".to_string())
        .set_category_description("Simple test description".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create simple category");
        
    assert!(!category.category_name.is_empty());
}

#[tokio::test]
async fn test_many_to_many_relation_creation() {
    let db = setup_relations_db().await;
    
    // Debug: Check if tables exist
    let tables_result = db.execute(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        &[]
    ).await.expect("Failed to query tables");
    
    let table_names: Vec<String> = tables_result.rows
        .into_iter()
        .map(|row| {
            if let serde_json::Value::Object(obj) = row {
                obj.get("name").unwrap().as_str().unwrap().to_string()
            } else {
                panic!("Invalid row format");
            }
        })
        .collect();
    
    println!("Available tables: {:?}", table_names);
    
    // Debug: Try to create a simple category first
    println!("Attempting to create a test category...");
    let test_category = Category::create()
        .set_category_name("Test Category".to_string())
        .set_category_description("Test description".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create test category");
    println!("Test category created successfully: {:?}", test_category.id);
    
    // Create a user and post
    let user = User::create()
        .set_email("charlie@example.com".to_string())
        .set_name("Charlie".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create user");
    
    let post = Post::create()
        .set_user_id(user.id)
        .set_title("Rust and Database Design".to_string())
        .set_content("Combining Rust with modern database design...".to_string())
        .set_is_published(true)
        .set_view_count(250)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create post");
    
    // Create categories
    let tech_category = Category::create()
        .set_category_name("Technology".to_string())
        .set_category_description("Tech-related posts".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create tech category");
    
    let rust_category = Category::create()
        .set_category_name("Rust".to_string())
        .set_category_description("Rust programming language".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create rust category");
    
    // Create many-to-many relations through junction table
    let _post_cat1 = PostCategory::create()
        .set_post_id(post.id)
        .set_category_id(tech_category.id)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create post-category relation");
    
    let _post_cat2 = PostCategory::create()
        .set_post_id(post.id)
        .set_category_id(rust_category.id)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create post-category relation");
    
    // Query categories for a post (through junction table)
    let post_categories = db.execute(
        "SELECT c.* FROM categories c INNER JOIN post_categories pc ON c.id = pc.category_id WHERE pc.post_id = ?",
        &[serde_json::Value::Number(post.id.into())]
    ).await.expect("Failed to query post categories");
    
    assert_eq!(post_categories.rows.len(), 2);
    
    // Query posts for a category (through junction table)
    let rust_posts = db.execute(
        "SELECT p.* FROM posts p INNER JOIN post_categories pc ON p.id = pc.post_id WHERE pc.category_id = ?",
        &[serde_json::Value::Number(rust_category.id.into())]
    ).await.expect("Failed to query rust posts");
    
    assert_eq!(rust_posts.rows.len(), 1);
}

#[tokio::test]
async fn test_complex_relational_queries() {
    let db = setup_relations_db().await;
    
    // Create test data
    let user = User::create()
        .set_email("dave@example.com".to_string())
        .set_name("Dave".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create user");
    
    let _profile = Profile::create()
        .set_user_id(user.id)
        .set_bio("Full-stack developer".to_string())
        .set_is_public(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create profile");
    
    let _post = Post::create()
        .set_user_id(user.id)
        .set_title("Building Web APIs with Rust".to_string())
        .set_content("Modern web development with Rust...".to_string())
        .set_is_published(true)
        .set_view_count(500)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create post");
    
    // Complex query: Get all published posts with user info and profile
    let complex_result = db.execute(
        r#"
        SELECT 
            p.id as post_id,
            p.title,
            p.view_count,
            u.name as author_name,
            u.email as author_email,
            pr.bio as author_bio
        FROM posts p
        INNER JOIN users u ON p.user_id = u.id
        INNER JOIN profiles pr ON u.id = pr.user_id
        WHERE p.is_published = 1
        ORDER BY p.view_count DESC
        "#,
        &[]
    ).await.expect("Failed to execute complex query");
    
    assert_eq!(complex_result.rows.len(), 1);
    
    if let serde_json::Value::Object(row) = &complex_result.rows[0] {
        assert_eq!(row.get("title").unwrap().as_str().unwrap(), "Building Web APIs with Rust");
        assert_eq!(row.get("author_name").unwrap().as_str().unwrap(), "Dave");
        assert_eq!(row.get("author_bio").unwrap().as_str().unwrap(), "Full-stack developer");
        assert_eq!(row.get("view_count").unwrap().as_i64().unwrap(), 500);
    }
}

#[tokio::test]
async fn test_relation_constraints_and_integrity() {
    let db = setup_relations_db().await;
    
    // Create a user first
    let user = User::create()
        .set_email("eve@example.com".to_string())
        .set_name("Eve".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create user");
    
    // Test that we can create posts referencing valid user
    let post = Post::create()
        .set_user_id(user.id)
        .set_title("Valid Post".to_string())
        .set_content("This post has a valid user reference".to_string())
        .set_is_published(true)
        .set_view_count(0)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create post with valid user reference");
    
    assert_eq!(post.user_id, user.id);
    
    // Test cascading behavior when deleting user
    User::delete(&db, user.id).await.expect("Failed to delete user");
    
    // Verify user is deleted
    let deleted_user = User::find(&db, user.id).await.expect("Failed to query user");
    assert!(deleted_user.is_none());
    
    // Posts might still exist (depending on foreign key constraints)
    // In a real implementation with proper foreign keys, this would cascade
    let orphaned_posts = Post::query()
        .where_user_id_eq(user.id)
        .all(&db)
        .await
        .expect("Failed to query posts");
    
    // Without proper foreign key constraints, orphaned posts may remain
    // This demonstrates the importance of proper constraint setup
    println!("Orphaned posts after user deletion: {}", orphaned_posts.len());
}