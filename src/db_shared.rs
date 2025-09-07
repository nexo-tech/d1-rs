// Shared database structures and logic between native and WASM targets
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Calendar {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub slug: String,
    pub timezone: String,
    pub duration: i32,
    pub buffer: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Booking {
    pub id: String,
    pub calendar_id: String,
    pub name: String,
    pub email: String,
    pub notes: Option<String>,
    pub start_time: String,
    pub end_time: String,
    pub duration: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkingHours {
    pub day: i32,
    pub enabled: bool,
    pub start: String,
    pub end: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateCalendarRequest {
    pub name: String,
    pub slug: String,
    pub timezone: String,
    pub duration: i32,
    pub buffer: i32,
}

#[derive(Debug, Deserialize)]
pub struct WorkingHoursRequest {
    pub days: Vec<i32>,
    pub times: Vec<WorkingTime>,
}

#[derive(Debug, Deserialize)]
pub struct WorkingTime {
    pub day: i32,
    pub start: String,
    pub end: String,
}

#[derive(Debug, Deserialize)]
pub struct BookingRequest {
    pub name: String,
    pub email: String,
    pub notes: Option<String>,
    pub slot: String,
}

// Shared business logic for generating time slots
pub fn generate_time_slots(
    start_time: &str, 
    end_time: &str, 
    duration: i32, 
    buffer: i32, 
    date: &str,
    booked_times: &[String]
) -> Vec<String> {
    let mut slots = Vec::new();
    let start_hour: i32 = start_time.split(':').next().unwrap_or("9").parse().unwrap_or(9);
    let start_min: i32 = start_time.split(':').nth(1).unwrap_or("0").parse().unwrap_or(0);
    let end_hour: i32 = end_time.split(':').next().unwrap_or("17").parse().unwrap_or(17);
    
    let mut current_hour = start_hour;
    let mut current_min = start_min;
    
    while current_hour < end_hour || (current_hour == end_hour && current_min == 0) {
        let slot_time = format!("{}T{:02}:{:02}:00", date, current_hour, current_min);
        
        // Check if this slot is already booked
        let is_booked = booked_times.iter().any(|booking_start| {
            booking_start.contains(&format!("{:02}:{:02}", current_hour, current_min))
        });
        
        if !is_booked {
            slots.push(slot_time);
        }
        
        // Move to next slot
        current_min += duration + buffer;
        while current_min >= 60 {
            current_min -= 60;
            current_hour += 1;
        }
    }
    
    slots
}

// Shared logic for parsing and validating booking slots
pub fn parse_booking_slot(slot: &str, duration: i32) -> Result<(String, String), String> {
    let start_time = slot.to_string();
    let start_dt = chrono::DateTime::parse_from_rfc3339(&format!("{}Z", slot))
        .map_err(|e| format!("Invalid slot time: {}", e))?;
    let end_dt = start_dt + chrono::Duration::minutes(duration as i64);
    let end_time = end_dt.format("%Y-%m-%dT%H:%M:%S").to_string();
    
    Ok((start_time, end_time))
}

// Shared logic for getting day of week
pub fn get_day_of_week(date: &str) -> Result<u32, String> {
    use chrono::Datelike;
    
    let date_parsed = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date: {}", e))?;
    Ok(date_parsed.weekday().num_days_from_monday() + 1)
}