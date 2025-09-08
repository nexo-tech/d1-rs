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
        .column("created_at", "DATETIME").default("CURRENT_TIMESTAMP");
    
    runner.add_migration(Box::new(create_users_migration));
    
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
                // Only mark as initialized if we know migrations are actually up to date
                // For now, let's be more careful about this
                worker::console_log!("🔍 Verifying migration state...");
                // Check if the users table exists to verify migrations are actually complete
                match db.execute("SELECT name FROM sqlite_master WHERE type='table' AND name='users'", &[]).await {
                    Ok(result) if !result.rows.is_empty() => {
                        worker::console_log!("✅ Users table exists - migrations are up to date");
                        MIGRATIONS_INITIALIZED.store(true, Ordering::Release);
                    },
                    _ => {
                        worker::console_log!("❌ Users table missing - migrations may not have run properly");
                        return Err(d1orm::D1OrmError::Database("Migration verification failed - users table not found".to_string()));
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
    <title>Cloudflare Worker with D1ORM</title>
    <style>
        body { font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }
        .card { background: #f5f5f5; padding: 20px; margin: 10px 0; border-radius: 8px; }
        .header { color: #1e40af; }
        .button { background: #1e40af; color: white; padding: 10px 20px; text-decoration: none; border-radius: 4px; display: inline-block; margin: 5px; }
        .button:hover { background: #1d4ed8; }
        pre { background: #1e293b; color: #e2e8f0; padding: 12px; border-radius: 6px; overflow-x: auto; }
    </style>
</head>
<body>
    <h1 class="header">Welcome to Cloudflare Worker with D1ORM</h1>
    
    <div class="card">
        <h2>🚀 Quick Actions</h2>
        <a href="/form" class="button">➕ Add New User</a>
        <a href="/users" class="button">👥 View All Users</a>
        <a href="/api/users" class="button">📊 API: Get Users JSON</a>
    </div>

    <div class="card">
        <h2>Type-Safe D1-First ORM</h2>
        <p>This Cloudflare Worker uses D1ORM, a type-safe D1-first ORM written in Rust with ergonomic query builders.</p>
        <h3>Features:</h3>
        <ul>
            <li>✅ Type-safe query builder with method generation</li>
            <li>✅ Direct D1 integration (no intermediate layers)</li>
            <li>✅ WASM-optimized compilation</li>
            <li>✅ Human-first API design</li>
            <li>✅ Rust-based migrations with distributed locking</li>
            <li>✅ Schema evolution support</li>
        </ul>
        <h3>API Endpoints:</h3>
        <ul>
            <li><a href="/users">View Users (HTML)</a></li>
            <li><a href="/api/users">GET /api/users - List all users</a></li>
            <li><a href="/api/user/1">GET /api/user/:id - Get user by ID</a></li>
            <li>POST /api/users - Create new user</li>
            <li>PUT /api/user/:id - Update user</li>
            <li>DELETE /api/user/:id - Delete user</li>
        </ul>
    </div>
</body>
</html>
            "#)
        })
        .get("/form", |_req, _ctx| {
            Response::from_html(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Add New User - D1ORM</title>
    <style>
        body { 
            font-family: Arial, sans-serif; 
            max-width: 600px; 
            margin: 0 auto; 
            padding: 20px; 
            background: #f8fafc;
        }
        .card { 
            background: white; 
            padding: 30px; 
            border-radius: 12px; 
            box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
        }
        .header { color: #1e40af; margin-bottom: 20px; }
        .form-group { margin-bottom: 20px; }
        label { 
            display: block; 
            margin-bottom: 8px; 
            font-weight: bold; 
            color: #374151;
        }
        input[type="text"], input[type="email"] { 
            width: 100%; 
            padding: 12px; 
            border: 2px solid #e5e7eb; 
            border-radius: 6px; 
            font-size: 16px;
            box-sizing: border-box;
        }
        input[type="text"]:focus, input[type="email"]:focus { 
            outline: none; 
            border-color: #1e40af; 
        }
        .submit-btn { 
            background: #16a34a; 
            color: white; 
            padding: 12px 24px; 
            border: none; 
            border-radius: 6px; 
            font-size: 16px; 
            cursor: pointer; 
            width: 100%;
        }
        .submit-btn:hover { background: #15803d; }
        .back-btn { 
            background: #6b7280; 
            color: white; 
            padding: 8px 16px; 
            text-decoration: none; 
            border-radius: 6px; 
            display: inline-block; 
            margin-bottom: 20px;
        }
        .back-btn:hover { background: #4b5563; }
        .result { 
            margin-top: 20px; 
            padding: 15px; 
            border-radius: 6px; 
            display: none;
        }
        .success { background: #dcfce7; color: #166534; border: 1px solid #bbf7d0; }
        .error { background: #fee2e2; color: #991b1b; border: 1px solid #fecaca; }
        .loading { color: #1e40af; }
    </style>
</head>
<body>
    <div class="card">
        <a href="/" class="back-btn">← Back to Home</a>
        <h1 class="header">➕ Add New User</h1>
        
        <form id="userForm">
            <div class="form-group">
                <label for="name">Name:</label>
                <input type="text" id="name" name="name" required placeholder="Enter user's full name">
            </div>
            
            <div class="form-group">
                <label for="email">Email:</label>
                <input type="email" id="email" name="email" required placeholder="user@example.com">
            </div>
            
            <button type="submit" class="submit-btn">Create User</button>
        </form>
        
        <div id="result" class="result"></div>
    </div>

    <script>
        document.getElementById('userForm').addEventListener('submit', async function(e) {
            e.preventDefault();
            
            const resultDiv = document.getElementById('result');
            const submitBtn = document.querySelector('.submit-btn');
            
            // Show loading state
            submitBtn.disabled = true;
            submitBtn.textContent = 'Creating User...';
            resultDiv.style.display = 'block';
            resultDiv.className = 'result loading';
            resultDiv.innerHTML = '⏳ Creating user...';
            
            try {
                const formData = new FormData(e.target);
                const userData = {
                    name: formData.get('name'),
                    email: formData.get('email')
                };
                
                const response = await fetch('/api/users', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                    },
                    body: JSON.stringify(userData)
                });
                
                const result = await response.json();
                
                if (result.success) {
                    resultDiv.className = 'result success';
                    resultDiv.innerHTML = `
                        ✅ <strong>Success!</strong><br>
                        User created with ID: ${result.data.id}<br>
                        Name: ${result.data.name}<br>
                        Email: ${result.data.email}<br>
                        <a href="/users" style="color: #16a34a;">View all users →</a>
                    `;
                    
                    // Reset form
                    e.target.reset();
                } else {
                    throw new Error(result.error || 'Unknown error occurred');
                }
            } catch (error) {
                resultDiv.className = 'result error';
                resultDiv.innerHTML = `❌ <strong>Error:</strong> ${error.message}`;
            } finally {
                // Reset button
                submitBtn.disabled = false;
                submitBtn.textContent = 'Create User';
            }
        });
    </script>
</body>
</html>
            "#)
        })
        .get_async("/users", |_req, ctx| async move {
            worker::console_log!("📥 GET /users - Request received");
            
            let d1_db = ctx.env.get_binding::<D1Database>("DB")?;
            let db = D1Client::new(d1_db);
            
            // Initialize database on first access
            initialize_database(&db).await.expect("CRITICAL: Database initialization failed! Cannot start application.");
            
            worker::console_log!("🔄 Fetching users from database...");
            
            let users = match User::query().all(&db).await {
                Ok(users) => users,
                Err(e) => {
                    return Response::from_json(&json!({
                        "error": format!("Failed to fetch users: {}", e)
                    })).map(|r| r.with_status(500));
                }
            };
            
            let mut users_html = String::new();
            for user in users {
                users_html.push_str(&format!(
                    "<li>ID: {} | Name: {} | Email: {} | Created: {}</li>\n",
                    user.id, user.name, user.email, user.created_at.format("%Y-%m-%d %H:%M:%S")
                ));
            }

            Response::from_html(&format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Users - D1ORM</title>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }}
        .card {{ background: #f5f5f5; padding: 20px; margin: 10px 0; border-radius: 8px; }}
        .header {{ color: #1e40af; }}
        ul {{ background: white; padding: 15px; border-radius: 4px; }}
    </style>
</head>
<body>
    <h1 class="header">Users from D1 Database (via D1ORM)</h1>
    <div class="card">
        <h2>User List</h2>
        <ul>
            {users_html}
        </ul>
        <p><a href="/">Back to Home</a></p>
    </div>
</body>
</html>
            "#, users_html = users_html))
        })
        .get_async("/api/users", |_req, ctx| async move {
            worker::console_log!("📥 GET /api/users - Request received");
            
            let d1_db = ctx.env.get_binding::<D1Database>("DB")?;
            let db = D1Client::new(d1_db);
            
            // Initialize database on first access
            initialize_database(&db).await.expect("CRITICAL: Database initialization failed! Cannot start application.");
            
            worker::console_log!("🔄 Fetching users from database...");
            
            match User::query().all(&db).await {
                Ok(users) => Response::from_json(&json!({
                    "success": true,
                    "data": users,
                    "count": users.len()
                })),
                Err(e) => {
                    worker::console_log!("❌ Error fetching users: {:?}", e);
                    Response::from_json(&json!({
                        "success": false,
                        "error": e.to_string()
                    })).map(|r| r.with_status(500))
                }
            }
        })
        .get_async("/api/user/:id", |_req, ctx| async move {
            let id = ctx.param("id")
                .and_then(|s| s.parse::<i64>().ok())
                .ok_or_else(|| worker::Error::RustError("Invalid user ID".to_string()))?;
            
            let d1_db = ctx.env.get_binding::<D1Database>("DB")?;
            let db = D1Client::new(d1_db);
            
            match User::find(&db, id).await {
                Ok(Some(user)) => Response::from_json(&json!({
                    "success": true,
                    "data": user
                })),
                Ok(None) => Response::from_json(&json!({
                    "success": false,
                    "error": "User not found"
                })).map(|r| r.with_status(404)),
                Err(e) => Response::from_json(&json!({
                    "success": false,
                    "error": e.to_string()
                })).map(|r| r.with_status(500)),
            }
        })
        .post_async("/api/users", |mut req, ctx| async move {
            worker::console_log!("📥 POST /api/users - Request received");
            
            #[derive(Deserialize)]
            struct CreateUserRequest {
                name: String,
                email: String,
            }
            
            let body: CreateUserRequest = req.json().await?;
            worker::console_log!("📝 Creating user: {} ({})", body.name, body.email);
            
            let d1_db = ctx.env.get_binding::<D1Database>("DB")?;
            let db = D1Client::new(d1_db);
            
            // Initialize database on first access
            initialize_database(&db).await.expect("CRITICAL: Database initialization failed! Cannot start application.");
            
            worker::console_log!("💾 Inserting user into database...");
            
            match User::create()
                .set_name(body.name)
                .set_email(body.email)
                .set_created_at(chrono::Utc::now())
                .save(&db)
                .await
            {
                Ok(user) => Response::from_json(&json!({
                    "success": true,
                    "data": user
                })),
                Err(e) => Response::from_json(&json!({
                    "success": false,
                    "error": e.to_string()
                })).map(|r| r.with_status(500)),
            }
        })
        .put_async("/api/user/:id", |mut req, ctx| async move {
            let id = ctx.param("id")
                .and_then(|s| s.parse::<i64>().ok())
                .ok_or_else(|| worker::Error::RustError("Invalid user ID".to_string()))?;
            
            #[derive(Deserialize)]
            struct UpdateUserRequest {
                name: Option<String>,
                email: Option<String>,
            }
            
            let body: UpdateUserRequest = req.json().await?;
            let d1_db = ctx.env.get_binding::<D1Database>("DB")?;
            let db = D1Client::new(d1_db);
            
            if body.name.is_none() && body.email.is_none() {
                return Response::from_json(&json!({
                    "success": false,
                    "error": "No fields to update"
                })).map(|r| r.with_status(400));
            }
            
            let mut update_builder = User::update(id);
            
            if let Some(name) = body.name {
                update_builder = update_builder.set_name(name);
            }
            if let Some(email) = body.email {
                update_builder = update_builder.set_email(email);
            }
            
            match update_builder.save(&db).await {
                Ok(user) => Response::from_json(&json!({
                    "success": true,
                    "data": user
                })),
                Err(d1orm::D1OrmError::NotFound) => Response::from_json(&json!({
                    "success": false,
                    "error": "User not found"
                })).map(|r| r.with_status(404)),
                Err(e) => Response::from_json(&json!({
                    "success": false,
                    "error": e.to_string()
                })).map(|r| r.with_status(500)),
            }
        })
        .delete_async("/api/user/:id", |_req, ctx| async move {
            let id = ctx.param("id")
                .and_then(|s| s.parse::<i64>().ok())
                .ok_or_else(|| worker::Error::RustError("Invalid user ID".to_string()))?;
            
            let d1_db = ctx.env.get_binding::<D1Database>("DB")?;
            let db = D1Client::new(d1_db);
            
            match User::delete(&db, id).await {
                Ok(()) => Response::from_json(&json!({
                    "success": true,
                    "message": "User deleted"
                })),
                Err(e) => Response::from_json(&json!({
                    "success": false,
                    "error": e.to_string()
                })).map(|r| r.with_status(500)),
            }
        })
        .run(req, env)
        .await
}