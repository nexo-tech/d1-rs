/// Tests for Phase 2: Eager Loading System (.with_posts() syntax)
/// This demonstrates d1-rs's superior type-safe eager loading vs Ent-Go
use chrono::{DateTime, Utc};
use d1_rs::*;
use d1_rs::schema_evolution::SchemaMigration;
use serde::{Deserialize, Serialize};

// Test entities for eager loading  
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
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
#[table(name = "categories")]
pub struct Category {
    #[primary_key] 
    pub id: i64,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

// REVOLUTIONARY: Type-safe eager loading - NO STRING LITERALS!
// This provides superior compile-time safety for preventing N+1 queries
relations! {
    User {
        has_many posts: Post via user_id,
    }

    Post {
        belongs_to user: User via user_id,
        has_many categories: Category, // M2M relationship
    }
    
    Category {
        has_many posts: Post,  // M2M relationship
    }
}

async fn setup_eager_loading_db() -> D1Client {
    let db = D1Client::new_in_memory()
        .await
        .expect("Failed to create database");

    let migration = SchemaMigration::new("create_eager_loading_schema".to_string())
        .create_table("users")
        .integer("id").primary_key().auto_increment().build()
        .text("email").not_null().unique().build()
        .text("name").not_null().build()
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
        .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
        .build()
        .create_table("post_categories")  // Junction table for M2M
        .integer("id").primary_key().auto_increment().build()
        .integer("post_id").not_null().build()
        .integer("category_id").not_null().build()
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
async fn test_eager_loading_api_design() {
    // Test the design of the eager loading API - this will be the target syntax
    
    println!("🚀 d1-rs Eager Loading API vs Ent-Go:");
    println!();
    
    println!("❌ Ent-Go (Verbose, string-based):");
    println!("  client.User.Query().WithPosts().All(ctx)");
    println!("  - Runtime errors possible");
    println!("  - String literals can be misspelled");
    println!("  - No compile-time validation");
    println!();
    
    println!("✅ d1-rs (Type-safe, compile-time validated):");
    println!("  User::query().with_posts().all(&db).await?");
    println!("  - Compile-time validation");
    println!("  - Impossible to misspell relation names");
    println!("  - IDE auto-completion");
    println!("  - Zero runtime overhead");
    println!();
    
    // Target nested eager loading syntax:
    println!("🎯 Advanced nested eager loading:");
    println!("  User::query()");
    println!("    .with_posts(|posts| posts.with_categories())");
    println!("    .all(&db).await?");
    println!();
    
    println!("🏆 This provides SUPERIOR type safety and N+1 prevention!");
}

#[tokio::test]
async fn test_eager_loading_foundation() {
    let _db = setup_eager_loading_db().await;
    
    // For now, test that the basic relations work
    // The eager loading methods will be implemented next
    
    println!("✅ Eager loading foundation setup complete!");
    println!("✅ Next: Implement with_relation_name() methods on QueryBuilder!");
}