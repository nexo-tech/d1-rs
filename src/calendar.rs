use crate::models::CalendarEventsResponse;
use chrono::{DateTime, Utc};
use serde_json::json;
use worker::Result as WorkerResult;

pub async fn fetch_calendar_events(
    access_token: &str,
    start_time: &DateTime<Utc>,
    end_time: &DateTime<Utc>,
    calendar_id: Option<&str>,
) -> WorkerResult<Vec<serde_json::Value>> {
    let client = reqwest::Client::new();
    let calendar = calendar_id.unwrap_or("primary");
    
    // Format datetime as RFC3339 with Z suffix for UTC (Google Calendar API requirement)
    let time_min = start_time.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let time_max = end_time.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    
    let url = format!(
        "https://www.googleapis.com/calendar/v3/calendars/{}/events",
        calendar
    );
    
    let response = client.get(&url)
        .bearer_auth(access_token)
        .header("Accept", "application/json")
        .query(&[
            ("timeMin", time_min.as_str()),
            ("timeMax", time_max.as_str()),
            ("singleEvents", "true"),
            ("orderBy", "startTime"),
        ])
        .send()
        .await
        .map_err(|e| worker::Error::RustError(format!("HTTP request failed: {}", e)))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_body = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        worker::console_error!("Calendar API error - Status: {}, Body: {}", status, error_body);
        worker::console_error!("Request URL: {}", url);
        worker::console_error!("Access token (first 20 chars): {}...", &access_token[..std::cmp::min(20, access_token.len())]);
        return Err(worker::Error::RustError(format!("Calendar API request failed: {} - {}", status, error_body)));
    }
    
    let events_response: CalendarEventsResponse = response.json()
        .await
        .map_err(|e| worker::Error::RustError(format!("Failed to parse events response: {}", e)))?;
    
    let mut events = Vec::new();
    for event in events_response.items {
        let start_str = if let Some(ref start) = event.start {
            start.date_time.clone().or_else(|| start.date.clone())
        } else {
            None
        };
        
        let end_str = if let Some(ref end) = event.end {
            end.date_time.clone().or_else(|| end.date.clone())
        } else {
            None
        };
        
        events.push(json!({
            "summary": event.summary,
            "start": start_str,
            "end": end_str
        }));
    }
    
    Ok(events)
}

pub async fn create_calendar_event(
    access_token: &str,
    start_time: &DateTime<Utc>,
    end_time: &DateTime<Utc>,
    title: &str,
    guest_email: &str,
    notes: Option<&str>,
) -> WorkerResult<String> {
    let client = reqwest::Client::new();
    let url = "https://www.googleapis.com/calendar/v3/calendars/primary/events";
    
    let event_data = json!({
        "summary": title,
        "description": notes.unwrap_or(""),
        "start": {
            "dateTime": start_time.to_rfc3339(),
            "timeZone": "UTC"
        },
        "end": {
            "dateTime": end_time.to_rfc3339(),
            "timeZone": "UTC"
        },
        "attendees": [
            {
                "email": guest_email
            }
        ],
        "reminders": {
            "useDefault": true
        }
    });
    
    let response = client.post(url)
        .bearer_auth(access_token)
        .json(&event_data)
        .send()
        .await
        .map_err(|e| worker::Error::RustError(format!("HTTP request failed: {}", e)))?;
    
    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(worker::Error::RustError(format!("Failed to create event: {}", error_text)));
    }
    
    let event_response: serde_json::Value = response.json()
        .await
        .map_err(|e| worker::Error::RustError(format!("Failed to parse event response: {}", e)))?;
    
    Ok(event_response["id"].as_str().unwrap_or("unknown").to_string())
}