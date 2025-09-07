// Native server using shared Axum router
use tokio;
use tower_http::services::ServeDir;

use calendar_app::{AppState, create_router, database};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenvy::dotenv().ok();
    
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Initialize database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://./calendar.db?mode=rwc".to_string());
    
    let db = sea_orm::Database::connect(&database_url).await?;
    
    // Run migrations (if needed)
    // sea_orm_migration::Migrator::up(&db, None).await?;
    
    // Create application state
    let app_state = AppState { db };
    
    // Create shared router
    let app = create_router(app_state)
        .nest_service("/static", ServeDir::new("public"));

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    
    println!("Starting server on http://127.0.0.1:{}", port);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}