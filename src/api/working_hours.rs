use crate::database::{get_database, initialize_database};
use crate::models::{WorkingHours, WorkingHoursRequest};
use crate::templates::working_hours_form;
use chrono::Utc;
use d1orm::*;
use serde_json::json;
use worker::{Request, Response, Result as WorkerResult, RouteContext};

pub async fn working_hours_form_handler(_req: Request, ctx: RouteContext<()>) -> WorkerResult<Response> {
    let db = get_database(&ctx)?;
    initialize_database(&db).await?;
    
    // Get existing working hours if any
    let working_hours = WorkingHours::query()
        .first(&db).await
        .map_err(|e| worker::Error::RustError(format!("Database query failed: {}", e)))?;
    
    let (start_time, end_time, timezone, working_days) = if let Some(wh) = working_hours {
        (wh.start_time, wh.end_time, wh.timezone, wh.working_days)
    } else {
        ("09:00".to_string(), "17:00".to_string(), "UTC".to_string(), 
         "monday,tuesday,wednesday,thursday,friday".to_string())
    };
    
    let days: Vec<&str> = working_days.split(',').collect();
    
    Response::from_html(working_hours_form(&start_time, &end_time, &timezone, days))
}

pub async fn save_working_hours_handler(mut req: Request, ctx: RouteContext<()>) -> WorkerResult<Response> {
    let working_hours_data: WorkingHoursRequest = req.json().await?;
    
    let db = get_database(&ctx)?;
    initialize_database(&db).await?;
    
    // Check if working hours already exist
    let existing = WorkingHours::query()
        .first(&db).await
        .map_err(|e| worker::Error::RustError(format!("Database query failed: {}", e)))?;
    
    if let Some(wh) = existing {
        // Update existing record
        WorkingHours::update(wh.id)
            .set_start_time(working_hours_data.start_time)
            .set_end_time(working_hours_data.end_time)
            .set_timezone(working_hours_data.timezone)
            .set_working_days(working_hours_data.working_days)
            .set_updated_at(Utc::now())
            .save(&db).await
            .map_err(|e| worker::Error::RustError(format!("Failed to update working hours: {}", e)))?;
    } else {
        // Insert new record
        WorkingHours::create()
            .set_start_time(working_hours_data.start_time)
            .set_end_time(working_hours_data.end_time)
            .set_timezone(working_hours_data.timezone)
            .set_working_days(working_hours_data.working_days)
            .set_created_at(Utc::now())
            .set_updated_at(Utc::now())
            .save(&db).await
            .map_err(|e| worker::Error::RustError(format!("Failed to insert working hours: {}", e)))?;
    }
    
    Response::from_json(&json!({
        "success": true,
        "message": "Working hours saved successfully"
    }))
}