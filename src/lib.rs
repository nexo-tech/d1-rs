use worker::{*, Result as WorkerResult};
use worker::d1::D1Database;
use gluesql::prelude::*;
use gluesql::core::ast::*;
use gluesql::core::ast_builder::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

mod d1_storage;
use d1_storage::D1Storage;

trait FromGlueValue: Sized {
    fn from_glue_row(labels: &[String], row: &[Value]) -> std::result::Result<Self, String>;
}

trait ToGlueValues {
    fn to_glue_values(&self) -> HashMap<&'static str, Value>;
}

trait DatabaseEntity: FromGlueValue + ToGlueValues + Serialize {
    const TABLE_NAME: &'static str;
    
    async fn find_all(glue: &mut Glue<D1Storage>) -> std::result::Result<Vec<Self>, String> {
        let select_stmt = table(Self::TABLE_NAME).select().build()
            .map_err(|e| format!("Failed to build query: {:?}", e))?;
        let result = glue.execute_stmt(&select_stmt).await
            .map_err(|e| format!("Query failed: {:?}", e))?;
        
        let mut entities = Vec::new();
        if let Payload::Select { labels, rows } = result {
            for row in rows {
                entities.push(Self::from_glue_row(&labels, &row)?);
            }
        }
        Ok(entities)
    }
    
    async fn find_by_id(glue: &mut Glue<D1Storage>, id: i64) -> std::result::Result<Option<Self>, String> {
        let select_stmt = table(Self::TABLE_NAME)
            .select()
            .filter(col("id").eq(id))
            .build()
            .map_err(|e| format!("Failed to build statement: {:?}", e))?;
        let result = glue.execute_stmt(&select_stmt).await
            .map_err(|e| format!("Query failed: {:?}", e))?;
        
        if let Payload::Select { labels, rows } = result {
            if let Some(row) = rows.into_iter().next() {
                return Ok(Some(Self::from_glue_row(&labels, &row)?));
            }
        }
        Ok(None)
    }
}

macro_rules! impl_glue_entity {
    ($struct_name:ident {
        $(
            $field:ident: $field_type:tt
        ),* $(,)?
    }) => {
        impl FromGlueValue for $struct_name {
            fn from_glue_row(labels: &[String], row: &[Value]) -> std::result::Result<Self, String> {
                $(
                    let mut $field: Option<$field_type> = None;
                )*
                
                for (label, value) in labels.iter().zip(row.iter()) {
                    match label.as_str() {
                        $(
                            stringify!($field) => {
                                $field = Some(impl_glue_entity!(@convert_value value, $field_type)?);
                            }
                        )*
                        _ => {}
                    }
                }
                
                Ok($struct_name {
                    $(
                        $field: $field.ok_or_else(|| format!("Missing field: {}", stringify!($field)))?,
                    )*
                })
            }
        }
        
        impl ToGlueValues for $struct_name {
            fn to_glue_values(&self) -> HashMap<&'static str, Value> {
                let mut values = HashMap::new();
                $(
                    values.insert(stringify!($field), impl_glue_entity!(@to_value &self.$field, $field_type));
                )*
                values
            }
        }
    };
    
    (@convert_value $value:expr, i64) => {
        {
            match $value {
                Value::I64(i) => Ok(*i),
                _ => Err(format!("Expected i64, got {:?}", $value)),
            }
        }
    };
    
    (@convert_value $value:expr, String) => {
        {
            match $value {
                Value::Str(s) => Ok(s.clone()),
                Value::Null => Ok(String::new()),
                _ => Err(format!("Expected String, got {:?}", $value)),
            }
        }
    };
    
    (@to_value $field:expr, i64) => {
        Value::I64(*$field)
    };
    
    (@to_value $field:expr, String) => {
        Value::Str($field.clone())
    };
}

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: i64,
    name: String,
    email: String,
    created_at: String,
}

impl_glue_entity!(User {
    id: i64,
    name: String,
    email: String,
    created_at: String,
});

impl DatabaseEntity for User {
    const TABLE_NAME: &'static str = "users";
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
    <title>Cloudflare Worker with D1 and GlueSQL</title>
    <style>
        body { font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }
        .card { background: #f5f5f5; padding: 20px; margin: 10px 0; border-radius: 8px; }
        .header { color: #1e40af; }
        pre { background: #1e293b; color: #e2e8f0; padding: 12px; border-radius: 6px; overflow-x: auto; }
    </style>
</head>
<body>
    <h1 class="header">Welcome to Cloudflare Worker with GlueSQL</h1>
    <div class="card">
        <h2>Type-Safe ORM with D1 Database</h2>
        <p>This Cloudflare Worker uses GlueSQL, a type-safe SQL database library written in Rust, with a custom D1 storage adapter.</p>
        <h3>Features:</h3>
        <ul>
            <li>✅ Full SQL support with type safety</li>
            <li>✅ Custom D1 storage backend</li>
            <li>✅ WASM-compatible (no incompatible dependencies)</li>
            <li>✅ No raw SQL statements needed</li>
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

// Update user
PUT /api/user/1
{
  "name": "Jane Doe",
  "email": "jane@example.com"
}</pre>
    </div>
</body>
</html>
            "#)
        })
        .get_async("/users", |_req, ctx| async move {
            let db = ctx.env.get_binding::<D1Database>("DB")?;
            let storage = D1Storage::new(db);
            let mut glue = Glue::new(storage);
            
            // Type-safe query builder - no raw SQL!
            let select_stmt = table("users").select().build()
                .map_err(|e| worker::Error::RustError(format!("Failed to build query: {:?}", e)))?;
            let result = glue.execute_stmt(&select_stmt).await
                .map_err(|e| worker::Error::RustError(format!("Query failed: {:?}", e)))?;
            
            let mut users_html = String::new();
            if let Payload::Select { labels, rows } = result {
                for row in rows {
                    let mut user_data = String::new();
                    for (label, value) in labels.iter().zip(row.iter()) {
                        user_data.push_str(&format!("{}: {:?} ", label, value));
                    }
                    users_html.push_str(&format!("<li>{}</li>\n", user_data));
                }
            }

            Response::from_html(&format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Users - GlueSQL + D1</title>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }}
        .card {{ background: #f5f5f5; padding: 20px; margin: 10px 0; border-radius: 8px; }}
        .header {{ color: #1e40af; }}
        ul {{ background: white; padding: 15px; border-radius: 4px; }}
    </style>
</head>
<body>
    <h1 class="header">Users from D1 Database (via GlueSQL)</h1>
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
            let db = ctx.env.get_binding::<D1Database>("DB")?;
            let storage = D1Storage::new(db);
            let mut glue = Glue::new(storage);
            
            match User::find_all(&mut glue).await {
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
            
            let db = ctx.env.get_binding::<D1Database>("DB")?;
            let storage = D1Storage::new(db);
            let mut glue = Glue::new(storage);
            
            match User::find_by_id(&mut glue, id).await {
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
            
            let db = ctx.env.get_binding::<D1Database>("DB")?;
            let storage = D1Storage::new(db);
            let mut glue = Glue::new(storage);
            
            // First, ensure the table exists - Type-safe table creation
            let create_table_stmt = Statement::CreateTable {
                if_not_exists: true,
                name: "users".to_string(),
                columns: Some(vec![
                    ColumnDef {
                        name: "id".to_string(),
                        data_type: DataType::Int,
                        nullable: false,
                        default: None,
                        unique: None,
                    },
                    ColumnDef {
                        name: "name".to_string(),
                        data_type: DataType::Text,
                        nullable: false,
                        default: None,
                        unique: None,
                    },
                    ColumnDef {
                        name: "email".to_string(),
                        data_type: DataType::Text,
                        nullable: false,
                        default: None,
                        unique: Some(gluesql::core::ast::ColumnUniqueOption { is_primary: false }),
                    },
                    ColumnDef {
                        name: "created_at".to_string(),
                        data_type: DataType::Timestamp,
                        nullable: true,
                        default: None,
                        unique: None,
                    },
                ]),
                engine: None,
                source: None,
            };
            
            glue.execute_stmt(&create_table_stmt).await
                .map_err(|e| worker::Error::RustError(format!("Failed to create table: {:?}", e)))?;
            
            // Type-safe insert - no raw SQL!
            let insert_stmt = table("users")
                .insert()
                .columns(vec!["name", "email"])
                .values(vec![vec![body.name.as_str(), body.email.as_str()]])
                .build()
                .map_err(|e| worker::Error::RustError(format!("Failed to build statement: {:?}", e)))?;
            
            let _result = glue.execute_stmt(&insert_stmt).await
                .map_err(|e| worker::Error::RustError(format!("Failed to insert user: {:?}", e)))?;
            
            // Get the inserted user - Type-safe query
            let select_stmt = table("users")
                .select()
                .filter(col("email").eq(body.email.as_str()))
                .build()
                .map_err(|e| worker::Error::RustError(format!("Failed to build statement: {:?}", e)))?;
            let select_result = glue.execute_stmt(&select_stmt).await
                .map_err(|e| worker::Error::RustError(format!("Failed to fetch user: {:?}", e)))?;
            
            if let Payload::Select { labels, rows } = select_result {
                if let Some(row) = rows.into_iter().next() {
                    match User::from_glue_row(&labels, &row) {
                        Ok(user) => return Response::from_json(&json!({
                            "success": true,
                            "data": user
                        })),
                        Err(e) => return Response::from_json(&json!({
                            "success": false,
                            "error": format!("Failed to parse user: {}", e)
                        })).map(|r| r.with_status(500)),
                    }
                }
            }
            
            Response::from_json(&json!({
                "success": true,
                "message": "User created"
            }))
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
            
            let db = ctx.env.get_binding::<D1Database>("DB")?;
            let storage = D1Storage::new(db);
            let mut glue = Glue::new(storage);
            
            if body.name.is_none() && body.email.is_none() {
                return Response::from_json(&json!({
                    "success": false,
                    "error": "No fields to update"
                })).map(|r| r.with_status(400));
            }
            
            // Type-safe update - no raw SQL!
            let mut update_node = table("users").update();
            
            if let Some(name) = body.name {
                update_node = update_node.set("name", name);
            }
            if let Some(email) = body.email {
                update_node = update_node.set("email", email);
            }
            
            let update_stmt = update_node
                .filter(col("id").eq(id))
                .build()
                .map_err(|e| worker::Error::RustError(format!("Failed to build statement: {:?}", e)))?;
            
            glue.execute_stmt(&update_stmt).await
                .map_err(|e| worker::Error::RustError(format!("Failed to update user: {:?}", e)))?;
            
            // Get the updated user - Type-safe query
            let select_stmt = table("users")
                .select()
                .filter(col("id").eq(id))
                .build()
                .map_err(|e| worker::Error::RustError(format!("Failed to build statement: {:?}", e)))?;
            let select_result = glue.execute_stmt(&select_stmt).await
                .map_err(|e| worker::Error::RustError(format!("Failed to fetch user: {:?}", e)))?;
            
            if let Payload::Select { labels, rows } = select_result {
                if let Some(row) = rows.into_iter().next() {
                    match User::from_glue_row(&labels, &row) {
                        Ok(user) => return Response::from_json(&json!({
                            "success": true,
                            "data": user
                        })),
                        Err(e) => return Response::from_json(&json!({
                            "success": false,
                            "error": format!("Failed to parse user: {}", e)
                        })).map(|r| r.with_status(500)),
                    }
                }
            }
            
            Response::from_json(&json!({
                "success": false,
                "error": "User not found"
            })).map(|r| r.with_status(404))
        })
        .delete_async("/api/user/:id", |_req, ctx| async move {
            let id = ctx.param("id")
                .and_then(|s| s.parse::<i64>().ok())
                .ok_or_else(|| worker::Error::RustError("Invalid user ID".to_string()))?;
            
            let db = ctx.env.get_binding::<D1Database>("DB")?;
            let storage = D1Storage::new(db);
            let mut glue = Glue::new(storage);
            
            // Type-safe delete - no raw SQL!
            let delete_stmt = table("users")
                .delete()
                .filter(col("id").eq(id))
                .build()
                .map_err(|e| worker::Error::RustError(format!("Failed to build statement: {:?}", e)))?;
            let _result = glue.execute_stmt(&delete_stmt).await
                .map_err(|e| worker::Error::RustError(format!("Failed to delete user: {:?}", e)))?;
            
            Response::from_json(&json!({
                "success": true,
                "message": "User deleted"
            }))
        })
        .run(req, env)
        .await
}