use worker::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tera::Context;

use crate::auth;
use crate::db;
use crate::models;
use crate::templates;

// Home handler - redirects to login or dashboard
pub async fn home_handler(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    Response::redirect(Url::parse("/login").unwrap())
}

// Login page handler
pub async fn login_page_handler(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let tera = templates::init_templates();
    let context = Context::new();
    let html = tera.render("login.html", &context).unwrap();
    
    Response::from_html(html)
}

// Login API handler
#[derive(Debug, Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

pub async fn login_handler(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let form_data: LoginRequest = req.json().await?;
    
    // Get D1 database binding
    let d1 = ctx.env.d1("DB").expect("D1 binding not found");
    
    // Verify user credentials
    match auth::verify_user(&d1, &form_data.email, &form_data.password).await {
        Ok(user) => {
            // Generate JWT token
            let token = auth::generate_token(&user)?;
            
            // Create response with cookie
            let mut headers = Headers::new();
            headers.set("Set-Cookie", &format!("auth_token={}; HttpOnly; Path=/; Max-Age=86400", token))?;
            
            Ok(Response::ok(json!({ "success": true }).to_string())?
                .with_headers(headers))
        }
        Err(_) => {
            Ok(Response::error("Invalid credentials", 401)?)
        }
    }
}

// Register page handler
pub async fn register_page_handler(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let tera = templates::init_templates();
    let context = Context::new();
    let html = tera.render("register.html", &context).unwrap();
    
    Response::from_html(html)
}

// Register API handler
#[derive(Debug, Deserialize)]
struct RegisterRequest {
    name: String,
    email: String,
    password: String,
}

pub async fn register_handler(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let form_data: RegisterRequest = req.json().await?;
    
    // Get D1 database binding
    let d1 = ctx.env.d1("DB").expect("D1 binding not found");
    
    // Create user
    match auth::create_user(&d1, &form_data.name, &form_data.email, &form_data.password).await {
        Ok(user) => {
            // Generate JWT token
            let token = auth::generate_token(&user)?;
            
            // Create response with cookie
            let mut headers = Headers::new();
            headers.set("Set-Cookie", &format!("auth_token={}; HttpOnly; Path=/; Max-Age=86400", token))?;
            
            Ok(Response::ok(json!({ "success": true }).to_string())?
                .with_headers(headers))
        }
        Err(e) => {
            Ok(Response::error(&format!("Registration failed: {}", e), 400)?)
        }
    }
}

// Logout handler
pub async fn logout_handler(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let mut headers = Headers::new();
    headers.set("Set-Cookie", "auth_token=; HttpOnly; Path=/; Max-Age=0")?;
    
    Response::redirect_with_headers(Url::parse("/login").unwrap(), headers)
}

// Dashboard handler
pub async fn dashboard_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // Verify authentication
    let user = match auth::get_current_user(&req, &ctx).await {
        Ok(user) => user,
        Err(_) => return Response::redirect(Url::parse("/login").unwrap()),
    };
    
    let d1 = ctx.env.d1("DB")?;
    
    // Get counts for dashboard
    let calendar_count = db::get_calendar_count(&d1, user.id).await?;
    let booking_count = db::get_booking_count(&d1, user.id).await?;
    
    let tera = templates::init_templates();
    let mut context = Context::new();
    context.insert("user", &user);
    context.insert("calendar_count", &calendar_count);
    context.insert("booking_count", &booking_count);
    
    let html = tera.render("dashboard.html", &context).unwrap();
    Response::from_html(html)
}

// Calendar list handler
pub async fn calendars_list_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // Verify authentication
    let user = match auth::get_current_user(&req, &ctx).await {
        Ok(user) => user,
        Err(_) => return Response::redirect(Url::parse("/login").unwrap()),
    };
    
    let d1 = ctx.env.d1("DB")?;
    
    // Get user's calendars
    let calendars = db::get_user_calendars(&d1, user.id).await?;
    
    let tera = templates::init_templates();
    let mut context = Context::new();
    context.insert("user", &user);
    context.insert("calendars", &calendars);
    
    let html = tera.render("calendar_list.html", &context).unwrap();
    Response::from_html(html)
}

// Calendar new page handler
pub async fn calendar_new_page_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // Verify authentication
    let user = match auth::get_current_user(&req, &ctx).await {
        Ok(user) => user,
        Err(_) => return Response::redirect(Url::parse("/login").unwrap()),
    };
    
    let tera = templates::init_templates();
    let mut context = Context::new();
    context.insert("user", &user);
    
    let html = tera.render("calendar_form.html", &context).unwrap();
    Response::from_html(html)
}

// Calendar create API handler
#[derive(Debug, Deserialize)]
struct CreateCalendarRequest {
    name: String,
    slug: String,
    timezone: String,
    duration: i32,
    buffer: i32,
}

pub async fn calendar_create_handler(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // Verify authentication
    let user = match auth::get_current_user(&req, &ctx).await {
        Ok(user) => user,
        Err(_) => return Ok(Response::error("Unauthorized", 401)?),
    };
    
    let form_data: CreateCalendarRequest = req.json().await?;
    let d1 = ctx.env.d1("DB")?;
    
    // Create calendar
    let calendar = db::create_calendar(
        &d1,
        user.id,
        &form_data.name,
        &form_data.slug,
        &form_data.timezone,
        form_data.duration,
        form_data.buffer,
    ).await?;
    
    Ok(Response::ok(json!({ "id": calendar.id }).to_string())?)
}

// Calendar detail handler
pub async fn calendar_detail_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // Verify authentication
    let user = match auth::get_current_user(&req, &ctx).await {
        Ok(user) => user,
        Err(_) => return Response::redirect(Url::parse("/login").unwrap()),
    };
    
    let calendar_id = ctx.param("id").unwrap();
    let d1 = ctx.env.d1("DB")?;
    
    // Get calendar
    let calendar = db::get_calendar(&d1, calendar_id, user.id).await?;
    
    let tera = templates::init_templates();
    let mut context = Context::new();
    context.insert("user", &user);
    context.insert("calendar", &calendar);
    
    let html = tera.render("calendar_detail.html", &context).unwrap();
    Response::from_html(html)
}

// Calendar update API handler
pub async fn calendar_update_handler(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // Verify authentication
    let user = match auth::get_current_user(&req, &ctx).await {
        Ok(user) => user,
        Err(_) => return Ok(Response::error("Unauthorized", 401)?),
    };
    
    let calendar_id = ctx.param("id").unwrap();
    let form_data: CreateCalendarRequest = req.json().await?;
    let d1 = ctx.env.d1("DB")?;
    
    // Update calendar
    db::update_calendar(
        &d1,
        calendar_id,
        user.id,
        &form_data.name,
        &form_data.timezone,
        form_data.duration,
        form_data.buffer,
    ).await?;
    
    Ok(Response::ok(json!({ "success": true }).to_string())?)
}

// Calendar delete API handler
pub async fn calendar_delete_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // Verify authentication
    let user = match auth::get_current_user(&req, &ctx).await {
        Ok(user) => user,
        Err(_) => return Ok(Response::error("Unauthorized", 401)?),
    };
    
    let calendar_id = ctx.param("id").unwrap();
    let d1 = ctx.env.d1("DB")?;
    
    // Delete calendar
    db::delete_calendar(&d1, calendar_id, user.id).await?;
    
    Ok(Response::ok(json!({ "success": true }).to_string())?)
}

// Working hours page handler
pub async fn working_hours_page_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // Verify authentication
    let user = match auth::get_current_user(&req, &ctx).await {
        Ok(user) => user,
        Err(_) => return Response::redirect(Url::parse("/login").unwrap()),
    };
    
    let calendar_id = ctx.param("id").unwrap();
    let d1 = ctx.env.d1("DB")?;
    
    // Get calendar and working hours
    let calendar = db::get_calendar(&d1, calendar_id, user.id).await?;
    let working_hours = db::get_working_hours(&d1, calendar_id).await?;
    
    let tera = templates::init_templates();
    let mut context = Context::new();
    context.insert("user", &user);
    context.insert("calendar", &calendar);
    context.insert("working_hours", &working_hours);
    
    let html = tera.render("working_hours.html", &context).unwrap();
    Response::from_html(html)
}

// Working hours update API handler
#[derive(Debug, Deserialize)]
struct WorkingHoursRequest {
    days: Vec<i32>,
    times: Vec<WorkingTime>,
}

#[derive(Debug, Deserialize)]
pub struct WorkingTime {
    pub day: i32,
    pub start: String,
    pub end: String,
}

pub async fn working_hours_update_handler(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // Verify authentication
    let user = match auth::get_current_user(&req, &ctx).await {
        Ok(user) => user,
        Err(_) => return Ok(Response::error("Unauthorized", 401)?),
    };
    
    let calendar_id = ctx.param("id").unwrap();
    let form_data: WorkingHoursRequest = req.json().await?;
    let d1 = ctx.env.d1("DB")?;
    
    // Update working hours
    db::update_working_hours(&d1, calendar_id, user.id, &form_data.times).await?;
    
    Ok(Response::ok(json!({ "success": true }).to_string())?)
}

// Public calendar view handler
pub async fn public_calendar_handler(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let slug = ctx.param("slug").unwrap();
    let d1 = ctx.env.d1("DB")?;
    
    // Get calendar by slug
    let calendar = db::get_calendar_by_slug(&d1, slug).await?;
    
    let tera = templates::init_templates();
    let mut context = Context::new();
    context.insert("calendar", &calendar);
    context.insert("today", &chrono::Utc::now().format("%Y-%m-%d").to_string());
    context.insert("max_date", &(chrono::Utc::now() + chrono::Duration::days(30)).format("%Y-%m-%d").to_string());
    
    let html = tera.render("public_calendar.html", &context).unwrap();
    Response::from_html(html)
}

// Slots API handler
pub async fn slots_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let slug = ctx.param("slug").unwrap();
    let url = req.url()?;
    let date = url.query_pairs()
        .find(|(key, _)| key == "date")
        .map(|(_, value)| value.to_string())
        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());
    
    let d1 = ctx.env.d1("DB")?;
    
    // Get available slots
    let slots = db::get_available_slots(&d1, slug, &date).await?;
    
    Ok(Response::ok(serde_json::to_string(&slots)?)?)
}

// Booking API handler
#[derive(Debug, Deserialize)]
struct BookingRequest {
    name: String,
    email: String,
    notes: Option<String>,
    slot: String,
}

pub async fn booking_handler(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let slug = ctx.param("slug").unwrap();
    let form_data: BookingRequest = req.json().await?;
    let d1 = ctx.env.d1("DB")?;
    
    // Create booking
    let booking = db::create_booking(
        &d1,
        slug,
        &form_data.name,
        &form_data.email,
        form_data.notes.as_deref(),
        &form_data.slot,
    ).await?;
    
    Ok(Response::ok(json!({ "id": booking.id }).to_string())?)
}

// OAuth callback handler
pub async fn oauth_callback_handler(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let url = req.url()?;
    let code = url.query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.to_string());
    
    if let Some(code) = code {
        // Exchange code for token (simplified)
        // In real implementation, would exchange with Google OAuth
        console_log!("OAuth callback with code: {}", code);
    }
    
    Response::redirect(Url::parse("/dashboard").unwrap())
}

// Static file handler
pub async fn static_handler(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let path = ctx.param("path").unwrap();
    
    // Serve static files from KV storage or embedded
    // For now, return 404
    Response::error("Not found", 404)
}