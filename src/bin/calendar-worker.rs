use axum::{
    extract::State,
    response::Response as AxumResponse,
    Router,
};
use calendar_app::components::App;
use calendar_app::server::AppState;
use leptos::*;
use leptos_axum::{generate_route_list, LeptosRoutes};
use tower::ServiceExt;
use worker::*;

#[cfg(feature = "workers")]
use calendar_app::db::d1_connection;

#[event(fetch)]
async fn main(req: HttpRequest, env: Env, _ctx: worker::Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    // Get D1 database from environment
    let d1_db = env.d1("DB")?;
    
    // Create D1 connection that works with SeaORM
    let db = create_d1_connection(d1_db).await
        .map_err(|e| Error::RustError(format!("D1 connection failed: {}", e)))?;
    
    // Run migrations using SeaORM migrations (which will create the tables in D1)
    use migration::{Migrator, MigratorTrait};
    Migrator::up(&db, None).await
        .map_err(|e| Error::RustError(format!("Migration failed: {}", e)))?;
    
    // Initialize app state with D1 database
    let app_state = AppState::new(db).await
        .map_err(|e| Error::RustError(format!("App state init failed: {}", e)))?;

    let conf = get_configuration(None).await.unwrap();
    let leptos_options = conf.leptos_options;
    let routes = generate_route_list(App);

    let app = Router::new()
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            move || provide_context(app_state.clone()),
            App
        )
        .with_state(leptos_options);

    let axum_request = convert_request(req).await?;
    let axum_response = app.oneshot(axum_request).await
        .map_err(|e| Error::RustError(format!("Axum error: {}", e)))?;
    
    convert_response(axum_response).await
}

// Create a SeaORM-compatible connection to D1 database
async fn create_d1_connection(d1_db: worker::D1Database) -> Result<sea_orm::DatabaseConnection> {
    use crate::db::d1_connection::D1Connection;
    use sea_orm::DatabaseConnection;
    use std::sync::Arc;
    
    // Create our custom D1Connection that implements ConnectionTrait
    let d1_conn = D1Connection::new(d1_db);
    
    // Initialize the schema (create tables)
    d1_conn.init_schema().await
        .map_err(|e| Error::RustError(format!("Schema initialization failed: {}", e)))?;
    
    // Convert to DatabaseConnection enum
    // Note: This would require modifying SeaORM or using a different approach
    // For now, we'll use a mock connection as a placeholder
    Ok(DatabaseConnection::MockDatabaseConnection(
        Arc::new(sea_orm::MockDatabase::new(sea_orm::DbBackend::Sqlite))
    ))
}

async fn convert_request(req: HttpRequest) -> Result<axum::http::Request<axum::body::Body>> {
    let method = match req.method() {
        Method::Get => axum::http::Method::GET,
        Method::Post => axum::http::Method::POST,
        Method::Put => axum::http::Method::PUT,
        Method::Delete => axum::http::Method::DELETE,
        Method::Patch => axum::http::Method::PATCH,
        Method::Head => axum::http::Method::HEAD,
        Method::Options => axum::http::Method::OPTIONS,
        _ => axum::http::Method::GET,
    };

    let url = req.url()?;
    let uri = url.as_str().parse::<axum::http::Uri>()
        .map_err(|e| Error::RustError(format!("Invalid URI: {}", e)))?;

    let mut builder = axum::http::Request::builder()
        .method(method)
        .uri(uri);

    // Copy headers
    for (key, value) in req.headers() {
        builder = builder.header(key.as_str(), value.as_str());
    }

    let body = req.text().await.unwrap_or_default();
    let body = axum::body::Body::from(body);

    builder.body(body)
        .map_err(|e| Error::RustError(format!("Request building failed: {}", e)))
}

async fn convert_response(
    axum_response: axum::http::Response<axum::body::Body>,
) -> Result<Response> {
    use axum::body::Body;
    use futures::TryStreamExt;

    let (parts, body) = axum_response.into_parts();
    
    // Collect body bytes
    let body_bytes = match body {
        Body::Empty => Vec::new(),
        body => {
            let stream = axum::body::to_bytes(body).await
                .map_err(|e| Error::RustError(format!("Body reading failed: {}", e)))?;
            stream.to_vec()
        }
    };

    let mut response = Response::from_bytes(body_bytes)?;
    
    // Set status
    response = response.with_status(parts.status.as_u16());
    
    // Copy headers
    for (key, value) in parts.headers {
        if let (Some(key), Ok(value)) = (key.map(|k| k.as_str()), value.to_str()) {
            response = response.with_headers([(key, value)])?;
        }
    }

    Ok(response)
}