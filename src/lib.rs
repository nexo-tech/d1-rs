use worker::{event, Env, Request, Response, Result as WorkerResult, Router};

mod api;
mod auth;
mod calendar;
mod database;
mod models;
mod templates;

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: worker::Context) -> WorkerResult<Response> {
    Router::new()
        // Home and authentication routes
        .get_async("/", api::auth::home_handler)
        .get_async("/auth/google", api::auth::google_auth_handler)
        .get_async("/auth/callback", api::auth::auth_callback_handler)
        
        // Dashboard and management routes
        .get_async("/dashboard", api::calendar::dashboard_handler)
        .get_async("/working-hours", api::working_hours::working_hours_form_handler)
        
        // Public booking routes
        .get_async("/calendar/:email", api::booking::public_calendar_handler)
        
        // API endpoints
        .get_async("/api/slots/:email", api::booking::get_available_slots_handler)
        .post_async("/api/book", api::booking::create_booking_handler)
        .post_async("/api/working-hours", api::working_hours::save_working_hours_handler)
        .get_async("/api/emails", api::calendar::get_emails_handler)
        
        .run(req, env)
        .await
}