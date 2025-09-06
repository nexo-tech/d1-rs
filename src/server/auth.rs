use anyhow::Result;
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::{distributions::Alphanumeric, Rng};
use sea_orm::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{user, session, User, Session};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // Subject (user id)
    pub email: String,
    pub exp: usize, // Expiry
    pub iat: usize, // Issued at
}

#[derive(Debug, Clone)]
pub struct AuthService {
    db: DatabaseConnection,
    jwt_secret: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub full_name: String,
    pub company_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i32,
    pub email: String,
    pub full_name: String,
    pub company_name: Option<String>,
    pub is_verified: bool,
}

impl AuthService {
    pub fn new(db: DatabaseConnection, jwt_secret: String) -> Self {
        Self { db, jwt_secret }
    }

    pub async fn register(&self, req: RegisterRequest) -> Result<AuthResponse> {
        // Check if user already exists
        if let Ok(_) = User::find()
            .filter(user::Column::Email.eq(&req.email))
            .one(&self.db)
            .await?
        {
            return Err(anyhow::anyhow!("User already exists"));
        }

        // Hash password
        let password_hash = hash(&req.password, DEFAULT_COST)?;

        // Generate verification token
        let verification_token = generate_token();

        // Create user
        let user_active_model = user::ActiveModel {
            email: ActiveValue::Set(req.email.clone()),
            password_hash: ActiveValue::Set(password_hash),
            full_name: ActiveValue::Set(req.full_name),
            company_name: ActiveValue::Set(req.company_name.clone()),
            verification_token: ActiveValue::Set(Some(verification_token)),
            ..Default::default()
        };

        let user = User::insert(user_active_model).exec(&self.db).await?;

        // Get the created user
        let created_user = User::find_by_id(user.last_insert_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to create user"))?;

        // Generate JWT token
        let token = self.generate_jwt(&created_user)?;

        Ok(AuthResponse {
            token,
            user: UserInfo {
                id: created_user.id,
                email: created_user.email,
                full_name: created_user.full_name,
                company_name: created_user.company_name,
                is_verified: created_user.is_verified,
            },
        })
    }

    pub async fn login(&self, req: LoginRequest) -> Result<AuthResponse> {
        // Find user by email
        let user = User::find()
            .filter(user::Column::Email.eq(&req.email))
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Invalid credentials"))?;

        // Verify password
        if !verify(&req.password, &user.password_hash)? {
            return Err(anyhow::anyhow!("Invalid credentials"));
        }

        if !user.is_active {
            return Err(anyhow::anyhow!("Account is deactivated"));
        }

        // Generate JWT token
        let token = self.generate_jwt(&user)?;

        Ok(AuthResponse {
            token,
            user: UserInfo {
                id: user.id,
                email: user.email,
                full_name: user.full_name,
                company_name: user.company_name,
                is_verified: user.is_verified,
            },
        })
    }

    pub async fn create_session(&self, user_id: i32, ip_address: Option<String>, user_agent: Option<String>) -> Result<String> {
        let session_id = Uuid::new_v4().to_string();
        let expires_at = Utc::now().naive_utc() + Duration::days(30);

        let session_active_model = session::ActiveModel {
            id: ActiveValue::Set(session_id.clone()),
            user_id: ActiveValue::Set(user_id),
            ip_address: ActiveValue::Set(ip_address),
            user_agent: ActiveValue::Set(user_agent),
            expires_at: ActiveValue::Set(expires_at),
            ..Default::default()
        };

        Session::insert(session_active_model).exec(&self.db).await?;

        Ok(session_id)
    }

    pub async fn validate_session(&self, session_id: &str) -> Result<user::Model> {
        let session = Session::find_by_id(session_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Invalid session"))?;

        if session.expires_at < Utc::now().naive_utc() {
            // Clean up expired session
            Session::delete_by_id(session_id).exec(&self.db).await?;
            return Err(anyhow::anyhow!("Session expired"));
        }

        let user = User::find_by_id(session.user_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("User not found"))?;

        if !user.is_active {
            return Err(anyhow::anyhow!("Account deactivated"));
        }

        Ok(user)
    }

    pub async fn logout(&self, session_id: &str) -> Result<()> {
        Session::delete_by_id(session_id).exec(&self.db).await?;
        Ok(())
    }

    pub fn verify_jwt(&self, token: &str) -> Result<Claims> {
        let validation = Validation::new(Algorithm::HS256);
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_ref()),
            &validation,
        )?;

        Ok(token_data.claims)
    }

    pub async fn get_user_by_id(&self, user_id: i32) -> Result<user::Model> {
        let user = User::find_by_id(user_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("User not found"))?;
        Ok(user)
    }

    fn generate_jwt(&self, user: &user::Model) -> Result<String> {
        let now = Utc::now();
        let expires = now + Duration::days(7); // JWT expires in 7 days

        let claims = Claims {
            sub: user.id.to_string(),
            email: user.email.clone(),
            exp: expires.timestamp() as usize,
            iat: now.timestamp() as usize,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_ref()),
        )?;

        Ok(token)
    }
}

fn generate_token() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}