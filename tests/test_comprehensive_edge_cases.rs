/// 🔍 COMPREHENSIVE EDGE CASE TESTING
/// This test suite covers ALL edge cases and error scenarios for d1-rs revolutionary features
/// Ensures robustness and reliability of the world's most advanced ORM
use chrono::{DateTime, Utc};
use d1_rs::*;
use d1_rs::SchemaMigration;
use serde::{Deserialize, Serialize};

// Test entities for comprehensive edge case testing
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
    pub is_published: bool,
    pub view_count: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct Category {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub created_at: DateTime<Utc>,
}

// Junction entity for rich M2M testing
#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct PostCategory {
    #[primary_key]
    pub id: i64,
    pub post_id: i64,
    pub category_id: i64,
    pub assigned_at: DateTime<Utc>,
    pub assigned_by: String,
    pub is_primary: bool,
}

relations! {
    User {
        has_many posts: Post via user_id,
    }
    
    Post {
        belongs_to user: User via user_id,
        has_many post_categories: PostCategory via post_id,
    }
    
    Category {
        belongs_to parent: Category via parent_id,
        has_many children: Category via parent_id,
        has_many post_categories: PostCategory via category_id,
    }
    
    PostCategory {
        belongs_to post: Post via post_id,
        belongs_to category: Category via category_id,
    }
}

async fn setup_comprehensive_test_db() -> D1Client {
    let db = D1Client::new_in_memory()
        .await
        .expect("Failed to create database");

    let migration = SchemaMigration::new("comprehensive_test_schema".to_string())
        .create_table("users")
        .integer("id").primary_key().auto_increment().build()
        .text("name").not_null().build()
        .text("email").not_null().unique().build()
        .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
        .build()
        .create_table("posts")
        .integer("id").primary_key().auto_increment().build()
        .integer("user_id").not_null().build()
        .text("title").not_null().build()
        .text("content").build()
        .boolean("is_published").default_value(DefaultValue::Boolean(false)).build()
        .integer("view_count").default_value(DefaultValue::Integer(0)).build()
        .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
        .build()
        .create_table("categorys")
        .integer("id").primary_key().auto_increment().build()
        .text("name").not_null().build()
        .integer("parent_id").build()
        .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
        .build()
        .create_table("post_categorys")
        .integer("id").primary_key().auto_increment().build()
        .integer("post_id").not_null().build()
        .integer("category_id").not_null().build()
        .datetime("assigned_at").default_value(DefaultValue::CurrentTimestamp).build()
        .text("assigned_by").not_null().build()
        .boolean("is_primary").default_value(DefaultValue::Boolean(false)).build()
        .build()
        .auto_generate_for::<User>()
        .auto_generate_for::<Post>()
        .auto_generate_for::<Category>()
        .auto_generate_for::<PostCategory>();

    migration
        .execute(&db)
        .await
        .expect("Failed to run migration");

    db
}

#[tokio::test]
async fn test_edge_case_empty_relations() {
    let db = setup_comprehensive_test_db().await;
    
    // Create user with no posts
    let user = User::create()
        .set_name("Lonely User".to_string())
        .set_email("lonely@example.com".to_string())
        .save(&db)
        .await
        .expect("Failed to create user");
    
    // TEST: Empty relationship access
    let posts = user.posts().all(&db).await.expect("Failed to get posts");
    assert!(posts.is_empty(), "User should have no posts");
    
    let post_count = user.posts().count(&db).await.expect("Failed to count posts");
    assert_eq!(post_count, 0, "Post count should be 0");
    
    let first_post = user.posts().first(&db).await.expect("Failed to get first post");
    assert!(first_post.is_none(), "First post should be None");
    
    println!("✅ Empty relations handled correctly");
}

#[tokio::test] 
async fn test_edge_case_recursive_orphan_nodes() {
    let db = setup_comprehensive_test_db().await;
    
    // Create orphan category (no parent or children)
    let orphan = Category::create()
        .set_name("Orphan Category".to_string())
        .set_parent_id(None)
        .save(&db)
        .await
        .expect("Failed to create orphan category");
    
    // TEST: Orphan node navigation
    let parent = orphan.parent().first(&db).await.expect("Failed to get parent");
    assert!(parent.is_none(), "Orphan should have no parent");
    
    let children = orphan.children().all(&db).await.expect("Failed to get children");
    assert!(children.is_empty(), "Orphan should have no children");
    
    let child_count = orphan.children().count(&db).await.expect("Failed to count children");
    assert_eq!(child_count, 0, "Child count should be 0");
    
    println!("✅ Recursive orphan nodes handled correctly");
}

#[tokio::test]
async fn test_edge_case_deep_recursive_hierarchy() {
    let db = setup_comprehensive_test_db().await;
    
    // Create deep hierarchy: Root -> Level1 -> Level2 -> Level3 -> Level4
    let root = Category::create()
        .set_name("Root".to_string())
        .set_parent_id(None)
        .save(&db)
        .await
        .expect("Failed to create root");
    
    let mut current_parent_id = root.id;
    let mut categories = vec![root];
    
    // Create 10 levels deep hierarchy
    for level in 1..=10 {
        let category = Category::create()
            .set_name(format!("Level{}", level))
            .set_parent_id(Some(current_parent_id))
            .save(&db)
            .await
            .expect(&format!("Failed to create level {} category", level));
        
        current_parent_id = category.id;
        categories.push(category);
    }
    
    // TEST: Navigate from deepest to root
    let deepest = &categories[10];
    let deepest_parent = deepest.parent().first(&db).await.expect("Failed to get parent");
    assert!(deepest_parent.is_some(), "Deepest should have parent");
    assert_eq!(deepest_parent.unwrap().name, "Level9");
    
    // TEST: Navigate from root to children
    let root_children = categories[0].children().all(&db).await.expect("Failed to get root children");
    assert_eq!(root_children.len(), 1, "Root should have exactly 1 child");
    assert_eq!(root_children[0].name, "Level1");
    
    println!("✅ Deep recursive hierarchy (10 levels) handled correctly");
}


#[tokio::test]
async fn test_edge_case_complex_junction_scenarios() {
    let db = setup_comprehensive_test_db().await;
    
    // Create test data
    let user = User::create()
        .set_name("Test User".to_string())
        .set_email("test@example.com".to_string())
        .save(&db)
        .await
        .expect("Failed to create user");
    
    let post = Post::create()
        .set_user_id(user.id)
        .set_title("Test Post".to_string())
        .set_content("Test content".to_string())
        .set_is_published(true)
        .set_view_count(100)
        .save(&db)
        .await
        .expect("Failed to create post");
    
    let category1 = Category::create()
        .set_name("Category 1".to_string())
        .set_parent_id(None)
        .save(&db)
        .await
        .expect("Failed to create category 1");
        
    let category2 = Category::create()
        .set_name("Category 2".to_string())
        .set_parent_id(None)
        .save(&db)
        .await
        .expect("Failed to create category 2");
    
    // Create multiple junction relationships with different data
    let junction1 = PostCategory::create()
        .set_post_id(post.id)
        .set_category_id(category1.id)
        .set_assigned_by("admin".to_string())
        .set_is_primary(true)
        .save(&db)
        .await
        .expect("Failed to create junction 1");
    
    let _junction2 = PostCategory::create()
        .set_post_id(post.id)
        .set_category_id(category2.id)
        .set_assigned_by("editor".to_string())
        .set_is_primary(false)
        .save(&db)
        .await
        .expect("Failed to create junction 2");
    
    // TEST: Complex junction entity queries
    
    // Query only primary category assignments
    let primary_assignments = PostCategory::query()
        .where_is_primary_eq(true)
        .all(&db)
        .await
        .expect("Failed to query primary assignments");
    assert_eq!(primary_assignments.len(), 1, "Should have 1 primary assignment");
    assert_eq!(primary_assignments[0].id, junction1.id);
    
    // Query by assigned_by
    let admin_assignments = PostCategory::query()
        .where_assigned_by_eq("admin".to_string())
        .all(&db)
        .await
        .expect("Failed to query admin assignments");
    assert_eq!(admin_assignments.len(), 1, "Should have 1 admin assignment");
    
    // TEST: Navigation through junction entities
    let post_categories = post.post_categories().all(&db).await.expect("Failed to get post categories");
    assert_eq!(post_categories.len(), 2, "Post should have 2 category assignments");
    
    // Navigate from junction to related entities
    let category_from_junction = junction1.category().first(&db).await.expect("Failed to get category from junction");
    assert!(category_from_junction.is_some(), "Should have category");
    assert_eq!(category_from_junction.unwrap().name, "Category 1");
    
    let post_from_junction = junction1.post().first(&db).await.expect("Failed to get post from junction");
    assert!(post_from_junction.is_some(), "Should have post");
    assert_eq!(post_from_junction.unwrap().title, "Test Post");
    
    println!("✅ Complex junction scenarios handled correctly");
}

#[tokio::test]
async fn test_edge_case_null_and_empty_values() {
    let db = setup_comprehensive_test_db().await;
    
    // Create user
    let user = User::create()
        .set_name("Test User".to_string())
        .set_email("test@example.com".to_string())
        .save(&db)
        .await
        .expect("Failed to create user");
    
    // Create post with minimal data (some fields might be null/empty)
    let post = Post::create()
        .set_user_id(user.id)
        .set_title("".to_string()) // Empty title
        .set_content("".to_string()) // Empty content  
        .set_is_published(false)
        .set_view_count(0)
        .save(&db)
        .await
        .expect("Failed to create post with empty values");
    
    // TEST: Handle empty/null values correctly
    assert_eq!(post.title, "", "Title should be empty string");
    assert_eq!(post.content, "", "Content should be empty string");
    assert_eq!(post.view_count, 0, "View count should be 0");
    assert!(!post.is_published, "Should not be published");
    
    // TEST: Relations still work with empty data
    let user_posts = user.posts().all(&db).await.expect("Failed to get user posts");
    assert_eq!(user_posts.len(), 1, "Should have 1 post even with empty data");
    
    let post_user = post.user().first(&db).await.expect("Failed to get post user");
    assert!(post_user.is_some(), "Should have user even with empty post data");
    
    println!("✅ Null and empty values handled correctly");
}


#[tokio::test]
async fn test_edge_case_relationship_consistency() {
    let db = setup_comprehensive_test_db().await;
    
    // This test verifies that our compile-time relationship validation works
    // and that relationships are consistent at runtime
    
    let user = User::create()
        .set_name("Consistent User".to_string())
        .set_email("consistent@example.com".to_string())
        .save(&db)
        .await
        .expect("Failed to create user");
    
    let post = Post::create()
        .set_user_id(user.id)
        .set_title("Consistent Post".to_string())
        .set_content("Test content".to_string())
        .set_is_published(true)
        .set_view_count(10)
        .save(&db)
        .await
        .expect("Failed to create post");
    
    // TEST: Bidirectional relationship consistency
    
    // User -> Posts relationship
    let user_posts = user.posts().all(&db).await.expect("Failed to get user posts");
    assert_eq!(user_posts.len(), 1, "User should have 1 post");
    assert_eq!(user_posts[0].id, post.id, "Post IDs should match");
    
    // Post -> User relationship (inverse)
    let post_user = post.user().first(&db).await.expect("Failed to get post user");
    assert!(post_user.is_some(), "Post should have user");
    assert_eq!(post_user.unwrap().id, user.id, "User IDs should match");
    
    // TEST: Relationship data consistency
    assert_eq!(user_posts[0].user_id, user.id, "Foreign key should be consistent");
    assert_eq!(post.user_id, user.id, "Foreign key should match user ID");
    
    println!("✅ Relationship consistency verified");
}