use actix_files::Files;
use actix_web::*;
use calendar_app::components::App;
use calendar_app::db::init_db;
use calendar_app::server::{provide_app_state, AppState};
use leptos::*;
use leptos_actix::{generate_route_list, LeptosRoutes};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables
    dotenvy::dotenv().ok();
    
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Initialize database
    let db = init_db().await.expect("Failed to initialize database");
    
    // Initialize app state
    let app_state = AppState::new(db).await.expect("Failed to initialize app state");

    // Setting get_configuration(None) means we'll be using cargo-leptos's env values
    // For deployment these variables are:
    // <https://github.com/leptos-rs/start-axum#executing-a-server-on-a-remote-machine-without-the-toolchain>
    // Alternately a file can be specified such as Some("Cargo.toml")
    // The file would need to be included with the executable when moved to deployment
    let conf = get_configuration(None).await.unwrap();
    let addr = conf.leptos_options.site_addr;
    
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);
    println!("listening on http://{}", &addr);

    HttpServer::new(move || {
        let leptos_options = &conf.leptos_options;
        let site_root = &leptos_options.site_root;
        let app_state_for_routes = app_state.clone();

        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .route("/api/{tail:.*}", leptos_actix::handle_server_fns())
            .leptos_routes_with_context(
                leptos_options.to_owned(),
                routes.to_owned(),
                move || provide_context(app_state_for_routes.clone()),
                App
            )
            .service(Files::new("/", site_root))
        //.wrap(middleware::Compress::default())
    })
    .bind(&addr)?
    .run()
    .await
}