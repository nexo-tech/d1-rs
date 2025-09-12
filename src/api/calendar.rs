use crate::database::get_database;
use crate::models::Token;
use crate::templates::dashboard_page;
use d1orm::*;
use serde_json::json;
use worker::{Request, Response, Result as WorkerResult, RouteContext};

pub async fn dashboard_handler(_req: Request, ctx: RouteContext<()>) -> WorkerResult<Response> {
    let db = get_database(&ctx)?;
    
    // Get all connected calendars
    let tokens = Token::query()
        .all(&db).await
        .map_err(|e| worker::Error::RustError(format!("Database query failed: {}", e)))?;
    
    let emails: Vec<String> = tokens.iter().map(|t| t.user_email.clone()).collect();
    
    Response::from_html(dashboard_page(&emails))
}

pub async fn get_emails_handler(_req: Request, ctx: RouteContext<()>) -> WorkerResult<Response> {
    let db = get_database(&ctx)?;
    
    // Get all connected calendars
    let tokens = Token::query()
        .all(&db).await
        .map_err(|e| worker::Error::RustError(format!("Database query failed: {}", e)))?;
    
    let emails: Vec<String> = tokens.iter().map(|t| t.user_email.clone()).collect();
    
    Response::from_json(&json!({
        "emails": emails
    }))
}