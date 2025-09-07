use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// Shared data structures used by both native and Workers
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCalendarRequest {
    pub name: String,
    pub slug: Option<String>,
    pub timezone: String,
    pub duration: i32,
    pub buffer: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkingHoursRequest {
    pub working_hours: Vec<WorkingTime>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkingTime {
    pub day_of_week: i32,
    pub start_time: String,
    pub end_time: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BookingRequest {
    pub name: String,
    pub email: String,
    pub notes: Option<String>,
    pub datetime: String,
}

// Shared entity structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calendar {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub slug: String,
    pub timezone: String,
    pub duration: i32,
    pub buffer: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Booking {
    pub id: String,
    pub calendar_id: String,
    pub name: String,
    pub email: String,
    pub notes: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarWorkingHours {
    pub id: String,
    pub calendar_id: String,
    pub day_of_week: i32,
    pub start_time: String,
    pub end_time: String,
    pub enabled: bool,
}