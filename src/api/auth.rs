use crate::auth::{exchange_code_for_token, generate_oauth_url, generate_state, get_user_info, save_token};
use crate::database::{get_database, initialize_database};
use crate::templates::{auth_error_page, auth_success_page, home_page};
use std::collections::HashMap;
use worker::{console_error, console_log, Request, Response, Result as WorkerResult, RouteContext};

pub async fn home_handler(_req: Request, _ctx: RouteContext<()>) -> WorkerResult<Response> {
    Response::from_html(home_page())
}

pub async fn google_auth_handler(_req: Request, ctx: RouteContext<()>) -> WorkerResult<Response> {
    console_log!("Starting Google OAuth flow");
    
    let state = generate_state();
    console_log!("Generated state: {}", state);
    
    match generate_oauth_url(&ctx, &state) {
        Ok(auth_url) => {
            console_log!("Generated auth URL successfully");
            console_log!("Redirecting to: {}", auth_url);
            
            let mut response = Response::empty()?.with_status(302);
            response.headers_mut().set("Location", &auth_url)?;
            response.headers_mut().set("Set-Cookie", &format!("oauth_state={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=600", state))?;
            Ok(response)
        }
        Err(e) => {
            console_error!("Failed to generate OAuth URL: {:?}", e);
            Response::from_html(auth_error_page())
        }
    }
}

pub async fn auth_callback_handler(req: Request, ctx: RouteContext<()>) -> WorkerResult<Response> {
    let url = req.url()?;
    let query_pairs: HashMap<String, String> = url.query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    
    let code = query_pairs.get("code")
        .ok_or_else(|| worker::Error::RustError("No authorization code received".to_string()))?;
    
    let state = query_pairs.get("state")
        .ok_or_else(|| worker::Error::RustError("No state parameter received".to_string()))?;
    
    // Verify state matches
    let cookie_header = req.headers().get("Cookie")?;
    
    // Debug: Log all cookies
    if let Some(cookies) = &cookie_header {
        worker::console_log!("Received cookies: {}", cookies);
    } else {
        worker::console_log!("No cookies received");
    }
    
    let stored_state = cookie_header
        .and_then(|cookies| {
            cookies.split(';')
                .find(|c| c.trim().starts_with("oauth_state="))
                .and_then(|c| c.split('=').nth(1).map(|s| s.to_string()))
        })
        .ok_or_else(|| worker::Error::RustError("No state cookie found".to_string()))?;
    
    if state != &stored_state {
        return Err(worker::Error::RustError("State mismatch - possible CSRF attack".to_string()));
    }
    
    // Exchange code for token
    let token_response = exchange_code_for_token(code, &ctx).await?;
    
    // Get user info
    let user_info = get_user_info(&token_response.access_token).await?;
    
    // Save token to database
    let db = get_database(&ctx)?;
    initialize_database(&db).await?;
    
    save_token(&db, user_info.email.clone(), token_response).await?;
    
    // Clear state cookie and redirect to success page
    let mut response = Response::from_html(auth_success_page(&user_info.email))?;
    response.headers_mut().set("Set-Cookie", "oauth_state=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0")?;
    
    Ok(response)
}

