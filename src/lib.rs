use worker::{*, Result as WorkerResult};
use worker::d1::D1Database;
use serde::{Deserialize, Serialize};
use serde_json::json;
use chrono::{DateTime, Utc, Datelike};
use d1orm::*;
use std::sync::atomic::{AtomicBool, Ordering};
use url::Url;

// Custom deserializer for D1 datetime format
mod datetime_format {
    use chrono::{DateTime, Utc, NaiveDateTime};
    use serde::{self, Deserialize, Serializer, Deserializer};

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
#[table(name = "tokens")]
struct Token {
    #[primary_key]
    id: i64,
    #[unique]
    user_email: String,
    access_token: String,
    refresh_token: String,
    token_type: String,
    #[serde(with = "datetime_format")]
    expiry: DateTime<Utc>,
    #[serde(with = "datetime_format")]
    created_at: DateTime<Utc>,
    #[serde(with = "datetime_format")]
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity)]
#[table(name = "calendars")]
struct Calendar {
    #[primary_key]
    id: i64,
    user_id: i64,
    title: String,
    description: Option<String>,
    timezone: String,
    #[serde(with = "datetime_format")]
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity)]
#[table(name = "working_hours")]
struct WorkingHours {
    #[primary_key]
    id: i64,
    start_time: String, // Format: "HH:MM"
    end_time: String,   // Format: "HH:MM"
    timezone: String,
    working_days: String, // Comma-separated days: "monday,tuesday,wednesday,thursday,friday"
    #[serde(with = "datetime_format")]
    created_at: DateTime<Utc>,
    #[serde(with = "datetime_format")]
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity)]
#[table(name = "bookings")]
struct Booking {
    #[primary_key]
    id: i64,
    calendar_id: i64,
    guest_email: String,
    guest_name: String,
    title: String,
    notes: Option<String>,
    #[serde(with = "datetime_format")]
    start_time: DateTime<Utc>,
    #[serde(with = "datetime_format")]
    end_time: DateTime<Utc>,
    status: String, // "confirmed", "cancelled"
    #[serde(with = "datetime_format")]
    created_at: DateTime<Utc>,
}

static MIGRATIONS_INITIALIZED: AtomicBool = AtomicBool::new(false);

// Global migration runner for this application
fn get_migration_runner() -> MigrationRunner {
    let mut runner = MigrationRunner::new();
    
    // Register application-specific migrations
    let create_tokens_migration = CreateTableMigration::new("create_tokens", 1, "tokens".to_string())
        .column("id", "INTEGER").primary_key()
        .column("user_email", "TEXT").not_null().unique()
        .column("access_token", "TEXT").not_null()
        .column("refresh_token", "TEXT").not_null()
        .column("token_type", "TEXT").not_null()
        .column("expiry", "DATETIME").not_null()
        .column("created_at", "DATETIME").default("CURRENT_TIMESTAMP")
        .column("updated_at", "DATETIME").default("CURRENT_TIMESTAMP");
    
    let create_calendars_migration = CreateTableMigration::new("create_calendars", 2, "calendars".to_string())
        .column("id", "INTEGER").primary_key()
        .column("user_id", "INTEGER").not_null()
        .column("title", "TEXT").not_null()
        .column("description", "TEXT")
        .column("timezone", "TEXT").not_null()
        .column("created_at", "DATETIME").default("CURRENT_TIMESTAMP");
    
    let create_working_hours_migration = CreateTableMigration::new("create_working_hours", 3, "working_hours".to_string())
        .column("id", "INTEGER").primary_key()
        .column("start_time", "TEXT").not_null()
        .column("end_time", "TEXT").not_null()
        .column("timezone", "TEXT").not_null()
        .column("working_days", "TEXT").not_null()
        .column("created_at", "DATETIME").default("CURRENT_TIMESTAMP")
        .column("updated_at", "DATETIME").default("CURRENT_TIMESTAMP");
    
    let create_bookings_migration = CreateTableMigration::new("create_bookings", 4, "bookings".to_string())
        .column("id", "INTEGER").primary_key()
        .column("calendar_id", "INTEGER").not_null()
        .column("guest_email", "TEXT").not_null()
        .column("guest_name", "TEXT").not_null()
        .column("title", "TEXT").not_null()
        .column("notes", "TEXT")
        .column("start_time", "DATETIME").not_null()
        .column("end_time", "DATETIME").not_null()
        .column("status", "TEXT").not_null()
        .column("created_at", "DATETIME").default("CURRENT_TIMESTAMP");
    
    runner.add_migration(Box::new(create_tokens_migration));
    runner.add_migration(Box::new(create_calendars_migration));
    runner.add_migration(Box::new(create_working_hours_migration));
    runner.add_migration(Box::new(create_bookings_migration));
    
    runner
}

#[derive(Deserialize)]
struct GoogleUserInfo {
    email: String,
}

#[derive(Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    token_type: String,
    expires_in: i64,
}

async fn exchange_code_for_token(code: &str, ctx: &RouteContext<()>) -> WorkerResult<OAuthTokenResponse> {
    let client_id = ctx.env.var("GOOGLE_CLIENT_ID")?.to_string();
    let client_secret = ctx.env.var("GOOGLE_CLIENT_SECRET")?.to_string();
    let redirect_uri = ctx.env.var("GOOGLE_REDIRECT_URL")?.to_string();
    
    let token_url = "https://oauth2.googleapis.com/token";
    let params = [
        ("code", code),
        ("client_id", &client_id),
        ("client_secret", &client_secret),
        ("redirect_uri", &redirect_uri),
        ("grant_type", "authorization_code"),
    ];
    
    let client = reqwest::Client::new();
    let response = client.post(token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| worker::Error::RustError(format!("HTTP request failed: {}", e)))?;
        
    if !response.status().is_success() {
        return Err(worker::Error::RustError(format!("OAuth token exchange failed: {}", response.status())));
    }
    
    let token_response: OAuthTokenResponse = response.json()
        .await
        .map_err(|e| worker::Error::RustError(format!("Failed to parse token response: {}", e)))?;
    
    Ok(token_response)
}

async fn get_user_info(access_token: &str) -> WorkerResult<GoogleUserInfo> {
    let client = reqwest::Client::new();
    let response = client.get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| worker::Error::RustError(format!("HTTP request failed: {}", e)))?;
        
    if !response.status().is_success() {
        return Err(worker::Error::RustError(format!("Failed to get user info: {}", response.status())));
    }
    
    let user_info: GoogleUserInfo = response.json()
        .await
        .map_err(|e| worker::Error::RustError(format!("Failed to parse user info: {}", e)))?;
    
    Ok(user_info)
}

fn generate_oauth_url(ctx: &RouteContext<()>, state: &str) -> WorkerResult<String> {
    console_log!("Getting GOOGLE_CLIENT_ID from env");
    let client_id = ctx.env.var("GOOGLE_CLIENT_ID")?.to_string();
    console_log!("Client ID: {}", client_id);
    
    console_log!("Getting GOOGLE_REDIRECT_URL from env");
    let redirect_uri = ctx.env.var("GOOGLE_REDIRECT_URL")?.to_string();
    console_log!("Redirect URI: {}", redirect_uri);
    
    console_log!("Parsing base OAuth URL");
    let mut url = Url::parse("https://accounts.google.com/o/oauth2/v2/auth")
        .map_err(|e| worker::Error::RustError(format!("Invalid URL: {}", e)))?;
    console_log!("Base URL parsed successfully");
        
    console_log!("Adding query parameters");
    url.query_pairs_mut()
        .append_pair("client_id", &client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", "https://www.googleapis.com/auth/calendar https://www.googleapis.com/auth/userinfo.email")
        .append_pair("access_type", "offline")
        .append_pair("approval_prompt", "force")
        .append_pair("state", state);
    console_log!("Query parameters added");
        
    let final_url = url.to_string();
    console_log!("Final URL length: {}", final_url.len());
    Ok(final_url)
}

fn generate_state() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .to_string()
}

#[derive(Deserialize)]
struct CalendarEvent {
    summary: Option<String>,
    start: Option<CalendarDateTime>,
    end: Option<CalendarDateTime>,
}

#[derive(Deserialize)]
struct CalendarDateTime {
    #[serde(rename = "dateTime")]
    date_time: Option<String>,
    date: Option<String>,
}

#[derive(Deserialize)]
struct CalendarEventsResponse {
    items: Vec<CalendarEvent>,
}

async fn fetch_calendar_events(access_token: &str, start_time: &DateTime<Utc>, end_time: &DateTime<Utc>) -> WorkerResult<Vec<serde_json::Value>> {
    let client = reqwest::Client::new();
    let url = format!("https://www.googleapis.com/calendar/v3/calendars/primary/events?timeMin={}&timeMax={}&singleEvents=true&orderBy=startTime",
        start_time.to_rfc3339(),
        end_time.to_rfc3339()
    );
    
    let response = client.get(&url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| worker::Error::RustError(format!("HTTP request failed: {}", e)))?;
    
    if !response.status().is_success() {
        return Err(worker::Error::RustError(format!("Calendar API request failed: {}", response.status())));
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

async fn create_calendar_event(access_token: &str, start_time: &DateTime<Utc>, end_time: &DateTime<Utc>, title: &str, guest_email: &str, notes: Option<&str>) -> WorkerResult<String> {
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
        "sendUpdates": "all"
    });
    
    let response = client.post(url)
        .bearer_auth(access_token)
        .json(&event_data)
        .send()
        .await
        .map_err(|e| worker::Error::RustError(format!("HTTP request failed: {}", e)))?;
    
    if !response.status().is_success() {
        return Err(worker::Error::RustError(format!("Calendar event creation failed: {}", response.status())));
    }
    
    let created_event: serde_json::Value = response.json()
        .await
        .map_err(|e| worker::Error::RustError(format!("Failed to parse event response: {}", e)))?;
    
    let event_id = created_event["id"].as_str()
        .ok_or_else(|| worker::Error::RustError("Event ID not found in response".to_string()))?;
    
    Ok(event_id.to_string())
}

async fn refresh_access_token(refresh_token: &str, ctx: &RouteContext<()>) -> WorkerResult<OAuthTokenResponse> {
    let client_id = ctx.env.var("GOOGLE_CLIENT_ID")?.to_string();
    let client_secret = ctx.env.var("GOOGLE_CLIENT_SECRET")?.to_string();
    
    let token_url = "https://oauth2.googleapis.com/token";
    let params = [
        ("refresh_token", refresh_token),
        ("client_id", &client_id),
        ("client_secret", &client_secret),
        ("grant_type", "refresh_token"),
    ];
    
    let client = reqwest::Client::new();
    let response = client.post(token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| worker::Error::RustError(format!("HTTP request failed: {}", e)))?;
        
    if !response.status().is_success() {
        return Err(worker::Error::RustError(format!("Token refresh failed: {}", response.status())));
    }
    
    let token_response: OAuthTokenResponse = response.json()
        .await
        .map_err(|e| worker::Error::RustError(format!("Failed to parse token response: {}", e)))?;
    
    Ok(token_response)
}

async fn initialize_database(db: &D1Client) -> d1orm::Result<()> {
    // Check if this worker instance has already run migrations
    if MIGRATIONS_INITIALIZED.load(Ordering::Acquire) {
        worker::console_log!("🟢 Database already initialized in this worker instance");
        return Ok(());
    }

    worker::console_log!("🚀 WORKER STARTUP: Initializing database with schema-independent migration system...");
    
    let runner = get_migration_runner();
    worker::console_log!("📋 Migration runner configured for this application");
    worker::console_log!("🔒 Attempting to acquire distributed migration lock...");
    
    // This handles distributed locking automatically
    match runner.run_pending_migrations(db).await {
        Ok(applied) => {
            if applied.is_empty() {
                worker::console_log!("⏭️ No migrations applied - either up to date or another worker acquired lock");
                worker::console_log!("🔍 Verifying migration state...");
                // Use the ORM's generic verification method instead of hardcoded table checks
                match runner.verify_migrations_up_to_date(db).await {
                    Ok(true) => {
                        worker::console_log!("✅ All migrations are up to date");
                        MIGRATIONS_INITIALIZED.store(true, Ordering::Release);
                    },
                    Ok(false) => {
                        worker::console_log!("❌ Some migrations are missing - database may not be up to date");
                        return Err(d1orm::D1OrmError::Database("Migration verification failed - not all migrations are applied".to_string()));
                    },
                    Err(e) => {
                        worker::console_log!("❌ Failed to verify migration state: {}", e);
                        return Err(e);
                    }
                }
            } else {
                worker::console_log!("✅ Successfully applied {} migrations: {:?}", applied.len(), applied);
                worker::console_log!("🎉 Database initialization completed by this worker");
                MIGRATIONS_INITIALIZED.store(true, Ordering::Release);
            }
            
            worker::console_log!("🟢 Worker database initialization complete");
            Ok(())
        },
        Err(e) => {
            worker::console_log!("❌ CRITICAL: Database initialization failed: {}", e);
            Err(e)
        }
    }
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> WorkerResult<Response> {
    let router = Router::new();

    router
        .get("/", |_req, _ctx| {
            Response::from_html(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Calendar Booking System</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; background: #f5f5f5; }
        .card { background: white; padding: 30px; margin: 20px 0; border-radius: 10px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
        .header { color: #4CAF50; text-align: center; margin-bottom: 30px; }
        .button { background: #4CAF50; color: white; padding: 12px 24px; text-decoration: none; border-radius: 5px; display: inline-block; margin: 8px; transition: background 0.3s; }
        .button:hover { background: #45a049; }
        .google-btn { background: #db4437; }
        .google-btn:hover { background: #c23321; }
    </style>
</head>
<body>
    <div class="card">
        <h1 class="header">📅 Calendar Booking System</h1>
        <p style="text-align: center; color: #666; font-size: 18px;">Connect your Google Calendar and let people book time slots with you</p>
    </div>

    <div class="card">
        <h2>🚀 Get Started</h2>
        <p>Welcome! Here's how to get started with the calendar booking system:</p>
        <ol>
            <li><strong>Sign in with Google</strong> - Connect your Google Calendar account</li>
            <li><strong>Set up working hours</strong> - Configure your availability</li>
            <li><strong>Share your link</strong> - People can book time slots using your public booking link</li>
        </ol>
    </div>

    <div class="card" style="text-align: center;">
        <h2>🔐 Sign in with Google</h2>
        <p>Connect your Google Calendar to get started</p>
        <a href="/auth/google" class="button google-btn">
            📅 Sign in with Google Calendar
        </a>
        <br><br>
        <p style="color: #666; font-size: 14px;">
            We'll access your Google Calendar to check availability and create bookings.<br>
            Your calendar data is never stored permanently.
        </p>
    </div>
</body>
</html>
            "#)
        })
        // Google OAuth endpoints (no database needed for this route)
        .get_async("/auth/google", |_req, ctx| async move {
            console_log!("Starting /auth/google route");
            
            // Check if environment variables are set
            let client_id_check = ctx.env.var("GOOGLE_CLIENT_ID");
            if client_id_check.is_err() {
                console_error!("GOOGLE_CLIENT_ID not found in environment variables");
                return Response::error("Configuration error: GOOGLE_CLIENT_ID not set", 500);
            }
            
            let redirect_url_check = ctx.env.var("GOOGLE_REDIRECT_URL");
            if redirect_url_check.is_err() {
                console_error!("GOOGLE_REDIRECT_URL not found in environment variables");
                return Response::error("Configuration error: GOOGLE_REDIRECT_URL not set", 500);
            }
            
            console_log!("Environment variables found, generating OAuth URL");
            
            let state = generate_state();
            let auth_url = match generate_oauth_url(&ctx, &state) {
                Ok(url) => url,
                Err(e) => {
                    console_error!("Failed to generate OAuth URL: {:?}", e);
                    return Response::error(format!("Failed to generate OAuth URL: {}", e), 500);
                }
            };
            
            console_log!("OAuth URL generated: {}", auth_url);
            
            console_log!("Creating headers");
            let mut headers = Headers::new();
            headers.set("Set-Cookie", &format!("oauth_state={}; HttpOnly; Secure; SameSite=Lax; Max-Age=300", state))?;
            console_log!("Headers created");
            
            console_log!("Creating manual redirect response");
            
            // Create a manual redirect response instead of using Response::redirect
            let mut response = Response::empty()?;
            response = response.with_status(302);
            
            console_log!("Setting Location header");
            headers.set("Location", &auth_url)?;
            console_log!("Location header set");
            
            console_log!("Applying headers to response");
            response = response.with_headers(headers);
            console_log!("Headers applied");
            
            Ok(response)
        })
        .get_async("/auth/callback", |req, ctx| async move {
            let url = req.url()?;
            let query_pairs: std::collections::HashMap<String, String> = url.query_pairs().into_owned().collect();
            
            let code = query_pairs.get("code")
                .ok_or_else(|| worker::Error::RustError("Authorization code not found".to_string()))?;
            
            let state = query_pairs.get("state")
                .ok_or_else(|| worker::Error::RustError("State parameter not found".to_string()))?;
            
            // Verify state from cookie (simplified for demo - in production, use proper cookie parsing)
            // TODO: Implement proper cookie verification
            
            let token_response = exchange_code_for_token(code, &ctx).await?;
            let user_info = get_user_info(&token_response.access_token).await?;
            
            let d1_db = ctx.env.get_binding::<D1Database>("DB")?;
            let db = D1Client::new(d1_db);
            
            initialize_database(&db).await.expect("Database initialization failed");
            
            let expiry = chrono::Utc::now() + chrono::Duration::seconds(token_response.expires_in);
            
            // Save or update token
            match Token::create()
                .set_user_email(user_info.email.clone())
                .set_access_token(token_response.access_token)
                .set_refresh_token(token_response.refresh_token.unwrap_or_default())
                .set_token_type(token_response.token_type)
                .set_expiry(expiry)
                .set_created_at(chrono::Utc::now())
                .set_updated_at(chrono::Utc::now())
                .save(&db)
                .await
            {
                Ok(_) => {
                    Response::from_html(&format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Authentication Successful</title>
    <style>
        body {{ font-family: Arial, sans-serif; text-align: center; padding: 50px; background: #f5f5f5; }}
        .card {{ background: white; padding: 30px; border-radius: 10px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); display: inline-block; }}
        .success {{ color: #4CAF50; }}
        .button {{ background: #4CAF50; color: white; padding: 12px 24px; text-decoration: none; border-radius: 5px; display: inline-block; margin: 10px; }}
    </style>
</head>
<body>
    <div class="card">
        <h1 class="success">✅ Authentication Successful!</h1>
        <p>Welcome, {}!</p>
        <p>Your Google Calendar has been connected successfully.</p>
        <a href="/dashboard" class="button">Go to Dashboard</a>
        <a href="/working-hours" class="button">Set Working Hours</a>
    </div>
</body>
</html>
                    "#, user_info.email))
                },
                Err(e) => Response::from_json(&json!({
                    "success": false,
                    "error": format!("Failed to save token: {}", e)
                })).map(|r| r.with_status(500)),
            }
        })
        // Dashboard page
        .get_async("/dashboard", |_req, ctx| async move {
            let d1_db = ctx.env.get_binding::<D1Database>("DB")?;
            let db = D1Client::new(d1_db);
            
            initialize_database(&db).await.expect("Database initialization failed");
            
            // Get all connected calendars (tokens)
            let tokens = match Token::query().all(&db).await {
                Ok(tokens) => tokens,
                Err(_) => vec![], 
            };
            
            let emails_list = tokens.iter()
                .map(|token| token.user_email.as_str())
                .collect::<Vec<&str>>()
                .join("<br>");
            
            let emails_code = tokens.iter()
                .map(|token| format!("/calendar/{}", token.user_email))
                .collect::<Vec<String>>()
                .join("<br>");
            
            Response::from_html(&format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Dashboard - Calendar System</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 1000px; margin: 0 auto; padding: 20px; background: #f5f5f5; }}
        .card {{ background: white; padding: 30px; margin: 20px 0; border-radius: 10px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }}
        .header {{ color: #4CAF50; text-align: center; margin-bottom: 30px; }}
        .button {{ background: #4CAF50; color: white; padding: 12px 24px; text-decoration: none; border-radius: 5px; display: inline-block; margin: 8px; transition: background 0.3s; }}
        .button:hover {{ background: #45a049; }}
        .button.secondary {{ background: #6b7280; }}
        .button.secondary:hover {{ background: #4b5563; }}
        .grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 20px; margin-top: 20px; }}
        .status-card {{ background: #e8f5e9; border-left: 4px solid #4CAF50; padding: 20px; border-radius: 5px; }}
    </style>
</head>
<body>
    <div class="card">
        <h1 class="header">📅 Calendar Dashboard</h1>
        <p style="text-align: center; color: #666;">Manage your Google Calendar integration and bookings</p>
        
        <div class="grid">
            <div class="status-card">
                <h3>🕐 Working Hours</h3>
                <p>Set up your availability for bookings</p>
                <a href="/working-hours" class="button">Configure Working Hours</a>
            </div>
            
            <div class="status-card">
                <h3>📅 Connected Calendars</h3>
                <p>Currently connected: {}</p>
                <p style="font-size: 12px; color: #666;">{}</p>
                <a href="/auth/google" class="button">Add Another Calendar</a>
            </div>
            
            <div class="status-card">
                <h3>📋 View All Connected Calendars</h3>
                <p>See events from all your connected accounts</p>
                <a href="/api/emails" class="button">View Connected Emails</a>
            </div>
        </div>
        
        <div style="text-align: center; margin-top: 30px;">
            <h3>🔗 Your Public Booking Links</h3>
            <p>Share these links for people to book time with you:</p>
            <div style="background: #f5f5f5; padding: 15px; border-radius: 5px; font-family: monospace; font-size: 14px;">
                {}
            </div>
        </div>
    </div>
</body>
</html>
            "#, tokens.len(), emails_list, emails_code))
        })
        
        // Working Hours page
        .get_async("/working-hours", |_req, ctx| async move {
            let d1_db = ctx.env.get_binding::<D1Database>("DB")?;
            let db = D1Client::new(d1_db);
            
            initialize_database(&db).await.expect("Database initialization failed");
            
            // Get existing working hours
            let existing_wh = WorkingHours::query().first(&db).await.unwrap_or(None);
            
            let (start_time, end_time, timezone, working_days) = if let Some(wh) = existing_wh {{
                (wh.start_time, wh.end_time, wh.timezone, wh.working_days)
            }} else {{
                ("09:00".to_string(), "17:00".to_string(), "UTC".to_string(), "".to_string())
            }};
            
            let days = working_days.split(",").collect::<Vec<&str>>();
            
            Response::from_html(&format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Working Hours - Calendar System</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; background: #f5f5f5; }}
        .card {{ background: white; padding: 30px; margin: 20px 0; border-radius: 10px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }}
        .header {{ color: #4CAF50; text-align: center; margin-bottom: 30px; }}
        .form-group {{ margin-bottom: 20px; }}
        label {{ display: block; margin-bottom: 5px; font-weight: 500; color: #555; }}
        input[type="time"], input[type="text"], select {{ width: 100%; padding: 10px; border: 1px solid #ddd; border-radius: 5px; font-size: 16px; box-sizing: border-box; }}
        .checkbox-group {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(120px, 1fr)); gap: 10px; margin-top: 10px; }}
        .checkbox-item {{ display: flex; align-items: center; }}
        .checkbox-item input {{ width: auto; margin-right: 8px; }}
        .button {{ background: #4CAF50; color: white; padding: 12px 24px; border: none; border-radius: 5px; font-size: 16px; cursor: pointer; }}
        .button:hover {{ background: #45a049; }}
        .result {{ margin-top: 20px; padding: 15px; border-radius: 5px; display: none; }}
        .success {{ background: #dcfce7; color: #166534; border: 1px solid #bbf7d0; }}
        .error {{ background: #fee2e2; color: #991b1b; border: 1px solid #fecaca; }}
    </style>
</head>
<body>
    <div class="card">
        <h1 class="header">🕐 Working Hours Configuration</h1>
        <p style="text-align: center; color: #666;">Set your availability for bookings</p>
        
        <form id="workingHoursForm">
            <div class="form-group">
                <label for="start_time">Start Time:</label>
                <input type="time" id="start_time" name="start_time" value="{}" required>
            </div>
            
            <div class="form-group">
                <label for="end_time">End Time:</label>
                <input type="time" id="end_time" name="end_time" value="{}" required>
            </div>
            
            <div class="form-group">
                <label for="timezone">Timezone:</label>
                <select id="timezone" name="timezone" required>
                    <option value="UTC"{}>UTC</option>
                    <option value="America/New_York"{}>Eastern Time (ET)</option>
                    <option value="America/Chicago"{}>Central Time (CT)</option>
                    <option value="America/Denver"{}>Mountain Time (MT)</option>
                    <option value="America/Los_Angeles"{}>Pacific Time (PT)</option>
                    <option value="Europe/London"{}>London</option>
                    <option value="Europe/Paris"{}>Paris</option>
                    <option value="Asia/Tokyo"{}>Tokyo</option>
                </select>
            </div>
            
            <div class="form-group">
                <label>Working Days:</label>
                <div class="checkbox-group">
                    <div class="checkbox-item">
                        <input type="checkbox" id="monday" name="days" value="monday"{}> 
                        <label for="monday">Monday</label>
                    </div>
                    <div class="checkbox-item">
                        <input type="checkbox" id="tuesday" name="days" value="tuesday"{}> 
                        <label for="tuesday">Tuesday</label>
                    </div>
                    <div class="checkbox-item">
                        <input type="checkbox" id="wednesday" name="days" value="wednesday"{}> 
                        <label for="wednesday">Wednesday</label>
                    </div>
                    <div class="checkbox-item">
                        <input type="checkbox" id="thursday" name="days" value="thursday"{}> 
                        <label for="thursday">Thursday</label>
                    </div>
                    <div class="checkbox-item">
                        <input type="checkbox" id="friday" name="days" value="friday"{}> 
                        <label for="friday">Friday</label>
                    </div>
                    <div class="checkbox-item">
                        <input type="checkbox" id="saturday" name="days" value="saturday"{}> 
                        <label for="saturday">Saturday</label>
                    </div>
                    <div class="checkbox-item">
                        <input type="checkbox" id="sunday" name="days" value="sunday"{}> 
                        <label for="sunday">Sunday</label>
                    </div>
                </div>
            </div>
            
            <button type="submit" class="button">Save Working Hours</button>
        </form>
        
        <div id="result" class="result"></div>
        
        <div style="text-align: center; margin-top: 30px;">
            <a href="/dashboard" style="color: #4CAF50; text-decoration: none;">← Back to Dashboard</a>
        </div>
    </div>
    
    <script>
        document.getElementById('workingHoursForm').addEventListener('submit', async function(e) {{
            e.preventDefault();
            
            const resultDiv = document.getElementById('result');
            const submitBtn = e.target.querySelector('button');
            
            submitBtn.disabled = true;
            submitBtn.textContent = 'Saving...';
            resultDiv.style.display = 'block';
            resultDiv.className = 'result';
            resultDiv.innerHTML = '⏳ Saving working hours...';
            
            try {{
                const formData = new FormData(e.target);
                const checkedDays = Array.from(formData.getAll('days'));
                
                const workingHoursData = {{
                    start_time: formData.get('start_time'),
                    end_time: formData.get('end_time'),
                    timezone: formData.get('timezone'),
                    working_days: checkedDays.join(',')
                }};
                
                const response = await fetch('/api/working-hours', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify(workingHoursData)
                }});
                
                const result = await response.json();
                
                if (result.success) {{
                    resultDiv.className = 'result success';
                    resultDiv.innerHTML = '✅ <strong>Working hours saved successfully!</strong>';
                }} else {{
                    throw new Error(result.error || 'Failed to save working hours');
                }}
            }} catch (error) {{
                resultDiv.className = 'result error';
                resultDiv.innerHTML = `❌ <strong>Error:</strong> ${{error.message}}`;
            }} finally {{
                submitBtn.disabled = false;
                submitBtn.textContent = 'Save Working Hours';
            }}
        }});
    </script>
</body>
</html>
            "#, 
                start_time, end_time,
                if timezone == "UTC" { " selected" } else { "" },
                if timezone == "America/New_York" { " selected" } else { "" },
                if timezone == "America/Chicago" { " selected" } else { "" },
                if timezone == "America/Denver" { " selected" } else { "" },
                if timezone == "America/Los_Angeles" { " selected" } else { "" },
                if timezone == "Europe/London" { " selected" } else { "" },
                if timezone == "Europe/Paris" { " selected" } else { "" },
                if timezone == "Asia/Tokyo" { " selected" } else { "" },
                if days.contains(&"monday") { " checked" } else { "" },
                if days.contains(&"tuesday") { " checked" } else { "" },
                if days.contains(&"wednesday") { " checked" } else { "" },
                if days.contains(&"thursday") { " checked" } else { "" },
                if days.contains(&"friday") { " checked" } else { "" },
                if days.contains(&"saturday") { " checked" } else { "" },
                if days.contains(&"sunday") { " checked" } else { "" }
            ))
        })
        
        // Public calendar view - updated for email-based system
        .get_async("/calendar/:email", |_req, ctx| async move {
            let email = ctx.param("email")
                .ok_or_else(|| worker::Error::RustError("Email parameter required".to_string()))?;
            
            let d1_db = ctx.env.get_binding::<D1Database>("DB")?;
            let db = D1Client::new(d1_db);
            
            initialize_database(&db).await.expect("Database initialization failed");
            
            // Check if token exists for this email
            match Token::query().where_user_email_eq(email.to_string()).first(&db).await {
                Ok(Some(_)) => {
                    // Token exists, show calendar
                    Response::from_html(&format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Book with {} - Calendar</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 0; padding: 20px; background: #f5f5f5; }}
        .container {{ max-width: 1200px; margin: 0 auto; background: white; border-radius: 10px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); overflow: hidden; }}
        .header {{ background: #4CAF50; color: white; padding: 20px; text-align: center; }}
        .calendar-container {{ display: flex; min-height: 600px; }}
        .calendar {{ flex: 1; padding: 20px; }}
        .slots-panel {{ width: 350px; border-left: 1px solid #ddd; padding: 20px; background: #fafafa; }}
        .month-header {{ display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; }}
        .month-nav {{ background: #4CAF50; color: white; border: none; padding: 8px 12px; border-radius: 5px; cursor: pointer; }}
        .calendar-grid {{ display: grid; grid-template-columns: repeat(7, 1fr); gap: 1px; background: #ddd; border: 1px solid #ddd; border-radius: 8px; overflow: hidden; }}
        .calendar-header {{ background: #f0f0f0; padding: 15px 10px; text-align: center; font-weight: bold; }}
        .calendar-day {{ background: white; padding: 15px 10px; text-align: center; cursor: pointer; min-height: 50px; display: flex; align-items: center; justify-content: center; }}
        .calendar-day:hover {{ background: #e8f5e9; }}
        .calendar-day.selected {{ background: #4CAF50; color: white; }}
        .timezone-info {{ background: #e3f2fd; padding: 10px; border-radius: 5px; margin-bottom: 20px; }}
        .slot-item {{ background: white; margin-bottom: 10px; padding: 15px; border-radius: 5px; border-left: 4px solid #4CAF50; cursor: pointer; }}
        .slot-item:hover {{ background: #f9f9f9; }}
        .slot-item.unavailable {{ border-left-color: #ccc; background: #f5f5f5; cursor: not-allowed; opacity: 0.6; }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>📅 Book with {}</h1>
            <p>Select a time slot to book a meeting</p>
        </div>
        <div class="calendar-container">
            <div class="calendar">
                <div class="month-header">
                    <button class="month-nav" onclick="previousMonth()">&lt;</button>
                    <h2 id="monthYear"></h2>
                    <button class="month-nav" onclick="nextMonth()">&gt;</button>
                </div>
                <div id="calendar-grid" class="calendar-grid">
                    <div class="calendar-header">Sun</div>
                    <div class="calendar-header">Mon</div>
                    <div class="calendar-header">Tue</div>
                    <div class="calendar-header">Wed</div>
                    <div class="calendar-header">Thu</div>
                    <div class="calendar-header">Fri</div>
                    <div class="calendar-header">Sat</div>
                </div>
            </div>
            <div class="slots-panel">
                <div class="timezone-info">
                    Times shown in: <span id="user-timezone"></span>
                </div>
                <h3>Available Time Slots</h3>
                <div id="selected-date">Select a date to view available slots</div>
                <div id="time-slots"></div>
            </div>
        </div>
    </div>

    <script>
        const calendarEmail = '{}';
        let currentMonth = new Date().getMonth();
        let currentYear = new Date().getFullYear();
        let selectedDate = null;
        let userTimezone = Intl.DateTimeFormat().resolvedOptions().timeZone;

        document.getElementById('user-timezone').textContent = userTimezone;

        function renderCalendar() {{
            const monthNames = ["January", "February", "March", "April", "May", "June",
                "July", "August", "September", "October", "November", "December"];
            
            document.getElementById('monthYear').textContent = monthNames[currentMonth] + ' ' + currentYear;
            
            const firstDay = new Date(currentYear, currentMonth, 1).getDay();
            const daysInMonth = new Date(currentYear, currentMonth + 1, 0).getDate();
            
            let daysHTML = '';
            
            for (let day = 1; day <= daysInMonth; day++) {{
                const dateStr = currentYear + '-' + String(currentMonth + 1).padStart(2, '0') + '-' + String(day).padStart(2, '0');
                const isToday = new Date().getDate() === day && 
                               new Date().getMonth() === currentMonth && 
                               new Date().getFullYear() === currentYear;
                
                const classes = 'calendar-day' + (isToday ? ' today' : '');
                daysHTML += '<div class="' + classes + '" onclick="selectDate(\'' + dateStr + '\')" data-date="' + dateStr + '">' + day + '</div>';
            }}
            
            const calendarGrid = document.getElementById('calendar-grid');
            const existingDays = calendarGrid.querySelectorAll('.calendar-day');
            existingDays.forEach(day => day.remove());
            
            calendarGrid.insertAdjacentHTML('beforeend', daysHTML);
        }}

        function selectDate(dateStr) {{
            document.querySelectorAll('.calendar-day').forEach(day => {{
                day.classList.remove('selected');
            }});
            
            const dayElement = document.querySelector('[data-date="' + dateStr + '"]');
            if (dayElement) dayElement.classList.add('selected');
            selectedDate = dateStr;
            loadTimeSlots(dateStr);
        }}

        function loadTimeSlots(dateStr) {{
            document.getElementById('selected-date').textContent = 'Loading slots for ' + dateStr + '...';
            document.getElementById('time-slots').innerHTML = '';
            
            fetch('/api/slots/' + encodeURIComponent(calendarEmail) + '?date=' + dateStr + '&timezone=' + encodeURIComponent(userTimezone))
                .then(response => response.json())
                .then(data => {{
                    document.getElementById('selected-date').textContent = 'Available slots for ' + dateStr;
                    const slots = data.slots || [];
                    
                    if (slots.length > 0) {{
                        let slotsHTML = '';
                        slots.forEach(slot => {{
                            const available = slot.available;
                            const classes = 'slot-item' + (available ? '' : ' unavailable');
                            const onclick = available ? `onclick="bookSlot('${{slot.start}}', '${{slot.end}}', '${{slot.duration}}')"` : '';
                            
                            slotsHTML += '<div class="' + classes + '" ' + onclick + '>';
                            slotsHTML += '<div style="font-weight: bold;">' + slot.startTime + ' - ' + slot.endTime + '</div>';
                            slotsHTML += '<div style="color: #666; font-size: 14px;">' + slot.duration + ' minutes</div>';
                            if (!available) {{
                                slotsHTML += '<div style="color: #999; font-size: 12px;">Not available</div>';
                            }}
                            slotsHTML += '</div>';
                        }});
                        document.getElementById('time-slots').innerHTML = slotsHTML;
                    }} else {{
                        document.getElementById('time-slots').innerHTML = '<p>No available time slots for this date.</p>';
                    }}
                }})
                .catch(error => {{
                    document.getElementById('selected-date').textContent = 'Error loading slots for ' + dateStr;
                    console.error('Error:', error);
                }});
        }}

        function bookSlot(start, end, duration) {{
            const guestEmail = prompt('Enter your email address:');
            const guestName = prompt('Enter your name:');
            const title = prompt('Meeting title:', 'Meeting with ' + calendarEmail);
            
            if (!guestEmail || !guestName || !title) return;
            
            const bookingData = {{
                calendar_email: calendarEmail,
                guest_email: guestEmail,
                guest_name: guestName,
                title: title,
                start_time: start,
                end_time: end,
                notes: 'Meeting duration: ' + duration + ' minutes'
            }};
            
            fetch('/api/book', {{
                method: 'POST',
                headers: {{ 'Content-Type': 'application/json' }},
                body: JSON.stringify(bookingData)
            }})
            .then(response => response.json())
            .then(data => {{
                if (data.success) {{
                    alert('Booking confirmed! Event created in Google Calendar. You should receive a confirmation email shortly.');
                    loadTimeSlots(selectedDate);
                }} else {{
                    alert('Error: ' + (data.error || 'Booking failed'));
                }}
            }})
            .catch(error => {{
                alert('Error creating booking: ' + error);
            }});
        }}

        function previousMonth() {{
            currentMonth--;
            if (currentMonth < 0) {{
                currentMonth = 11;
                currentYear--;
            }}
            renderCalendar();
        }}

        function nextMonth() {{
            currentMonth++;
            if (currentMonth > 11) {{
                currentMonth = 0;
                currentYear++;
            }}
            renderCalendar();
        }}

        renderCalendar();
    </script>
</body>
</html>
                    "#, email, email, email))
                },
                Ok(None) => {
                    Response::from_html(&format!(r#"
<!DOCTYPE html>
<html>
<head><title>Calendar Not Found</title></head>
<body style="font-family: Arial, sans-serif; text-align: center; padding: 50px;">
    <h1>📅 Calendar Not Found</h1>
    <p>No calendar found for email: {}</p>
    <p>This email address is not connected to our calendar system.</p>
    <p><a href="/">← Back to Home</a></p>
</body>
</html>
                    "#, email))
                }
                Err(e) => {
                    Response::from_json(&json!({
                        "error": format!("Database error: {}", e)
                    })).map(|r| r.with_status(500))
                }
            }
        })
        
        // API endpoints for slots and booking - email-based slots
        .get_async("/api/slots/:email", |req, ctx| async move {
            let email = ctx.param("email")
                .ok_or_else(|| worker::Error::RustError("Email parameter required".to_string()))?;
            
            let url = req.url()?;
            let query_pairs: std::collections::HashMap<String, String> = url.query_pairs().into_owned().collect();
            
            let date = query_pairs.get("date")
                .ok_or_else(|| worker::Error::RustError("Date parameter required".to_string()))?;
            
            let timezone = query_pairs.get("timezone").unwrap_or(&"UTC".to_string()).clone();
            
            let d1_db = ctx.env.get_binding::<D1Database>("DB")?;
            let db = D1Client::new(d1_db);
            
            initialize_database(&db).await.expect("Database initialization failed");
            
            // Parse the date
            let date_parsed = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .map_err(|_| worker::Error::RustError("Invalid date format. Use YYYY-MM-DD".to_string()))?;
                
            let start_of_day = date_parsed.and_hms_opt(0, 0, 0).unwrap().and_utc();
            let end_of_day = start_of_day + chrono::Duration::days(1);
            
            // Get working hours
            let working_hours = WorkingHours::query().first(&db).await.unwrap_or(None);
            
            let mut slots = Vec::new();
            
            if let Some(wh) = working_hours {
                // Check if the requested day is a working day
                let day_of_week = date_parsed.weekday().to_string().to_lowercase();
                let working_days: Vec<&str> = wh.working_days.split(",").collect();
                
                if working_days.contains(&day_of_week.as_str()) {
                    // Parse working hours times
                    if let (Ok(start_time), Ok(end_time)) = (
                        chrono::NaiveTime::parse_from_str(&wh.start_time, "%H:%M"),
                        chrono::NaiveTime::parse_from_str(&wh.end_time, "%H:%M")
                    ) {
                        let mut current_time = date_parsed.and_time(start_time).and_utc();
                        let end_datetime = date_parsed.and_time(end_time).and_utc();
                        
                        // Generate 30 and 60 minute slots
                        for duration in [30, 60] {
                            let mut slot_time = current_time;
                            
                            while slot_time + chrono::Duration::minutes(duration) <= end_datetime {
                                let slot_end = slot_time + chrono::Duration::minutes(duration);
                                
                                slots.push(json!({
                                    "start": slot_time.to_rfc3339(),
                                    "end": slot_end.to_rfc3339(),
                                    "startTime": slot_time.format("%H:%M").to_string(),
                                    "endTime": slot_end.format("%H:%M").to_string(),
                                    "duration": duration,
                                    "available": true
                                }));
                                
                                slot_time = slot_time + chrono::Duration::minutes(30); // 30 min increments
                            }
                        }
                    }
                }
            }
            
            // Get existing events from Google Calendar to mark conflicts
            if let Ok(token) = Token::query().where_user_email_eq(email.to_string()).first(&db).await {
                if let Some(token) = token {
                    // Check if token is expired and refresh if needed
                    let now = chrono::Utc::now();
                    let access_token = if now >= token.expiry && !token.refresh_token.is_empty() {
                        match refresh_access_token(&token.refresh_token, &ctx).await {
                            Ok(new_token) => {
                                // Update token in database
                                let _ = Token::update(token.id)
                                    .set_access_token(new_token.access_token.clone())
                                    .set_expiry(now + chrono::Duration::seconds(new_token.expires_in))
                                    .set_updated_at(now)
                                    .save(&db).await;
                                new_token.access_token
                            },
                            Err(_) => token.access_token.clone(), // Use existing token if refresh fails
                        }
                    } else {
                        token.access_token.clone()
                    };
                    
                    // Fetch events for the day
                    if let Ok(events) = fetch_calendar_events(&access_token, &start_of_day, &end_of_day).await {
                        // Mark conflicting slots as unavailable
                        for i in 0..slots.len() {
                            let slot_start = slots[i]["start"].as_str().unwrap();
                            let slot_end = slots[i]["end"].as_str().unwrap();
                            
                            if let (Ok(slot_start_time), Ok(slot_end_time)) = (
                                chrono::DateTime::parse_from_rfc3339(slot_start),
                                chrono::DateTime::parse_from_rfc3339(slot_end)
                            ) {
                                for event in &events {
                                    if let (Some(event_start_str), Some(event_end_str)) = (
                                        event["start"].as_str(),
                                        event["end"].as_str()
                                    ) {
                                        if let (Ok(event_start), Ok(event_end)) = (
                                            chrono::DateTime::parse_from_rfc3339(event_start_str),
                                            chrono::DateTime::parse_from_rfc3339(event_end_str)
                                        ) {
                                            // Check for overlap
                                            if slot_start_time < event_end && event_start < slot_end_time {
                                                slots[i]["available"] = json!(false);
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // Sort slots by start time and duration
            slots.sort_by(|a, b| {
                let start_cmp = a["start"].as_str().cmp(&b["start"].as_str());
                if start_cmp == std::cmp::Ordering::Equal {
                    a["duration"].as_i64().cmp(&b["duration"].as_i64())
                } else {
                    start_cmp
                }
            });
            
            Response::from_json(&json!({
                "success": true,
                "date": date,
                "slots": slots
            }))
        })
        
        .post_async("/api/book", |mut req, ctx| async move {
            #[derive(Deserialize)]
            struct BookingRequest {
                calendar_email: String, // Email of calendar owner
                guest_email: String,
                guest_name: String,
                title: String,
                start_time: String,
                end_time: String,
                notes: Option<String>,
            }
            
            let body: BookingRequest = req.json().await?;
            let d1_db = ctx.env.get_binding::<D1Database>("DB")?;
            let db = D1Client::new(d1_db);
            
            initialize_database(&db).await.expect("Database initialization failed");
            
            // Parse datetime strings
            let start_time = chrono::DateTime::parse_from_rfc3339(&body.start_time)
                .map_err(|_| worker::Error::RustError("Invalid start time format".to_string()))?
                .with_timezone(&chrono::Utc);
                
            let end_time = chrono::DateTime::parse_from_rfc3339(&body.end_time)
                .map_err(|_| worker::Error::RustError("Invalid end time format".to_string()))?
                .with_timezone(&chrono::Utc);
            
            // Get the calendar owner's token
            let token = match Token::query()
                .where_user_email_eq(body.calendar_email.clone())
                .first(&db)
                .await
            {
                Ok(Some(token)) => token,
                Ok(None) => return Response::from_json(&json!({
                    "success": false,
                    "error": format!("Calendar not found for email: {}", body.calendar_email)
                })).map(|r| r.with_status(404)),
                Err(e) => return Response::from_json(&json!({
                    "success": false,
                    "error": format!("Database error: {}", e)
                })).map(|r| r.with_status(500)),
            };
            
            // Check if token is expired and refresh if needed
            let now = chrono::Utc::now();
            let access_token = if now >= token.expiry && !token.refresh_token.is_empty() {
                match refresh_access_token(&token.refresh_token, &ctx).await {
                    Ok(new_token) => {
                        // Update token in database
                        let _ = Token::update(token.id)
                            .set_access_token(new_token.access_token.clone())
                            .set_expiry(now + chrono::Duration::seconds(new_token.expires_in))
                            .set_updated_at(now)
                            .save(&db).await;
                        new_token.access_token
                    },
                    Err(e) => return Response::from_json(&json!({
                        "success": false,
                        "error": format!("Failed to refresh access token: {}", e)
                    })).map(|r| r.with_status(401)),
                }
            } else {
                token.access_token.clone()
            };
            
            // Create event in Google Calendar
            match create_calendar_event(
                &access_token,
                &start_time,
                &end_time,
                &body.title,
                &body.guest_email,
                body.notes.as_deref()
            ).await {
                Ok(event_id) => {
                    // Also store the booking locally for reference
                    let _ = Booking::create()
                        .set_calendar_id(1) // Dummy ID, we're using Google Calendar now
                        .set_guest_email(body.guest_email.clone())
                        .set_guest_name(body.guest_name.clone())
                        .set_title(body.title.clone())
                        .set_notes(body.notes.clone())
                        .set_start_time(start_time)
                        .set_end_time(end_time)
                        .set_status("confirmed".to_string())
                        .set_created_at(chrono::Utc::now())
                        .save(&db)
                        .await;
                    
                    Response::from_json(&json!({
                        "success": true,
                        "data": {
                            "event_id": event_id,
                            "title": body.title,
                            "guest_name": body.guest_name,
                            "guest_email": body.guest_email,
                            "calendar_email": body.calendar_email,
                            "start_time": start_time.to_rfc3339(),
                            "end_time": end_time.to_rfc3339()
                        }
                    }))
                },
                Err(e) => Response::from_json(&json!({
                    "success": false,
                    "error": format!("Failed to create calendar event: {}", e)
                })).map(|r| r.with_status(500)),
            }
        })
        
        .post_async("/api/working-hours", |mut req, ctx| async move {
            #[derive(Deserialize)]
            struct WorkingHoursRequest {
                start_time: String,
                end_time: String,
                timezone: String,
                working_days: String,
            }
            
            let body: WorkingHoursRequest = req.json().await?;
            let d1_db = ctx.env.get_binding::<D1Database>("DB")?;
            let db = D1Client::new(d1_db);
            
            initialize_database(&db).await.expect("Database initialization failed");
            
            // Check if working hours already exist (since we only store one record)
            let existing_wh = WorkingHours::query().first(&db).await.unwrap_or(None);
            
            let result = if let Some(wh) = existing_wh {
                // Update existing record
                WorkingHours::update(wh.id)
                    .set_start_time(body.start_time.clone())
                    .set_end_time(body.end_time.clone())
                    .set_timezone(body.timezone.clone())
                    .set_working_days(body.working_days.clone())
                    .set_updated_at(chrono::Utc::now())
                    .save(&db).await
            } else {
                // Create new record
                WorkingHours::create()
                    .set_start_time(body.start_time)
                    .set_end_time(body.end_time)
                    .set_timezone(body.timezone)
                    .set_working_days(body.working_days)
                    .set_created_at(chrono::Utc::now())
                    .set_updated_at(chrono::Utc::now())
                    .save(&db)
                    .await
            };
            
            match result {
                Ok(working_hours) => Response::from_json(&json!({
                    "success": true,
                    "data": working_hours
                })),
                Err(e) => Response::from_json(&json!({
                    "success": false,
                    "error": format!("Failed to save working hours: {}", e)
                })).map(|r| r.with_status(500)),
            }
        })
        
        .get_async("/api/emails", |_req, ctx| async move {
            let d1_db = ctx.env.get_binding::<D1Database>("DB")?;
            let db = D1Client::new(d1_db);
            
            initialize_database(&db).await.expect("Database initialization failed");
            
            match Token::query().all(&db).await {
                Ok(tokens) => {
                    let emails: Vec<String> = tokens.iter().map(|t| t.user_email.clone()).collect();
                    Response::from_json(&json!({
                        "emails": emails
                    }))
                },
                Err(e) => Response::from_json(&json!({
                    "success": false,
                    "error": format!("Failed to get emails: {}", e)
                })).map(|r| r.with_status(500)),
            }
        })
        
        .get_async("/api/bookings/:calendar_id", |_req, ctx| async move {
            let calendar_id = ctx.param("calendar_id")
                .and_then(|s| s.parse::<i64>().ok())
                .ok_or_else(|| worker::Error::RustError("Invalid calendar ID".to_string()))?;
            
            let d1_db = ctx.env.get_binding::<D1Database>("DB")?;
            let db = D1Client::new(d1_db);
            
            initialize_database(&db).await.expect("Database initialization failed");
            
            match Booking::query()
                .where_calendar_id_eq(calendar_id)
                .where_status_eq("confirmed".to_string())
                .all(&db)
                .await
            {
                Ok(bookings) => Response::from_json(&json!({
                    "success": true,
                    "data": bookings
                })),
                Err(e) => Response::from_json(&json!({
                    "success": false,
                    "error": format!("Failed to fetch bookings: {}", e)
                })).map(|r| r.with_status(500)),
            }
        })
        .run(req, env)
        .await
}