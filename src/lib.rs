pub mod auth;
pub mod templates;
pub mod shared_types;
pub mod database_trait;

#[cfg(feature = "native")]
pub mod db_shared;
#[cfg(feature = "native")]
pub mod entities;
#[cfg(feature = "native")]
pub mod database;
#[cfg(feature = "native")]
pub mod routes;

#[cfg(feature = "workers")]
pub mod worker_handlers;

// Re-export main components
pub use auth::{User, Claims};

#[cfg(feature = "native")]
pub use routes::{AppState, create_router};

// Shared data structures for both targets
pub use shared_types::{CreateCalendarRequest, WorkingHoursRequest, WorkingTime, BookingRequest, Calendar, Booking, CalendarWorkingHours};

#[derive(serde::Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}