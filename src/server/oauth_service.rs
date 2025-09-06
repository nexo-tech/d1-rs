use anyhow::Result;
use chrono::{DateTime, Utc};
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use sea_orm::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::{calendar_token, CalendarToken};

#[derive(Debug, Clone)]
pub struct OAuthService {
    db: DatabaseConnection,
    google_client: BasicClient,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthConnectionRequest {
    pub calendar_id: i32,
    pub provider: String, // "google"
    pub redirect_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthCallbackRequest {
    pub calendar_id: i32,
    pub provider: String,
    pub code: String,
    pub state: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthResponse {
    pub auth_url: String,
    pub state: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleUserInfo {
    pub email: String,
    pub name: String,
    pub given_name: String,
    pub family_name: String,
    pub picture: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleEvent {
    pub id: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub start: GoogleEventDateTime,
    pub end: GoogleEventDateTime,
    pub attendees: Option<Vec<GoogleAttendee>>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleEventDateTime {
    #[serde(rename = "dateTime")]
    pub date_time: Option<String>,
    pub date: Option<String>,
    #[serde(rename = "timeZone")]
    pub time_zone: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleAttendee {
    pub email: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(rename = "responseStatus")]
    pub response_status: Option<String>,
}

impl OAuthService {
    pub fn new(db: DatabaseConnection) -> Result<Self> {
        let client_id = std::env::var("GOOGLE_CLIENT_ID")
            .map_err(|_| anyhow::anyhow!("GOOGLE_CLIENT_ID not set"))?;
        let client_secret = std::env::var("GOOGLE_CLIENT_SECRET")
            .map_err(|_| anyhow::anyhow!("GOOGLE_CLIENT_SECRET not set"))?;

        let google_client = BasicClient::new(
            ClientId::new(client_id),
            Some(ClientSecret::new(client_secret)),
            AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())?,
            Some(TokenUrl::new("https://www.googleapis.com/oauth2/v3/token".to_string())?),
        );

        Ok(Self { db, google_client })
    }

    pub async fn initiate_oauth_flow(&self, req: OAuthConnectionRequest) -> Result<OAuthResponse> {
        let redirect_url = RedirectUrl::new(req.redirect_url)?;
        let client = self.google_client.clone().set_redirect_uri(redirect_url);

        let (pkce_challenge, _pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (auth_url, csrf_token) = client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("https://www.googleapis.com/auth/calendar".to_string()))
            .add_scope(Scope::new("https://www.googleapis.com/auth/userinfo.email".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        Ok(OAuthResponse {
            auth_url: auth_url.to_string(),
            state: csrf_token.secret().clone(),
        })
    }

    pub async fn handle_oauth_callback(&self, user_id: i32, req: OAuthCallbackRequest) -> Result<()> {
        let redirect_url = std::env::var("GOOGLE_REDIRECT_URL")
            .map_err(|_| anyhow::anyhow!("GOOGLE_REDIRECT_URL not set"))?;
        let redirect_url = RedirectUrl::new(redirect_url)?;
        
        let client = self.google_client.clone().set_redirect_uri(redirect_url);

        // Exchange the code for an access token
        let token_result = client
            .exchange_code(AuthorizationCode::new(req.code))
            .request_async(async_http_client)
            .await?;

        let access_token = token_result.access_token().secret();
        let refresh_token = token_result
            .refresh_token()
            .map(|t| t.secret())
            .unwrap_or("")
            .to_string();

        // Get user info from Google
        let user_info = self.get_google_user_info(access_token).await?;

        // Calculate expiry
        let expiry = if let Some(expires_in) = token_result.expires_in() {
            Utc::now().naive_utc() + chrono::Duration::seconds(expires_in.as_secs() as i64)
        } else {
            Utc::now().naive_utc() + chrono::Duration::hours(1)
        };

        // Save or update the token
        let existing_token = CalendarToken::find()
            .filter(calendar_token::Column::CalendarId.eq(req.calendar_id))
            .filter(calendar_token::Column::Provider.eq(&req.provider))
            .one(&self.db)
            .await?;

        if let Some(token) = existing_token {
            // Update existing token
            let mut token: calendar_token::ActiveModel = token.into();
            token.access_token = ActiveValue::Set(access_token.to_string());
            token.refresh_token = ActiveValue::Set(refresh_token);
            token.expiry = ActiveValue::Set(expiry);
            token.provider_email = ActiveValue::Set(user_info.email);
            token.update(&self.db).await?;
        } else {
            // Create new token
            let token_model = calendar_token::ActiveModel {
                calendar_id: ActiveValue::Set(req.calendar_id),
                provider: ActiveValue::Set(req.provider),
                provider_email: ActiveValue::Set(user_info.email),
                access_token: ActiveValue::Set(access_token.to_string()),
                refresh_token: ActiveValue::Set(refresh_token),
                token_type: ActiveValue::Set("Bearer".to_string()),
                expiry: ActiveValue::Set(expiry),
                scopes: ActiveValue::Set(Some(serde_json::to_string(&[
                    "https://www.googleapis.com/auth/calendar",
                    "https://www.googleapis.com/auth/userinfo.email"
                ])?)),
                ..Default::default()
            };
            CalendarToken::insert(token_model).exec(&self.db).await?;
        }

        Ok(())
    }

    pub async fn get_calendar_events(
        &self,
        calendar_id: i32,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<GoogleEvent>> {
        let token = self.get_valid_token(calendar_id, "google").await?;
        
        let client = reqwest::Client::new();
        let response = client
            .get("https://www.googleapis.com/calendar/v3/calendars/primary/events")
            .bearer_auth(&token.access_token)
            .query(&[
                ("timeMin", start_time.to_rfc3339()),
                ("timeMax", end_time.to_rfc3339()),
                ("singleEvents", "true".to_string()),
                ("orderBy", "startTime".to_string()),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to fetch calendar events: {}", response.status()));
        }

        let json: serde_json::Value = response.json().await?;
        let events: Vec<GoogleEvent> = serde_json::from_value(json["items"].clone())?;
        
        Ok(events)
    }

    pub async fn create_calendar_event(
        &self,
        calendar_id: i32,
        title: &str,
        description: Option<&str>,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        attendee_email: &str,
    ) -> Result<String> {
        let token = self.get_valid_token(calendar_id, "google").await?;
        
        let event = serde_json::json!({
            "summary": title,
            "description": description,
            "start": {
                "dateTime": start_time.to_rfc3339(),
                "timeZone": "UTC"
            },
            "end": {
                "dateTime": end_time.to_rfc3339(),
                "timeZone": "UTC"
            },
            "attendees": [
                {
                    "email": attendee_email
                }
            ]
        });

        let client = reqwest::Client::new();
        let response = client
            .post("https://www.googleapis.com/calendar/v3/calendars/primary/events")
            .bearer_auth(&token.access_token)
            .query(&[("sendUpdates", "all")])
            .json(&event)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Failed to create calendar event: {}", error_text));
        }

        let created_event: serde_json::Value = response.json().await?;
        let event_id = created_event["id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No event ID returned"))?;
        
        Ok(event_id.to_string())
    }

    pub async fn disconnect_provider(&self, calendar_id: i32, provider: &str) -> Result<()> {
        CalendarToken::delete_many()
            .filter(calendar_token::Column::CalendarId.eq(calendar_id))
            .filter(calendar_token::Column::Provider.eq(provider))
            .exec(&self.db)
            .await?;
        Ok(())
    }

    async fn get_valid_token(&self, calendar_id: i32, provider: &str) -> Result<calendar_token::Model> {
        let mut token = CalendarToken::find()
            .filter(calendar_token::Column::CalendarId.eq(calendar_id))
            .filter(calendar_token::Column::Provider.eq(provider))
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("No OAuth token found for provider: {}", provider))?;

        // Check if token is expired and refresh if needed
        if token.expiry < Utc::now().naive_utc() && !token.refresh_token.is_empty() {
            token = self.refresh_token(token).await?;
        }

        Ok(token)
    }

    async fn refresh_token(&self, token: calendar_token::Model) -> Result<calendar_token::Model> {
        let redirect_url = std::env::var("GOOGLE_REDIRECT_URL")
            .map_err(|_| anyhow::anyhow!("GOOGLE_REDIRECT_URL not set"))?;
        let redirect_url = RedirectUrl::new(redirect_url)?;
        
        let client = self.google_client.clone().set_redirect_uri(redirect_url);

        let token_result = client
            .exchange_refresh_token(&oauth2::RefreshToken::new(token.refresh_token.clone()))
            .request_async(async_http_client)
            .await?;

        let new_access_token = token_result.access_token().secret();
        let new_refresh_token = token_result
            .refresh_token()
            .map(|t| t.secret())
            .unwrap_or(&token.refresh_token);

        // Calculate new expiry
        let expiry = if let Some(expires_in) = token_result.expires_in() {
            Utc::now().naive_utc() + chrono::Duration::seconds(expires_in.as_secs() as i64)
        } else {
            Utc::now().naive_utc() + chrono::Duration::hours(1)
        };

        // Update token in database
        let mut token_active: calendar_token::ActiveModel = token.into();
        token_active.access_token = ActiveValue::Set(new_access_token.to_string());
        token_active.refresh_token = ActiveValue::Set(new_refresh_token.to_string());
        token_active.expiry = ActiveValue::Set(expiry);
        
        let updated_token = token_active.update(&self.db).await?;
        Ok(updated_token)
    }

    async fn get_google_user_info(&self, access_token: &str) -> Result<GoogleUserInfo> {
        let client = reqwest::Client::new();
        let response = client
            .get("https://www.googleapis.com/oauth2/v2/userinfo")
            .bearer_auth(access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to get user info from Google"));
        }

        let user_info: GoogleUserInfo = response.json().await?;
        Ok(user_info)
    }
}