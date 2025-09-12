use crate::models::{GoogleUserInfo, OAuthTokenResponse, Token};
use chrono::{Duration, Utc};
use d1orm::*;
use url::Url;
use worker::{console_error, console_log, Result as WorkerResult, RouteContext};

pub async fn exchange_code_for_token(code: &str, ctx: &RouteContext<()>) -> WorkerResult<OAuthTokenResponse> {
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

pub async fn get_user_info(access_token: &str) -> WorkerResult<GoogleUserInfo> {
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

pub fn generate_oauth_url(ctx: &RouteContext<()>, state: &str) -> WorkerResult<String> {
    console_log!("Getting GOOGLE_CLIENT_ID from env");
    let client_id = match ctx.env.var("GOOGLE_CLIENT_ID") {
        Ok(var) => var.to_string(),
        Err(e) => {
            console_error!("Failed to get GOOGLE_CLIENT_ID: {:?}", e);
            return Err(worker::Error::RustError(format!("GOOGLE_CLIENT_ID not set: {:?}", e)));
        }
    };
    console_log!("Client ID: {}", client_id);
    
    console_log!("Getting GOOGLE_REDIRECT_URL from env");
    let redirect_uri = match ctx.env.var("GOOGLE_REDIRECT_URL") {
        Ok(var) => var.to_string(),
        Err(e) => {
            console_error!("Failed to get GOOGLE_REDIRECT_URL: {:?}", e);
            return Err(worker::Error::RustError(format!("GOOGLE_REDIRECT_URL not set: {:?}", e)));
        }
    };
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

pub fn generate_state() -> String {
    // Use js_sys::Date for WASM compatibility instead of std::time
    let timestamp = js_sys::Date::now() as u64;
    let random = (timestamp * 9973) % 1000000; // Simple pseudo-random
    format!("{}{}", timestamp, random)
}

pub async fn refresh_token_if_needed(token: &Token, ctx: &RouteContext<()>) -> WorkerResult<Option<Token>> {
    let now = Utc::now();
    
    // Check if token is expired or expires within 5 minutes
    if token.expiry <= now + Duration::minutes(5) {
        console_log!("Token expired or expiring soon, refreshing...");
        
        let client_id = ctx.env.var("GOOGLE_CLIENT_ID")?.to_string();
        let client_secret = ctx.env.var("GOOGLE_CLIENT_SECRET")?.to_string();
        
        let token_url = "https://oauth2.googleapis.com/token";
        let params = [
            ("refresh_token", token.refresh_token.as_str()),
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
        
        let refresh_response: OAuthTokenResponse = response.json()
            .await
            .map_err(|e| worker::Error::RustError(format!("Failed to parse refresh response: {}", e)))?;
        
        // Create updated token
        let mut updated_token = token.clone();
        updated_token.access_token = refresh_response.access_token;
        updated_token.expiry = Utc::now() + Duration::seconds(refresh_response.expires_in);
        updated_token.updated_at = Utc::now();
        
        // If a new refresh token was provided, update it
        if let Some(new_refresh_token) = refresh_response.refresh_token {
            updated_token.refresh_token = new_refresh_token;
        }
        
        Ok(Some(updated_token))
    } else {
        Ok(None)
    }
}

pub async fn save_token(
    db: &D1Client,
    user_email: String,
    token_response: OAuthTokenResponse,
) -> WorkerResult<Token> {
    let expiry = Utc::now() + Duration::seconds(token_response.expires_in);
    let now = Utc::now();
    
    // Check if token already exists for this user
    let existing_token = Token::query()
        .where_user_email_eq(user_email.clone())
        .first(db)
        .await
        .map_err(|e| worker::Error::RustError(format!("Database query failed: {}", e)))?;
    
    if let Some(mut token) = existing_token {
        // Update existing token
        let new_access_token = token_response.access_token;
        let new_refresh_token = token_response.refresh_token
            .unwrap_or(token.refresh_token.clone());
        let new_token_type = token_response.token_type;
        
        Token::update(token.id)
            .set_access_token(new_access_token.clone())
            .set_refresh_token(new_refresh_token.clone())
            .set_token_type(new_token_type.clone())
            .set_expiry(expiry)
            .set_updated_at(now)
            .save(db).await
            .map_err(|e| worker::Error::RustError(format!("Failed to update token: {}", e)))?;
        
        // Update the token struct with new values
        token.access_token = new_access_token;
        token.refresh_token = new_refresh_token;
        token.token_type = new_token_type;
        token.expiry = expiry;
        token.updated_at = now;
        
        Ok(token)
    } else {
        // Insert new token
        let inserted_token = Token::create()
            .set_user_email(user_email.clone())
            .set_access_token(token_response.access_token)
            .set_refresh_token(token_response.refresh_token
                .ok_or_else(|| worker::Error::RustError("No refresh token provided".to_string()))?)
            .set_token_type(token_response.token_type)
            .set_expiry(expiry)
            .set_created_at(now)
            .set_updated_at(now)
            .save(db).await
            .map_err(|e| worker::Error::RustError(format!("Failed to insert token: {}", e)))?;
        
        Ok(inserted_token)
    }
}