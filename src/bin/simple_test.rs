use sea_orm::{Database, DatabaseConnection, EntityTrait, ActiveModelTrait, Set, ColumnTrait, QueryFilter, QueryOrder};
use chrono::{Utc, NaiveDateTime};

// Simple test models without full app dependencies
#[derive(Clone, Debug, PartialEq, Eq, sea_orm::DeriveEntityModel)]
#[sea_orm(table_name = "test_users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub email: String,
    pub full_name: String,
    pub created_at: NaiveDateTime,
}

#[derive(Copy, Clone, Debug, sea_orm::EnumIter, sea_orm::DeriveRelation)]
pub enum Relation {}

impl sea_orm::ActiveModelBehavior for ActiveModel {}

pub use Entity as TestUser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing basic SQLite functionality...");
    
    // Connect to in-memory SQLite database
    let db = Database::connect("sqlite::memory:").await?;
    
    // Create table manually
    use sea_orm::{Statement, DbBackend};
    let create_table = Statement::from_string(
        DbBackend::Sqlite,
        r#"
        CREATE TABLE test_users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            email TEXT UNIQUE NOT NULL,
            full_name TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )
        "#.to_string(),
    );
    
    db.execute(create_table).await?;
    println!("✅ Table created successfully");
    
    // Test insert
    let user = ActiveModel {
        email: Set("test@example.com".to_string()),
        full_name: Set("Test User".to_string()),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    };
    
    let result = user.insert(&db).await?;
    println!("✅ User inserted with ID: {}", result.id);
    
    // Test query
    let users = Entity::find()
        .filter(Column::Email.eq("test@example.com"))
        .all(&db)
        .await?;
    
    println!("✅ Found {} users", users.len());
    for user in users {
        println!("  - {} ({})", user.full_name, user.email);
    }
    
    println!("🎉 Basic SQLite test completed successfully!");
    
    Ok(())
}