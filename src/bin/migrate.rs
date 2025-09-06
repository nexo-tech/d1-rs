use calendar_app::db::init_db;
use migration::{Migrator, MigratorTrait};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenvy::dotenv().ok();
    
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    println!("Running database migrations...");
    
    // Initialize database connection
    let db = init_db().await?;
    
    // Run migrations
    Migrator::up(&db, None).await?;
    
    println!("Migrations completed successfully!");
    
    Ok(())
}