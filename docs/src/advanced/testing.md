# Testing Strategies

This guide covers comprehensive testing strategies for d1-rs applications, from unit tests to integration tests and performance testing.

## Test Environment Setup

### In-Memory Database Testing

d1-rs makes testing easy with automatic SQLite fallback:

```rust
use d1_rs::*;
use tokio;

#[tokio::test]
async fn test_user_crud() {
    // Creates an in-memory SQLite database
    let db = D1Client::new_in_memory().await.unwrap();
    
    // Run migrations
    setup_test_schema(&db).await.unwrap();
    
    // Your test code here
    let user = User::create()
        .set_name("Test User".to_string())
        .set_email("test@example.com".to_string())
        .set_is_active(true)
        .save(&db)
        .await
        .unwrap();
    
    assert_eq!(user.name, "Test User");
    assert_eq!(user.email, "test@example.com");
    assert!(user.is_active);
}

async fn setup_test_schema(db: &D1Client) -> Result<()> {
    let migration = SchemaMigration::new("test_schema".to_string())
        .create_table("users")
            .integer("id").primary_key().auto_increment().build()
            .text("name").not_null().build()
            .text("email").not_null().unique().build()
            .boolean("is_active").default_value(DefaultValue::Boolean(true)).build()
            .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
        .build();
    
    migration.execute(db).await
}
```

### Test Utilities

Create reusable test utilities:

```rust
// tests/common/mod.rs
use d1_rs::*;
use chrono::{DateTime, Utc};

pub struct TestDatabase {
    pub db: D1Client,
}

impl TestDatabase {
    pub async fn new() -> Self {
        let db = D1Client::new_in_memory().await.unwrap();
        setup_complete_schema(&db).await.unwrap();
        Self { db }
    }
    
    pub async fn with_test_data() -> Self {
        let test_db = Self::new().await;
        create_test_data(&test_db.db).await.unwrap();
        test_db
    }
}

async fn setup_complete_schema(db: &D1Client) -> Result<()> {
    let migration = SchemaMigration::new("complete_test_schema".to_string())
        .create_table("users")
            .integer("id").primary_key().auto_increment().build()
            .text("name").not_null().build()
            .text("email").not_null().unique().build()
            .boolean("is_active").default_value(DefaultValue::Boolean(true)).build()
            .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
        .build()
        
        .create_table("posts")
            .integer("id").primary_key().auto_increment().build()
            .integer("user_id").not_null().build()
            .text("title").not_null().build()
            .text("content").not_null().build()
            .boolean("is_published").default_value(DefaultValue::Boolean(false)).build()
            .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
        .build()
        
        .create_table("categories")
            .integer("id").primary_key().auto_increment().build()
            .text("name").not_null().unique().build()
            .text("description").build()
        .build()
        
        .create_table("post_categories")
            .integer("id").primary_key().auto_increment().build()
            .integer("post_id").not_null().build()
            .integer("category_id").not_null().build()
            .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
        .build()
        
        .create_relation("user_posts", "users", "posts")
            .one_to_many("user_id", "id")
        .build()
        
        .create_relation("post_categories_rel", "posts", "categories")
            .many_to_many("post_categories", "post_id", "id", "category_id", "id")
        .build();
    
    migration.execute(db).await
}

async fn create_test_data(db: &D1Client) -> Result<()> {
    // Create test users
    let alice = User::create()
        .set_name("Alice Johnson".to_string())
        .set_email("alice@example.com".to_string())
        .set_is_active(true)
        .save(db)
        .await?;
    
    let bob = User::create()
        .set_name("Bob Smith".to_string())
        .set_email("bob@example.com".to_string())
        .set_is_active(true)
        .save(db)
        .await?;
    
    // Create test categories
    let tech = Category::create()
        .set_name("Technology".to_string())
        .set_description(Some("Tech-related posts".to_string()))
        .save(db)
        .await?;
    
    let lifestyle = Category::create()
        .set_name("Lifestyle".to_string())
        .set_description(Some("Lifestyle posts".to_string()))
        .save(db)
        .await?;
    
    // Create test posts
    let post1 = Post::create()
        .set_user_id(alice.id)
        .set_title("Getting Started with Rust".to_string())
        .set_content("Rust is an amazing language...".to_string())
        .set_is_published(true)
        .set_created_at(Utc::now())
        .save(db)
        .await?;
    
    let post2 = Post::create()
        .set_user_id(bob.id)
        .set_title("Coffee Brewing Tips".to_string())
        .set_content("Here's how to brew the perfect cup...".to_string())
        .set_is_published(true)
        .set_created_at(Utc::now())
        .save(db)
        .await?;
    
    // Create post-category associations
    PostCategory::create()
        .set_post_id(post1.id)
        .set_category_id(tech.id)
        .set_created_at(Utc::now())
        .save(db)
        .await?;
    
    PostCategory::create()
        .set_post_id(post2.id)
        .set_category_id(lifestyle.id)
        .set_created_at(Utc::now())
        .save(db)
        .await?;
    
    Ok(())
}

// Test data builders for flexible test setup
pub struct UserBuilder {
    name: String,
    email: String,
    is_active: bool,
}

impl UserBuilder {
    pub fn new() -> Self {
        Self {
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            is_active: true,
        }
    }
    
    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }
    
    pub fn email(mut self, email: &str) -> Self {
        self.email = email.to_string();
        self
    }
    
    pub fn inactive(mut self) -> Self {
        self.is_active = false;
        self
    }
    
    pub async fn save(self, db: &D1Client) -> Result<User> {
        User::create()
            .set_name(self.name)
            .set_email(self.email)
            .set_is_active(self.is_active)
            .save(db)
            .await
    }
}

pub struct PostBuilder {
    user_id: i64,
    title: String,
    content: String,
    is_published: bool,
}

impl PostBuilder {
    pub fn for_user(user_id: i64) -> Self {
        Self {
            user_id,
            title: "Test Post".to_string(),
            content: "Test content".to_string(),
            is_published: false,
        }
    }
    
    pub fn title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }
    
    pub fn content(mut self, content: &str) -> Self {
        self.content = content.to_string();
        self
    }
    
    pub fn published(mut self) -> Self {
        self.is_published = true;
        self
    }
    
    pub async fn save(self, db: &D1Client) -> Result<Post> {
        Post::create()
            .set_user_id(self.user_id)
            .set_title(self.title)
            .set_content(self.content)
            .set_is_published(self.is_published)
            .set_created_at(Utc::now())
            .save(db)
            .await
    }
}
```

## Unit Testing

### Entity CRUD Tests

Test basic entity operations:

```rust
#[cfg(test)]
mod entity_tests {
    use super::*;
    use crate::common::*;
    
    #[tokio::test]
    async fn test_user_creation() {
        let test_db = TestDatabase::new().await;
        
        let user = UserBuilder::new()
            .name("Alice")
            .email("alice@test.com")
            .save(&test_db.db)
            .await
            .unwrap();
        
        assert_eq!(user.name, "Alice");
        assert_eq!(user.email, "alice@test.com");
        assert!(user.is_active);
        assert!(user.id > 0);
    }
    
    #[tokio::test]
    async fn test_user_update() {
        let test_db = TestDatabase::new().await;
        
        let user = UserBuilder::new()
            .name("Original Name")
            .save(&test_db.db)
            .await
            .unwrap();
        
        let updated = User::update(user.id)
            .set_name("Updated Name".to_string())
            .set_is_active(false)
            .save(&test_db.db)
            .await
            .unwrap();
        
        assert_eq!(updated.name, "Updated Name");
        assert!(!updated.is_active);
        assert_eq!(updated.id, user.id);
    }
    
    #[tokio::test]
    async fn test_user_deletion() {
        let test_db = TestDatabase::new().await;
        
        let user = UserBuilder::new().save(&test_db.db).await.unwrap();
        let user_id = user.id;
        
        User::delete(&test_db.db, user_id).await.unwrap();
        
        let found = User::find(&test_db.db, user_id).await.unwrap();
        assert!(found.is_none());
    }
    
    #[tokio::test]
    async fn test_unique_constraint_violation() {
        let test_db = TestDatabase::new().await;
        
        // Create first user
        UserBuilder::new()
            .email("duplicate@test.com")
            .save(&test_db.db)
            .await
            .unwrap();
        
        // Try to create second user with same email
        let result = UserBuilder::new()
            .name("Different Name")
            .email("duplicate@test.com")
            .save(&test_db.db)
            .await;
        
        assert!(result.is_err());
    }
}
```

### Query Builder Tests

Test query building and filtering:

```rust
#[cfg(test)]
mod query_tests {
    use super::*;
    use crate::common::*;
    
    #[tokio::test]
    async fn test_where_clauses() {
        let test_db = TestDatabase::with_test_data().await;
        
        // Test exact match
        let alice = User::query()
            .where_name_eq("Alice Johnson".to_string())
            .first(&test_db.db)
            .await
            .unwrap();
        assert!(alice.is_some());
        
        // Test contains
        let users_with_john = User::query()
            .where_name_contains("John")
            .all(&test_db.db)
            .await
            .unwrap();
        assert_eq!(users_with_john.len(), 1);
        
        // Test boolean filtering
        let active_users = User::query()
            .where_is_active_eq(true)
            .all(&test_db.db)
            .await
            .unwrap();
        assert!(active_users.len() >= 2);
    }
    
    #[tokio::test]
    async fn test_ordering() {
        let test_db = TestDatabase::new().await;
        
        // Create users in specific order
        let user_c = UserBuilder::new().name("Charlie").save(&test_db.db).await.unwrap();
        let user_a = UserBuilder::new().name("Alice").email("alice@test.com").save(&test_db.db).await.unwrap();
        let user_b = UserBuilder::new().name("Bob").email("bob@test.com").save(&test_db.db).await.unwrap();
        
        // Test ascending order
        let users_asc = User::query()
            .order_by_name_asc()
            .all(&test_db.db)
            .await
            .unwrap();
        
        assert_eq!(users_asc[0].name, "Alice");
        assert_eq!(users_asc[1].name, "Bob");
        assert_eq!(users_asc[2].name, "Charlie");
        
        // Test descending order
        let users_desc = User::query()
            .order_by_name_desc()
            .all(&test_db.db)
            .await
            .unwrap();
        
        assert_eq!(users_desc[0].name, "Charlie");
        assert_eq!(users_desc[1].name, "Bob");
        assert_eq!(users_desc[2].name, "Alice");
    }
    
    #[tokio::test]
    async fn test_pagination() {
        let test_db = TestDatabase::new().await;
        
        // Create multiple users
        for i in 1..=10 {
            UserBuilder::new()
                .name(&format!("User {}", i))
                .email(&format!("user{}@test.com", i))
                .save(&test_db.db)
                .await
                .unwrap();
        }
        
        // Test first page
        let page1 = User::query()
            .order_by_id_asc()
            .limit(3)
            .offset(0)
            .all(&test_db.db)
            .await
            .unwrap();
        assert_eq!(page1.len(), 3);
        
        // Test second page
        let page2 = User::query()
            .order_by_id_asc()
            .limit(3)
            .offset(3)
            .all(&test_db.db)
            .await
            .unwrap();
        assert_eq!(page2.len(), 3);
        assert_ne!(page1[0].id, page2[0].id);
    }
    
    #[tokio::test]
    async fn test_count_query() {
        let test_db = TestDatabase::new().await;
        
        // Create some users
        for i in 1..=5 {
            UserBuilder::new()
                .name(&format!("User {}", i))
                .email(&format!("user{}@test.com", i))
                .save(&test_db.db)
                .await
                .unwrap();
        }
        
        let total_count = User::query().count(&test_db.db).await.unwrap();
        assert_eq!(total_count, 5);
        
        let active_count = User::query()
            .where_is_active_eq(true)
            .count(&test_db.db)
            .await
            .unwrap();
        assert_eq!(active_count, 5);
    }
}
```

## Relationship Testing

### One-to-Many Relationships

```rust
#[cfg(test)]
mod relationship_tests {
    use super::*;
    use crate::common::*;
    
    #[tokio::test]
    async fn test_one_to_many_traversal() {
        let test_db = TestDatabase::new().await;
        
        let user = UserBuilder::new().save(&test_db.db).await.unwrap();
        
        // Create posts for the user
        let post1 = PostBuilder::for_user(user.id)
            .title("First Post")
            .published()
            .save(&test_db.db)
            .await
            .unwrap();
        
        let post2 = PostBuilder::for_user(user.id)
            .title("Second Post")
            .save(&test_db.db)
            .await
            .unwrap();
        
        // Test traversal from user to posts
        let posts = user.traverse::<Post>(&test_db.db, "posts").await.unwrap();
        assert_eq!(posts.len(), 2);
        
        let titles: Vec<_> = posts.iter().map(|p| &p.title).collect();
        assert!(titles.contains(&&"First Post".to_string()));
        assert!(titles.contains(&&"Second Post".to_string()));
        
        // Test reverse traversal from post to user
        let users = post1.traverse::<User>(&test_db.db, "user").await.unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].id, user.id);
    }
    
    #[tokio::test]
    async fn test_eager_loading() {
        let test_db = TestDatabase::with_test_data().await;
        
        let users_with_posts = User::query()
            .with(vec!["posts"])
            .all(&test_db.db)
            .await
            .unwrap();
        
        assert!(users_with_posts.len() >= 2);
        
        // Verify that posts can be accessed
        for user in users_with_posts {
            let posts = user.traverse::<Post>(&test_db.db, "posts").await.unwrap();
            // Each user should have at least one post from test data
            assert!(!posts.is_empty());
        }
    }
}
```

### Many-to-Many Relationships

```rust
#[cfg(test)]
mod many_to_many_tests {
    use super::*;
    use crate::common::*;
    
    #[tokio::test]
    async fn test_many_to_many_associations() {
        let test_db = TestDatabase::new().await;
        
        let user = UserBuilder::new().save(&test_db.db).await.unwrap();
        let post = PostBuilder::for_user(user.id).save(&test_db.db).await.unwrap();
        
        let tech_category = Category::create()
            .set_name("Technology".to_string())
            .save(&test_db.db)
            .await
            .unwrap();
        
        let programming_category = Category::create()
            .set_name("Programming".to_string())
            .save(&test_db.db)
            .await
            .unwrap();
        
        // Associate post with categories
        PostCategory::create()
            .set_post_id(post.id)
            .set_category_id(tech_category.id)
            .set_created_at(Utc::now())
            .save(&test_db.db)
            .await
            .unwrap();
        
        PostCategory::create()
            .set_post_id(post.id)
            .set_category_id(programming_category.id)
            .set_created_at(Utc::now())
            .save(&test_db.db)
            .await
            .unwrap();
        
        // Test traversal from post to categories
        let categories = post.traverse::<Category>(&test_db.db, "categories").await.unwrap();
        assert_eq!(categories.len(), 2);
        
        let category_names: Vec<_> = categories.iter().map(|c| &c.name).collect();
        assert!(category_names.contains(&&"Technology".to_string()));
        assert!(category_names.contains(&&"Programming".to_string()));
        
        // Test reverse traversal from category to posts
        let posts = tech_category.traverse::<Post>(&test_db.db, "posts").await.unwrap();
        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0].id, post.id);
    }
    
    #[tokio::test]
    async fn test_junction_table_operations() {
        let test_db = TestDatabase::new().await;
        
        let user = UserBuilder::new().save(&test_db.db).await.unwrap();
        let post = PostBuilder::for_user(user.id).save(&test_db.db).await.unwrap();
        let category = Category::create()
            .set_name("Test Category".to_string())
            .save(&test_db.db)
            .await
            .unwrap();
        
        // Test association creation
        let association = PostCategory::create()
            .set_post_id(post.id)
            .set_category_id(category.id)
            .set_created_at(Utc::now())
            .save(&test_db.db)
            .await
            .unwrap();
        
        assert_eq!(association.post_id, post.id);
        assert_eq!(association.category_id, category.id);
        
        // Test finding associations
        let found_associations = PostCategory::query()
            .where_post_id_eq(post.id)
            .all(&test_db.db)
            .await
            .unwrap();
        
        assert_eq!(found_associations.len(), 1);
        
        // Test association deletion
        PostCategory::delete(&test_db.db, association.id).await.unwrap();
        
        let remaining_associations = PostCategory::query()
            .where_post_id_eq(post.id)
            .all(&test_db.db)
            .await
            .unwrap();
        
        assert_eq!(remaining_associations.len(), 0);
    }
}
```

## Integration Testing

### Complete Workflow Tests

Test entire user workflows:

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::common::*;
    
    #[tokio::test]
    async fn test_blog_workflow() {
        let test_db = TestDatabase::new().await;
        
        // 1. Create a user
        let author = UserBuilder::new()
            .name("Jane Author")
            .email("jane@blogsite.com")
            .save(&test_db.db)
            .await
            .unwrap();
        
        // 2. Create categories
        let tech_category = Category::create()
            .set_name("Technology".to_string())
            .set_description(Some("Tech articles".to_string()))
            .save(&test_db.db)
            .await
            .unwrap();
        
        let tutorial_category = Category::create()
            .set_name("Tutorials".to_string())
            .save(&test_db.db)
            .await
            .unwrap();
        
        // 3. Author creates a draft post
        let draft_post = PostBuilder::for_user(author.id)
            .title("Learning Rust: A Beginner's Guide")
            .content("Rust is a systems programming language...")
            .save(&test_db.db)
            .await
            .unwrap();
        
        assert!(!draft_post.is_published);
        
        // 4. Associate post with categories
        PostCategory::create()
            .set_post_id(draft_post.id)
            .set_category_id(tech_category.id)
            .set_created_at(Utc::now())
            .save(&test_db.db)
            .await
            .unwrap();
        
        PostCategory::create()
            .set_post_id(draft_post.id)
            .set_category_id(tutorial_category.id)
            .set_created_at(Utc::now())
            .save(&test_db.db)
            .await
            .unwrap();
        
        // 5. Publish the post
        let published_post = Post::update(draft_post.id)
            .set_is_published(true)
            .save(&test_db.db)
            .await
            .unwrap();
        
        assert!(published_post.is_published);
        
        // 6. Verify author's published posts
        let author_published = Post::query()
            .where_user_id_eq(author.id)
            .where_is_published_eq(true)
            .all(&test_db.db)
            .await
            .unwrap();
        
        assert_eq!(author_published.len(), 1);
        
        // 7. Verify post categories
        let post_categories = published_post
            .traverse::<Category>(&test_db.db, "categories")
            .await
            .unwrap();
        
        assert_eq!(post_categories.len(), 2);
        
        // 8. Verify category posts
        let tech_posts = tech_category
            .traverse::<Post>(&test_db.db, "posts")
            .await
            .unwrap();
        
        assert_eq!(tech_posts.len(), 1);
        assert_eq!(tech_posts[0].id, published_post.id);
    }
    
    #[tokio::test]
    async fn test_user_content_management() {
        let test_db = TestDatabase::new().await;
        
        let user = UserBuilder::new().save(&test_db.db).await.unwrap();
        
        // Create multiple posts
        let published_count = 3;
        let draft_count = 2;
        
        for i in 1..=published_count {
            PostBuilder::for_user(user.id)
                .title(&format!("Published Post {}", i))
                .published()
                .save(&test_db.db)
                .await
                .unwrap();
        }
        
        for i in 1..=draft_count {
            PostBuilder::for_user(user.id)
                .title(&format!("Draft Post {}", i))
                .save(&test_db.db)
                .await
                .unwrap();
        }
        
        // Test content statistics
        let all_posts = user.traverse::<Post>(&test_db.db, "posts").await.unwrap();
        assert_eq!(all_posts.len(), (published_count + draft_count) as usize);
        
        let published_posts = Post::query()
            .where_user_id_eq(user.id)
            .where_is_published_eq(true)
            .all(&test_db.db)
            .await
            .unwrap();
        assert_eq!(published_posts.len(), published_count as usize);
        
        let draft_posts = Post::query()
            .where_user_id_eq(user.id)
            .where_is_published_eq(false)
            .all(&test_db.db)
            .await
            .unwrap();
        assert_eq!(draft_posts.len(), draft_count as usize);
    }
}
```

## Performance Testing

### Load Testing

Test performance with larger datasets:

```rust
#[cfg(test)]
mod performance_tests {
    use super::*;
    use crate::common::*;
    use std::time::Instant;
    
    #[tokio::test]
    async fn test_bulk_operations_performance() {
        let test_db = TestDatabase::new().await;
        
        let start = Instant::now();
        
        // Create 1000 users
        for i in 1..=1000 {
            UserBuilder::new()
                .name(&format!("User {}", i))
                .email(&format!("user{}@perf-test.com", i))
                .save(&test_db.db)
                .await
                .unwrap();
        }
        
        let creation_time = start.elapsed();
        println!("Created 1000 users in {:?}", creation_time);
        
        // Query performance test
        let query_start = Instant::now();
        
        let active_users = User::query()
            .where_is_active_eq(true)
            .order_by_name_asc()
            .limit(50)
            .all(&test_db.db)
            .await
            .unwrap();
        
        let query_time = query_start.elapsed();
        println!("Queried 50 users from 1000 in {:?}", query_time);
        
        assert_eq!(active_users.len(), 50);
        
        // Ensure reasonable performance (adjust thresholds as needed)
        assert!(creation_time.as_millis() < 5000); // 5 seconds
        assert!(query_time.as_millis() < 100); // 100ms
    }
    
    #[tokio::test]
    async fn test_relationship_traversal_performance() {
        let test_db = TestDatabase::new().await;
        
        // Create users and posts
        let mut users = Vec::new();
        for i in 1..=10 {
            let user = UserBuilder::new()
                .name(&format!("User {}", i))
                .email(&format!("user{}@test.com", i))
                .save(&test_db.db)
                .await
                .unwrap();
            users.push(user);
        }
        
        // Create 10 posts per user
        for user in &users {
            for j in 1..=10 {
                PostBuilder::for_user(user.id)
                    .title(&format!("Post {} by {}", j, user.name))
                    .published()
                    .save(&test_db.db)
                    .await
                    .unwrap();
            }
        }
        
        // Test traversal performance
        let start = Instant::now();
        
        for user in &users {
            let posts = user.traverse::<Post>(&test_db.db, "posts").await.unwrap();
            assert_eq!(posts.len(), 10);
        }
        
        let traversal_time = start.elapsed();
        println!("Traversed relationships for 10 users with 10 posts each in {:?}", traversal_time);
        
        // Should complete reasonably quickly
        assert!(traversal_time.as_millis() < 1000); // 1 second
    }
}
```

### Memory Usage Testing

Test memory efficiency:

```rust
#[cfg(test)]
mod memory_tests {
    use super::*;
    use crate::common::*;
    
    #[tokio::test]
    async fn test_large_result_set_handling() {
        let test_db = TestDatabase::new().await;
        
        // Create a large number of posts
        let user = UserBuilder::new().save(&test_db.db).await.unwrap();
        
        for i in 1..=1000 {
            PostBuilder::for_user(user.id)
                .title(&format!("Post {}", i))
                .content(&format!("Content for post number {}", i))
                .save(&test_db.db)
                .await
                .unwrap();
        }
        
        // Test pagination to avoid loading all at once
        let page_size = 50;
        let mut total_loaded = 0;
        let mut page = 0;
        
        loop {
            let posts = Post::query()
                .where_user_id_eq(user.id)
                .order_by_id_asc()
                .limit(page_size)
                .offset(page * page_size)
                .all(&test_db.db)
                .await
                .unwrap();
            
            if posts.is_empty() {
                break;
            }
            
            total_loaded += posts.len();
            page += 1;
            
            // Verify we're not loading too much at once
            assert!(posts.len() <= page_size as usize);
        }
        
        assert_eq!(total_loaded, 1000);
    }
}
```

## Error Handling Tests

Test error scenarios:

```rust
#[cfg(test)]
mod error_tests {
    use super::*;
    use crate::common::*;
    
    #[tokio::test]
    async fn test_constraint_violations() {
        let test_db = TestDatabase::new().await;
        
        // Test unique constraint violation
        UserBuilder::new()
            .email("duplicate@test.com")
            .save(&test_db.db)
            .await
            .unwrap();
        
        let result = UserBuilder::new()
            .email("duplicate@test.com")
            .save(&test_db.db)
            .await;
        
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_not_found_scenarios() {
        let test_db = TestDatabase::new().await;
        
        // Test finding non-existent user
        let result = User::find(&test_db.db, 999999).await.unwrap();
        assert!(result.is_none());
        
        // Test updating non-existent user
        let result = User::update(999999)
            .set_name("Updated".to_string())
            .save(&test_db.db)
            .await;
        
        assert!(result.is_err());
        
        // Test deleting non-existent user
        let result = User::delete(&test_db.db, 999999).await;
        // Note: Delete might succeed even if record doesn't exist
        // depending on SQLite behavior
    }
    
    #[tokio::test]
    async fn test_foreign_key_constraints() {
        let test_db = TestDatabase::new().await;
        
        // Try to create post with non-existent user
        let result = PostBuilder::for_user(999999)
            .save(&test_db.db)
            .await;
        
        // This should succeed in current implementation
        // but would fail with proper foreign key constraints
        // When FK constraints are enforced, uncomment:
        // assert!(result.is_err());
        
        assert!(result.is_ok()); // Current behavior
    }
}
```

## Test Organization

### Test Configuration

```rust
// tests/lib.rs
mod common;

mod unit {
    mod entity_tests;
    mod query_tests;
    mod boolean_conversion_tests;
}

mod integration {
    mod relationship_tests;
    mod workflow_tests;
    mod migration_tests;
}

mod performance {
    mod load_tests;
    mod memory_tests;
    mod query_performance_tests;
}

#[cfg(test)]
mod test_config {
    use super::*;
    
    // Global test setup
    fn init_test_logging() {
        // Initialize logging for tests
        env_logger::init();
    }
    
    // Test environment validation
    #[tokio::test]
    async fn test_environment_setup() {
        let db = d1_rs::D1Client::new_in_memory().await.unwrap();
        
        // Verify we can create tables
        let migration = d1_rs::SchemaMigration::new("test_setup".to_string())
            .create_table("test_table")
                .integer("id").primary_key().auto_increment().build()
                .text("name").not_null().build()
            .build();
        
        let result = migration.execute(&db).await;
        assert!(result.is_ok());
    }
}
```

## Continuous Integration

### GitHub Actions Configuration

```yaml
# .github/workflows/test.yml
name: Tests

on:
  push:
    branches: [ main, dev ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    runs-on: ubuntu-latest
    
    steps:
    - uses: actions/checkout@v4
    
    - name: Install Rust
      uses: actions-rust-lang/setup-rust-toolchain@v1
      with:
        toolchain: stable
    
    - name: Run tests
      run: cargo test --verbose
    
    - name: Run tests with all features
      run: cargo test --verbose --all-features
    
    - name: Run performance tests
      run: cargo test --verbose --release performance_tests
    
    - name: Check test coverage
      run: |
        cargo install cargo-tarpaulin
        cargo tarpaulin --verbose --all-features --timeout 120
```

## Best Practices

### Test Organization

1. **Separate unit and integration tests**
2. **Use descriptive test names**
3. **Create reusable test utilities**
4. **Test error scenarios**
5. **Include performance tests**

### Test Data Management

1. **Use builders for flexible test data creation**
2. **Clean slate for each test (in-memory DB)**
3. **Create minimal test data**
4. **Test with realistic data volumes**

### Assertions

1. **Test both positive and negative cases**
2. **Verify side effects**
3. **Check relationship consistency**
4. **Validate data integrity**

## Next Steps

- Learn about [Boolean Handling](./boolean-handling.md) for type conversion testing
- Explore [Performance Optimization](./performance.md) for production readiness
- Check out [Schema Evolution](../migrations.md) for migration testing