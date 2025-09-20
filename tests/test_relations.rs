use chrono::{DateTime, Utc};
use d1_rs::SchemaMigration;
use d1_rs::*;
use serde::{Deserialize, Serialize};

// Test entities with much cleaner relation definitions
#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct User {
    #[primary_key]
    pub id: i64,
    pub email: String,
    pub name: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct Profile {
    #[primary_key]
    pub id: i64,
    pub user_id: i64,
    pub bio: String,
    pub avatar_url: Option<String>,
    pub is_public: bool,
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
    pub view_count: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
#[table(name = "categories")]
pub struct Category {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub description: String,
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

// Define relations using the new, clean macro syntax
relations! {
    User {
        has_many posts: Post via user_id,
        has_one profile: Profile via user_id,
    }

    Post {
        belongs_to user: User via user_id,
        has_many_through categories: Category via post_id,
    }

    Category {
        has_many_through posts: Post via category_id,
    }

    Profile {
        belongs_to user: User via user_id,
    }
}

// Setup test database with the improved migration API
async fn setup_relations_db() -> D1Client {
    let db = D1Client::new_in_memory()
        .await
        .expect("Failed to create database");

    // Use migration system with auto-generated relations!
    let migration = SchemaMigration::new("create_relations_schema".to_string())
        .create_table("users")
        .integer("id")
        .primary_key()
        .auto_increment()
        .build()
        .text("email")
        .not_null()
        .unique()
        .build()
        .text("name")
        .not_null()
        .build()
        .boolean("is_active")
        .default_value(DefaultValue::Boolean(true))
        .build()
        .datetime("created_at")
        .default_value(DefaultValue::CurrentTimestamp)
        .build()
        .build()
        .create_table("profiles")
        .integer("id")
        .primary_key()
        .auto_increment()
        .build()
        .integer("user_id")
        .not_null()
        .build()
        .text("bio")
        .build()
        .text("avatar_url")
        .build()
        .boolean("is_public")
        .default_value(DefaultValue::Boolean(true))
        .build()
        .datetime("created_at")
        .default_value(DefaultValue::CurrentTimestamp)
        .build()
        .build()
        .create_table("posts")
        .integer("id")
        .primary_key()
        .auto_increment()
        .build()
        .integer("user_id")
        .not_null()
        .build()
        .text("title")
        .not_null()
        .build()
        .text("content")
        .not_null()
        .build()
        .boolean("is_published")
        .default_value(DefaultValue::Boolean(false))
        .build()
        .integer("view_count")
        .default_value(DefaultValue::Integer(0))
        .build()
        .datetime("created_at")
        .default_value(DefaultValue::CurrentTimestamp)
        .build()
        .build()
        .create_table("categories")
        .integer("id")
        .primary_key()
        .auto_increment()
        .build()
        .text("name")
        .not_null()
        .unique()
        .build()
        .text("description")
        .build()
        .boolean("is_active")
        .default_value(DefaultValue::Boolean(true))
        .build()
        .datetime("created_at")
        .default_value(DefaultValue::CurrentTimestamp)
        .build()
        .build()
        .create_table("post_categories")
        .integer("id")
        .primary_key()
        .auto_increment()
        .build()
        .integer("post_id")
        .not_null()
        .build()
        .integer("category_id")
        .not_null()
        .build()
        .datetime("created_at")
        .default_value(DefaultValue::CurrentTimestamp)
        .build()
        .build()
        // Auto-generate relations - this should now work!
        .auto_generate_for::<User>()
        .auto_generate_for::<Post>()
        .auto_generate_for::<Category>()
        .auto_generate_for::<Profile>();

    migration
        .execute(&db)
        .await
        .expect("Failed to execute migration");
    db
}

#[tokio::test]
async fn test_new_relations_api_schema_creation() {
    let db = D1Client::new_in_memory()
        .await
        .expect("Failed to create database");

    // Test the new, much simpler schema creation
    let migration = SchemaMigration::new("test_schema".to_string())
        .create_table("users")
        .integer("id")
        .primary_key()
        .auto_increment()
        .build()
        .text("name")
        .not_null()
        .build()
        .text("email")
        .not_null()
        .unique()
        .build()
        .build()
        .create_table("posts")
        .integer("id")
        .primary_key()
        .auto_increment()
        .build()
        .integer("user_id")
        .not_null()
        .build()
        .text("title")
        .not_null()
        .build()
        .build()
        // Type-safe edge creation (much cleaner!)
        .add_edge::<User, Post>()
        .one_to_many()
        .build();

    migration
        .execute(&db)
        .await
        .expect("Failed to execute migration");

    // Verify tables were created
    let tables_result = db.execute(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        &[]
    ).await.expect("Failed to query tables");

    assert!(tables_result.rows.len() >= 2);
}

#[tokio::test]
async fn test_new_associations_api() {
    let db = setup_relations_db().await;

    // Create test data
    let user = User::create()
        .set_email("alice@example.com".to_string())
        .set_name("Alice Johnson".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create user");

    let post1 = Post::create()
        .set_user_id(user.id)
        .set_title("Getting Started with Rust".to_string())
        .set_content("Rust is an amazing language...".to_string())
        .set_is_published(true)
        .set_view_count(100)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create post1");

    let _post2 = Post::create()
        .set_user_id(user.id)
        .set_title("Advanced Rust Patterns".to_string())
        .set_content("Let's explore advanced concepts...".to_string())
        .set_is_published(false)
        .set_view_count(0)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create post2");

    // TEST NEW API: Much cleaner association access
    // Instead of: user.traverse::<Post>(&db, "posts").await?
    // Now: user.posts().all(&db).await?
    let user_posts = user
        .posts()
        .all(&db)
        .await
        .expect("Failed to get user posts");
    assert_eq!(user_posts.len(), 2);

    // TEST: First post
    let first_post = user
        .posts()
        .first(&db)
        .await
        .expect("Failed to get first post");
    assert!(first_post.is_some());
    assert_eq!(first_post.unwrap().title, "Getting Started with Rust");

    // TEST: Reverse association
    let post_user = post1
        .user()
        .first(&db)
        .await
        .expect("Failed to get post user");
    assert!(post_user.is_some());
    assert_eq!(post_user.unwrap().id, user.id);
}

#[tokio::test]
async fn test_eager_loading_with_new_api() {
    let db = setup_relations_db().await;

    // Create test data
    let user1 = User::create()
        .set_email("bob@example.com".to_string())
        .set_name("Bob Smith".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create user1");

    let user2 = User::create()
        .set_email("charlie@example.com".to_string())
        .set_name("Charlie Brown".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create user2");

    // Create posts for both users
    let _post1 = Post::create()
        .set_user_id(user1.id)
        .set_title("Bob's First Post".to_string())
        .set_content("Content from Bob".to_string())
        .set_is_published(true)
        .set_view_count(50)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create post1");

    let _post2 = Post::create()
        .set_user_id(user2.id)
        .set_title("Charlie's Post".to_string())
        .set_content("Content from Charlie".to_string())
        .set_is_published(true)
        .set_view_count(75)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create post2");

    // TEST NEW API: Type-safe relations - load users and check their posts
    let all_users = User::query().all(&db).await.expect("Failed to load users");
    assert_eq!(all_users.len(), 2);

    // Verify we can get posts for each user using type-safe methods
    for user in all_users {
        let posts = user.posts().all(&db).await.expect("Failed to load posts");
        assert!(!posts.is_empty(), "User should have posts");
    }
}

#[tokio::test]
async fn test_many_to_many_with_new_api() {
    let db = setup_relations_db().await;

    // Create test data
    let user = User::create()
        .set_email("dave@example.com".to_string())
        .set_name("Dave Wilson".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create user");

    let post = Post::create()
        .set_user_id(user.id)
        .set_title("Rust and Database Design".to_string())
        .set_content("Exploring modern database patterns...".to_string())
        .set_is_published(true)
        .set_view_count(200)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create post");

    let tech_category = Category::create()
        .set_name("Technology".to_string())
        .set_description("Tech-related articles".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create tech category");

    let tutorial_category = Category::create()
        .set_name("Tutorials".to_string())
        .set_description("How-to guides".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create tutorial category");

    // Create junction table entries
    PostCategory::create()
        .set_post_id(post.id)
        .set_category_id(tech_category.id)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create post-category relation");

    PostCategory::create()
        .set_post_id(post.id)
        .set_category_id(tutorial_category.id)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create post-category relation");

    let post_categories = post
        .categories()
        .all(&db)
        .await
        .expect("Failed to get post categories");
    assert_eq!(post_categories.len(), 2);

    let category_names: Vec<String> = post_categories.iter().map(|c| c.name.clone()).collect();
    assert!(category_names.contains(&"Technology".to_string()));
    assert!(category_names.contains(&"Tutorials".to_string()));

    // TEST: Reverse many-to-many
    let tech_posts = tech_category
        .posts()
        .all(&db)
        .await
        .expect("Failed to get tech posts");
    assert_eq!(tech_posts.len(), 1);
    assert_eq!(tech_posts[0].id, post.id);
}

#[tokio::test]
async fn test_complex_predicates() {
    let db = setup_relations_db().await;

    // Create test data
    let active_user = User::create()
        .set_email("active@example.com".to_string())
        .set_name("Active User".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create active user");

    let _inactive_user = User::create()
        .set_email("inactive@example.com".to_string())
        .set_name("Inactive User".to_string())
        .set_is_active(false)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create inactive user");

    // Create posts
    Post::create()
        .set_user_id(active_user.id)
        .set_title("Active User's Post".to_string())
        .set_content("Content from active user".to_string())
        .set_is_published(true)
        .set_view_count(100)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create active user's post");

    // TEST: Type-safe basic queries - get all users and verify relations
    let all_users = User::query()
        .all(&db)
        .await
        .expect("Failed to get all users");
    assert!(all_users.len() >= 2);

    // TEST: Verify each user's posts using type-safe API
    for user in all_users {
        let posts = user.posts().all(&db).await.expect("Failed to get posts");
        if user.email == "active@example.com" {
            assert!(!posts.is_empty(), "Active user should have posts");
        }
    }
}

#[tokio::test]
async fn test_relation_based_filtering() {
    let db = setup_relations_db().await;

    // Create users with and without posts
    let user_with_posts = User::create()
        .set_email("author@example.com".to_string())
        .set_name("Author".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create author");

    let _user_without_posts = User::create()
        .set_email("reader@example.com".to_string())
        .set_name("Reader".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create reader");

    // Create post for the author
    Post::create()
        .set_user_id(user_with_posts.id)
        .set_title("Author's Post".to_string())
        .set_content("Content from author".to_string())
        .set_is_published(true)
        .set_view_count(150)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create post");

    // TEST: Get all users and verify type-safe relations work
    let all_users = User::query().all(&db).await.expect("Failed to get users");
    assert_eq!(all_users.len(), 2); // We created 2 users in this test

    // Find the user with posts (author)
    let author = all_users
        .iter()
        .find(|u| u.email == "author@example.com")
        .expect("Author not found");

    // TEST: Verify user has posts using type-safe API - NO STRING LITERALS!
    let user_posts = author
        .posts()
        .all(&db)
        .await
        .expect("Failed to get user posts");
    assert_eq!(user_posts.len(), 1); // This test creates 1 post, not 2

    // TEST: Count posts using type-safe API
    let post_count = author
        .posts()
        .count(&db)
        .await
        .expect("Failed to count posts");
    assert_eq!(post_count, 1);
}

#[tokio::test]
async fn test_graph_traversal() {
    let db = setup_relations_db().await;

    // Create complex relationship network
    let user = User::create()
        .set_email("author@example.com".to_string())
        .set_name("Graph Author".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create user");

    let post = Post::create()
        .set_user_id(user.id)
        .set_title("Graph Databases".to_string())
        .set_content("Understanding graph database concepts...".to_string())
        .set_is_published(true)
        .set_view_count(300)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create post");

    let category = Category::create()
        .set_name("Database Design".to_string())
        .set_description("Database architecture and design".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create category");

    // Link post to category
    PostCategory::create()
        .set_post_id(post.id)
        .set_category_id(category.id)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to link post to category");

    // TEST: Multi-level graph traversal
    // Navigate from User -> Posts -> Categories
    let user_posts = user
        .posts()
        .all(&db)
        .await
        .expect("Failed to get user posts");
    assert!(!user_posts.is_empty());

    let first_post = &user_posts[0];
    let post_categories = first_post
        .categories()
        .all(&db)
        .await
        .expect("Failed to get post categories");
    assert!(!post_categories.is_empty());
    assert_eq!(post_categories[0].name, "Database Design");
}

#[tokio::test]
async fn test_association_methods_api() {
    let db = setup_relations_db().await;

    // Create test data
    let user = User::create()
        .set_email("methods@example.com".to_string())
        .set_name("Method Tester".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create user");

    // TEST: Association method chaining
    let post_count = user
        .posts()
        .count(&db)
        .await
        .expect("Failed to count posts");
    assert_eq!(post_count, 0);

    // Create a post
    Post::create()
        .set_user_id(user.id)
        .set_title("Test Post".to_string())
        .set_content("Testing association methods".to_string())
        .set_is_published(true)
        .set_view_count(50)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create post");

    // TEST: Updated count
    let updated_count = user
        .posts()
        .count(&db)
        .await
        .expect("Failed to count posts");
    assert_eq!(updated_count, 1);

    // TEST: First method
    let first_post = user
        .posts()
        .first(&db)
        .await
        .expect("Failed to get first post");
    assert!(first_post.is_some());
    assert_eq!(first_post.unwrap().title, "Test Post");
}

#[tokio::test]
async fn test_migration_auto_generation() {
    let db = D1Client::new_in_memory()
        .await
        .expect("Failed to create database");

    // TEST: Auto-generate migrations from entity relations
    let migration = SchemaMigration::new("auto_generated".to_string())
        .create_table("users")
        .integer("id")
        .primary_key()
        .auto_increment()
        .build()
        .text("name")
        .not_null()
        .build()
        .build()
        .create_table("posts")
        .integer("id")
        .primary_key()
        .auto_increment()
        .build()
        .integer("user_id")
        .not_null()
        .build()
        .text("title")
        .not_null()
        .build()
        .build()
        // Auto-generate all relations for User entity
        .auto_generate_for::<User>();

    migration
        .execute(&db)
        .await
        .expect("Failed to execute auto-generated migration");

    // Verify that tables were created
    let tables = db
        .execute(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
            &[],
        )
        .await
        .expect("Failed to query tables");

    assert!(tables.rows.len() >= 2);
}

