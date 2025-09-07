// Direct Worker handlers that use shared business logic without Axum
use worker::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tera::Context;

use crate::{auth, shared_types::*, templates, database_trait::{DatabaseTrait, D1Database}};

// Helper to parse query parameters
fn parse_query_params(url: &Url) -> HashMap<String, String> {
    let mut params = HashMap::new();
    for (key, value) in url.query_pairs() {
        params.insert(key.to_string(), value.to_string());
    }
    params
}

// Helper to render templates
fn render_template(template_name: &str, context: &Context) -> Result<String> {
    let tera = templates::init_templates();
    tera.render(template_name, context)
        .map_err(|e| Error::RustError(format!("Template error: {}", e)))
}

// Helper to get current user from cookies
async fn get_current_user_from_cookies(headers: &Headers) -> Result<Option<auth::User>> {
    let cookie_header = match headers.get("cookie")? {
        Some(cookies) => cookies,
        None => return Ok(None),
    };
    
    let token = cookie_header
        .split(';')
        .find(|c| c.trim().starts_with("auth_token="))
        .and_then(|c| c.split('=').nth(1))
        .ok_or_else(|| Error::RustError("No auth token found".to_string()))?;
    
    let jwt_secret = "your-secret-key-here"; // TODO: Get from env
    let claims = auth::verify_token_shared(token, jwt_secret)
        .map_err(|e| Error::RustError(format!("Token verification failed: {}", e)))?;
    
    Ok(Some(auth::User {
        id: claims.sub,
        name: claims.name,
        email: claims.email,
    }))
}

pub async fn handle_request(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let url = req.url()?;
    let method = req.method();
    let path = url.path();
    
    let db = D1Database { env };

    match (method, path) {
        // Public routes
        (Method::Get, "/") => {
            Response::redirect(Url::parse("http://localhost/login")?)
        },
        
        (Method::Get, "/login") => {
            let context = Context::new();
            let html = render_template("login.html", &context)?;
            Response::from_html(html)
        },
        
        (Method::Post, "/api/login") => {
            let login_data: LoginRequest = req.json().await?;
            
            match db.verify_user_password(&login_data.email, &login_data.password).await {
                Ok(Some(user)) => {
                    let token = auth::generate_token(&user)
                        .map_err(|e| Error::RustError(format!("Token generation failed: {}", e)))?;
                    
                    let mut response = Response::from_json(&json!({"success": true}))?;
                    response.headers_mut().set("set-cookie", &format!("auth_token={}; HttpOnly; Path=/; Max-Age=86400", token))?;
                    response.headers_mut().set("content-type", "application/json")?;
                    Ok(response)
                },
                Ok(None) => Response::error("Unauthorized", 401),
                Err(_) => Response::error("Internal Server Error", 500),
            }
        },
        
        (Method::Get, "/register") => {
            let context = Context::new();
            let html = render_template("register.html", &context)?;
            Response::from_html(html)
        },
        
        (Method::Post, "/api/register") => {
            let register_data: RegisterRequest = req.json().await?;
            let password_hash = auth::hash_password(&register_data.password)
                .map_err(|e| Error::RustError(format!("Password hashing failed: {}", e)))?;
            
            match db.create_user(&register_data.name, &register_data.email, &password_hash).await {
                Ok(user) => {
                    let token = auth::generate_token(&user)
                        .map_err(|e| Error::RustError(format!("Token generation failed: {}", e)))?;
                    
                    let mut response = Response::from_json(&json!({"success": true}))?;
                    response.headers_mut().set("set-cookie", &format!("auth_token={}; HttpOnly; Path=/; Max-Age=86400", token))?;
                    response.headers_mut().set("content-type", "application/json")?;
                    Ok(response)
                },
                Err(_) => Response::error("Bad Request", 400),
            }
        },
        
        (Method::Post, "/api/logout") => {
            let mut response = Response::empty()?;
            response = response.with_status(302);
            response.headers_mut().set("location", "/login")?;
            response.headers_mut().set("set-cookie", "auth_token=; HttpOnly; Path=/; Max-Age=0")?;
            Ok(response)
        },
        
        (Method::Get, "/dashboard") => {
            let user = match get_current_user_from_cookies(&req.headers()).await? {
                Some(user) => user,
                None => return Response::redirect(Url::parse("http://localhost/login")?),
            };
            
            let calendar_count = db.get_calendar_count(&user.id).await.unwrap_or(0) as i32;
            let booking_count = db.get_booking_count(&user.id).await.unwrap_or(0) as i32;
            
            let mut context = Context::new();
            context.insert("user", &user);
            context.insert("calendar_count", &calendar_count);
            context.insert("booking_count", &booking_count);
            
            let html = render_template("dashboard.html", &context)?;
            Response::from_html(html)
        },
        
        (Method::Get, "/dashboard/calendars") => {
            let user = match get_current_user_from_cookies(&req.headers()).await? {
                Some(user) => user,
                None => return Response::redirect(Url::parse("http://localhost/login")?),
            };
            
            let calendars = db.get_user_calendars(&user.id).await.unwrap_or_default();
            
            let mut context = Context::new();
            context.insert("user", &user);
            context.insert("calendars", &calendars);
            
            let html = render_template("calendar_list.html", &context)?;
            Response::from_html(html)
        },
        
        (Method::Get, "/dashboard/calendars/new") => {
            let user = match get_current_user_from_cookies(&req.headers()).await? {
                Some(user) => user,
                None => return Response::redirect(Url::parse("http://localhost/login")?),
            };
            
            let mut context = Context::new();
            context.insert("user", &user);
            
            let html = render_template("calendar_form.html", &context)?;
            Response::from_html(html)
        },
        
        (Method::Post, "/api/calendars") => {
            let user = match get_current_user_from_cookies(&req.headers()).await? {
                Some(user) => user,
                None => return Response::error("Unauthorized", 401),
            };
            
            let form_data: CreateCalendarRequest = req.json().await?;
            
            match db.create_calendar(&user.id, &form_data).await {
                Ok(calendar) => Response::from_json(&json!({"id": calendar.id})),
                Err(_) => Response::error("Bad Request", 400),
            }
        },
        
        // Public calendar routes
        (Method::Get, path) if path.starts_with("/book/") => {
            let slug = path.strip_prefix("/book/").unwrap_or("");
            
            match db.get_calendar_by_slug(slug).await {
                Ok(Some(calendar)) => {
                    let mut context = Context::new();
                    context.insert("calendar", &calendar);
                    context.insert("today", &chrono::Utc::now().format("%Y-%m-%d").to_string());
                    context.insert("max_date", &(chrono::Utc::now() + chrono::Duration::days(30)).format("%Y-%m-%d").to_string());
                    
                    let html = render_template("public_calendar.html", &context)?;
                    Response::from_html(html)
                },
                Ok(None) => Response::error("Not Found", 404),
                Err(_) => Response::error("Internal Server Error", 500),
            }
        },
        
        (Method::Get, path) if path.starts_with("/api/slots/") => {
            let slug = path.strip_prefix("/api/slots/").unwrap_or("");
            let params = parse_query_params(&url);
            let date = params.get("date")
                .cloned()
                .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());
            
            match db.get_available_slots(slug, &date).await {
                Ok(slots) => Response::from_json(&slots),
                Err(_) => Response::error("Internal Server Error", 500),
            }
        },
        
        (Method::Post, path) if path.starts_with("/api/book/") => {
            let slug = path.strip_prefix("/api/book/").unwrap_or("");
            let form_data: BookingRequest = req.json().await?;
            
            match db.create_booking(slug, &form_data).await {
                Ok(booking) => Response::from_json(&json!({"id": booking.id})),
                Err(_) => Response::error("Bad Request", 400),
            }
        },
        
        _ => Response::error("Not Found", 404),
    }
}

#[derive(Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct RegisterRequest {
    name: String,
    email: String,
    password: String,
}