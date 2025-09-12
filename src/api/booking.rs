use crate::auth::refresh_token_if_needed;
use crate::calendar::{create_calendar_event, fetch_calendar_events};
use crate::database::get_database;
use crate::models::{BookingRequest, Token, WorkingHours};
use crate::templates::{calendar_not_found_page, public_calendar_page};
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Utc, Weekday};
use d1orm::*;
use serde_json::json;
use std::collections::HashMap;
use worker::{Request, Response, Result as WorkerResult, RouteContext};

pub async fn public_calendar_handler(_req: Request, ctx: RouteContext<()>) -> WorkerResult<Response> {
    let email = ctx.param("email")
        .ok_or_else(|| worker::Error::RustError("Email parameter required".to_string()))?;
    
    let db = get_database(&ctx)?;
    
    // Check if token exists for this email
    match Token::query().where_user_email_eq(email.to_string()).first(&db).await {
        Ok(Some(_)) => {
            Response::from_html(public_calendar_page(email))
        }
        _ => {
            Response::from_html(calendar_not_found_page(email))
        }
    }
}

pub async fn get_available_slots_handler(req: Request, ctx: RouteContext<()>) -> WorkerResult<Response> {
    let email = ctx.param("email")
        .ok_or_else(|| worker::Error::RustError("Email parameter required".to_string()))?;
    
    let url = req.url()?;
    let query_pairs: HashMap<String, String> = url.query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    
    let date = query_pairs.get("date")
        .ok_or_else(|| worker::Error::RustError("Date parameter required".to_string()))?;
    
    let timezone = query_pairs.get("timezone").unwrap_or(&"UTC".to_string()).clone();
    let slot_type = query_pairs.get("slot_type").unwrap_or(&"30".to_string()).clone();
    
    let db = get_database(&ctx)?;
    
    // Get token for this email
    let mut token = Token::query()
        .where_user_email_eq(email.to_string())
        .first(&db).await
        .map_err(|e| worker::Error::RustError(format!("Database query failed: {}", e)))?
        .ok_or_else(|| worker::Error::RustError("Calendar not found".to_string()))?;
    
    // Refresh token if needed
    if let Some(refreshed_token) = refresh_token_if_needed(&token, &ctx).await? {
        Token::update(refreshed_token.id)
            .set_access_token(refreshed_token.access_token.clone())
            .set_refresh_token(refreshed_token.refresh_token.clone())
            .set_expiry(refreshed_token.expiry)
            .set_updated_at(refreshed_token.updated_at)
            .save(&db).await
            .map_err(|e| worker::Error::RustError(format!("Failed to update token: {}", e)))?;
        token = refreshed_token;
    }
    
    // Get working hours configuration
    let working_hours = WorkingHours::query()
        .first(&db).await
        .map_err(|e| worker::Error::RustError(format!("Database query failed: {}", e)))?
        .unwrap_or(WorkingHours {
            id: 0,
            start_time: "09:00".to_string(),
            end_time: "17:00".to_string(),
            timezone: "UTC".to_string(),
            working_days: "monday,tuesday,wednesday,thursday,friday".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });
    
    // Parse the date
    let date_parts: Vec<&str> = date.split('-').collect();
    let year = date_parts[0].parse::<i32>().unwrap();
    let month = date_parts[1].parse::<u32>().unwrap();
    let day = date_parts[2].parse::<u32>().unwrap();
    
    let naive_date = NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| worker::Error::RustError("Invalid date".to_string()))?;
    
    // Check if this is a working day
    let weekday = naive_date.weekday();
    let weekday_str = match weekday {
        Weekday::Mon => "monday",
        Weekday::Tue => "tuesday",
        Weekday::Wed => "wednesday",
        Weekday::Thu => "thursday",
        Weekday::Fri => "friday",
        Weekday::Sat => "saturday",
        Weekday::Sun => "sunday",
    };
    
    let working_days: Vec<&str> = working_hours.working_days.split(',').collect();
    if !working_days.contains(&weekday_str) {
        return Response::from_json(&json!({
            "slots": []
        }));
    }
    
    // Parse working hours
    let start_parts: Vec<&str> = working_hours.start_time.split(':').collect();
    let start_hour = start_parts[0].parse::<u32>().unwrap();
    let start_minute = start_parts[1].parse::<u32>().unwrap();
    
    let end_parts: Vec<&str> = working_hours.end_time.split(':').collect();
    let end_hour = end_parts[0].parse::<u32>().unwrap();
    let end_minute = end_parts[1].parse::<u32>().unwrap();
    
    // Fetch existing events from Google Calendar
    let start_of_day = DateTime::<Utc>::from_naive_utc_and_offset(
        NaiveDateTime::new(naive_date, NaiveTime::from_hms_opt(0, 0, 0).unwrap()),
        Utc
    );
    let end_of_day = start_of_day + Duration::days(1);
    
    let existing_events = fetch_calendar_events(&token.access_token, &start_of_day, &end_of_day, None).await?;
    
    // Generate time slots
    let mut slots = Vec::new();
    let mut current_time = NaiveTime::from_hms_opt(start_hour, start_minute, 0).unwrap();
    let end_time = NaiveTime::from_hms_opt(end_hour, end_minute, 0).unwrap();
    let now = Utc::now();
    
    // Generate slots based on requested type
    while current_time < end_time {
        // Generate 30-minute slot if requested
        if slot_type == "30" {
            let slot_end_30 = current_time + Duration::minutes(30);
            if slot_end_30 <= end_time {
                let slot_start_dt = DateTime::<Utc>::from_naive_utc_and_offset(
                    NaiveDateTime::new(naive_date, current_time),
                    Utc
                );
                let slot_end_dt = DateTime::<Utc>::from_naive_utc_and_offset(
                    NaiveDateTime::new(naive_date, slot_end_30),
                    Utc
                );
                
                // Check if this slot conflicts with any existing event
                let mut is_available = slot_end_dt > now; // Slot must end after current time
                
                if is_available {
                    for event in &existing_events {
                        if let (Some(event_start_str), Some(event_end_str)) = 
                            (event.get("start").and_then(|s| s.as_str()),
                             event.get("end").and_then(|e| e.as_str())) {
                            
                            if let (Ok(event_start), Ok(event_end)) = 
                                (DateTime::parse_from_rfc3339(event_start_str),
                                 DateTime::parse_from_rfc3339(event_end_str)) {
                                
                                let event_start_utc = event_start.with_timezone(&Utc);
                                let event_end_utc = event_end.with_timezone(&Utc);
                                
                                // Check for overlap
                                if slot_start_dt < event_end_utc && slot_end_dt > event_start_utc {
                                    is_available = false;
                                    break;
                                }
                            }
                        }
                    }
                }
                
                slots.push(json!({
                    "start": slot_start_dt.to_rfc3339(),
                    "end": slot_end_dt.to_rfc3339(),
                    "display": format!("{} - {} (30 min)", 
                        current_time.format("%I:%M %p"),
                        slot_end_30.format("%I:%M %p")),
                    "duration": 30,
                    "available": is_available
                }));
            }
        }
        
        // Generate 60-minute slot if requested
        if slot_type == "60" {
            let slot_end_60 = current_time + Duration::minutes(60);
            if slot_end_60 <= end_time {
                let slot_start_dt = DateTime::<Utc>::from_naive_utc_and_offset(
                    NaiveDateTime::new(naive_date, current_time),
                    Utc
                );
                let slot_end_dt = DateTime::<Utc>::from_naive_utc_and_offset(
                    NaiveDateTime::new(naive_date, slot_end_60),
                    Utc
                );
                
                // Check if this slot conflicts with any existing event
                let mut is_available = slot_end_dt > now; // Slot must end after current time
                
                if is_available {
                    for event in &existing_events {
                        if let (Some(event_start_str), Some(event_end_str)) = 
                            (event.get("start").and_then(|s| s.as_str()),
                             event.get("end").and_then(|e| e.as_str())) {
                            
                            if let (Ok(event_start), Ok(event_end)) = 
                                (DateTime::parse_from_rfc3339(event_start_str),
                                 DateTime::parse_from_rfc3339(event_end_str)) {
                                
                                let event_start_utc = event_start.with_timezone(&Utc);
                                let event_end_utc = event_end.with_timezone(&Utc);
                                
                                // Check for overlap
                                if slot_start_dt < event_end_utc && slot_end_dt > event_start_utc {
                                    is_available = false;
                                    break;
                                }
                            }
                        }
                    }
                }
                
                slots.push(json!({
                    "start": slot_start_dt.to_rfc3339(),
                    "end": slot_end_dt.to_rfc3339(),
                    "display": format!("{} - {} (60 min)", 
                        current_time.format("%I:%M %p"),
                        slot_end_60.format("%I:%M %p")),
                    "duration": 60,
                    "available": is_available
                }));
            }
        }
        
        // Move to next 30-minute interval
        current_time = current_time + Duration::minutes(30);
    }
    
    Response::from_json(&json!({
        "slots": slots,
        "timezone": timezone,
        "date": date
    }))
}

pub async fn create_booking_handler(mut req: Request, ctx: RouteContext<()>) -> WorkerResult<Response> {
    let booking_request: BookingRequest = req.json().await?;
    
    let db = get_database(&ctx)?;
    
    // Get token for the calendar owner
    let mut token = Token::query()
        .where_user_email_eq(booking_request.calendar_email.clone())
        .first(&db).await
        .map_err(|e| worker::Error::RustError(format!("Database query failed: {}", e)))?
        .ok_or_else(|| worker::Error::RustError("Calendar not found".to_string()))?;
    
    // Refresh token if needed
    if let Some(refreshed_token) = refresh_token_if_needed(&token, &ctx).await? {
        Token::update(refreshed_token.id)
            .set_access_token(refreshed_token.access_token.clone())
            .set_refresh_token(refreshed_token.refresh_token.clone())
            .set_expiry(refreshed_token.expiry)
            .set_updated_at(refreshed_token.updated_at)
            .save(&db).await
            .map_err(|e| worker::Error::RustError(format!("Failed to update token: {}", e)))?;
        token = refreshed_token;
    }
    
    // Parse times
    let start_time = DateTime::parse_from_rfc3339(&booking_request.start_time)
        .map_err(|e| worker::Error::RustError(format!("Invalid start time: {}", e)))?
        .with_timezone(&Utc);
    
    let end_time = DateTime::parse_from_rfc3339(&booking_request.end_time)
        .map_err(|e| worker::Error::RustError(format!("Invalid end time: {}", e)))?
        .with_timezone(&Utc);
    
    // Create event in Google Calendar
    let event_id = create_calendar_event(
        &token.access_token,
        &start_time,
        &end_time,
        &booking_request.title,
        &booking_request.guest_email,
        booking_request.notes.as_deref(),
    ).await?;
    
    Response::from_json(&json!({
        "success": true,
        "event_id": event_id,
        "message": "Booking created successfully"
    }))
}