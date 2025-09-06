use sea_orm::{Database, DatabaseConnection};
use migration::{Migrator, MigratorTrait};
use chrono::{DateTime, Utc};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🧪 Testing local SQLite database functionality...");
    
    // Connect to SQLite database
    let database_url = "sqlite://./test_calendar.db?mode=rwc";
    let db = sea_orm::Database::connect(database_url).await?;
    
    // Initialize schema using migrations
    println!("📦 Running database migrations...");
    Migrator::up(&db, None).await?;
    println!("✅ Migrations completed successfully");
    
    // Test basic database operations using SeaORM directly
    use calendar_app::models::{user, calendar};
    use sea_orm::*;
    
    // Test user creation
    println!("👤 Testing user creation...");
    let user_active_model = user::ActiveModel {
        email: ActiveValue::Set("test@example.com".to_string()),
        password_hash: ActiveValue::Set("hashed_password".to_string()),
        full_name: ActiveValue::Set("Test User".to_string()),
        company_name: ActiveValue::Set(Some("Test Company".to_string())),
        ..Default::default()
    };
    
    let result = user::Entity::insert(user_active_model).exec(&db).await?;
    let created_user = user::Entity::find_by_id(result.last_insert_id)
        .one(&db)
        .await?
        .expect("Failed to create user");
    
    println!("✅ User created: {} (ID: {})", created_user.email, created_user.id);
    
    // Test calendar creation
    println!("📅 Testing calendar creation...");
    let calendar_active_model = calendar::ActiveModel {
        user_id: ActiveValue::Set(created_user.id),
        name: ActiveValue::Set("My Test Calendar".to_string()),
        slug: ActiveValue::Set("test-calendar".to_string()),
        description: ActiveValue::Set(Some("A test calendar for demo purposes".to_string())),
        timezone: ActiveValue::Set("UTC".to_string()),
        ..Default::default()
    };
    
    let result = calendar::Entity::insert(calendar_active_model).exec(&db).await?;
    let created_calendar = calendar::Entity::find_by_id(result.last_insert_id)
        .one(&db)
        .await?
        .expect("Failed to create calendar");
    
    println!("✅ Calendar created: {} (Slug: {})", created_calendar.name, created_calendar.slug);
    
    // Test fetching user by email
    println!("🔍 Testing user lookup...");
    let found_user = user::Entity::find()
        .filter(user::Column::Email.eq("test@example.com"))
        .one(&db)
        .await?;
    
    match found_user {
        Some(user) => println!("✅ Found user: {} ({})", user.full_name, user.email),
        None => println!("❌ User not found"),
    }
    
    // Test fetching calendar by slug
    println!("🔍 Testing calendar lookup...");
    let found_calendar = calendar::Entity::find()
        .filter(calendar::Column::Slug.eq("test-calendar"))
        .filter(calendar::Column::IsActive.eq(true))
        .one(&db)
        .await?;
    
    match found_calendar {
        Some(cal) => println!("✅ Found calendar: {} ({})", cal.name, cal.slug),
        None => println!("❌ Calendar not found"),
    }
    
    // Test fetching user calendars
    println!("📋 Testing user calendars list...");
    let user_calendars = calendar::Entity::find()
        .filter(calendar::Column::UserId.eq(created_user.id))
        .order_by_desc(calendar::Column::CreatedAt)
        .all(&db)
        .await?;
    
    println!("✅ Found {} calendars for user", user_calendars.len());
    for cal in user_calendars {
        println!("  - {} ({})", cal.name, cal.slug);
    }
    
    println!("🎉 All database tests passed successfully!");
    
    Ok(())
}