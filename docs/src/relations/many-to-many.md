# Many-to-Many Relations

Many-to-many relations represent complex relationships where multiple records in one table can be associated with multiple records in another table. These are implemented using junction tables.

## Basic Many-to-Many

### Schema Definition

```rust
// Posts have many Categories (and Categories have many Posts)
let migration = SchemaMigration::new("create_post_categories".to_string())
    .create_table("posts")
        .integer("id").primary_key().auto_increment().build()
        .text("title").not_null().build()
        .text("content").not_null().build()
        .boolean("is_published").default_value(DefaultValue::Boolean(false)).build()
    .build()
    
    .create_table("categories")
        .integer("id").primary_key().auto_increment().build()
        .text("name").not_null().unique().build()
        .text("description").build()
    .build()
    
    // Junction table for many-to-many relationship
    .create_table("post_categories")
        .integer("id").primary_key().auto_increment().build()
        .integer("post_id").not_null().build()
        .integer("category_id").not_null().build()
        .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
    .build()
    
    // Define the many-to-many relation
    .create_relation("post_categories_rel", "posts", "categories")
        .many_to_many("post_categories", "post_id", "id", "category_id", "id")
    .build();

migration.execute(&db).await?;
```

### Entity Definitions

```rust
use d1_rs::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone, Entity, RelationalEntity, PartialEq)]
pub struct Post {
    #[primary_key]
    pub id: i64,
    pub title: String,
    pub content: String,
    pub is_published: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity, RelationalEntity, PartialEq)]
#[table(name = "categories")]
pub struct Category {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
}

// Junction table entity (optional - useful for additional data)
#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct PostCategory {
    #[primary_key]
    pub id: i64,
    pub post_id: i64,
    pub category_id: i64,
    pub created_at: DateTime<Utc>,
}
```

## Traversing Many-to-Many Relations

### From Post to Categories

```rust
// Get all categories for a post
let post = Post::find(&db, 1).await?.unwrap();
let categories = post.traverse::<Category>(&db, "categories").await?;

println!("Post '{}' is in {} categories:", post.title, categories.len());
for category in categories {
    println!("  - {}", category.name);
}
```

### From Category to Posts

```rust
// Get all posts in a category
let category = Category::find(&db, 1).await?.unwrap();
let posts = category.traverse::<Post>(&db, "posts").await?;

println!("Category '{}' has {} posts:", category.name, posts.len());
for post in posts {
    println!("  - {}", post.title);
}
```

## Eager Loading

Load posts with their categories efficiently:

```rust
// Load posts with all their categories
let posts_with_categories = Post::query()
    .with(vec!["categories"])
    .all(&db)
    .await?;

for post in posts_with_categories {
    let categories = post.traverse::<Category>(&db, "categories").await?;
    
    let category_names: Vec<String> = categories.iter()
        .map(|c| c.name.clone())
        .collect();
    
    println!("{}: [{}]", post.title, category_names.join(", "));
}
```

## Managing Associations

### Adding Categories to Posts

```rust
impl Post {
    // Add a category to this post
    pub async fn add_category(&self, db: &D1Client, category_id: i64) -> Result<PostCategory> {
        // Check if association already exists
        let existing = PostCategory::query()
            .where_post_id_eq(self.id)
            .where_category_id_eq(category_id)
            .first(db)
            .await?;
        
        if existing.is_some() {
            return Err(D1RsError::Database("Category already associated with post".to_string()));
        }
        
        PostCategory::create()
            .set_post_id(self.id)
            .set_category_id(category_id)
            .set_created_at(Utc::now())
            .save(db)
            .await
    }
    
    // Remove a category from this post
    pub async fn remove_category(&self, db: &D1Client, category_id: i64) -> Result<bool> {
        let association = PostCategory::query()
            .where_post_id_eq(self.id)
            .where_category_id_eq(category_id)
            .first(db)
            .await?;
        
        if let Some(assoc) = association {
            PostCategory::delete(db, assoc.id).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    // Set all categories for this post (replaces existing)
    pub async fn set_categories(&self, db: &D1Client, category_ids: Vec<i64>) -> Result<()> {
        // Remove all existing associations
        let existing = PostCategory::query()
            .where_post_id_eq(self.id)
            .all(db)
            .await?;
        
        for assoc in existing {
            PostCategory::delete(db, assoc.id).await?;
        }
        
        // Add new associations
        for category_id in category_ids {
            self.add_category(db, category_id).await?;
        }
        
        Ok(())
    }
    
    // Get category IDs (useful for forms/APIs)
    pub async fn category_ids(&self, db: &D1Client) -> Result<Vec<i64>> {
        let associations = PostCategory::query()
            .where_post_id_eq(self.id)
            .all(db)
            .await?;
        
        Ok(associations.into_iter().map(|a| a.category_id).collect())
    }
}
```

### Category Management

```rust
impl Category {
    // Add a post to this category
    pub async fn add_post(&self, db: &D1Client, post_id: i64) -> Result<PostCategory> {
        let post = Post::find(db, post_id).await?
            .ok_or(D1RsError::NotFound)?;
        
        post.add_category(db, self.id).await
    }
    
    // Get posts in this category with filtering
    pub async fn published_posts(&self, db: &D1Client) -> Result<Vec<Post>> {
        let associations = PostCategory::query()
            .where_category_id_eq(self.id)
            .all(db)
            .await?;
        
        let mut posts = Vec::new();
        for assoc in associations {
            if let Some(post) = Post::find(db, assoc.post_id).await? {
                if post.is_published {
                    posts.push(post);
                }
            }
        }
        
        Ok(posts)
    }
    
    // Get post count in category
    pub async fn post_count(&self, db: &D1Client) -> Result<i64> {
        PostCategory::query()
            .where_category_id_eq(self.id)
            .count(db)
            .await
    }
    
    // Get categories ordered by post count
    pub async fn popular_categories(db: &D1Client, limit: i64) -> Result<Vec<(Category, i64)>> {
        let categories = Category::query().all(db).await?;
        let mut category_counts = Vec::new();
        
        for category in categories {
            let count = category.post_count(db).await?;
            category_counts.push((category, count));
        }
        
        // Sort by count descending
        category_counts.sort_by(|a, b| b.1.cmp(&a.1));
        category_counts.truncate(limit as usize);
        
        Ok(category_counts)
    }
}
```

## Advanced Patterns

### Tagged System

```rust
#[derive(Entity, RelationalEntity)]
pub struct Article {
    #[primary_key]
    pub id: i64,
    pub title: String,
    pub content: String,
    pub author: String,
    pub published_at: DateTime<Utc>,
}

#[derive(Entity, RelationalEntity)]
pub struct Tag {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub color: String, // Hex color for UI
    pub usage_count: i64, // Denormalized count for performance
}

#[derive(Entity)]
pub struct ArticleTag {
    #[primary_key]
    pub id: i64,
    pub article_id: i64,
    pub tag_id: i64,
    pub added_by: String, // Who tagged it
    pub created_at: DateTime<Utc>,
}

impl Article {
    // Get related articles based on shared tags
    pub async fn related_articles(&self, db: &D1Client, limit: i64) -> Result<Vec<Article>> {
        let my_tags = self.traverse::<Tag>(db, "tags").await?;
        let tag_ids: Vec<i64> = my_tags.iter().map(|t| t.id).collect();
        
        if tag_ids.is_empty() {
            return Ok(Vec::new());
        }
        
        // Find articles that share tags (excluding self)
        let mut related_scores: std::collections::HashMap<i64, usize> = std::collections::HashMap::new();
        
        for tag_id in tag_ids {
            let tag_articles = ArticleTag::query()
                .where_tag_id_eq(tag_id)
                .all(db)
                .await?;
            
            for article_tag in tag_articles {
                if article_tag.article_id != self.id {
                    *related_scores.entry(article_tag.article_id).or_insert(0) += 1;
                }
            }
        }
        
        // Sort by shared tag count and load articles
        let mut scored_articles: Vec<_> = related_scores.into_iter().collect();
        scored_articles.sort_by(|a, b| b.1.cmp(&a.1));
        
        let mut related = Vec::new();
        for (article_id, _score) in scored_articles.into_iter().take(limit as usize) {
            if let Some(article) = Article::find(db, article_id).await? {
                related.push(article);
            }
        }
        
        Ok(related)
    }
}

impl Tag {
    // Update usage count when tags are added/removed
    pub async fn increment_usage(&self, db: &D1Client) -> Result<Tag> {
        Tag::update(self.id)
            .set_usage_count(self.usage_count + 1)
            .save(db)
            .await
    }
    
    pub async fn decrement_usage(&self, db: &D1Client) -> Result<Tag> {
        let new_count = std::cmp::max(0, self.usage_count - 1);
        Tag::update(self.id)
            .set_usage_count(new_count)
            .save(db)
            .await
    }
    
    // Get most popular tags
    pub async fn most_popular(db: &D1Client, limit: i64) -> Result<Vec<Tag>> {
        Tag::query()
            .order_by_usage_count_desc()
            .limit(limit)
            .all(db)
            .await
    }
}
```

### User Permissions System

```rust
#[derive(Entity, RelationalEntity)]
pub struct User {
    #[primary_key]
    pub id: i64,
    pub username: String,
    pub email: String,
    pub is_active: bool,
}

#[derive(Entity, RelationalEntity)]
pub struct Role {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub description: String,
}

#[derive(Entity, RelationalEntity)]
pub struct Permission {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub resource: String,
    pub action: String, // "read", "write", "delete", etc.
}

#[derive(Entity)]
pub struct UserRole {
    #[primary_key]
    pub id: i64,
    pub user_id: i64,
    pub role_id: i64,
    pub granted_at: DateTime<Utc>,
    pub granted_by: i64,
}

#[derive(Entity)]
pub struct RolePermission {
    #[primary_key]
    pub id: i64,
    pub role_id: i64,
    pub permission_id: i64,
}

impl User {
    // Get all permissions for a user (through roles)
    pub async fn all_permissions(&self, db: &D1Client) -> Result<Vec<Permission>> {
        let roles = self.traverse::<Role>(db, "roles").await?;
        let mut permissions = Vec::new();
        
        for role in roles {
            let role_permissions = role.traverse::<Permission>(db, "permissions").await?;
            permissions.extend(role_permissions);
        }
        
        // Remove duplicates
        permissions.sort_by_key(|p| p.id);
        permissions.dedup_by_key(|p| p.id);
        
        Ok(permissions)
    }
    
    // Check if user has specific permission
    pub async fn has_permission(&self, db: &D1Client, resource: &str, action: &str) -> Result<bool> {
        let permissions = self.all_permissions(db).await?;
        
        Ok(permissions.iter().any(|p| p.resource == resource && p.action == action))
    }
    
    // Add role to user
    pub async fn add_role(&self, db: &D1Client, role_id: i64, granted_by: i64) -> Result<UserRole> {
        UserRole::create()
            .set_user_id(self.id)
            .set_role_id(role_id)
            .set_granted_at(Utc::now())
            .set_granted_by(granted_by)
            .save(db)
            .await
    }
}
```

## Performance Considerations

### Junction Table Optimization

```rust
// Optimize junction table with compound indexes
let migration = SchemaMigration::new("optimize_post_categories".to_string())
    .alter_table("post_categories")
        // Compound index for foreign key pair (prevents duplicates efficiently)
        .add_unique_index("idx_post_category_unique", vec!["post_id", "category_id"])
        // Individual indexes for reverse lookups
        .add_index("idx_post_categories_post_id", vec!["post_id"])
        .add_index("idx_post_categories_category_id", vec!["category_id"])
        // Index on created_at for temporal queries
        .add_index("idx_post_categories_created_at", vec!["created_at"])
    .build();
```

### Batch Operations

```rust
impl Post {
    // Efficiently set multiple categories at once
    pub async fn set_categories_batch(&self, db: &D1Client, category_ids: Vec<i64>) -> Result<()> {
        // Remove existing associations in batch
        let existing = PostCategory::query()
            .where_post_id_eq(self.id)
            .all(db)
            .await?;
        
        // In a real implementation with transactions:
        // db.transaction(|tx| async {
        //     for assoc in existing {
        //         PostCategory::delete(&tx, assoc.id).await?;
        //     }
        //     
        //     for category_id in category_ids {
        //         PostCategory::create()
        //             .set_post_id(self.id)
        //             .set_category_id(category_id)
        //             .set_created_at(Utc::now())
        //             .save(&tx)
        //             .await?;
        //     }
        //     Ok(())
        // }).await?;
        
        // Current implementation (without transactions)
        for assoc in existing {
            PostCategory::delete(db, assoc.id).await?;
        }
        
        for category_id in category_ids {
            PostCategory::create()
                .set_post_id(self.id)
                .set_category_id(category_id)
                .set_created_at(Utc::now())
                .save(db)
                .await?;
        }
        
        Ok(())
    }
}
```

### Denormalization for Performance

```rust
// Add counts to improve query performance
#[derive(Entity, RelationalEntity)]
pub struct OptimizedCategory {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub post_count: i64,        // Denormalized count
    pub published_post_count: i64, // Count of published posts only
    pub last_updated: DateTime<Utc>,
}

impl OptimizedCategory {
    // Recalculate and update counts
    pub async fn refresh_counts(&self, db: &D1Client) -> Result<OptimizedCategory> {
        let total_count = PostCategory::query()
            .where_category_id_eq(self.id)
            .count(db)
            .await?;
        
        // Count published posts
        let associations = PostCategory::query()
            .where_category_id_eq(self.id)
            .all(db)
            .await?;
        
        let mut published_count = 0;
        for assoc in associations {
            if let Some(post) = Post::find(db, assoc.post_id).await? {
                if post.is_published {
                    published_count += 1;
                }
            }
        }
        
        OptimizedCategory::update(self.id)
            .set_post_count(total_count)
            .set_published_post_count(published_count)
            .set_last_updated(Utc::now())
            .save(db)
            .await
    }
}
```

## Common Use Cases

### Content Management System

```rust
// CMS with pages, authors, and tags
#[derive(Entity, RelationalEntity)]
pub struct Page {
    #[primary_key] pub id: i64,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub status: String, // "draft", "published", "archived"
}

#[derive(Entity, RelationalEntity)]
pub struct Author {
    #[primary_key] pub id: i64,
    pub name: String,
    pub email: String,
}

#[derive(Entity, RelationalEntity)]
pub struct ContentTag {
    #[primary_key] pub id: i64,
    pub name: String,
    pub category: String, // "topic", "format", "difficulty", etc.
}

// Junction tables for many-to-many relationships
#[derive(Entity)] pub struct PageAuthor { /* ... */ }
#[derive(Entity)] pub struct PageTag { /* ... */ }
```

### E-learning Platform

```rust
#[derive(Entity, RelationalEntity)]
pub struct Student {
    #[primary_key] pub id: i64,
    pub name: String,
    pub email: String,
}

#[derive(Entity, RelationalEntity)]
pub struct Course {
    #[primary_key] pub id: i64,
    pub title: String,
    pub description: String,
}

#[derive(Entity)]
pub struct Enrollment {
    #[primary_key] pub id: i64,
    pub student_id: i64,
    pub course_id: i64,
    pub enrolled_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub grade: Option<f64>,
}

impl Student {
    pub async fn enroll_in_course(&self, db: &D1Client, course_id: i64) -> Result<Enrollment> {
        Enrollment::create()
            .set_student_id(self.id)
            .set_course_id(course_id)
            .set_enrolled_at(Utc::now())
            .save(db)
            .await
    }
    
    pub async fn completed_courses(&self, db: &D1Client) -> Result<Vec<Course>> {
        let completed_enrollments = Enrollment::query()
            .where_student_id_eq(self.id)
            .where_completed_at_is_not_null()
            .all(db)
            .await?;
        
        let mut courses = Vec::new();
        for enrollment in completed_enrollments {
            if let Some(course) = Course::find(db, enrollment.course_id).await? {
                courses.push(course);
            }
        }
        
        Ok(courses)
    }
}
```

## Next Steps

- Learn about [Advanced Relations](./advanced.md) for complex graph traversal and optimization
- Explore [Performance Optimization](../advanced/performance.md) for scaling many-to-many relationships
- Check out [Testing Relations](../advanced/testing.md) for comprehensive relationship testing