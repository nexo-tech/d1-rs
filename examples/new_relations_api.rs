/// Example showing the new, ent-go inspired relations API
/// This demonstrates the much cleaner and more intuitive design

use d1_rs::*;
use d1_rs::schema_evolution::SchemaMigration;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// Define entities
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
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
#[table(name = "categories")]
pub struct Category {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct PostCategory {
    #[primary_key]
    pub id: i64,
    pub post_id: i64,
    pub category_id: i64,
    pub created_at: DateTime<Utc>,
}

// Define relations using the new macro (much cleaner!)
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

async fn demonstrate_new_api() -> Result<()> {
    let db = D1Client::new_in_memory().await?;
    
    // 1. MUCH SIMPLER MIGRATIONS
    // Junction tables are created automatically, foreign keys handled automatically
    let migration = SchemaMigration::new("create_blog_schema".to_string())
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
        
        // Auto-generate all relations from entity definitions!
        .auto_generate_for::<User>()
        .auto_generate_for::<Post>()
        .auto_generate_for::<Category>();
    
    migration.execute(&db).await?;
    
    // 2. CREATE TEST DATA
    let user = User::create()
        .set_name("Alice Johnson".to_string())
        .set_email("alice@example.com".to_string())
        .set_is_active(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await?;
    
    let tech_category = Category::create()
        .set_name("Technology".to_string())
        .set_description(Some("Tech articles".to_string()))
        .save(&db)
        .await?;
    
    let tutorial_category = Category::create()
        .set_name("Tutorials".to_string())
        .set_description(Some("How-to articles".to_string()))
        .save(&db)
        .await?;
    
    let post = Post::create()
        .set_user_id(user.id)
        .set_title("Getting Started with Rust".to_string())
        .set_content("Rust is amazing...".to_string())
        .set_is_published(true)
        .set_created_at(Utc::now())
        .save(&db)
        .await?;
    
    // 3. MUCH CLEANER ASSOCIATION API - NO MORE STRING LITERALS!
    // Instead of: user.traverse::<Post>(&db, "posts").await?
    // Now type-safe association methods:
    let user_posts = user.posts().all(&db).await?;
    println!("User has {} posts", user_posts.len());
    
    // 4. TYPE-SAFE EAGER LOADING 
    // For now using basic queries - eager loading will be enhanced later
    let users_with_posts = User::query()
        .all(&db)
        .await?;
    
    println!("Found {} users", users_with_posts.len());
    
    // 5. TYPE-SAFE MANY-TO-MANY RELATIONS
    // Get post categories using type-safe association methods
    let post_categories = post.categories().all(&db).await?;
    println!("Post has {} categories", post_categories.len());
    
    // 6. ASSOCIATION METHODS FOR DIFFERENT RELATION TYPES
    // Many-to-many: Post -> Categories
    let tech_post_categories = post.categories().all(&db).await?;
    println!("Tech post has {} categories", tech_post_categories.len());
    
    // One-to-many: User -> Posts  
    let all_user_posts = user.posts().all(&db).await?;
    println!("User has {} posts total", all_user_posts.len());
    
    // Many-to-one: Post -> User
    let post_user = post.user().all(&db).await?;
    println!("Post belongs to {} users", post_user.len());
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    demonstrate_new_api().await?;
    println!("New relations API demonstration completed successfully!");
    Ok(())
}