use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Common data structures shared between SeaORM and D1 implementations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRecord {
    pub id: i32,
    pub email: String,
    pub password_hash: String,
    pub full_name: String,
    pub company_name: Option<String>,
    pub is_active: bool,
    pub is_verified: bool,
    pub verification_token: Option<String>,
    pub reset_token: Option<String>,
    pub reset_token_expires: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarRecord {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub timezone: String,
    pub is_active: bool,
    pub booking_buffer_minutes: i32,
    pub max_booking_days_ahead: i32,
    pub min_booking_notice_hours: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookingRecord {
    pub id: i32,
    pub calendar_id: i32,
    pub external_event_id: Option<String>,
    pub guest_email: String,
    pub guest_name: String,
    pub title: String,
    pub description: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub status: String,
    pub meeting_link: Option<String>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub cancellation_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingHoursRecord {
    pub id: i32,
    pub calendar_id: i32,
    pub day_of_week: i32,
    pub start_time: String,
    pub end_time: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub password_hash: String,
    pub full_name: String,
    pub company_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCalendarRequest {
    pub user_id: i32,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub timezone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBookingRequest {
    pub calendar_id: i32,
    pub guest_email: String,
    pub guest_name: String,
    pub title: String,
    pub description: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

// Abstract database service trait
#[async_trait]
pub trait DatabaseService: Send + Sync + Clone {
    // Initialize database schema
    async fn init_schema(&self) -> Result<()>;
    
    // User operations
    async fn create_user(&self, req: CreateUserRequest) -> Result<UserRecord>;
    async fn get_user_by_email(&self, email: &str) -> Result<Option<UserRecord>>;
    async fn get_user_by_id(&self, id: i32) -> Result<Option<UserRecord>>;
    
    // Calendar operations
    async fn create_calendar(&self, req: CreateCalendarRequest) -> Result<CalendarRecord>;
    async fn get_calendar_by_slug(&self, slug: &str) -> Result<Option<CalendarRecord>>;
    async fn get_user_calendars(&self, user_id: i32) -> Result<Vec<CalendarRecord>>;
    async fn get_calendar_by_id(&self, calendar_id: i32) -> Result<Option<CalendarRecord>>;
    
    // Working hours operations
    async fn get_calendar_working_hours(&self, calendar_id: i32) -> Result<Vec<WorkingHoursRecord>>;
    async fn set_calendar_working_hours(&self, calendar_id: i32, working_hours: Vec<WorkingHoursRecord>) -> Result<()>;
    
    // Booking operations
    async fn create_booking(&self, req: CreateBookingRequest) -> Result<BookingRecord>;
    async fn get_booking_by_id(&self, id: i32) -> Result<Option<BookingRecord>>;
    async fn get_calendar_bookings(&self, calendar_id: i32, limit: Option<u64>) -> Result<Vec<BookingRecord>>;
    async fn check_booking_conflicts(&self, calendar_id: i32, start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> Result<Vec<BookingRecord>>;
}

// SeaORM implementation for local development
#[cfg(not(feature = "workers"))]
#[derive(Clone)]
pub struct SeaOrmDatabaseService {
    db: sea_orm::DatabaseConnection,
}

#[cfg(not(feature = "workers"))]
impl SeaOrmDatabaseService {
    pub fn new(db: sea_orm::DatabaseConnection) -> Self {
        Self { db }
    }
}

#[cfg(not(feature = "workers"))]
#[async_trait]
impl DatabaseService for SeaOrmDatabaseService {
    async fn init_schema(&self) -> Result<()> {
        use migration::{Migrator, MigratorTrait};
        Migrator::up(&self.db, None).await?;
        Ok(())
    }
    
    async fn create_user(&self, req: CreateUserRequest) -> Result<UserRecord> {
        use crate::models::user;
        use sea_orm::*;
        
        let user_active_model = user::ActiveModel {
            email: ActiveValue::Set(req.email),
            password_hash: ActiveValue::Set(req.password_hash),
            full_name: ActiveValue::Set(req.full_name),
            company_name: ActiveValue::Set(req.company_name),
            ..Default::default()
        };

        let result = user::Entity::insert(user_active_model).exec(&self.db).await?;
        let created_user = user::Entity::find_by_id(result.last_insert_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to create user"))?;

        Ok(UserRecord {
            id: created_user.id,
            email: created_user.email,
            password_hash: created_user.password_hash,
            full_name: created_user.full_name,
            company_name: created_user.company_name,
            is_active: created_user.is_active,
            is_verified: created_user.is_verified,
            verification_token: created_user.verification_token,
            reset_token: created_user.reset_token,
            reset_token_expires: created_user.reset_token_expires.map(|dt| DateTime::from_utc(dt, Utc)),
            created_at: DateTime::from_utc(created_user.created_at, Utc),
            updated_at: DateTime::from_utc(created_user.updated_at, Utc),
        })
    }
    
    async fn get_user_by_email(&self, email: &str) -> Result<Option<UserRecord>> {
        use crate::models::user;
        use sea_orm::*;
        
        let user = user::Entity::find()
            .filter(user::Column::Email.eq(email))
            .one(&self.db)
            .await?;
            
        Ok(user.map(|u| UserRecord {
            id: u.id,
            email: u.email,
            password_hash: u.password_hash,
            full_name: u.full_name,
            company_name: u.company_name,
            is_active: u.is_active,
            is_verified: u.is_verified,
            verification_token: u.verification_token,
            reset_token: u.reset_token,
            reset_token_expires: u.reset_token_expires.map(|dt| DateTime::from_utc(dt, Utc)),
            created_at: DateTime::from_utc(u.created_at, Utc),
            updated_at: DateTime::from_utc(u.updated_at, Utc),
        }))
    }
    
    async fn get_user_by_id(&self, id: i32) -> Result<Option<UserRecord>> {
        use crate::models::user;
        use sea_orm::*;
        
        let user = user::Entity::find_by_id(id).one(&self.db).await?;
        Ok(user.map(|u| UserRecord {
            id: u.id,
            email: u.email,
            password_hash: u.password_hash,
            full_name: u.full_name,
            company_name: u.company_name,
            is_active: u.is_active,
            is_verified: u.is_verified,
            verification_token: u.verification_token,
            reset_token: u.reset_token,
            reset_token_expires: u.reset_token_expires.map(|dt| DateTime::from_utc(dt, Utc)),
            created_at: DateTime::from_utc(u.created_at, Utc),
            updated_at: DateTime::from_utc(u.updated_at, Utc),
        }))
    }
    
    async fn create_calendar(&self, req: CreateCalendarRequest) -> Result<CalendarRecord> {
        use crate::models::calendar;
        use sea_orm::*;
        
        let calendar_active_model = calendar::ActiveModel {
            user_id: ActiveValue::Set(req.user_id),
            name: ActiveValue::Set(req.name),
            slug: ActiveValue::Set(req.slug),
            description: ActiveValue::Set(req.description),
            timezone: ActiveValue::Set(req.timezone),
            ..Default::default()
        };

        let result = calendar::Entity::insert(calendar_active_model).exec(&self.db).await?;
        let created_calendar = calendar::Entity::find_by_id(result.last_insert_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to create calendar"))?;

        Ok(CalendarRecord {
            id: created_calendar.id,
            user_id: created_calendar.user_id,
            name: created_calendar.name,
            slug: created_calendar.slug,
            description: created_calendar.description,
            timezone: created_calendar.timezone,
            is_active: created_calendar.is_active,
            booking_buffer_minutes: created_calendar.booking_buffer_minutes,
            max_booking_days_ahead: created_calendar.max_booking_days_ahead,
            min_booking_notice_hours: created_calendar.min_booking_notice_hours,
            created_at: DateTime::from_utc(created_calendar.created_at, Utc),
            updated_at: DateTime::from_utc(created_calendar.updated_at, Utc),
        })
    }
    
    async fn get_calendar_by_slug(&self, slug: &str) -> Result<Option<CalendarRecord>> {
        use crate::models::calendar;
        use sea_orm::*;
        
        let calendar = calendar::Entity::find()
            .filter(calendar::Column::Slug.eq(slug))
            .filter(calendar::Column::IsActive.eq(true))
            .one(&self.db)
            .await?;
            
        Ok(calendar.map(|c| CalendarRecord {
            id: c.id,
            user_id: c.user_id,
            name: c.name,
            slug: c.slug,
            description: c.description,
            timezone: c.timezone,
            is_active: c.is_active,
            booking_buffer_minutes: c.booking_buffer_minutes,
            max_booking_days_ahead: c.max_booking_days_ahead,
            min_booking_notice_hours: c.min_booking_notice_hours,
            created_at: DateTime::from_utc(c.created_at, Utc),
            updated_at: DateTime::from_utc(c.updated_at, Utc),
        }))
    }
    
    async fn get_user_calendars(&self, user_id: i32) -> Result<Vec<CalendarRecord>> {
        use crate::models::calendar;
        use sea_orm::*;
        
        let calendars = calendar::Entity::find()
            .filter(calendar::Column::UserId.eq(user_id))
            .order_by_desc(calendar::Column::CreatedAt)
            .all(&self.db)
            .await?;
            
        Ok(calendars.into_iter().map(|c| CalendarRecord {
            id: c.id,
            user_id: c.user_id,
            name: c.name,
            slug: c.slug,
            description: c.description,
            timezone: c.timezone,
            is_active: c.is_active,
            booking_buffer_minutes: c.booking_buffer_minutes,
            max_booking_days_ahead: c.max_booking_days_ahead,
            min_booking_notice_hours: c.min_booking_notice_hours,
            created_at: DateTime::from_utc(c.created_at, Utc),
            updated_at: DateTime::from_utc(c.updated_at, Utc),
        }).collect())
    }
    
    async fn get_calendar_by_id(&self, calendar_id: i32) -> Result<Option<CalendarRecord>> {
        use crate::models::calendar;
        use sea_orm::*;
        
        let calendar = calendar::Entity::find_by_id(calendar_id).one(&self.db).await?;
        Ok(calendar.map(|c| CalendarRecord {
            id: c.id,
            user_id: c.user_id,
            name: c.name,
            slug: c.slug,
            description: c.description,
            timezone: c.timezone,
            is_active: c.is_active,
            booking_buffer_minutes: c.booking_buffer_minutes,
            max_booking_days_ahead: c.max_booking_days_ahead,
            min_booking_notice_hours: c.min_booking_notice_hours,
            created_at: DateTime::from_utc(c.created_at, Utc),
            updated_at: DateTime::from_utc(c.updated_at, Utc),
        }))
    }
    
    async fn get_calendar_working_hours(&self, calendar_id: i32) -> Result<Vec<WorkingHoursRecord>> {
        use crate::models::calendar_working_hours;
        use sea_orm::*;
        
        let working_hours = calendar_working_hours::Entity::find()
            .filter(calendar_working_hours::Column::CalendarId.eq(calendar_id))
            .filter(calendar_working_hours::Column::IsActive.eq(true))
            .order_by_asc(calendar_working_hours::Column::DayOfWeek)
            .all(&self.db)
            .await?;
            
        Ok(working_hours.into_iter().map(|wh| WorkingHoursRecord {
            id: wh.id,
            calendar_id: wh.calendar_id,
            day_of_week: wh.day_of_week,
            start_time: wh.start_time,
            end_time: wh.end_time,
            is_active: wh.is_active,
            created_at: DateTime::from_utc(wh.created_at, Utc),
            updated_at: DateTime::from_utc(wh.updated_at, Utc),
        }).collect())
    }
    
    async fn set_calendar_working_hours(&self, calendar_id: i32, working_hours: Vec<WorkingHoursRecord>) -> Result<()> {
        use crate::models::calendar_working_hours;
        use sea_orm::*;
        
        // Delete existing working hours
        calendar_working_hours::Entity::delete_many()
            .filter(calendar_working_hours::Column::CalendarId.eq(calendar_id))
            .exec(&self.db)
            .await?;
        
        // Insert new working hours
        for wh in working_hours {
            let working_hours_active_model = calendar_working_hours::ActiveModel {
                calendar_id: ActiveValue::Set(calendar_id),
                day_of_week: ActiveValue::Set(wh.day_of_week),
                start_time: ActiveValue::Set(wh.start_time),
                end_time: ActiveValue::Set(wh.end_time),
                is_active: ActiveValue::Set(wh.is_active),
                ..Default::default()
            };
            calendar_working_hours::Entity::insert(working_hours_active_model)
                .exec(&self.db)
                .await?;
        }
        
        Ok(())
    }
    
    async fn create_booking(&self, req: CreateBookingRequest) -> Result<BookingRecord> {
        use crate::models::booking;
        use sea_orm::*;
        
        let booking_active_model = booking::ActiveModel {
            calendar_id: ActiveValue::Set(req.calendar_id),
            guest_email: ActiveValue::Set(req.guest_email),
            guest_name: ActiveValue::Set(req.guest_name),
            title: ActiveValue::Set(req.title),
            description: ActiveValue::Set(req.description),
            start_time: ActiveValue::Set(req.start_time.naive_utc()),
            end_time: ActiveValue::Set(req.end_time.naive_utc()),
            status: ActiveValue::Set("confirmed".to_string()),
            ..Default::default()
        };

        let result = booking::Entity::insert(booking_active_model).exec(&self.db).await?;
        let created_booking = booking::Entity::find_by_id(result.last_insert_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to create booking"))?;

        Ok(BookingRecord {
            id: created_booking.id,
            calendar_id: created_booking.calendar_id,
            external_event_id: created_booking.external_event_id,
            guest_email: created_booking.guest_email,
            guest_name: created_booking.guest_name,
            title: created_booking.title,
            description: created_booking.description,
            start_time: DateTime::from_utc(created_booking.start_time, Utc),
            end_time: DateTime::from_utc(created_booking.end_time, Utc),
            status: created_booking.status,
            meeting_link: created_booking.meeting_link,
            cancelled_at: created_booking.cancelled_at.map(|dt| DateTime::from_utc(dt, Utc)),
            cancellation_reason: created_booking.cancellation_reason,
            created_at: DateTime::from_utc(created_booking.created_at, Utc),
            updated_at: DateTime::from_utc(created_booking.updated_at, Utc),
        })
    }
    
    async fn get_booking_by_id(&self, id: i32) -> Result<Option<BookingRecord>> {
        use crate::models::booking;
        use sea_orm::*;
        
        let booking = booking::Entity::find_by_id(id).one(&self.db).await?;
        Ok(booking.map(|b| BookingRecord {
            id: b.id,
            calendar_id: b.calendar_id,
            external_event_id: b.external_event_id,
            guest_email: b.guest_email,
            guest_name: b.guest_name,
            title: b.title,
            description: b.description,
            start_time: DateTime::from_utc(b.start_time, Utc),
            end_time: DateTime::from_utc(b.end_time, Utc),
            status: b.status,
            meeting_link: b.meeting_link,
            cancelled_at: b.cancelled_at.map(|dt| DateTime::from_utc(dt, Utc)),
            cancellation_reason: b.cancellation_reason,
            created_at: DateTime::from_utc(b.created_at, Utc),
            updated_at: DateTime::from_utc(b.updated_at, Utc),
        }))
    }
    
    async fn get_calendar_bookings(&self, calendar_id: i32, limit: Option<u64>) -> Result<Vec<BookingRecord>> {
        use crate::models::booking;
        use sea_orm::*;
        
        let mut query = booking::Entity::find()
            .filter(booking::Column::CalendarId.eq(calendar_id))
            .order_by_desc(booking::Column::StartTime);
            
        if let Some(limit_val) = limit {
            query = query.limit(limit_val);
        }
        
        let bookings = query.all(&self.db).await?;
        
        Ok(bookings.into_iter().map(|b| BookingRecord {
            id: b.id,
            calendar_id: b.calendar_id,
            external_event_id: b.external_event_id,
            guest_email: b.guest_email,
            guest_name: b.guest_name,
            title: b.title,
            description: b.description,
            start_time: DateTime::from_utc(b.start_time, Utc),
            end_time: DateTime::from_utc(b.end_time, Utc),
            status: b.status,
            meeting_link: b.meeting_link,
            cancelled_at: b.cancelled_at.map(|dt| DateTime::from_utc(dt, Utc)),
            cancellation_reason: b.cancellation_reason,
            created_at: DateTime::from_utc(b.created_at, Utc),
            updated_at: DateTime::from_utc(b.updated_at, Utc),
        }).collect())
    }
    
    async fn check_booking_conflicts(&self, calendar_id: i32, start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> Result<Vec<BookingRecord>> {
        use crate::models::booking;
        use sea_orm::*;
        
        let start_naive = start_time.naive_utc();
        let end_naive = end_time.naive_utc();
        
        let conflicts = booking::Entity::find()
            .filter(booking::Column::CalendarId.eq(calendar_id))
            .filter(booking::Column::Status.ne("cancelled"))
            .filter(
                Condition::any()
                    .add(Condition::all()
                        .add(booking::Column::StartTime.lte(start_naive))
                        .add(booking::Column::EndTime.gt(start_naive))
                    )
                    .add(Condition::all()
                        .add(booking::Column::StartTime.lt(end_naive))
                        .add(booking::Column::EndTime.gte(end_naive))
                    )
                    .add(Condition::all()
                        .add(booking::Column::StartTime.gte(start_naive))
                        .add(booking::Column::EndTime.lte(end_naive))
                    )
            )
            .all(&self.db)
            .await?;
            
        Ok(conflicts.into_iter().map(|b| BookingRecord {
            id: b.id,
            calendar_id: b.calendar_id,
            external_event_id: b.external_event_id,
            guest_email: b.guest_email,
            guest_name: b.guest_name,
            title: b.title,
            description: b.description,
            start_time: DateTime::from_utc(b.start_time, Utc),
            end_time: DateTime::from_utc(b.end_time, Utc),
            status: b.status,
            meeting_link: b.meeting_link,
            cancelled_at: b.cancelled_at.map(|dt| DateTime::from_utc(dt, Utc)),
            cancellation_reason: b.cancellation_reason,
            created_at: DateTime::from_utc(b.created_at, Utc),
            updated_at: DateTime::from_utc(b.updated_at, Utc),
        }).collect())
    }
}

// D1 implementation for Cloudflare Workers - uses SeaORM with D1 connection
#[cfg(feature = "workers")]
#[derive(Clone)]
pub struct D1DatabaseService {
    db: sea_orm::DatabaseConnection,
}

#[cfg(feature = "workers")]
impl D1DatabaseService {
    pub fn new(db: sea_orm::DatabaseConnection) -> Self {
        Self { db }
    }
}

#[cfg(feature = "workers")]
#[async_trait]
impl DatabaseService for D1DatabaseService {
    // D1DatabaseService uses the exact same SeaORM implementation as SeaOrmDatabaseService
    // The only difference is the underlying database connection (D1 vs SQLite)
    
    async fn init_schema(&self) -> Result<()> {
        use migration::{Migrator, MigratorTrait};
        Migrator::up(&self.db, None).await?;
        Ok(())
    }
    
    async fn create_user(&self, req: CreateUserRequest) -> Result<UserRecord> {
        use crate::models::user;
        use sea_orm::*;
        
        let user_active_model = user::ActiveModel {
            email: ActiveValue::Set(req.email),
            password_hash: ActiveValue::Set(req.password_hash),
            full_name: ActiveValue::Set(req.full_name),
            company_name: ActiveValue::Set(req.company_name),
            ..Default::default()
        };

        let result = user::Entity::insert(user_active_model).exec(&self.db).await?;
        let created_user = user::Entity::find_by_id(result.last_insert_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to create user"))?;

        Ok(UserRecord {
            id: created_user.id,
            email: created_user.email,
            password_hash: created_user.password_hash,
            full_name: created_user.full_name,
            company_name: created_user.company_name,
            is_active: created_user.is_active,
            is_verified: created_user.is_verified,
            verification_token: created_user.verification_token,
            reset_token: created_user.reset_token,
            reset_token_expires: created_user.reset_token_expires.map(|dt| DateTime::from_utc(dt, Utc)),
            created_at: DateTime::from_utc(created_user.created_at, Utc),
            updated_at: DateTime::from_utc(created_user.updated_at, Utc),
        })
    }
    
    async fn get_user_by_email(&self, email: &str) -> Result<Option<UserRecord>> {
        use crate::models::user;
        use sea_orm::*;
        
        let user = user::Entity::find()
            .filter(user::Column::Email.eq(email))
            .one(&self.db)
            .await?;
            
        Ok(user.map(|u| UserRecord {
            id: u.id,
            email: u.email,
            password_hash: u.password_hash,
            full_name: u.full_name,
            company_name: u.company_name,
            is_active: u.is_active,
            is_verified: u.is_verified,
            verification_token: u.verification_token,
            reset_token: u.reset_token,
            reset_token_expires: u.reset_token_expires.map(|dt| DateTime::from_utc(dt, Utc)),
            created_at: DateTime::from_utc(u.created_at, Utc),
            updated_at: DateTime::from_utc(u.updated_at, Utc),
        }))
    }
    
    async fn get_user_by_id(&self, id: i32) -> Result<Option<UserRecord>> {
        use crate::models::user;
        use sea_orm::*;
        
        let user = user::Entity::find_by_id(id).one(&self.db).await?;
        Ok(user.map(|u| UserRecord {
            id: u.id,
            email: u.email,
            password_hash: u.password_hash,
            full_name: u.full_name,
            company_name: u.company_name,
            is_active: u.is_active,
            is_verified: u.is_verified,
            verification_token: u.verification_token,
            reset_token: u.reset_token,
            reset_token_expires: u.reset_token_expires.map(|dt| DateTime::from_utc(dt, Utc)),
            created_at: DateTime::from_utc(u.created_at, Utc),
            updated_at: DateTime::from_utc(u.updated_at, Utc),
        }))
    }
    
    async fn create_calendar(&self, req: CreateCalendarRequest) -> Result<CalendarRecord> {
        use crate::models::calendar;
        use sea_orm::*;
        
        let calendar_active_model = calendar::ActiveModel {
            user_id: ActiveValue::Set(req.user_id),
            name: ActiveValue::Set(req.name),
            slug: ActiveValue::Set(req.slug),
            description: ActiveValue::Set(req.description),
            timezone: ActiveValue::Set(req.timezone),
            ..Default::default()
        };

        let result = calendar::Entity::insert(calendar_active_model).exec(&self.db).await?;
        let created_calendar = calendar::Entity::find_by_id(result.last_insert_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to create calendar"))?;

        Ok(CalendarRecord {
            id: created_calendar.id,
            user_id: created_calendar.user_id,
            name: created_calendar.name,
            slug: created_calendar.slug,
            description: created_calendar.description,
            timezone: created_calendar.timezone,
            is_active: created_calendar.is_active,
            booking_buffer_minutes: created_calendar.booking_buffer_minutes,
            max_booking_days_ahead: created_calendar.max_booking_days_ahead,
            min_booking_notice_hours: created_calendar.min_booking_notice_hours,
            created_at: DateTime::from_utc(created_calendar.created_at, Utc),
            updated_at: DateTime::from_utc(created_calendar.updated_at, Utc),
        })
    }
    
    async fn get_calendar_by_slug(&self, slug: &str) -> Result<Option<CalendarRecord>> {
        use crate::models::calendar;
        use sea_orm::*;
        
        let calendar = calendar::Entity::find()
            .filter(calendar::Column::Slug.eq(slug))
            .filter(calendar::Column::IsActive.eq(true))
            .one(&self.db)
            .await?;
            
        Ok(calendar.map(|c| CalendarRecord {
            id: c.id,
            user_id: c.user_id,
            name: c.name,
            slug: c.slug,
            description: c.description,
            timezone: c.timezone,
            is_active: c.is_active,
            booking_buffer_minutes: c.booking_buffer_minutes,
            max_booking_days_ahead: c.max_booking_days_ahead,
            min_booking_notice_hours: c.min_booking_notice_hours,
            created_at: DateTime::from_utc(c.created_at, Utc),
            updated_at: DateTime::from_utc(c.updated_at, Utc),
        }))
    }
    
    async fn get_user_calendars(&self, user_id: i32) -> Result<Vec<CalendarRecord>> {
        use crate::models::calendar;
        use sea_orm::*;
        
        let calendars = calendar::Entity::find()
            .filter(calendar::Column::UserId.eq(user_id))
            .order_by_desc(calendar::Column::CreatedAt)
            .all(&self.db)
            .await?;
            
        Ok(calendars.into_iter().map(|c| CalendarRecord {
            id: c.id,
            user_id: c.user_id,
            name: c.name,
            slug: c.slug,
            description: c.description,
            timezone: c.timezone,
            is_active: c.is_active,
            booking_buffer_minutes: c.booking_buffer_minutes,
            max_booking_days_ahead: c.max_booking_days_ahead,
            min_booking_notice_hours: c.min_booking_notice_hours,
            created_at: DateTime::from_utc(c.created_at, Utc),
            updated_at: DateTime::from_utc(c.updated_at, Utc),
        }).collect())
    }
    
    async fn get_calendar_by_id(&self, calendar_id: i32) -> Result<Option<CalendarRecord>> {
        use crate::models::calendar;
        use sea_orm::*;
        
        let calendar = calendar::Entity::find_by_id(calendar_id).one(&self.db).await?;
        Ok(calendar.map(|c| CalendarRecord {
            id: c.id,
            user_id: c.user_id,
            name: c.name,
            slug: c.slug,
            description: c.description,
            timezone: c.timezone,
            is_active: c.is_active,
            booking_buffer_minutes: c.booking_buffer_minutes,
            max_booking_days_ahead: c.max_booking_days_ahead,
            min_booking_notice_hours: c.min_booking_notice_hours,
            created_at: DateTime::from_utc(c.created_at, Utc),
            updated_at: DateTime::from_utc(c.updated_at, Utc),
        }))
    }
    
    async fn get_calendar_working_hours(&self, calendar_id: i32) -> Result<Vec<WorkingHoursRecord>> {
        use crate::models::calendar_working_hours;
        use sea_orm::*;
        
        let working_hours = calendar_working_hours::Entity::find()
            .filter(calendar_working_hours::Column::CalendarId.eq(calendar_id))
            .filter(calendar_working_hours::Column::IsActive.eq(true))
            .order_by_asc(calendar_working_hours::Column::DayOfWeek)
            .all(&self.db)
            .await?;
            
        Ok(working_hours.into_iter().map(|wh| WorkingHoursRecord {
            id: wh.id,
            calendar_id: wh.calendar_id,
            day_of_week: wh.day_of_week,
            start_time: wh.start_time,
            end_time: wh.end_time,
            is_active: wh.is_active,
            created_at: DateTime::from_utc(wh.created_at, Utc),
            updated_at: DateTime::from_utc(wh.updated_at, Utc),
        }).collect())
    }
    
    async fn set_calendar_working_hours(&self, calendar_id: i32, working_hours: Vec<WorkingHoursRecord>) -> Result<()> {
        use crate::models::calendar_working_hours;
        use sea_orm::*;
        
        // Delete existing working hours
        calendar_working_hours::Entity::delete_many()
            .filter(calendar_working_hours::Column::CalendarId.eq(calendar_id))
            .exec(&self.db)
            .await?;
        
        // Insert new working hours
        for wh in working_hours {
            let working_hours_active_model = calendar_working_hours::ActiveModel {
                calendar_id: ActiveValue::Set(calendar_id),
                day_of_week: ActiveValue::Set(wh.day_of_week),
                start_time: ActiveValue::Set(wh.start_time),
                end_time: ActiveValue::Set(wh.end_time),
                is_active: ActiveValue::Set(wh.is_active),
                ..Default::default()
            };
            calendar_working_hours::Entity::insert(working_hours_active_model)
                .exec(&self.db)
                .await?;
        }
        
        Ok(())
    }
    
    async fn create_booking(&self, req: CreateBookingRequest) -> Result<BookingRecord> {
        use crate::models::booking;
        use sea_orm::*;
        
        let booking_active_model = booking::ActiveModel {
            calendar_id: ActiveValue::Set(req.calendar_id),
            guest_email: ActiveValue::Set(req.guest_email),
            guest_name: ActiveValue::Set(req.guest_name),
            title: ActiveValue::Set(req.title),
            description: ActiveValue::Set(req.description),
            start_time: ActiveValue::Set(req.start_time.naive_utc()),
            end_time: ActiveValue::Set(req.end_time.naive_utc()),
            status: ActiveValue::Set("confirmed".to_string()),
            ..Default::default()
        };

        let result = booking::Entity::insert(booking_active_model).exec(&self.db).await?;
        let created_booking = booking::Entity::find_by_id(result.last_insert_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to create booking"))?;

        Ok(BookingRecord {
            id: created_booking.id,
            calendar_id: created_booking.calendar_id,
            external_event_id: created_booking.external_event_id,
            guest_email: created_booking.guest_email,
            guest_name: created_booking.guest_name,
            title: created_booking.title,
            description: created_booking.description,
            start_time: DateTime::from_utc(created_booking.start_time, Utc),
            end_time: DateTime::from_utc(created_booking.end_time, Utc),
            status: created_booking.status,
            meeting_link: created_booking.meeting_link,
            cancelled_at: created_booking.cancelled_at.map(|dt| DateTime::from_utc(dt, Utc)),
            cancellation_reason: created_booking.cancellation_reason,
            created_at: DateTime::from_utc(created_booking.created_at, Utc),
            updated_at: DateTime::from_utc(created_booking.updated_at, Utc),
        })
    }
    
    async fn get_booking_by_id(&self, id: i32) -> Result<Option<BookingRecord>> {
        use crate::models::booking;
        use sea_orm::*;
        
        let booking = booking::Entity::find_by_id(id).one(&self.db).await?;
        Ok(booking.map(|b| BookingRecord {
            id: b.id,
            calendar_id: b.calendar_id,
            external_event_id: b.external_event_id,
            guest_email: b.guest_email,
            guest_name: b.guest_name,
            title: b.title,
            description: b.description,
            start_time: DateTime::from_utc(b.start_time, Utc),
            end_time: DateTime::from_utc(b.end_time, Utc),
            status: b.status,
            meeting_link: b.meeting_link,
            cancelled_at: b.cancelled_at.map(|dt| DateTime::from_utc(dt, Utc)),
            cancellation_reason: b.cancellation_reason,
            created_at: DateTime::from_utc(b.created_at, Utc),
            updated_at: DateTime::from_utc(b.updated_at, Utc),
        }))
    }
    
    async fn get_calendar_bookings(&self, calendar_id: i32, limit: Option<u64>) -> Result<Vec<BookingRecord>> {
        use crate::models::booking;
        use sea_orm::*;
        
        let mut query = booking::Entity::find()
            .filter(booking::Column::CalendarId.eq(calendar_id))
            .order_by_desc(booking::Column::StartTime);
            
        if let Some(limit_val) = limit {
            query = query.limit(limit_val);
        }
        
        let bookings = query.all(&self.db).await?;
        
        Ok(bookings.into_iter().map(|b| BookingRecord {
            id: b.id,
            calendar_id: b.calendar_id,
            external_event_id: b.external_event_id,
            guest_email: b.guest_email,
            guest_name: b.guest_name,
            title: b.title,
            description: b.description,
            start_time: DateTime::from_utc(b.start_time, Utc),
            end_time: DateTime::from_utc(b.end_time, Utc),
            status: b.status,
            meeting_link: b.meeting_link,
            cancelled_at: b.cancelled_at.map(|dt| DateTime::from_utc(dt, Utc)),
            cancellation_reason: b.cancellation_reason,
            created_at: DateTime::from_utc(b.created_at, Utc),
            updated_at: DateTime::from_utc(b.updated_at, Utc),
        }).collect())
    }
    
    async fn check_booking_conflicts(&self, calendar_id: i32, start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> Result<Vec<BookingRecord>> {
        use crate::models::booking;
        use sea_orm::*;
        
        let start_naive = start_time.naive_utc();
        let end_naive = end_time.naive_utc();
        
        let conflicts = booking::Entity::find()
            .filter(booking::Column::CalendarId.eq(calendar_id))
            .filter(booking::Column::Status.ne("cancelled"))
            .filter(
                Condition::any()
                    .add(Condition::all()
                        .add(booking::Column::StartTime.lte(start_naive))
                        .add(booking::Column::EndTime.gt(start_naive))
                    )
                    .add(Condition::all()
                        .add(booking::Column::StartTime.lt(end_naive))
                        .add(booking::Column::EndTime.gte(end_naive))
                    )
                    .add(Condition::all()
                        .add(booking::Column::StartTime.gte(start_naive))
                        .add(booking::Column::EndTime.lte(end_naive))
                    )
            )
            .all(&self.db)
            .await?;
            
        Ok(conflicts.into_iter().map(|b| BookingRecord {
            id: b.id,
            calendar_id: b.calendar_id,
            external_event_id: b.external_event_id,
            guest_email: b.guest_email,
            guest_name: b.guest_name,
            title: b.title,
            description: b.description,
            start_time: DateTime::from_utc(b.start_time, Utc),
            end_time: DateTime::from_utc(b.end_time, Utc),
            status: b.status,
            meeting_link: b.meeting_link,
            cancelled_at: b.cancelled_at.map(|dt| DateTime::from_utc(dt, Utc)),
            cancellation_reason: b.cancellation_reason,
            created_at: DateTime::from_utc(b.created_at, Utc),
            updated_at: DateTime::from_utc(b.updated_at, Utc),
        }).collect())
    }
}