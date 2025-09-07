#[cfg(feature = "workers")]
use worker::*;
#[cfg(feature = "workers")]
use worker_sys::D1Database;
use serde::{Deserialize, Serialize};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{rand_core::OsRng, SaltString};

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user id
    pub email: String,
    pub name: String,
    pub exp: i64,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub email: String,
    pub password: String,
}

// Get JWT secret from environment (Workers version)
#[cfg(feature = "workers")]
fn get_jwt_secret(env: &Env) -> Result<String> {
    env.secret("JWT_SECRET")
        .map(|s| s.to_string())
        .map_err(|_| Error::RustError("JWT_SECRET not configured".to_string()))
}

// Get JWT secret from environment (native version)
#[cfg(feature = "native")]
fn get_jwt_secret_native() -> String {
    std::env::var("JWT_SECRET").unwrap_or_else(|_| "your-secret-key-here".to_string())
}

// Generate JWT token (works for both targets)
pub fn generate_token(user: &User) -> std::result::Result<String, String> {
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("valid timestamp")
        .timestamp();
    
    let claims = Claims {
        sub: user.id.clone(),
        email: user.email.clone(),
        name: user.name.clone(),
        exp: expiration,
    };
    
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "your-secret-key-here".to_string());
    
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    ).map_err(|e| format!("Failed to generate token: {}", e))
}

// Verify JWT token (shared implementation)
pub fn verify_token_shared(token: &str, secret: &str) -> std::result::Result<Claims, String> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::new(Algorithm::HS256),
    ).map_err(|e| format!("Invalid token: {}", e))?;
    
    Ok(token_data.claims)
}

// Workers version
#[cfg(feature = "workers")]
pub fn verify_token(token: &str, env: &Env) -> Result<Claims> {
    let secret = get_jwt_secret(env)?;
    verify_token_shared(token, &secret).map_err(|e| Error::RustError(e))
}

// Native version
#[cfg(feature = "native")]
pub fn verify_token_native(token: &str) -> Result<Claims, String> {
    let secret = get_jwt_secret_native();
    verify_token_shared(token, &secret)
}

// Workers version - get current user from request  
#[cfg(feature = "workers")]
pub async fn get_current_user(req: &Request, ctx: &RouteContext<()>) -> Result<User> {
    // Extract token from cookie
    let cookie_header = req.headers().get("Cookie")?
        .ok_or_else(|| Error::RustError("No cookie header".to_string()))?;
    
    let token = cookie_header
        .split(';')
        .find(|c| c.trim().starts_with("auth_token="))
        .and_then(|c| c.split('=').nth(1))
        .ok_or_else(|| Error::RustError("No auth token in cookie".to_string()))?;
    
    // Verify token and get claims
    let claims = verify_token(token, &ctx.env)?;
    
    Ok(User {
        id: claims.sub,
        name: claims.name,
        email: claims.email,
    })
}

// Native version - get current user from request
#[cfg(feature = "native")]
pub async fn get_current_user_native(cookie_header: &str) -> Result<User, String> {
    let token = cookie_header
        .split(';')
        .find(|c| c.trim().starts_with("auth_token="))
        .and_then(|c| c.split('=').nth(1))
        .ok_or_else(|| "No auth token in cookie".to_string())?;
    
    // Verify token and get claims
    let claims = verify_token_native(token)?;
    
    Ok(User {
        id: claims.sub,
        name: claims.name,
        email: claims.email,
    })
}

// Hash password using Argon2 (shared implementation)
pub fn hash_password(password: &str) -> std::result::Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("Failed to hash password: {}", e))?;
    
    Ok(password_hash.to_string())
}

// Verify password (shared implementation)
pub fn verify_password(password: &str, hash: &str) -> std::result::Result<bool, String> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| format!("Invalid password hash: {}", e))?;
    
    let argon2 = Argon2::default();
    
    match argon2.verify_password(password.as_bytes(), &parsed_hash) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

// Verify user credentials using SeaORM (works for both targets)
#[cfg(feature = "native")]
pub async fn verify_user(db: &sea_orm::DatabaseConnection, email: &str, password: &str) -> std::result::Result<User, String> {
    // Query user from database
    let user_model = crate::database::find_user_by_email(db, email)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or_else(|| "User not found".to_string())?;
    
    // Verify password
    if !verify_password(password, &user_model.password_hash)? {
        return Err("Invalid password".to_string());
    }
    
    Ok(User {
        id: user_model.id,
        name: user_model.name,
        email: user_model.email,
    })
}

// Create new user using SeaORM (works for both targets)
#[cfg(feature = "native")]
pub async fn create_user(db: &sea_orm::DatabaseConnection, name: &str, email: &str, password: &str) -> std::result::Result<User, String> {
    // Check if user already exists
    if crate::database::find_user_by_email(db, email)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .is_some()
    {
        return Err("User already exists".to_string());
    }
    
    // Hash password
    let password_hash = hash_password(password)?;
    
    // Create user
    let user_model = crate::database::create_user(db, name, email, &password_hash)
        .await
        .map_err(|e| format!("Failed to create user: {}", e))?;
    
    Ok(User {
        id: user_model.id,
        name: user_model.name,
        email: user_model.email,
    })
}