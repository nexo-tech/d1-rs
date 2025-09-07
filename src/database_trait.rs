use async_trait::async_trait;
use chrono::Utc;
use crate::{auth::User, shared_types::*};

/// Database trait that abstracts over SeaORM (native) and D1 (Workers)
#[async_trait]
pub trait DatabaseTrait {
    type Error: std::error::Error + Send + Sync + 'static;

    // User operations
    async fn create_user(&self, name: &str, email: &str, password_hash: &str) -> Result<User, Self::Error>;
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, Self::Error>;
    async fn verify_user_password(&self, email: &str, password: &str) -> Result<Option<User>, Self::Error>;

    // Calendar operations
    async fn create_calendar(&self, user_id: &str, req: &CreateCalendarRequest) -> Result<Calendar, Self::Error>;
    async fn get_calendar(&self, calendar_id: &str, user_id: &str) -> Result<Option<Calendar>, Self::Error>;
    async fn get_calendar_by_slug(&self, slug: &str) -> Result<Option<Calendar>, Self::Error>;
    async fn get_user_calendars(&self, user_id: &str) -> Result<Vec<Calendar>, Self::Error>;
    async fn get_calendar_count(&self, user_id: &str) -> Result<u64, Self::Error>;

    // Booking operations
    async fn create_booking(&self, slug: &str, req: &BookingRequest) -> Result<Booking, Self::Error>;
    async fn get_available_slots(&self, slug: &str, date: &str) -> Result<Vec<String>, Self::Error>;
    async fn get_booking_count(&self, user_id: &str) -> Result<u64, Self::Error>;
}

// SeaORM implementation for native
#[cfg(feature = "native")]
pub struct SeaOrmDatabase {
    pub connection: sea_orm::DatabaseConnection,
}

#[cfg(feature = "native")]
#[async_trait]
impl DatabaseTrait for SeaOrmDatabase {
    type Error = sea_orm::DbErr;

    async fn create_user(&self, name: &str, email: &str, password_hash: &str) -> Result<User, Self::Error> {
        crate::auth::create_user(&self.connection, name, email, password_hash).await
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, Self::Error> {
        use crate::entities::users;
        use sea_orm::*;
        
        let user = users::Entity::find()
            .filter(users::Column::Email.eq(email))
            .one(&self.connection)
            .await?;
            
        Ok(user.map(|u| User {
            id: u.id,
            name: u.name,
            email: u.email,
        }))
    }

    async fn verify_user_password(&self, email: &str, password: &str) -> Result<Option<User>, Self::Error> {
        crate::auth::verify_user(&self.connection, email, password).await
    }

    async fn create_calendar(&self, user_id: &str, req: &CreateCalendarRequest) -> Result<Calendar, Self::Error> {
        crate::database::create_calendar(&self.connection, user_id, req).await
    }

    async fn get_calendar(&self, calendar_id: &str, user_id: &str) -> Result<Option<Calendar>, Self::Error> {
        crate::database::get_calendar(&self.connection, calendar_id, user_id).await
    }

    async fn get_calendar_by_slug(&self, slug: &str) -> Result<Option<Calendar>, Self::Error> {
        crate::database::get_calendar_by_slug(&self.connection, slug).await
    }

    async fn get_user_calendars(&self, user_id: &str) -> Result<Vec<Calendar>, Self::Error> {
        crate::database::get_user_calendars(&self.connection, user_id).await
    }

    async fn get_calendar_count(&self, user_id: &str) -> Result<u64, Self::Error> {
        crate::database::get_calendar_count(&self.connection, user_id).await
    }

    async fn create_booking(&self, slug: &str, req: &BookingRequest) -> Result<Booking, Self::Error> {
        crate::database::create_booking(&self.connection, slug, req).await
    }

    async fn get_available_slots(&self, slug: &str, date: &str) -> Result<Vec<String>, Self::Error> {
        crate::database::get_available_slots(&self.connection, slug, date).await
    }

    async fn get_booking_count(&self, user_id: &str) -> Result<u64, Self::Error> {
        crate::database::get_booking_count(&self.connection, user_id).await
    }
}

// D1 implementation for Workers
#[cfg(feature = "workers")]
pub struct D1Database {
    pub env: worker::Env,
}

#[cfg(feature = "workers")]
#[async_trait]
impl DatabaseTrait for D1Database {
    type Error = worker::Error;

    async fn create_user(&self, name: &str, email: &str, password_hash: &str) -> Result<User, Self::Error> {
        let d1 = self.env.d1("DB")?;
        let user_id = uuid::Uuid::new_v4().to_string();
        
        let statement = d1.prepare("INSERT INTO users (id, name, email, password_hash) VALUES (?1, ?2, ?3, ?4)");
        let query = statement.bind(&[user_id.clone().into(), name.into(), email.into(), password_hash.into()])?;
        query.run().await?;
        
        Ok(User {
            id: user_id,
            name: name.to_string(),
            email: email.to_string(),
        })
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, Self::Error> {
        let d1 = self.env.d1("DB")?;
        let statement = d1.prepare("SELECT id, name, email FROM users WHERE email = ?1")
            .bind(&[email.into()])?;
        
        if let Some(row) = statement.first::<serde_json::Value>(None).await? {
            Ok(Some(User {
                id: row["id"].as_str().unwrap_or_default().to_string(),
                name: row["name"].as_str().unwrap_or_default().to_string(),
                email: row["email"].as_str().unwrap_or_default().to_string(),
            }))
        } else {
            Ok(None)
        }
    }

    async fn verify_user_password(&self, email: &str, password: &str) -> Result<Option<User>, Self::Error> {
        let d1 = self.env.d1("DB")?;
        let statement = d1.prepare("SELECT id, name, email, password_hash FROM users WHERE email = ?1")
            .bind(&[email.into()])?;
        
        if let Some(row) = statement.first::<serde_json::Value>(None).await? {
            let stored_hash = row["password_hash"].as_str().unwrap_or_default();
            
            // Verify password using argon2
            use argon2::{Argon2, PasswordHash, PasswordVerifier};
            let argon2 = Argon2::default();
            let parsed_hash = PasswordHash::new(stored_hash)
                .map_err(|e| worker::Error::RustError(format!("Password hash error: {}", e)))?;
            
            if argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok() {
                Ok(Some(User {
                    id: row["id"].as_str().unwrap_or_default().to_string(),
                    name: row["name"].as_str().unwrap_or_default().to_string(),
                    email: row["email"].as_str().unwrap_or_default().to_string(),
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    // Placeholder implementations for other methods
    async fn create_calendar(&self, user_id: &str, req: &CreateCalendarRequest) -> Result<Calendar, Self::Error> {
        let d1 = self.env.d1("DB")?;
        let calendar_id = uuid::Uuid::new_v4().to_string();
        let slug = req.slug.clone().unwrap_or_else(|| format!("{}-{}", req.name.to_lowercase().replace(' ', "-"), &calendar_id[..8]));
        
        let statement = d1.prepare("INSERT INTO calendars (id, user_id, name, slug, timezone, duration, buffer) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")
            .bind(&[
                calendar_id.clone().into(),
                user_id.into(),
                req.name.clone().into(),
                slug.clone().into(),
                req.timezone.clone().into(),
                req.duration.into(),
                req.buffer.into()
            ])?;
        statement.run().await?;
        
        Ok(Calendar {
            id: calendar_id,
            user_id: user_id.to_string(),
            name: req.name.clone(),
            slug,
            timezone: req.timezone.clone(),
            duration: req.duration,
            buffer: req.buffer,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    async fn get_calendar(&self, calendar_id: &str, user_id: &str) -> Result<Option<Calendar>, Self::Error> {
        let d1 = self.env.d1("DB")?;
        let statement = d1.prepare("SELECT * FROM calendars WHERE id = ?1 AND user_id = ?2")
            .bind(&[calendar_id.into(), user_id.into()])?;
        
        if let Some(row) = statement.first::<serde_json::Value>(None).await? {
            Ok(Some(Calendar {
                id: row["id"].as_str().unwrap_or_default().to_string(),
                user_id: row["user_id"].as_str().unwrap_or_default().to_string(),
                name: row["name"].as_str().unwrap_or_default().to_string(),
                slug: row["slug"].as_str().unwrap_or_default().to_string(),
                timezone: row["timezone"].as_str().unwrap_or_default().to_string(),
                duration: row["duration"].as_i64().unwrap_or_default() as i32,
                buffer: row["buffer"].as_i64().unwrap_or_default() as i32,
                created_at: Utc::now(), // TODO: Parse from DB
                updated_at: Utc::now(), // TODO: Parse from DB
            }))
        } else {
            Ok(None)
        }
    }

    async fn get_calendar_by_slug(&self, slug: &str) -> Result<Option<Calendar>, Self::Error> {
        let d1 = self.env.d1("DB")?;
        let statement = d1.prepare("SELECT * FROM calendars WHERE slug = ?1")
            .bind(&[slug.into()])?;
        
        if let Some(row) = statement.first::<serde_json::Value>(None).await? {
            Ok(Some(Calendar {
                id: row["id"].as_str().unwrap_or_default().to_string(),
                user_id: row["user_id"].as_str().unwrap_or_default().to_string(),
                name: row["name"].as_str().unwrap_or_default().to_string(),
                slug: row["slug"].as_str().unwrap_or_default().to_string(),
                timezone: row["timezone"].as_str().unwrap_or_default().to_string(),
                duration: row["duration"].as_i64().unwrap_or_default() as i32,
                buffer: row["buffer"].as_i64().unwrap_or_default() as i32,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }))
        } else {
            Ok(None)
        }
    }

    async fn get_user_calendars(&self, user_id: &str) -> Result<Vec<Calendar>, Self::Error> {
        let d1 = self.env.d1("DB")?;
        let statement = d1.prepare("SELECT * FROM calendars WHERE user_id = ?1")
            .bind(&[user_id.into()])?;
        
        let results = statement.all().await?;
        let mut calendars = Vec::new();
        
        for row in results.results::<serde_json::Value>() {
            if let Ok(row) = row {
                calendars.push(Calendar {
                    id: row["id"].as_str().unwrap_or_default().to_string(),
                    user_id: row["user_id"].as_str().unwrap_or_default().to_string(),
                    name: row["name"].as_str().unwrap_or_default().to_string(),
                    slug: row["slug"].as_str().unwrap_or_default().to_string(),
                    timezone: row["timezone"].as_str().unwrap_or_default().to_string(),
                    duration: row["duration"].as_i64().unwrap_or_default() as i32,
                    buffer: row["buffer"].as_i64().unwrap_or_default() as i32,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                });
            }
        }
        
        Ok(calendars)
    }

    async fn get_calendar_count(&self, user_id: &str) -> Result<u64, Self::Error> {
        let d1 = self.env.d1("DB")?;
        let statement = d1.prepare("SELECT COUNT(*) as count FROM calendars WHERE user_id = ?1")
            .bind(&[user_id.into()])?;
        
        if let Some(row) = statement.first::<serde_json::Value>(None).await? {
            Ok(row["count"].as_u64().unwrap_or_default())
        } else {
            Ok(0)
        }
    }

    async fn create_booking(&self, slug: &str, req: &BookingRequest) -> Result<Booking, Self::Error> {
        // TODO: Implement booking creation for D1
        Err(worker::Error::RustError("Booking creation not yet implemented for D1".to_string()))
    }

    async fn get_available_slots(&self, slug: &str, date: &str) -> Result<Vec<String>, Self::Error> {
        // TODO: Implement slot availability for D1
        Ok(vec!["09:00".to_string(), "10:00".to_string(), "11:00".to_string()])
    }

    async fn get_booking_count(&self, user_id: &str) -> Result<u64, Self::Error> {
        let d1 = self.env.d1("DB")?;
        let statement = d1.prepare("SELECT COUNT(*) as count FROM bookings b JOIN calendars c ON b.calendar_id = c.id WHERE c.user_id = ?1")
            .bind(&[user_id.into()])?;
        
        if let Some(row) = statement.first::<serde_json::Value>(None).await? {
            Ok(row["count"].as_u64().unwrap_or_default())
        } else {
            Ok(0)
        }
    }
}