use worker::{*, Result as WorkerResult};
use worker::d1::D1Database;
use serde::{Deserialize, Serialize};
use serde_json::json;
use chrono::{DateTime, Utc};
use d1orm::*;

#[derive(Debug, Serialize, Deserialize, Entity)]
#[table(name = "users")]
struct User {
    #[primary_key]
    id: i64,
    name: String,
    #[unique]
    email: String,
    created_at: DateTime<Utc>,
}

// Check if database is ready without running full migrations
async fn is_database_ready(db: &D1Client) -> bool {
    // Quick check - if users table exists, assume DB is ready
    let sql = "SELECT COUNT(*) as count FROM sqlite_master WHERE type='table' AND name='users'";
    match db.execute_returning_count(sql, &[]).await {
        Ok(count) => count > 0,
        Err(_) => false,
    }
}

async fn ensure_migrations(db: &D1Client) -> d1orm::Result<()> {
    // Quick check first - if DB is ready, skip migrations
    if is_database_ready(db).await {
        worker::console_log!("✅ Database ready - skipping migration check");
        return Ok(());
    }

    worker::console_log!("🔄 Database not ready - running migrations...");
    
    let mut runner = MigrationRunner::new();
    
    // Add initial migration to create users table
    let create_users_migration = CreateTableMigration::new("create_users", 1, "users".to_string())
        .column("id", "INTEGER").primary_key()
        .column("name", "TEXT").not_null()
        .column("email", "TEXT").not_null().unique()
        .column("created_at", "DATETIME").default("CURRENT_TIMESTAMP");
    
    runner.add_migration(Box::new(create_users_migration));
    
    worker::console_log!("📋 Added migration: create_users");
    
    // This returns which migrations were actually applied
    match runner.run_pending_migrations(db).await {
        Ok(applied) => {
            if applied.is_empty() {
                worker::console_log!("⏭️ No migrations needed - all up to date");
            } else {
                worker::console_log!("✅ Applied {} migrations: {:?}", applied.len(), applied);
            }
            Ok(())
        },
        Err(e) => {
            worker::console_log!("❌ Migration failed: {}", e);
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
        pre { background: #1e293b; color: #e2e8f0; padding: 12px; border-radius: 6px; overflow-x: auto; }
    </style>
</head>
<body>
    <h1 class="header">Welcome to Cloudflare Worker with D1ORM</h1>
    <div class="card">
        <h2>Type-Safe D1-First ORM</h2>
        <p>This Cloudflare Worker uses D1ORM, a type-safe D1-first ORM written in Rust with ergonomic query builders.</p>
        <h3>Features:</h3>
        <ul>
            <li>✅ Type-safe query builder with method generation</li>
            <li>✅ Direct D1 integration (no intermediate layers)</li>
            <li>✅ WASM-optimized compilation</li>
            <li>✅ Human-first API design</li>
            <li>✅ Rust-based migrations</li>
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
        <h3>Example Usage:</h3>
        <pre>// Create user
POST /api/users
{
  "name": "John Doe",
  "email": "john@example.com"
}

// Query with type-safe methods
users = User::query()
    .where_name_contains("John")
    .where_email_ends_with("@example.com")
    .order_by_created_at_desc()
    .limit(10)
    .all(&db)
    .await?;</pre>
    </div>
</body>
</html>
            "#)
        })
        .get_async("/users", |_req, ctx| async move {
            let d1_db = ctx.env.get_binding::<D1Database>("DB")?;
            let db = D1Client::new(d1_db);
            
            // Run migrations - CRITICAL: panic on failure
            ensure_migrations(&db).await.expect("CRITICAL: Database migration failed! Cannot start application.");
            
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
            worker::console_log!("🚀 GET /api/users - Starting request");
            
            let d1_db = ctx.env.get_binding::<D1Database>("DB")?;
            let db = D1Client::new(d1_db);
            worker::console_log!("📱 Database client created");
            
            // Run migrations - CRITICAL: panic on failure
            worker::console_log!("🔄 Running migrations...");
            ensure_migrations(&db).await.expect("CRITICAL: Database migration failed! Cannot start application.");
            worker::console_log!("✅ Migrations completed, fetching users...");
            
            match User::query().all(&db).await {
                Ok(users) => Response::from_json(&json!({
                    "success": true,
                    "data": users,
                    "count": users.len()
                })),
                Err(e) => Response::from_json(&json!({
                    "success": false,
                    "error": e.to_string()
                })).map(|r| r.with_status(500)),
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
            #[derive(Deserialize)]
            struct CreateUserRequest {
                name: String,
                email: String,
            }
            
            let body: CreateUserRequest = req.json().await?;
            let d1_db = ctx.env.get_binding::<D1Database>("DB")?;
            let db = D1Client::new(d1_db);
            
            // Run migrations - CRITICAL: panic on failure
            ensure_migrations(&db).await.expect("CRITICAL: Database migration failed! Cannot start application.");
            
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