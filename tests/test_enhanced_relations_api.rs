/// Tests for the enhanced relations API with Ent-Go style query builders
use chrono::{DateTime, Utc};
use d1_rs::*;
use d1_rs::SchemaMigration;
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

// Enhanced relations with query builder support
relations! {
    User {
        has_many posts: Post via user_id,
    }

    Post {
        belongs_to user: User via user_id,
        has_many_through categories: Category via post_id,
    }

    Category {
        has_many_through posts: Post via category_id,
    }
}

async fn setup_enhanced_relations_db() -> D1Client {
    let db = D1Client::new_in_memory()
        .await
        .expect("Failed to create database");

    // Create tables
    let migration = SchemaMigration::new("create_enhanced_relations_schema".to_string())
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
        .auto_generate_for::<User>()
        .auto_generate_for::<Post>()
        .auto_generate_for::<Category>();

    migration
        .execute(&db)
        .await
        .expect("Failed to run migration");

    db
}

#[tokio::test]
async fn test_enhanced_association_where_conditions() {
    let db = setup_enhanced_relations_db().await;

    // Create test user
    let user = User::create()
        .set_email("author@example.com".to_string())
        .set_name("Author".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create user");

    // Create multiple posts with different attributes
    Post::create()
        .set_user_id(user.id)
        .set_title("Published Post".to_string())
        .set_content("This is published".to_string())
        .set_is_published(true)
        .set_view_count(100)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create published post");

    Post::create()
        .set_user_id(user.id)
        .set_title("Draft Post".to_string())
        .set_content("This is a draft".to_string())
        .set_is_published(false)
        .set_view_count(5)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create draft post");

    Post::create()
        .set_user_id(user.id)
        .set_title("Popular Post".to_string())
        .set_content("This is popular".to_string())
        .set_is_published(true)
        .set_view_count(500)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create popular post");

    // TEST 1: Filter by boolean field - FULLY TYPE-SAFE!
    let published_posts = user.posts()
        .query()
        .expect("Failed to create query")
        .where_is_published_eq(true)
        .all(&db)
        .await
        .expect("Failed to get published posts");
    assert_eq!(published_posts.len(), 2);

    // TEST 2: Filter by integer comparison - FULLY TYPE-SAFE!
    let popular_posts = user.posts()
        .query()
        .expect("Failed to create query")
        .where_view_count_gt(50)
        .all(&db)
        .await
        .expect("Failed to get popular posts");
    assert_eq!(popular_posts.len(), 2);

    // TEST 3: Combine multiple conditions - FULLY TYPE-SAFE!
    let published_popular_posts = user.posts()
        .query()
        .expect("Failed to create query")
        .where_is_published_eq(true)
        .where_view_count_gt(50)
        .all(&db)
        .await
        .expect("Failed to get published popular posts");
    assert_eq!(published_popular_posts.len(), 2);

    // TEST 4: String pattern matching - FULLY TYPE-SAFE!
    let draft_posts = user.posts()
        .query()
        .expect("Failed to create query")
        .where_title_like("%Draft%")
        .all(&db)
        .await
        .expect("Failed to get draft posts");
    assert_eq!(draft_posts.len(), 1);
    assert_eq!(draft_posts[0].title, "Draft Post");
}

#[tokio::test]
async fn test_enhanced_association_ordering() {
    let db = setup_enhanced_relations_db().await;

    let user = User::create()
        .set_email("author2@example.com".to_string())
        .set_name("Author2".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create user");

    // Create posts with different view counts
    for (title, views) in [("Low", 10), ("High", 100), ("Medium", 50)] {
        Post::create()
            .set_user_id(user.id)
            .set_title(format!("{} Views Post", title))
            .set_content("Content".to_string())
            .set_is_published(true)
            .set_view_count(views)
            .set_created_at(Utc::now())
            .save(&db)
            .await
            .expect("Failed to create post");
    }

    // TEST 1: Order by view count descending - FULLY TYPE-SAFE!
    let posts_desc = user.posts()
        .query()
        .expect("Failed to create query")
        .order_by_view_count_desc()
        .all(&db)
        .await
        .expect("Failed to get posts ordered desc");
    assert_eq!(posts_desc[0].view_count, 100);
    assert_eq!(posts_desc[1].view_count, 50);
    assert_eq!(posts_desc[2].view_count, 10);

    // TEST 2: Order by view count ascending - FULLY TYPE-SAFE!
    let posts_asc = user.posts()
        .query()
        .expect("Failed to create query")
        .order_by_view_count_asc()
        .all(&db)
        .await
        .expect("Failed to get posts ordered asc");
    assert_eq!(posts_asc[0].view_count, 10);
    assert_eq!(posts_asc[1].view_count, 50);
    assert_eq!(posts_asc[2].view_count, 100);
}

#[tokio::test]
async fn test_enhanced_association_limit_offset() {
    let db = setup_enhanced_relations_db().await;

    let user = User::create()
        .set_email("author3@example.com".to_string())
        .set_name("Author3".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create user");

    // Create 5 posts
    for i in 1..=5 {
        Post::create()
            .set_user_id(user.id)
            .set_title(format!("Post {}", i))
            .set_content("Content".to_string())
            .set_is_published(true)
            .set_view_count(i as i64)
            .set_created_at(Utc::now())
            .save(&db)
            .await
            .expect("Failed to create post");
    }

    // TEST 1: Limit results - FULLY TYPE-SAFE!
    let limited_posts = user.posts()
        .query()
        .expect("Failed to create query")
        .order_by_view_count_asc()
        .limit(3)
        .all(&db)
        .await
        .expect("Failed to get limited posts");
    assert_eq!(limited_posts.len(), 3);

    // TEST 2: Offset and limit (pagination) - FULLY TYPE-SAFE!
    let paginated_posts = user.posts()
        .query()
        .expect("Failed to create query")
        .order_by_view_count_asc()
        .offset(2)
        .limit(2)
        .all(&db)
        .await
        .expect("Failed to get paginated posts");
    assert_eq!(paginated_posts.len(), 2);
    assert_eq!(paginated_posts[0].view_count, 3);
    assert_eq!(paginated_posts[1].view_count, 4);
}

#[tokio::test]
async fn test_enhanced_association_chaining() {
    let db = setup_enhanced_relations_db().await;

    let user = User::create()
        .set_email("author4@example.com".to_string())
        .set_name("Author4".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await
        .expect("Failed to create user");

    // Create diverse posts
    for (title, published, views) in [
        ("Popular Published", true, 200),
        ("Unpopular Published", true, 5),
        ("Popular Draft", false, 150),
        ("Unpopular Draft", false, 1),
    ] {
        Post::create()
            .set_user_id(user.id)
            .set_title(title.to_string())
            .set_content("Content".to_string())
            .set_is_published(published)
            .set_view_count(views)
            .set_created_at(Utc::now())
            .save(&db)
            .await
            .expect("Failed to create post");
    }

    // TEST: Complex chaining - published posts with high views, ordered, limited - FULLY TYPE-SAFE!
    let results = user.posts()
        .query()
        .expect("Failed to create query")
        .where_is_published_eq(true)
        .where_view_count_gt(10)
        .order_by_view_count_desc()
        .limit(1)
        .all(&db)
        .await
        .expect("Failed to get chained query results");
    
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Popular Published");
    assert_eq!(results[0].view_count, 200);
    assert!(results[0].is_published);
}