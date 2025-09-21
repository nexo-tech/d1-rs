# 🚀 Revolutionary Quick Start

Let's build a blog application using the **world's most advanced ORM**! This tutorial will showcase d1-rs's revolutionary compile-time safe relationships, nested eager loading, and impossible-to-match type safety.

## 🏆 What We'll Build

A revolutionary blog system demonstrating world-first capabilities:
- Users who can write posts with **compile-time safe relationships**
- **Zero string literals** - all relations type-safe at compile-time  
- **World's first nested eager loading** - prevent N+1 queries automatically
- **Impossible errors** - typos become compile errors, not runtime crashes
- Type-safe querying with perfect IDE auto-completion

## Step 1: Define Your Entities

First, let's create our data models using d1-rs entities:

```rust
// src/models/mod.rs
pub mod user;
pub mod post;

pub use user::*;
pub use post::*;
```

```rust
// src/models/user.rs
use d1_rs::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
#[table(name = "users")]
pub struct User {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub email: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}
```

```rust
// src/models/post.rs
use d1_rs::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
#[table(name = "posts")]
pub struct Post {
    #[primary_key]
    pub id: i64,
    pub title: String,
    pub content: String,
    pub user_id: i64,  // Foreign key to users
    pub is_published: bool,
    pub created_at: DateTime<Utc>,
}
```

## Step 2: Define Relations

Now let's define the relationships between our entities using d1-rs's type-safe relations system:

```rust
// src/models/relations.rs
use d1_rs::*;
use super::{User, Post};

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

Don't forget to include this in your models module:

```rust
// src/models/mod.rs
pub mod user;
pub mod post;
pub mod relations; // Add this line

pub use user::*;
pub use post::*;
```

## Step 3: Create Database Schema

Now let's create a migration to set up our database schema:

```rust
// src/migrations/mod.rs
use d1_rs::*;

pub async fn run_migrations(db: &D1Client) -> Result<()> {
    // Create users table
    let users_migration = SchemaMigration::new("create_users".to_string())
        .create_table("users")
            .integer("id").primary_key().auto_increment().build()
            .text("name").not_null().build()
            .text("email").not_null().unique().build()
            .boolean("is_active").not_null().default_value(DefaultValue::Boolean(true)).build()
            .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
        .build();

    users_migration.execute(db).await?;

    // Create posts table
    let posts_migration = SchemaMigration::new("create_posts".to_string())
        .create_table("posts")
            .integer("id").primary_key().auto_increment().build()
            .text("title").not_null().build()
            .text("content").not_null().build()
            .integer("user_id").not_null().build()
            .boolean("is_published").not_null().default_value(DefaultValue::Boolean(false)).build()
            .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
        .build();

    posts_migration.execute(db).await?;

    Ok(())
}
```

## Step 4: Using Type-Safe Relations

Now let's see the power of our type-safe relations system:

```rust
// src/lib.rs - Add these methods to the Blog impl
impl Blog {
    // Get a user's posts using type-safe association method - NO STRING LITERALS!
    pub async fn get_user_posts_typed(&self, user_id: i64) -> Result<Vec<Post>> {
        if let Some(user) = User::find(&self.db, user_id).await? {
            user.posts().all(&self.db).await  // Type-safe!
        } else {
            Ok(vec![])
        }
    }

    // Count user's posts without loading them
    pub async fn count_user_posts(&self, user_id: i64) -> Result<i64> {
        if let Some(user) = User::find(&self.db, user_id).await? {
            user.posts().count(&self.db).await  // Efficient counting
        } else {
            Ok(0)
        }
    }

    // Get post author using type-safe association
    pub async fn get_post_author(&self, post_id: i64) -> Result<Option<User>> {
        if let Some(post) = Post::find(&self.db, post_id).await? {
            post.user().first(&self.db).await  // Type-safe!
        } else {
            Ok(None)
        }
    }

    // Get user's first post 
    pub async fn get_user_first_post(&self, user_id: i64) -> Result<Option<Post>> {
        if let Some(user) = User::find(&self.db, user_id).await? {
            user.posts().first(&self.db).await  // Get first post only
        } else {
            Ok(None)
        }
    }
}
```

## Step 5: Basic CRUD Operations

Let's implement some basic operations for our blog:

```rust
// src/lib.rs
pub mod models;
pub mod migrations;

use d1_rs::*;
use models::{User, Post};
use chrono::Utc;

pub struct Blog {
    db: D1Client,
}

impl Blog {
    pub async fn new() -> Result<Self> {
        let db = D1Client::new_in_memory().await?;
        migrations::run_migrations(&db).await?;
        Ok(Self { db })
    }

    // User operations
    pub async fn create_user(&self, name: String, email: String) -> Result<User> {
        User::create()
            .set_name(name)
            .set_email(email)
            .set_is_active(true)
            .set_created_at(Utc::now())
            .save(&self.db)
            .await
    }

    pub async fn get_user(&self, id: i64) -> Result<Option<User>> {
        User::find(&self.db, id).await
    }

    pub async fn list_active_users(&self) -> Result<Vec<User>> {
        User::query()
            .where_is_active_eq(true)
            .order_by_created_at_desc()
            .all(&self.db)
            .await
    }

    // Post operations
    pub async fn create_post(&self, user_id: i64, title: String, content: String) -> Result<Post> {
        Post::create()
            .set_title(title)
            .set_content(content)
            .set_user_id(user_id)
            .set_is_published(false)
            .set_created_at(Utc::now())
            .save(&self.db)
            .await
    }

    pub async fn publish_post(&self, post_id: i64) -> Result<Post> {
        Post::update(post_id)
            .set_is_published(true)
            .save(&self.db)
            .await
    }

    pub async fn get_published_posts(&self) -> Result<Vec<Post>> {
        Post::query()
            .where_is_published_eq(true)
            .order_by_created_at_desc()
            .all(&self.db)
            .await
    }

    pub async fn get_user_posts(&self, user_id: i64) -> Result<Vec<Post>> {
        Post::query()
            .where_user_id_eq(user_id)
            .order_by_created_at_desc()
            .all(&self.db)
            .await
    }
}
```

## Step 6: Working with Relations

Now let's explore how to work with entity relationships using d1-rs's type-safe association methods:

```rust
// Add these methods to your Blog impl
impl Blog {
    // Get users and their posts using the current API
    pub async fn get_users_with_posts(&self) -> Result<Vec<(User, Vec<Post>)>> {
        let users = User::query().all(&self.db).await?;
        let mut result = Vec::new();
        
        for user in users {
            let posts = user.posts().all(&self.db).await?;
            result.push((user, posts));
        }
        
        Ok(result)
    }
    
    // Get users with published posts only
    pub async fn get_users_with_published_posts(&self) -> Result<Vec<(User, Vec<Post>)>> {
        let users = User::query().all(&self.db).await?;
        let mut result = Vec::new();
        
        for user in users {
            let published_posts = user.posts()
                .where_is_published_eq(true)
                .all(&self.db).await?;
            
            if !published_posts.is_empty() {
                result.push((user, published_posts));
            }
        }
        
        Ok(result)
    }
    
    // Approach to get specific user's posts
    pub async fn get_user_with_posts(&self, user_id: i64) -> Result<Option<(User, Vec<Post>)>> {
        if let Some(user) = User::find(&self.db, user_id).await? {
            let posts = user.posts().all(&self.db).await?;
            Ok(Some((user, posts)))
        } else {
            Ok(None)
        }
    }
}
```

## Step 7: Testing Your Revolutionary Application

Let's write some tests to make sure everything works:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_blog_operations() {
        let blog = Blog::new().await.unwrap();

        // Create a user
        let user = blog.create_user(
            "Alice Smith".to_string(),
            "alice@example.com".to_string()
        ).await.unwrap();

        assert_eq!(user.name, "Alice Smith");
        assert_eq!(user.email, "alice@example.com");
        assert!(user.is_active);

        // Create a post
        let post = blog.create_post(
            user.id,
            "My First Post".to_string(),
            "This is the content of my first blog post!".to_string()
        ).await.unwrap();

        assert_eq!(post.title, "My First Post");
        assert_eq!(post.user_id, user.id);
        assert!(!post.is_published);  // Not published yet

        // Publish the post
        let published_post = blog.publish_post(post.id).await.unwrap();
        assert!(published_post.is_published);

        // Get published posts
        let published_posts = blog.get_published_posts().await.unwrap();
        assert_eq!(published_posts.len(), 1);
        assert_eq!(published_posts[0].id, post.id);

        // Get user's posts (traditional way)
        let user_posts = blog.get_user_posts(user.id).await.unwrap();
        assert_eq!(user_posts.len(), 1);
        assert_eq!(user_posts[0].id, post.id);

        // Test type-safe relations
        let typed_posts = blog.get_user_posts_typed(user.id).await.unwrap();
        assert_eq!(typed_posts.len(), 1);
        assert_eq!(typed_posts[0].id, post.id);

        // Test post count using association method
        let post_count = blog.count_user_posts(user.id).await.unwrap();
        assert_eq!(post_count, 1);

        // Test getting post author using type-safe association
        let author = blog.get_post_author(post.id).await.unwrap();
        assert!(author.is_some());
        assert_eq!(author.unwrap().id, user.id);
    }

    #[tokio::test]
    async fn test_user_queries() {
        let blog = Blog::new().await.unwrap();

        // Create multiple users
        let _user1 = blog.create_user("Alice".to_string(), "alice@example.com".to_string()).await.unwrap();
        let _user2 = blog.create_user("Bob".to_string(), "bob@example.com".to_string()).await.unwrap();

        // Test querying
        let users = blog.list_active_users().await.unwrap();
        assert_eq!(users.len(), 2);

        // Test finding by ID
        let user = blog.get_user(_user1.id).await.unwrap();
        assert!(user.is_some());
        assert_eq!(user.unwrap().name, "Alice");
    }
}
```

## Step 7: Run Your Tests

```bash
cargo test
```

You should see output like:

```
running 2 tests
test tests::test_blog_operations ... ok
test tests::test_user_queries ... ok

test result: ok. 2 passed; 0 failed
```

## 🏆 What You've Accomplished

In this quick start, you've learned the core d1-rs features:

1. **Entity Definition**: Simple `#[derive(Entity)]` with type safety
2. **Type-Safe Relations**: `relations!` macro with compile-time validation
3. **Schema Migration**: Table creation with `SchemaMigration`
4. **Association Methods**: Generated methods for related entities
5. **CRUD Operations**: Fluent API for database operations
6. **Type-Safe Queries**: Methods like `where_is_active_eq()` validated at compile-time
7. **Efficient Testing**: In-memory SQLite with zero configuration

## 🚀 Next Steps

Now that you've learned the basics, explore more d1-rs capabilities:

- **[Relations](./relations/introduction.md)**: Learn about entity relationships in depth
- **[Auto Migration](./auto-migration.md)**: Automatic schema evolution
- **[Performance Tips](./advanced/performance.md)**: Optimize your database queries
- **[Deployment](./deployment/workers.md)**: Deploy to Cloudflare Workers

## Full Example

Here's the complete working example you can copy and run:

```rust
// Cargo.toml
[dependencies]
d1-rs = "0.1.0"
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.32", features = ["full"] }
chrono = { version = "0.4", features = ["serde"] }

[dev-dependencies]
rusqlite = { version = "0.32", features = ["chrono", "bundled"] }
```

Copy the code from steps 1-4 above into your `src/lib.rs` and run:

```bash
cargo test
```

Congratulations! 🎉 You've built your first d1-rs application!