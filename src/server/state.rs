use std::sync::Arc;
use sea_orm::DatabaseConnection;
use crate::server::{AuthService, CalendarService, OAuthService, BookingService};
use crate::server::database_service::{DatabaseService, SeaOrmDatabaseService};

#[cfg(feature = "workers")]
use crate::server::database_service::D1DatabaseService;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub db_service: Arc<dyn DatabaseService>,
    pub auth_service: AuthService,
    pub calendar_service: CalendarService,
    pub oauth_service: OAuthService,
    pub booking_service: BookingService,
}

impl AppState {
    pub async fn new(db: DatabaseConnection) -> anyhow::Result<Self> {
        let jwt_secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "dev-secret-key".to_string());
        
        // Create the appropriate database service based on feature flags
        let db_service: Arc<dyn DatabaseService> = {
            #[cfg(feature = "workers")]
            {
                Arc::new(D1DatabaseService::new(db.clone()))
            }
            #[cfg(not(feature = "workers"))]
            {
                Arc::new(SeaOrmDatabaseService::new(db.clone()))
            }
        };
        
        // Initialize database schema
        db_service.init_schema().await?;
            
        let auth_service = AuthService::new(db.clone(), jwt_secret);
        let calendar_service = CalendarService::new(db.clone());
        let oauth_service = OAuthService::new(db.clone())?;
        let booking_service = BookingService::new(db.clone(), oauth_service.clone());

        Ok(Self {
            db,
            db_service,
            auth_service,
            calendar_service,
            oauth_service,
            booking_service,
        })
    }
}

// Context for accessing services in server functions
use leptos::*;

pub fn provide_app_state(state: AppState) {
    provide_context(state);
}

pub fn use_app_state() -> AppState {
    expect_context::<AppState>()
}

// Helper to get current user from JWT in server functions
pub fn get_current_user_id() -> Option<i32> {
    use leptos_actix::extract;
    use actix_web::HttpRequest;
    
    let req = extract::<HttpRequest>()?;
    let auth_header = req.headers().get("authorization")?;
    let auth_str = auth_header.to_str().ok()?;
    
    if !auth_str.starts_with("Bearer ") {
        return None;
    }
    
    let token = &auth_str[7..];
    let state = use_app_state();
    
    match state.auth_service.verify_jwt(token) {
        Ok(claims) => claims.sub.parse().ok(),
        Err(_) => None,
    }
}

// Helper for cookie-based auth (for web sessions)
pub fn get_current_user_from_session() -> Option<i32> {
    // TODO: Implement session-based auth extraction
    None
}