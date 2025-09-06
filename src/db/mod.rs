use sea_orm::{Database, DatabaseConnection, DbErr};
use migration::{Migrator, MigratorTrait};
use std::env;

#[cfg(feature = "workers")]
pub mod d1_connection;

pub async fn init_db() -> Result<DatabaseConnection, DbErr> {
    let database_url = get_database_url();
    
    let db = Database::connect(&database_url).await?;
    
    // Run migrations automatically
    Migrator::up(&db, None).await?;
    
    Ok(db)
}

fn get_database_url() -> String {
    // Check if we're in Cloudflare Workers environment
    #[cfg(feature = "workers")]
    {
        // In Workers, we'll use D1 through the adapter
        // This will be configured in the worker binary
        env::var("D1_DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string())
    }
    
    #[cfg(not(feature = "workers"))]
    {
        // For local development, use SQLite file
        env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://./calendar.db?mode=rwc".to_string())
    }
}

// Helper function to check if database is ready
pub async fn check_db_health(db: &DatabaseConnection) -> Result<(), DbErr> {
    use sea_orm::{ConnectionTrait, Statement};
    
    let result = db
        .execute(Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT 1".to_string(),
        ))
        .await?;
    
    if result.rows_affected() == 0 {
        Ok(())
    } else {
        Ok(())
    }
}