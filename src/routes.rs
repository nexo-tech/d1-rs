// Shared Axum router and handlers for both native and Workers
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Json, Redirect, Response},
    routing::{get, post, put, delete},
    Router,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tera::Context;
use tower_http::cors::CorsLayer;

use crate::{auth, database, templates};

// Application state - conditional based on target
#[derive(Clone)]
pub struct AppState {
    #[cfg(feature = "native")]
    pub db: sea_orm::DatabaseConnection,
    #[cfg(feature = "workers")]
    pub env: worker::Env,
}

// Helper to render templates
fn render_template(template_name: &str, context: &Context) -> Result<Html<String>, StatusCode> {
    let tera = templates::init_templates();
    match tera.render(template_name, context) {
        Ok(html) => Ok(Html(html)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// Helper to get current user from request
async fn get_current_user(
    headers: &axum::http::HeaderMap,
    _state: &AppState,
) -> Result<auth::User, StatusCode> {
    let cookie_header = headers
        .get("cookie")
        .ok_or(StatusCode::UNAUTHORIZED)?
        .to_str()
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    
    let token = cookie_header
        .split(';')
        .find(|c| c.trim().starts_with("auth_token="))
        .and_then(|c| c.split('=').nth(1))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "your-secret-key-here".to_string());
    let claims = auth::verify_token_shared(token, &jwt_secret)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    
    Ok(auth::User {
        id: claims.sub,
        name: claims.name,
        email: claims.email,
    })
}

// Route handlers
pub async fn home_handler() -> Redirect {
    Redirect::permanent("/login")
}

pub async fn login_page_handler() -> Result<Html<String>, StatusCode> {
    let context = Context::new();
    render_template("login.html", &context)
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

pub async fn login_handler(
    State(state): State<AppState>,
    Json(login_data): Json<LoginRequest>,
) -> Result<Response, StatusCode> {
    match auth::verify_user(&state.db, &login_data.email, &login_data.password).await {
        Ok(user) => {
            let token = auth::generate_token(&user).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let cookie = format!("auth_token={}; HttpOnly; Path=/; Max-Age=86400", token);
            
            Ok((
                StatusCode::OK,
                [("set-cookie", cookie.as_str())],
                Json(json!({"success": true})),
            ).into_response())
        }
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}

pub async fn register_page_handler() -> Result<Html<String>, StatusCode> {
    let context = Context::new();
    render_template("register.html", &context)
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub email: String,
    pub password: String,
}

pub async fn register_handler(
    State(state): State<AppState>,
    Json(register_data): Json<RegisterRequest>,
) -> Result<Response, StatusCode> {
    match auth::create_user(&state.db, &register_data.name, &register_data.email, &register_data.password).await {
        Ok(user) => {
            let token = auth::generate_token(&user).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let cookie = format!("auth_token={}; HttpOnly; Path=/; Max-Age=86400", token);
            
            Ok((
                StatusCode::OK,
                [("set-cookie", cookie.as_str())],
                Json(json!({"success": true})),
            ).into_response())
        }
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

pub async fn logout_handler() -> Result<Response, StatusCode> {
    let cookie = "auth_token=; HttpOnly; Path=/; Max-Age=0";
    
    Ok((
        StatusCode::FOUND,
        [
            ("location", "/login"),
            ("set-cookie", cookie)
        ],
        "",
    ).into_response())
}

pub async fn dashboard_handler(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Html<String>, StatusCode> {
    let user = get_current_user(&headers, &state).await?;
    
    let calendar_count = database::get_calendar_count(&state.db, &user.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? as i32;
    let booking_count = database::get_booking_count(&state.db, &user.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? as i32;
    
    let mut context = Context::new();
    context.insert("user", &user);
    context.insert("calendar_count", &calendar_count);
    context.insert("booking_count", &booking_count);
    
    render_template("dashboard.html", &context)
}

pub async fn calendars_list_handler(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Html<String>, StatusCode> {
    let user = get_current_user(&headers, &state).await?;
    
    let calendars = database::get_user_calendars(&state.db, &user.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut context = Context::new();
    context.insert("user", &user);
    context.insert("calendars", &calendars);
    
    render_template("calendar_list.html", &context)
}

pub async fn calendar_new_page_handler(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Html<String>, StatusCode> {
    let user = get_current_user(&headers, &state).await?;
    
    let mut context = Context::new();
    context.insert("user", &user);
    
    render_template("calendar_form.html", &context)
}

pub async fn calendar_create_handler(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(form_data): Json<database::CreateCalendarRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user = get_current_user(&headers, &state).await?;
    
    match database::create_calendar(&state.db, &user.id, &form_data).await {
        Ok(calendar) => Ok(Json(json!({"id": calendar.id}))),
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

pub async fn calendar_detail_handler(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(calendar_id): Path<String>,
) -> Result<Html<String>, StatusCode> {
    let user = get_current_user(&headers, &state).await?;
    
    match database::get_calendar(&state.db, &calendar_id, &user.id).await {
        Ok(Some(calendar)) => {
            let mut context = Context::new();
            context.insert("user", &user);
            context.insert("calendar", &calendar);
            render_template("calendar_detail.html", &context)
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn public_calendar_handler(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Html<String>, StatusCode> {
    match database::get_calendar_by_slug(&state.db, &slug).await {
        Ok(Some(calendar)) => {
            let mut context = Context::new();
            context.insert("calendar", &calendar);
            context.insert("today", &chrono::Utc::now().format("%Y-%m-%d").to_string());
            context.insert("max_date", &(chrono::Utc::now() + chrono::Duration::days(30)).format("%Y-%m-%d").to_string());
            
            render_template("public_calendar.html", &context)
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn slots_handler(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<String>>, StatusCode> {
    let date = params.get("date")
        .map(|s| s.clone())
        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());
    
    match database::get_available_slots(&state.db, &slug, &date).await {
        Ok(slots) => Ok(Json(slots)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn booking_handler(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(form_data): Json<database::BookingRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match database::create_booking(&state.db, &slug, &form_data).await {
        Ok(booking) => Ok(Json(json!({"id": booking.id}))),
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

// Create the shared router
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Public calendar routes
        .route("/", get(home_handler))
        .route("/book/:slug", get(public_calendar_handler))
        .route("/api/book/:slug", post(booking_handler))
        .route("/api/slots/:slug", get(slots_handler))
        
        // Auth routes
        .route("/login", get(login_page_handler))
        .route("/api/login", post(login_handler))
        .route("/register", get(register_page_handler))
        .route("/api/register", post(register_handler))
        .route("/api/logout", post(logout_handler))
        
        // Dashboard routes (protected)
        .route("/dashboard", get(dashboard_handler))
        .route("/dashboard/calendars", get(calendars_list_handler))
        .route("/dashboard/calendars/new", get(calendar_new_page_handler))
        .route("/api/calendars", post(calendar_create_handler))
        .route("/dashboard/calendars/:id", get(calendar_detail_handler))
        
        // Add CORS for API routes
        .layer(CorsLayer::permissive())
        .with_state(state)
}