use worker::*;

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    let router = Router::new();

    router
        .get("/", |_req, ctx| {
            Response::from_html(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Cloudflare Worker with D1</title>
    <style>
        body { font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }
        .card { background: #f5f5f5; padding: 20px; margin: 10px 0; border-radius: 8px; }
        .header { color: #1e40af; }
    </style>
</head>
<body>
    <h1 class="header">Welcome to Cloudflare Worker</h1>
    <div class="card">
        <h2>Home Page</h2>
        <p>This is a minimal Cloudflare Worker with D1 database support.</p>
        <p><a href="/about">Visit About Page</a></p>
        <p><a href="/users">View Users (D1 Database)</a></p>
    </div>
</body>
</html>
            "#)
        })
        .get("/about", |_req, ctx| {
            Response::from_html(r#"
<!DOCTYPE html>
<html>
<head>
    <title>About - Cloudflare Worker</title>
    <style>
        body { font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }
        .card { background: #f5f5f5; padding: 20px; margin: 10px 0; border-radius: 8px; }
        .header { color: #1e40af; }
    </style>
</head>
<body>
    <h1 class="header">About This Worker</h1>
    <div class="card">
        <h2>About Page</h2>
        <p>This Cloudflare Worker is built with Rust and includes:</p>
        <ul>
            <li>Two HTML routes (/ and /about)</li>
            <li>D1 database integration</li>
            <li>Local development support</li>
            <li>Deployment automation</li>
        </ul>
        <p><a href="/">Back to Home</a></p>
        <p><a href="/users">View Users (D1 Database)</a></p>
    </div>
</body>
</html>
            "#)
        })
        .get("/users", |_req, ctx| async move {
            // Get D1 database binding
            let db = ctx.env.d1("DB")?;
            
            // Simple query to demonstrate D1 integration
            let statement = db.prepare("SELECT * FROM users LIMIT 10");
            let result = statement.all().await?;
            
            let users_html = if let Some(results) = result.results {
                results.iter()
                    .map(|row| {
                        format!(
                            "<li>User: {} (ID: {})</li>", 
                            row.get("name").unwrap_or(&"Unknown".into()),
                            row.get("id").unwrap_or(&"N/A".into())
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("")
            } else {
                "<li>No users found</li>".to_string()
            };

            Response::from_html(&format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Users - Cloudflare Worker</title>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }}
        .card {{ background: #f5f5f5; padding: 20px; margin: 10px 0; border-radius: 8px; }}
        .header {{ color: #1e40af; }}
        ul {{ background: white; padding: 15px; border-radius: 4px; }}
    </style>
</head>
<body>
    <h1 class="header">Users from D1 Database</h1>
    <div class="card">
        <h2>User List</h2>
        <ul>
            {users_html}
        </ul>
        <p><a href="/">Back to Home</a></p>
        <p><a href="/about">About Page</a></p>
    </div>
</body>
</html>
            "#, users_html = users_html))
        })
        .run(req, env)
        .await
}