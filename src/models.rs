use chrono::{DateTime, Utc};
use d1orm::*;
use serde::{Deserialize, Serialize};

// Custom deserializer for D1 datetime format
pub mod datetime_format {
    use chrono::{DateTime, NaiveDateTime, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = date.format("%Y-%m-%d %H:%M:%S").to_string();
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
            .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
            .map_err(serde::de::Error::custom)
    }
}


#[derive(Debug, Serialize, Deserialize, Clone, Entity)]
#[table(name = "users")]
pub struct User {
    #[primary_key]
    pub id: i64,
    #[unique]
    pub email: String, // Primary Google account email
    pub name: String,
    pub is_active: bool,
    #[serde(with = "datetime_format")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "datetime_format")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity)]
#[table(name = "tokens")]
pub struct Token {
    #[primary_key]
    pub id: i64,
    pub user_id: i64, // Foreign key to users table
    pub calendar_email: String, // Email of this specific calendar/token
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub is_primary: bool, // Whether this is the user's primary calendar
    #[serde(with = "datetime_format")]
    pub expiry: DateTime<Utc>,
    #[serde(with = "datetime_format")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "datetime_format")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity)]
#[table(name = "calendars")]
pub struct Calendar {
    #[primary_key]
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub timezone: String,
    #[serde(with = "datetime_format")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity)]
#[table(name = "working_hours")]
pub struct WorkingHours {
    #[primary_key]
    pub id: i64,
    pub start_time: String, // Format: "HH:MM"
    pub end_time: String,   // Format: "HH:MM"
    pub timezone: String,
    pub working_days: String, // Comma-separated days: "monday,tuesday,wednesday,thursday,friday"
    #[serde(with = "datetime_format")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "datetime_format")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity)]
#[table(name = "bookings")]
pub struct Booking {
    #[primary_key]
    pub id: i64,
    pub calendar_id: i64,
    pub guest_email: String,
    pub guest_name: String,
    pub title: String,
    pub notes: Option<String>,
    #[serde(with = "datetime_format")]
    pub start_time: DateTime<Utc>,
    #[serde(with = "datetime_format")]
    pub end_time: DateTime<Utc>,
    pub status: String, // "confirmed", "cancelled"
    #[serde(with = "datetime_format")]
    pub created_at: DateTime<Utc>,
}

// DTOs and request/response models
#[derive(Deserialize)]
pub struct GoogleUserInfo {
    pub email: String,
}

#[derive(Deserialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub expires_in: i64,
}

#[derive(Deserialize)]
pub struct CalendarEvent {
    pub summary: Option<String>,
    pub start: Option<CalendarDateTime>,
    pub end: Option<CalendarDateTime>,
}

#[derive(Deserialize)]
pub struct CalendarDateTime {
    #[serde(rename = "dateTime")]
    pub date_time: Option<String>,
    pub date: Option<String>,
}

#[derive(Deserialize)]
pub struct CalendarEventsResponse {
    pub items: Vec<CalendarEvent>,
}

#[derive(Deserialize, Serialize)]
pub struct BookingRequest {
    pub calendar_email: String,
    pub guest_name: String,
    pub guest_email: String,
    pub title: String,
    pub notes: Option<String>,
    pub start_time: String,
    pub end_time: String,
    pub timezone: String,
}

#[derive(Deserialize, Serialize)]
pub struct WorkingHoursRequest {
    pub start_time: String,
    pub end_time: String,
    pub timezone: String,
    pub working_days: String,
}