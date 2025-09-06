pub mod auth;
pub mod calendar_service;
pub mod oauth_service;
pub mod booking_service;
pub mod database_service;
pub mod state;

pub use auth::AuthService;
pub use calendar_service::CalendarService;
pub use oauth_service::OAuthService;
pub use booking_service::BookingService;
pub use database_service::{DatabaseService, SeaOrmDatabaseService};
pub use state::{AppState, provide_app_state, use_app_state};

#[cfg(feature = "workers")]
pub use database_service::D1DatabaseService;