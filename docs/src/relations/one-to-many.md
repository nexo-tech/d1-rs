# One-to-Many Relations

One-to-many relations represent relationships where one record can be associated with multiple records in another table. With d1-rs's new type-safe API, these relationships are completely string-literal-free and validated at compile time.

## Basic One-to-Many Setup

### Entity Definitions

```rust
use d1_rs::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct User {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub email: String,
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct Post {
    #[primary_key]
    pub id: i64,
    pub user_id: i64, // Foreign key to users
    pub title: String,
    pub content: String,
    pub is_published: bool,
    pub created_at: DateTime<Utc>,
}
```

### Relations Definition

```rust
// Define relations with the type-safe macro - NO STRING LITERALS!
relations! {
    User {
        has_many posts: Post via user_id,
    }
    
    Post {
        belongs_to user: User via user_id,
    }
}
```

### Schema Migration

```rust
// Create tables and let d1-rs handle relation setup
let migration = SchemaMigration::new("create_blog_tables".to_string())
    .create_table("users")
        .integer("id").primary_key().auto_increment().build()
        .text("name").not_null().build()
        .text("email").not_null().unique().build()
        .boolean("is_active").default_value(DefaultValue::Boolean(true)).build()
    .build()
    
    .create_table("posts")
        .integer("id").primary_key().auto_increment().build()
        .integer("user_id").not_null().build() // Foreign key
        .text("title").not_null().build()
        .text("content").not_null().build()
        .boolean("is_published").default_value(DefaultValue::Boolean(false)).build()
        .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
    .build();

migration.execute(&db).await?;
```

## Using Type-Safe Association Methods

### From Parent to Children (One-to-Many)

```rust
// Get user's posts - completely type-safe!
let user = User::find(&db, 1).await?.unwrap();
let posts = user.posts().all(&db).await?;

println!("User {} has {} posts", user.name, posts.len());
for post in posts {
    println!("  - {}: {}", post.title, 
        if post.is_published { "Published" } else { "Draft" });
}

// Count posts without loading them
let post_count = user.posts().count(&db).await?;
println!("User has {} total posts", post_count);

// Get first/latest post
let latest_post = user.posts().first(&db).await?;
if let Some(post) = latest_post {
    println!("Latest post: {}", post.title);
}
```

### From Child to Parent (Belongs-to)

```rust
// Get post's author - type-safe!
let post = Post::find(&db, 1).await?.unwrap();
let author = post.user().first(&db).await?;

if let Some(user) = author {
    println!("Post '{}' was written by {}", post.title, user.name);
}
```

## Working with Loaded Data

Check if related data is already loaded to avoid unnecessary queries:

```rust
// Check if posts are already loaded
if let Some(loaded_posts) = user.posts().loaded() {
    println!("Already have {} posts loaded", loaded_posts.len());
    // Use loaded data without database query
    for post in loaded_posts.iter().take(3) {
        println!("  - {}", post.title);
    }
} else {
    // This will query the database
    let posts = user.posts().all(&db).await?;
    println!("{} has {} posts", user.name, posts.len());
}
```

*Note: Full eager loading functionality is planned for future releases.*

## Creating Related Records

### Creating Posts for a User

```rust
// Create user first
let user = User::create()
    .set_name("Alice Johnson".to_string())
    .set_email("alice@example.com".to_string())
    .set_is_active(true)
    .save(&db)
    .await?;

// Create multiple posts for the user
let post_data = vec![
    ("Getting Started with Rust", "Rust is an amazing systems programming language..."),
    ("Advanced Rust Patterns", "Let's explore some advanced concepts..."),
    ("Web Development with Rust", "Building web applications in Rust..."),
];

for (title, content) in post_data {
    Post::create()
        .set_user_id(user.id) // Set the foreign key
        .set_title(title.to_string())
        .set_content(content.to_string())
        .set_is_published(false)
        .set_created_at(Utc::now())
        .save(&db)
        .await?;
}

// Verify the relationship works
let user_posts = user.posts().all(&db).await?;
println!("Created user with {} posts", user_posts.len());
```

## Advanced Patterns with Association Methods

### Custom Helper Methods

Combine the type-safe association methods with custom logic:

```rust
impl User {
    // Get only published posts using association method + filtering
    pub async fn published_posts(&self, db: &D1Client) -> Result<Vec<Post>> {
        Post::query()
            .where_user_id_eq(self.id)
            .where_is_published_eq(true)
            .order_by_created_at_desc()
            .all(db)
            .await
    }
    
    // Get recent posts
    pub async fn recent_posts(&self, db: &D1Client, days: i64) -> Result<Vec<Post>> {
        let cutoff = Utc::now() - chrono::Duration::days(days);
        
        Post::query()
            .where_user_id_eq(self.id)
            .where_created_at_gte(cutoff)
            .order_by_created_at_desc()
            .all(db)
            .await
    }
    
    // Use association method for simple counting
    pub async fn post_count(&self, db: &D1Client) -> Result<i64> {
        self.posts().count(db).await
    }
    
    // Combine association with query filtering
    pub async fn published_post_count(&self, db: &D1Client) -> Result<i64> {
        Post::query()
            .where_user_id_eq(self.id)
            .where_is_published_eq(true)
            .count(db)
            .await
    }
    
    // Check if user has any posts (using association method)
    pub async fn has_posts(&self, db: &D1Client) -> Result<bool> {
        let count = self.posts().count(db).await?;
        Ok(count > 0)
    }
}
```

### Bulk Operations on Related Records

```rust
impl User {
    // Publish all user's draft posts
    pub async fn publish_all_drafts(&self, db: &D1Client) -> Result<Vec<Post>> {
        // Use association method to get related posts, then filter
        let all_posts = self.posts().all(db).await?;
        let drafts: Vec<_> = all_posts.into_iter()
            .filter(|p| !p.is_published)
            .collect();
        
        let mut published = Vec::new();
        for draft in drafts {
            let updated = Post::update(draft.id)
                .set_is_published(true)
                .save(db)
                .await?;
            published.push(updated);
        }
        
        Ok(published)
    }
    
    // Delete all user's posts using association method
    pub async fn delete_all_posts(&self, db: &D1Client) -> Result<i64> {
        let posts = self.posts().all(db).await?;
        let count = posts.len() as i64;
        
        for post in posts {
            Post::delete(db, post.id).await?;
        }
        
        Ok(count)
    }
    
    // Get post statistics using association methods
    pub async fn post_stats(&self, db: &D1Client) -> Result<(i64, i64, i64)> {
        let all_posts = self.posts().all(db).await?;
        let total = all_posts.len() as i64;
        let published = all_posts.iter().filter(|p| p.is_published).count() as i64;
        let drafts = total - published;
        
        Ok((total, published, drafts))
    }
}
```

## Self-Referencing Relations (Hierarchical Data)

### Categories with Subcategories

```rust
#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
#[table(name = "categories")]
pub struct Category {
    #[primary_key]
    pub id: i64,
    pub parent_id: Option<i64>, // Self-referencing foreign key
    pub name: String,
    pub description: Option<String>,
}

// Define self-referencing relations
relations! {
    Category {
        has_many subcategories: Category via parent_id,
        belongs_to parent: Category via parent_id,
    }
}

// Schema migration
let migration = SchemaMigration::new("create_categories".to_string())
    .create_table("categories")
        .integer("id").primary_key().auto_increment().build()
        .integer("parent_id").build() // Optional parent
        .text("name").not_null().build()
        .text("description").build()
    .build();

impl Category {
    // Get subcategories using type-safe association method
    pub async fn get_subcategories(&self, db: &D1Client) -> Result<Vec<Category>> {
        self.subcategories().all(db).await
    }
    
    // Get parent category using type-safe association method  
    pub async fn get_parent(&self, db: &D1Client) -> Result<Option<Category>> {
        self.parent().first(db).await
    }
    
    // Count direct children
    pub async fn child_count(&self, db: &D1Client) -> Result<i64> {
        self.subcategories().count(db).await
    }
    
    // Get root categories (no parent)
    pub async fn roots(db: &D1Client) -> Result<Vec<Category>> {
        Category::query()
            .where_parent_id_is_null()
            .order_by_name_asc()
            .all(db)
            .await
    }
    
    // Check if this category has children
    pub async fn has_children(&self, db: &D1Client) -> Result<bool> {
        let count = self.subcategories().count(db).await?;
        Ok(count > 0)
    }
}
```

## Performance Optimization

### Efficient Querying with Association Methods

```rust
// Prefer counting over loading all records
let post_count = user.posts().count(&db).await?; // Efficient
let posts = user.posts().all(&db).await?; // Less efficient for counting
let count = posts.len(); // Avoid this pattern

// Use first() when you only need one record
let latest_post = user.posts().first(&db).await?; // Efficient

// Check loaded data first
if let Some(loaded_posts) = user.posts().loaded() {
    // Use loaded data - no database query
    println!("User has {} posts", loaded_posts.len());
} else {
    // Query database only when needed
    let count = user.posts().count(&db).await?;
    println!("User has {} posts", count);
}
```

### Pagination with Association Context

```rust
impl User {
    // Paginated posts while maintaining association context
    pub async fn posts_paginated(
        &self, 
        db: &D1Client, 
        page: i64, 
        per_page: i64
    ) -> Result<Vec<Post>> {
        Post::query()
            .where_user_id_eq(self.id)
            .order_by_created_at_desc()
            .limit(per_page)
            .offset(page * per_page)
            .all(db)
            .await
    }
    
    // Get total pages for pagination UI
    pub async fn post_page_count(&self, db: &D1Client, per_page: i64) -> Result<i64> {
        let total = self.posts().count(db).await?;
        Ok((total + per_page - 1) / per_page) // Ceiling division
    }
}

// Usage
let user = User::find(&db, 1).await?.unwrap();
let page_1 = user.posts_paginated(&db, 0, 10).await?; // First 10 posts
let total_pages = user.post_page_count(&db, 10).await?;
println!("Page 1 of {} pages", total_pages);
```

### Working with Association Data

```rust
// Transform association data efficiently
#[derive(Debug, Serialize, Deserialize)]
pub struct PostSummary {
    pub id: i64,
    pub title: String,
    pub is_published: bool,
    pub created_at: DateTime<Utc>,
}

impl User {
    // Use association method then transform results
    pub async fn post_summaries(&self, db: &D1Client, limit: Option<i64>) -> Result<Vec<PostSummary>> {
        // Get posts using type-safe association method
        let posts = if let Some(limit_val) = limit {
            Post::query()
                .where_user_id_eq(self.id)
                .order_by_created_at_desc()
                .limit(limit_val)
                .all(db)
                .await?
        } else {
            self.posts().all(db).await?
        };
        
        let summaries = posts.into_iter().map(|p| PostSummary {
            id: p.id,
            title: p.title,
            is_published: p.is_published,
            created_at: p.created_at,
        }).collect();
        
        Ok(summaries)
    }
    
    // Quick check for user activity
    pub async fn is_active_blogger(&self, db: &D1Client) -> Result<bool> {
        let recent_posts = Post::query()
            .where_user_id_eq(self.id)
            .where_created_at_gte(Utc::now() - chrono::Duration::days(30))
            .count(db)
            .await?;
        
        Ok(recent_posts >= 3) // Active if 3+ posts in last 30 days
    }
}
```

## Common Patterns

### Multi-Level Blog System

```rust
#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct Blog {
    #[primary_key]
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub description: String,
    pub is_public: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct BlogPost {
    #[primary_key]
    pub id: i64,
    pub blog_id: i64,
    pub title: String,
    pub content: String,
    pub status: String, // "draft", "published", "archived"
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

// Define the complete relationship chain
relations! {
    User {
        has_many blogs: Blog via user_id,
    }
    
    Blog {
        belongs_to user: User via user_id,
        has_many posts: BlogPost via blog_id,
    }
    
    BlogPost {
        belongs_to blog: Blog via blog_id,
    }
}

impl Blog {
    // Use association method + filtering
    pub async fn published_posts(&self, db: &D1Client) -> Result<Vec<BlogPost>> {
        BlogPost::query()
            .where_blog_id_eq(self.id)
            .where_status_eq("published".to_string())
            .where_published_at_is_not_null()
            .order_by_published_at_desc()
            .all(db)
            .await
    }
    
    // Count posts using association method
    pub async fn total_posts(&self, db: &D1Client) -> Result<i64> {
        self.posts().count(db).await
    }
}

impl User {
    // Get all posts across all user's blogs
    pub async fn all_blog_posts(&self, db: &D1Client) -> Result<Vec<BlogPost>> {
        let user_blogs = self.blogs().all(db).await?;
        let mut all_posts = Vec::new();
        
        for blog in user_blogs {
            let mut blog_posts = blog.posts().all(db).await?;
            all_posts.append(&mut blog_posts);
        }
        
        Ok(all_posts)
    }
}
```

### E-commerce Order System

```rust
#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct Customer {
    #[primary_key]
    pub id: i64,
    pub email: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct Order {
    #[primary_key]
    pub id: i64,
    pub customer_id: i64,
    pub status: String,
    pub total: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct OrderItem {
    #[primary_key]
    pub id: i64,
    pub order_id: i64,
    pub product_name: String,
    pub quantity: i32,
    pub unit_price: f64,
}

// Define the complete relationship chain
relations! {
    Customer {
        has_many orders: Order via customer_id,
    }
    
    Order {
        belongs_to customer: Customer via customer_id,
        has_many items: OrderItem via order_id,
    }
    
    OrderItem {
        belongs_to order: Order via order_id,
    }
}

impl Customer {
    // Use association method for order history
    pub async fn order_history(&self, db: &D1Client) -> Result<Vec<Order>> {
        self.orders().all(db).await
    }
    
    pub async fn total_spent(&self, db: &D1Client) -> Result<f64> {
        let orders = self.orders().all(db).await?;
        Ok(orders.iter().map(|o| o.total).sum())
    }
    
    // Count orders using association method
    pub async fn order_count(&self, db: &D1Client) -> Result<i64> {
        self.orders().count(db).await
    }
    
    // Check if customer has any orders
    pub async fn has_orders(&self, db: &D1Client) -> Result<bool> {
        let count = self.orders().count(db).await?;
        Ok(count > 0)
    }
}

impl Order {
    // Use association method for order items
    pub async fn get_items(&self, db: &D1Client) -> Result<Vec<OrderItem>> {
        self.items().all(db).await
    }
    
    // Count items using association method
    pub async fn item_count(&self, db: &D1Client) -> Result<i64> {
        self.items().count(db).await
    }
    
    // Get order owner using association method
    pub async fn get_customer(&self, db: &D1Client) -> Result<Option<Customer>> {
        self.customer().first(db).await
    }
}
```

## Next Steps

- Learn about [Many-to-Many Relations](./many-to-many.md) for complex junction table relationships
- Explore [Advanced Relations](./advanced.md) for complex patterns and performance optimization  
- Check out [One-to-One Relations](./one-to-one.md) for unique associations

The type-safe association methods eliminate the need for string literals and provide compile-time safety that makes relationships much more reliable and maintainable.