use worker::{*, Result as WorkerResult};
use worker::d1::D1Database;
use serde::{Deserialize, Serialize};
use serde_json::json;
use chrono::{DateTime, Utc};
use d1orm::*;
use std::sync::atomic::{AtomicBool, Ordering};

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

#[derive(Debug, Serialize, Deserialize, Entity)]
#[table(name = "users")]
struct User {
    #[primary_key]
    id: i64,
    name: String,
    #[unique]
    email: String,
    password_hash: String,
    #[serde(with = "datetime_format")]
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Entity)]
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

#[derive(Debug, Serialize, Deserialize, Entity)]
#[table(name = "working_hours")]
struct WorkingHours {
    #[primary_key]
    id: i64,
    calendar_id: i64,
    day_of_week: i64, // 0=Sunday, 1=Monday, ..., 6=Saturday
    start_time: String, // Format: "HH:MM"
    end_time: String,   // Format: "HH:MM"
    #[serde(with = "datetime_format")]
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Entity)]
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
    let create_users_migration = CreateTableMigration::new("create_users", 1, "users".to_string())
        .column("id", "INTEGER").primary_key()
        .column("name", "TEXT").not_null()
        .column("email", "TEXT").not_null().unique()
        .column("password_hash", "TEXT").not_null()
        .column("created_at", "DATETIME").default("CURRENT_TIMESTAMP");
    
    let create_calendars_migration = CreateTableMigration::new("create_calendars", 2, "calendars".to_string())
        .column("id", "INTEGER").primary_key()
        .column("user_id", "INTEGER").not_null()
        .column("title", "TEXT").not_null()
        .column("description", "TEXT")
        .column("timezone", "TEXT").not_null()
        .column("created_at", "DATETIME").default("CURRENT_TIMESTAMP");
    
    let create_working_hours_migration = CreateTableMigration::new("create_working_hours", 3, "working_hours".to_string())
        .column("id", "INTEGER").primary_key()
        .column("calendar_id", "INTEGER").not_null()
        .column("day_of_week", "INTEGER").not_null()
        .column("start_time", "TEXT").not_null()
        .column("end_time", "TEXT").not_null()
        .column("created_at", "DATETIME").default("CURRENT_TIMESTAMP");
    
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
    
    runner.add_migration(Box::new(create_users_migration));
    runner.add_migration(Box::new(create_calendars_migration));
    runner.add_migration(Box::new(create_working_hours_migration));
    runner.add_migration(Box::new(create_bookings_migration));
    
    runner
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
        .button.secondary { background: #6b7280; }
        .button.secondary:hover { background: #4b5563; }
        .form-group { margin-bottom: 20px; }
        label { display: block; margin-bottom: 5px; font-weight: 500; color: #555; }
        input[type="text"], input[type="email"], input[type="password"] { width: 100%; padding: 10px; border: 1px solid #ddd; border-radius: 5px; font-size: 16px; box-sizing: border-box; }
        input[type="text"]:focus, input[type="email"]:focus, input[type="password"]:focus { outline: none; border-color: #4CAF50; }
        .result { margin-top: 20px; padding: 15px; border-radius: 5px; display: none; }
        .success { background: #dcfce7; color: #166534; border: 1px solid #bbf7d0; }
        .error { background: #fee2e2; color: #991b1b; border: 1px solid #fecaca; }
    </style>
</head>
<body>
    <div class="card">
        <h1 class="header">📅 Calendar Booking System</h1>
        <p style="text-align: center; color: #666; font-size: 18px;">Create your calendar and let people book time slots with you</p>
    </div>

    <div class="card">
        <h2>🚀 Get Started</h2>
        <p>Welcome! Here's how to get started with the calendar booking system:</p>
        <ol>
            <li><strong>Register</strong> - Create your account below</li>
            <li><strong>Set up your calendar</strong> - Configure your working hours and availability</li>
            <li><strong>Share your link</strong> - People can book time slots at <code>/calendars/your-email@domain.com</code></li>
        </ol>
    </div>

    <div class="card">
        <h2>👤 Register or Login</h2>
        
        <div style="display: flex; gap: 20px; margin-bottom: 20px;">
            <button class="button" onclick="showRegistrationForm()">Register New Account</button>
            <button class="button secondary" onclick="showLoginForm()">Login to Existing</button>
        </div>

        <div id="registration-form" style="display: none;">
            <h3>Register New Account</h3>
            <form id="registerForm">
                <div class="form-group">
                    <label for="register-name">Full Name:</label>
                    <input type="text" id="register-name" name="name" required placeholder="Your full name">
                </div>
                <div class="form-group">
                    <label for="register-email">Email:</label>
                    <input type="email" id="register-email" name="email" required placeholder="your-email@domain.com">
                </div>
                <div class="form-group">
                    <label for="register-password">Password:</label>
                    <input type="password" id="register-password" name="password" required placeholder="Choose a secure password">
                </div>
                <button type="submit" class="button">Create Account</button>
            </form>
            <div id="register-result" class="result"></div>
        </div>

        <div id="login-form" style="display: none;">
            <h3>Login to Existing Account</h3>
            <form id="loginForm">
                <div class="form-group">
                    <label for="login-email">Email:</label>
                    <input type="email" id="login-email" name="email" required placeholder="your-email@domain.com">
                </div>
                <div class="form-group">
                    <label for="login-password">Password:</label>
                    <input type="password" id="login-password" name="password" required placeholder="Your password">
                </div>
                <button type="submit" class="button">Login</button>
            </form>
            <div id="login-result" class="result"></div>
        </div>
    </div>

    <script>
        function showRegistrationForm() {
            document.getElementById('registration-form').style.display = 'block';
            document.getElementById('login-form').style.display = 'none';
        }

        function showLoginForm() {
            document.getElementById('login-form').style.display = 'block';
            document.getElementById('registration-form').style.display = 'none';
        }

        document.getElementById('registerForm').addEventListener('submit', async function(e) {
            e.preventDefault();
            
            const resultDiv = document.getElementById('register-result');
            const submitBtn = e.target.querySelector('button');
            
            submitBtn.disabled = true;
            submitBtn.textContent = 'Creating Account...';
            resultDiv.style.display = 'block';
            resultDiv.className = 'result';
            resultDiv.innerHTML = '⏳ Creating your account...';
            
            try {
                const formData = new FormData(e.target);
                const userData = {
                    name: formData.get('name'),
                    email: formData.get('email'),
                    password: formData.get('password')
                };
                
                const response = await fetch('/api/auth/register', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(userData)
                });
                
                const result = await response.json();
                
                if (result.success) {
                    resultDiv.className = 'result success';
                    resultDiv.innerHTML = `
                        ✅ <strong>Account created successfully!</strong><br>
                        Welcome ${result.data.name}!<br>
                        <a href="/dashboard" class="button" style="margin-top: 10px;">Go to Dashboard →</a>
                    `;
                    e.target.reset();
                } else {
                    throw new Error(result.error || 'Registration failed');
                }
            } catch (error) {
                resultDiv.className = 'result error';
                resultDiv.innerHTML = `❌ <strong>Error:</strong> ${error.message}`;
            } finally {
                submitBtn.disabled = false;
                submitBtn.textContent = 'Create Account';
            }
        });

        document.getElementById('loginForm').addEventListener('submit', async function(e) {
            e.preventDefault();
            
            const resultDiv = document.getElementById('login-result');
            const submitBtn = e.target.querySelector('button');
            
            submitBtn.disabled = true;
            submitBtn.textContent = 'Logging in...';
            resultDiv.style.display = 'block';
            resultDiv.className = 'result';
            resultDiv.innerHTML = '⏳ Logging in...';
            
            try {
                const formData = new FormData(e.target);
                const loginData = {
                    email: formData.get('email'),
                    password: formData.get('password')
                };
                
                const response = await fetch('/api/auth/login', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(loginData)
                });
                
                const result = await response.json();
                
                if (result.success) {
                    resultDiv.className = 'result success';
                    resultDiv.innerHTML = `
                        ✅ <strong>Welcome back!</strong><br>
                        <a href="/dashboard" class="button" style="margin-top: 10px;">Go to Dashboard →</a>
                    `;
                    e.target.reset();
                } else {
                    throw new Error(result.error || 'Login failed');
                }
            } catch (error) {
                resultDiv.className = 'result error';
                resultDiv.innerHTML = `❌ <strong>Error:</strong> ${error.message}`;
            } finally {
                submitBtn.disabled = false;
                submitBtn.textContent = 'Login';
            }
        });
    </script>
</body>
</html>
            "#)
        })
        // Authentication endpoints
        .post_async("/api/auth/register", |mut req, ctx| async move {
            #[derive(Deserialize)]
            struct RegisterRequest {
                name: String,
                email: String,
                password: String,
            }
            
            let body: RegisterRequest = req.json().await?;
            let d1_db = ctx.env.get_binding::<D1Database>("DB")?;
            let db = D1Client::new(d1_db);
            
            initialize_database(&db).await.expect("Database initialization failed");
            
            // Simple password hashing (in production, use proper bcrypt)
            let password_hash = format!("hash_{}", body.password); // TODO: Implement proper hashing
            
            match User::create()
                .set_name(body.name.clone())
                .set_email(body.email)
                .set_password_hash(password_hash)
                .set_created_at(chrono::Utc::now())
                .save(&db)
                .await
            {
                Ok(user) => {
                    // Create a default calendar for the user
                    let calendar = match Calendar::create()
                        .set_user_id(user.id)
                        .set_title(format!("{}'s Calendar", body.name))
                        .set_description(Some("My personal booking calendar".to_string()))
                        .set_timezone("UTC".to_string())
                        .set_created_at(chrono::Utc::now())
                        .save(&db)
                        .await
                    {
                        Ok(cal) => cal,
                        Err(e) => return Response::from_json(&json!({
                            "success": false,
                            "error": format!("Failed to create calendar: {}", e)
                        })).map(|r| r.with_status(500)),
                    };
                    
                    Response::from_json(&json!({
                        "success": true,
                        "data": {
                            "id": user.id,
                            "name": user.name,
                            "email": user.email,
                            "calendar_id": calendar.id
                        }
                    }))
                },
                Err(e) => Response::from_json(&json!({
                    "success": false,
                    "error": format!("Registration failed: {}", e)
                })).map(|r| r.with_status(400)),
            }
        })
        .post_async("/api/auth/login", |mut req, ctx| async move {
            #[derive(Deserialize)]
            struct LoginRequest {
                email: String,
                password: String,
            }
            
            let body: LoginRequest = req.json().await?;
            let d1_db = ctx.env.get_binding::<D1Database>("DB")?;
            let db = D1Client::new(d1_db);
            
            initialize_database(&db).await.expect("Database initialization failed");
            
            let password_hash = format!("hash_{}", body.password);
            
            match User::query()
                .where_email_eq(body.email)
                .where_password_hash_eq(password_hash)
                .first(&db)
                .await
            {
                Ok(Some(user)) => Response::from_json(&json!({
                    "success": true,
                    "data": {
                        "id": user.id,
                        "name": user.name,
                        "email": user.email
                    }
                })),
                Ok(None) => Response::from_json(&json!({
                    "success": false,
                    "error": "Invalid email or password"
                })).map(|r| r.with_status(401)),
                Err(e) => Response::from_json(&json!({
                    "success": false,
                    "error": format!("Login failed: {}", e)
                })).map(|r| r.with_status(500)),
            }
        })
        // Dashboard page
        .get("/dashboard", |_req, _ctx| {
            Response::from_html(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Dashboard - Calendar System</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 1000px; margin: 0 auto; padding: 20px; background: #f5f5f5; }
        .card { background: white; padding: 30px; margin: 20px 0; border-radius: 10px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
        .header { color: #4CAF50; text-align: center; margin-bottom: 30px; }
        .button { background: #4CAF50; color: white; padding: 12px 24px; text-decoration: none; border-radius: 5px; display: inline-block; margin: 8px; transition: background 0.3s; }
        .button:hover { background: #45a049; }
        .button.secondary { background: #6b7280; }
        .button.secondary:hover { background: #4b5563; }
        .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 20px; margin-top: 20px; }
        .status-card { background: #e8f5e9; border-left: 4px solid #4CAF50; padding: 20px; border-radius: 5px; }
    </style>
</head>
<body>
    <div class="card">
        <h1 class="header">📅 Calendar Dashboard</h1>
        <p style="text-align: center; color: #666;">Manage your calendar and bookings</p>
        
        <div class="grid">
            <div class="status-card">
                <h3>🕐 Working Hours</h3>
                <p>Set up your availability for bookings</p>
                <a href="/working-hours" class="button">Configure Working Hours</a>
            </div>
            
            <div class="status-card">
                <h3>📅 My Calendar</h3>
                <p>View and manage your calendar settings</p>
                <a href="/my-calendar" class="button">Manage Calendar</a>
            </div>
            
            <div class="status-card">
                <h3>📋 Bookings</h3>
                <p>View all your upcoming bookings</p>
                <a href="/bookings" class="button">View Bookings</a>
            </div>
        </div>
        
        <div style="text-align: center; margin-top: 30px;">
            <h3>🔗 Your Public Booking Link</h3>
            <p>Share this link for people to book time with you:</p>
            <code style="background: #f5f5f5; padding: 10px; border-radius: 5px; display: inline-block;">
                /calendars/your-email@domain.com
            </code>
        </div>
    </div>
</body>
</html>
            "#)
        })
        
        // Public calendar view
        .get_async("/calendars/:email", |_req, ctx| async move {
            let email = ctx.param("email")
                .ok_or_else(|| worker::Error::RustError("Email parameter required".to_string()))?;
            
            let d1_db = ctx.env.get_binding::<D1Database>("DB")?;
            let db = D1Client::new(d1_db);
            
            initialize_database(&db).await.expect("Database initialization failed");
            
            // Find user by email
            let user = match User::query().where_email_eq(email.to_string()).first(&db).await {
                Ok(Some(user)) => user,
                Ok(None) => {
                    return Response::from_html(&format!(r#"
<!DOCTYPE html>
<html>
<head><title>Calendar Not Found</title></head>
<body style="font-family: Arial, sans-serif; text-align: center; padding: 50px;">
    <h1>📅 Calendar Not Found</h1>
    <p>No calendar found for email: {}</p>
    <p><a href="/">← Back to Home</a></p>
</body>
</html>
                    "#, email));
                }
                Err(e) => {
                    return Response::from_json(&json!({
                        "error": format!("Database error: {}", e)
                    })).map(|r| r.with_status(500));
                }
            };
            
            // Get user's calendar
            let calendar = match Calendar::query()
                .where_user_id_eq(user.id)
                .first(&db)
                .await
            {
                Ok(Some(calendar)) => calendar,
                _ => {
                    return Response::from_html(&format!(r#"
<!DOCTYPE html>
<html>
<head><title>Calendar Not Found</title></head>
<body style="font-family: Arial, sans-serif; text-align: center; padding: 50px;">
    <h1>📅 Calendar Not Available</h1>
    <p>Calendar for {} is not available yet.</p>
    <p><a href="/">← Back to Home</a></p>
</body>
</html>
                    "#, email));
                }
            };

            // Generate the calendar booking page similar to the Go version
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
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>📅 Book with {}</h1>
            <p>{}</p>
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
        const calendarId = {calendar_id};
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
            
            fetch('/api/slots/' + calendarId + '?date=' + dateStr + '&timezone=' + encodeURIComponent(userTimezone))
                .then(response => response.json())
                .then(data => {{
                    document.getElementById('selected-date').textContent = 'Available slots for ' + dateStr;
                    const slots = data.slots || [];
                    
                    if (slots.length > 0) {{
                        let slotsHTML = '';
                        slots.forEach(slot => {{
                            if (slot.available) {{
                                slotsHTML += '<div style="background: white; margin-bottom: 10px; padding: 15px; border-radius: 5px; border-left: 4px solid #4CAF50; cursor: pointer;" onclick="bookSlot(\'' + slot.start + '\', \'' + slot.end + '\')">';
                                slotsHTML += '<div style="font-weight: bold;">' + slot.startTime + ' - ' + slot.endTime + '</div>';
                                slotsHTML += '<div style="color: #666; font-size: 14px;">' + slot.duration + ' minutes</div>';
                                slotsHTML += '</div>';
                            }}
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

        function bookSlot(start, end) {{
            const guestEmail = prompt('Enter your email address:');
            const guestName = prompt('Enter your name:');
            const title = prompt('Meeting title:');
            
            if (!guestEmail || !guestName || !title) return;
            
            const bookingData = {{
                calendar_id: calendarId,
                guest_email: guestEmail,
                guest_name: guestName,
                title: title,
                start_time: start,
                end_time: end,
                timezone: userTimezone
            }};
            
            fetch('/api/book', {{
                method: 'POST',
                headers: {{ 'Content-Type': 'application/json' }},
                body: JSON.stringify(bookingData)
            }})
            .then(response => response.json())
            .then(data => {{
                if (data.success) {{
                    alert('Booking confirmed! You should receive a confirmation email shortly.');
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
            "#, user.name, calendar.title, calendar.description.unwrap_or_default(), calendar_id = calendar.id))
        })
        
        // API endpoints for slots and booking
        .get_async("/api/slots/:calendar_id", |req, ctx| async move {
            let calendar_id = ctx.param("calendar_id")
                .and_then(|s| s.parse::<i64>().ok())
                .ok_or_else(|| worker::Error::RustError("Invalid calendar ID".to_string()))?;
            
            let url = req.url()?;
            let date = url.query_pairs()
                .find(|(key, _)| key == "date")
                .map(|(_, value)| value.to_string());
            
            let date = match date {
                Some(d) => d,
                None => return Response::from_json(&json!({
                    "success": false,
                    "error": "Date parameter required"
                })).map(|r| r.with_status(400)),
            };
            
            let d1_db = ctx.env.get_binding::<D1Database>("DB")?;
            let db = D1Client::new(d1_db);
            
            initialize_database(&db).await.expect("Database initialization failed");
            
            // For now, return sample slots (in a real implementation, you'd calculate based on working hours and existing bookings)
            let sample_slots = vec![
                json!({
                    "start": format!("{}T09:00:00Z", date),
                    "end": format!("{}T09:30:00Z", date),
                    "startTime": "09:00",
                    "endTime": "09:30",
                    "duration": 30,
                    "available": true
                }),
                json!({
                    "start": format!("{}T10:00:00Z", date),
                    "end": format!("{}T10:30:00Z", date),
                    "startTime": "10:00",
                    "endTime": "10:30",
                    "duration": 30,
                    "available": true
                }),
                json!({
                    "start": format!("{}T11:00:00Z", date),
                    "end": format!("{}T12:00:00Z", date),
                    "startTime": "11:00",
                    "endTime": "12:00",
                    "duration": 60,
                    "available": true
                })
            ];
            
            Response::from_json(&json!({
                "success": true,
                "date": date,
                "slots": sample_slots
            }))
        })
        
        .post_async("/api/book", |mut req, ctx| async move {
            #[derive(Deserialize)]
            struct BookingRequest {
                calendar_id: i64,
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
            
            // Create the booking
            match Booking::create()
                .set_calendar_id(body.calendar_id)
                .set_guest_email(body.guest_email)
                .set_guest_name(body.guest_name)
                .set_title(body.title)
                .set_notes(body.notes)
                .set_start_time(start_time)
                .set_end_time(end_time)
                .set_status("confirmed".to_string())
                .set_created_at(chrono::Utc::now())
                .save(&db)
                .await
            {
                Ok(booking) => Response::from_json(&json!({
                    "success": true,
                    "data": {
                        "id": booking.id,
                        "title": booking.title,
                        "guest_name": booking.guest_name,
                        "start_time": booking.start_time,
                        "end_time": booking.end_time
                    }
                })),
                Err(e) => Response::from_json(&json!({
                    "success": false,
                    "error": format!("Failed to create booking: {}", e)
                })).map(|r| r.with_status(500)),
            }
        })
        
        .post_async("/api/working-hours", |mut req, ctx| async move {
            #[derive(Deserialize)]
            struct WorkingHoursRequest {
                calendar_id: i64,
                day_of_week: i64,
                start_time: String,
                end_time: String,
            }
            
            let body: WorkingHoursRequest = req.json().await?;
            let d1_db = ctx.env.get_binding::<D1Database>("DB")?;
            let db = D1Client::new(d1_db);
            
            initialize_database(&db).await.expect("Database initialization failed");
            
            match WorkingHours::create()
                .set_calendar_id(body.calendar_id)
                .set_day_of_week(body.day_of_week)
                .set_start_time(body.start_time)
                .set_end_time(body.end_time)
                .set_created_at(chrono::Utc::now())
                .save(&db)
                .await
            {
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