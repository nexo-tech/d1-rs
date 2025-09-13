# Quick Start

Let's build a simple blog application to get you familiar with d1-rs! This tutorial will cover the essential concepts while creating something practical.

## What We'll Build

A basic blog system with:
- Users who can write posts
- Posts with titles, content, and metadata
- Simple querying and CRUD operations

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

## Step 2: Create Database Schema

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

## Step 3: Basic CRUD Operations

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

## Step 4: Testing Your Application

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

        // Get user's posts
        let user_posts = blog.get_user_posts(user.id).await.unwrap();
        assert_eq!(user_posts.len(), 1);
        assert_eq!(user_posts[0].id, post.id);
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

## Step 5: Run Your Tests

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

## What You've Learned

In this quick start, you've learned:

1. **Entity Definition**: How to create database models with the `#[derive(Entity)]` macro
2. **Schema Migration**: Using `SchemaMigration` to create database tables
3. **CRUD Operations**: Creating, reading, updating with the fluent API
4. **Type-Safe Queries**: Using generated query methods like `where_is_active_eq()`
5. **Testing**: Writing tests with in-memory SQLite databases

## Next Steps

Now that you have the basics down, explore these advanced features:

- **[Relations](./relations/introduction.md)**: Connect your entities with one-to-one, one-to-many, and many-to-many relationships
- **[Advanced Queries](./queries.md)**: Learn about complex filtering, joining, and aggregation
- **[Boolean Handling](./advanced/boolean-handling.md)**: Understand how d1-rs handles SQLite's integer-based booleans
- **[Deployment](./deployment/workers.md)**: Deploy your application to Cloudflare Workers

## Full Example

Here's the complete working example you can copy and run:

```rust
// Cargo.toml
[dependencies]
d1-rs = "0.1.0"
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
chrono = { version = "0.4", features = ["serde"] }

[dev-dependencies]
rusqlite = "0.30"
```

Copy the code from steps 1-4 above into your `src/lib.rs` and run:

```bash
cargo test
```

Congratulations! 🎉 You've built your first d1-rs application!