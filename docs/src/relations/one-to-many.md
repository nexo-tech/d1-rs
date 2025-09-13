# One-to-Many Relations

One-to-many relations represent relationships where one record can be associated with multiple records in another table. This is the most common type of database relationship.

## Basic One-to-Many

### Schema Definition

```rust
// User has many Posts
let migration = SchemaMigration::new("create_user_posts_relation".to_string())
    .create_table("users")
        .integer("id").primary_key().auto_increment().build()
        .text("name").not_null().build()
        .text("email").not_null().unique().build()
        .boolean("is_active").default_value(DefaultValue::Boolean(true)).build()
    .build()
    
    .create_table("posts")
        .integer("id").primary_key().auto_increment().build()
        .integer("user_id").not_null().build() // Foreign key to users
        .text("title").not_null().build()
        .text("content").not_null().build()
        .boolean("is_published").default_value(DefaultValue::Boolean(false)).build()
        .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
    .build()
    
    // Define the relation
    .create_relation("user_posts", "users", "posts")
        .one_to_many("user_id", "id")
    .build();

migration.execute(&db).await?;
```

### Entity Definitions

```rust
use d1_rs::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone, Entity, RelationalEntity, PartialEq)]
pub struct User {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub email: String,
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity, RelationalEntity, PartialEq)]
pub struct Post {
    #[primary_key]
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub content: String,
    pub is_published: bool,
    pub created_at: DateTime<Utc>,
}
```

## Traversing One-to-Many Relations

### From Parent to Children

```rust
// Get user's posts
let user = User::find(&db, 1).await?.unwrap();
let posts = user.traverse::<Post>(&db, "posts").await?;

println!("User {} has {} posts", user.name, posts.len());
for post in posts {
    println!("  - {}: {}", post.title, 
        if post.is_published { "Published" } else { "Draft" });
}
```

### From Child to Parent

```rust
// Get post's author
let post = Post::find(&db, 1).await?.unwrap();
let users = post.traverse::<User>(&db, "user").await?;

if let Some(author) = users.first() {
    println!("Post '{}' was written by {}", post.title, author.name);
}
```

## Eager Loading

Load users with all their posts efficiently:

```rust
// Load users with their posts
let users_with_posts = User::query()
    .with(vec!["posts"])
    .all(&db)
    .await?;

for user in users_with_posts {
    let posts = user.traverse::<Post>(&db, "posts").await?;
    println!("{} has {} posts", user.name, posts.len());
    
    for post in posts.iter().take(3) { // Show first 3 posts
        println!("  - {}", post.title);
    }
}
```

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
let posts = vec![
    ("Getting Started with Rust", "Rust is an amazing systems programming language..."),
    ("Advanced Rust Patterns", "Let's explore some advanced concepts..."),
    ("Web Development with Rust", "Building web applications in Rust..."),
];

for (title, content) in posts {
    Post::create()
        .set_user_id(user.id)
        .set_title(title.to_string())
        .set_content(content.to_string())
        .set_is_published(false) // Start as drafts
        .set_created_at(Utc::now())
        .save(&db)
        .await?;
}

println!("Created user with {} posts", 3);
```

## Advanced Query Patterns

### Filtering Related Records

```rust
impl User {
    // Get only published posts
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
    
    // Get post count
    pub async fn post_count(&self, db: &D1Client) -> Result<i64> {
        Post::query()
            .where_user_id_eq(self.id)
            .count(db)
            .await
    }
    
    // Get published post count
    pub async fn published_post_count(&self, db: &D1Client) -> Result<i64> {
        Post::query()
            .where_user_id_eq(self.id)
            .where_is_published_eq(true)
            .count(db)
            .await
    }
}
```

### Bulk Operations on Related Records

```rust
impl User {
    // Publish all user's draft posts
    pub async fn publish_all_drafts(&self, db: &D1Client) -> Result<Vec<Post>> {
        let drafts = Post::query()
            .where_user_id_eq(self.id)
            .where_is_published_eq(false)
            .all(db)
            .await?;
        
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
    
    // Delete all user's posts
    pub async fn delete_all_posts(&self, db: &D1Client) -> Result<i64> {
        let posts = Post::query()
            .where_user_id_eq(self.id)
            .all(db)
            .await?;
        
        let count = posts.len() as i64;
        for post in posts {
            Post::delete(db, post.id).await?;
        }
        
        Ok(count)
    }
}
```

## Complex Hierarchical Relations

### Categories and Subcategories

```rust
#[derive(Debug, Serialize, Deserialize, Clone, Entity, RelationalEntity, PartialEq)]
#[table(name = "categories")]
pub struct Category {
    #[primary_key]
    pub id: i64,
    pub parent_id: Option<i64>, // Self-referencing foreign key
    pub name: String,
    pub description: Option<String>,
}

// Migration for hierarchical categories
let migration = SchemaMigration::new("create_categories".to_string())
    .create_table("categories")
        .integer("id").primary_key().auto_increment().build()
        .integer("parent_id").build() // Optional parent
        .text("name").not_null().build()
        .text("description").build()
    .build()
    
    // Self-referencing relation
    .create_relation("category_subcategories", "categories", "categories")
        .one_to_many("parent_id", "id")
    .build();

impl Category {
    // Get subcategories
    pub async fn subcategories(&self, db: &D1Client) -> Result<Vec<Category>> {
        Category::query()
            .where_parent_id_eq(self.id)
            .order_by_name_asc()
            .all(db)
            .await
    }
    
    // Get parent category
    pub async fn parent(&self, db: &D1Client) -> Result<Option<Category>> {
        if let Some(parent_id) = self.parent_id {
            Category::find(db, parent_id).await
        } else {
            Ok(None)
        }
    }
    
    // Get root categories (no parent)
    pub async fn roots(db: &D1Client) -> Result<Vec<Category>> {
        Category::query()
            .where_parent_id_is_null()
            .order_by_name_asc()
            .all(db)
            .await
    }
    
    // Get all descendants recursively
    pub async fn all_descendants(&self, db: &D1Client) -> Result<Vec<Category>> {
        let mut all_descendants = Vec::new();
        let mut to_process = vec![self.id];
        
        while let Some(current_id) = to_process.pop() {
            let children = Category::query()
                .where_parent_id_eq(current_id)
                .all(db)
                .await?;
            
            for child in children {
                to_process.push(child.id);
                all_descendants.push(child);
            }
        }
        
        Ok(all_descendants)
    }
}
```

## Performance Optimization

### Indexing Strategy

```rust
// Add indexes for foreign keys and commonly queried fields
let migration = SchemaMigration::new("add_post_indexes".to_string())
    .alter_table("posts")
        .add_index("idx_posts_user_id", vec!["user_id"])
        .add_index("idx_posts_published", vec!["is_published"])
        .add_index("idx_posts_created_at", vec!["created_at"])
        .add_index("idx_posts_user_published", vec!["user_id", "is_published"])
    .build();
```

### Pagination for Large Result Sets

```rust
impl User {
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
}

// Usage
let user = User::find(&db, 1).await?.unwrap();
let page_1 = user.posts_paginated(&db, 0, 10).await?; // First 10 posts
let page_2 = user.posts_paginated(&db, 1, 10).await?; // Next 10 posts
```

### Selective Loading

```rust
// Load only essential post data
#[derive(Debug, Serialize, Deserialize)]
pub struct PostSummary {
    pub id: i64,
    pub title: String,
    pub is_published: bool,
    pub created_at: DateTime<Utc>,
}

impl User {
    // This would require raw SQL support (future feature)
    pub async fn post_summaries(&self, db: &D1Client) -> Result<Vec<PostSummary>> {
        // For now, use regular query and transform
        let posts = Post::query()
            .where_user_id_eq(self.id)
            .order_by_created_at_desc()
            .limit(50) // Limit to avoid loading too much data
            .all(db)
            .await?;
        
        let summaries = posts.into_iter().map(|p| PostSummary {
            id: p.id,
            title: p.title,
            is_published: p.is_published,
            created_at: p.created_at,
        }).collect();
        
        Ok(summaries)
    }
}
```

## Common Patterns

### Blog System

```rust
#[derive(Entity, RelationalEntity)]
pub struct Blog {
    #[primary_key]
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub description: String,
    pub is_public: bool,
}

#[derive(Entity, RelationalEntity)]  
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

impl Blog {
    pub async fn published_posts(&self, db: &D1Client) -> Result<Vec<BlogPost>> {
        BlogPost::query()
            .where_blog_id_eq(self.id)
            .where_status_eq("published".to_string())
            .where_published_at_is_not_null()
            .order_by_published_at_desc()
            .all(db)
            .await
    }
}
```

### E-commerce Order System

```rust
#[derive(Entity, RelationalEntity)]
pub struct Customer {
    #[primary_key]
    pub id: i64,
    pub email: String,
    pub name: String,
}

#[derive(Entity, RelationalEntity)]
pub struct Order {
    #[primary_key]
    pub id: i64,
    pub customer_id: i64,
    pub status: String,
    pub total: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Entity, RelationalEntity)]
pub struct OrderItem {
    #[primary_key]
    pub id: i64,
    pub order_id: i64,
    pub product_name: String,
    pub quantity: i32,
    pub unit_price: f64,
}

impl Customer {
    pub async fn order_history(&self, db: &D1Client) -> Result<Vec<Order>> {
        Order::query()
            .where_customer_id_eq(self.id)
            .order_by_created_at_desc()
            .all(db)
            .await
    }
    
    pub async fn total_spent(&self, db: &D1Client) -> Result<f64> {
        let orders = self.order_history(db).await?;
        Ok(orders.iter().map(|o| o.total).sum())
    }
}

impl Order {
    pub async fn items(&self, db: &D1Client) -> Result<Vec<OrderItem>> {
        OrderItem::query()
            .where_order_id_eq(self.id)
            .all(db)
            .await
    }
}
```

## Next Steps

- Learn about [Many-to-Many Relations](./many-to-many.md) for complex associations
- Explore [Advanced Relations](./advanced.md) for graph traversal and complex queries
- Check out [Performance Optimization](../advanced/performance.md) for scaling large datasets