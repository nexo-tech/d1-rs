// Shared database operations using SeaORM for both native and WASM targets
use sea_orm::{*, prelude::Expr};
use chrono::Datelike;
use crate::entities::*;
use crate::db_shared::{generate_time_slots, get_day_of_week, parse_booking_slot};

// DTOs for API requests/responses
#[derive(Debug, serde::Deserialize)]
pub struct CreateCalendarRequest {
    pub name: String,
    pub slug: String,
    pub timezone: String,
    pub duration: i32,
    pub buffer: i32,
}

#[derive(Debug, serde::Deserialize)]
pub struct WorkingHoursRequest {
    pub days: Vec<i32>,
    pub times: Vec<WorkingTime>,
}

#[derive(Debug, serde::Deserialize)]
pub struct WorkingTime {
    pub day: i32,
    pub start: String,
    pub end: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct BookingRequest {
    pub name: String,
    pub email: String,
    pub notes: Option<String>,
    pub slot: String,
}

// User operations
pub async fn create_user(db: &DatabaseConnection, name: &str, email: &str, password_hash: &str) -> Result<user::Model, DbErr> {
    let user_id = uuid::Uuid::new_v4().to_string();
    
    let user = user::ActiveModel {
        id: Set(user_id.clone()),
        name: Set(name.to_string()),
        email: Set(email.to_string()),
        password_hash: Set(password_hash.to_string()),
        created_at: Set(chrono::Utc::now()),
    };
    
    User::insert(user).exec(db).await?;
    
    User::find_by_id(&user_id).one(db).await?.ok_or(DbErr::RecordNotFound("User not found after creation".to_string()))
}

pub async fn find_user_by_email(db: &DatabaseConnection, email: &str) -> Result<Option<user::Model>, DbErr> {
    User::find()
        .filter(user::Column::Email.eq(email))
        .one(db)
        .await
}

// Calendar operations
pub async fn get_calendar_count(db: &DatabaseConnection, user_id: &str) -> Result<u64, DbErr> {
    Calendar::find()
        .filter(calendar::Column::UserId.eq(user_id))
        .count(db)
        .await
}

pub async fn get_booking_count(db: &DatabaseConnection, user_id: &str) -> Result<u64, DbErr> {
    Booking::find()
        .join(JoinType::InnerJoin, booking::Relation::Calendar.def())
        .filter(calendar::Column::UserId.eq(user_id))
        .count(db)
        .await
}

pub async fn get_user_calendars(db: &DatabaseConnection, user_id: &str) -> Result<Vec<calendar::Model>, DbErr> {
    Calendar::find()
        .filter(calendar::Column::UserId.eq(user_id))
        .order_by_desc(calendar::Column::CreatedAt)
        .all(db)
        .await
}

pub async fn create_calendar(
    db: &DatabaseConnection,
    user_id: &str,
    req: &CreateCalendarRequest,
) -> Result<calendar::Model, DbErr> {
    // Check if slug already exists
    if Calendar::find()
        .filter(calendar::Column::Slug.eq(&req.slug))
        .one(db)
        .await?
        .is_some()
    {
        return Err(DbErr::RecordNotFound("Slug already exists".to_string()));
    }
    
    let calendar_id = uuid::Uuid::new_v4().to_string();
    
    // Create calendar
    let calendar = calendar::ActiveModel {
        id: Set(calendar_id.clone()),
        user_id: Set(user_id.to_string()),
        name: Set(req.name.clone()),
        slug: Set(req.slug.clone()),
        timezone: Set(req.timezone.clone()),
        duration: Set(req.duration),
        buffer: Set(req.buffer),
        created_at: Set(chrono::Utc::now()),
    };
    
    Calendar::insert(calendar).exec(db).await?;
    
    // Initialize default working hours (Monday-Friday 9-5)
    for day in 1..=7 {
        let enabled = day <= 5; // Monday-Friday enabled, weekend disabled
        let working_hours = calendar_working_hours::ActiveModel {
            id: Set(uuid::Uuid::new_v4().to_string()),
            calendar_id: Set(calendar_id.clone()),
            day_of_week: Set(day),
            start_time: Set("09:00".to_string()),
            end_time: Set("17:00".to_string()),
            enabled: Set(enabled),
        };
        
        CalendarWorkingHours::insert(working_hours).exec(db).await?;
    }
    
    Calendar::find_by_id(&calendar_id)
        .one(db)
        .await?
        .ok_or(DbErr::RecordNotFound("Calendar not found after creation".to_string()))
}

pub async fn get_calendar(db: &DatabaseConnection, calendar_id: &str, user_id: &str) -> Result<Option<calendar::Model>, DbErr> {
    Calendar::find_by_id(calendar_id)
        .filter(calendar::Column::UserId.eq(user_id))
        .one(db)
        .await
}

pub async fn get_calendar_by_slug(db: &DatabaseConnection, slug: &str) -> Result<Option<calendar::Model>, DbErr> {
    Calendar::find()
        .filter(calendar::Column::Slug.eq(slug))
        .one(db)
        .await
}

pub async fn update_calendar(
    db: &DatabaseConnection,
    calendar_id: &str,
    user_id: &str,
    req: &CreateCalendarRequest,
) -> Result<(), DbErr> {
    Calendar::update_many()
        .filter(calendar::Column::Id.eq(calendar_id))
        .filter(calendar::Column::UserId.eq(user_id))
        .col_expr(calendar::Column::Name, Expr::value(req.name.clone()))
        .col_expr(calendar::Column::Timezone, Expr::value(req.timezone.clone()))
        .col_expr(calendar::Column::Duration, Expr::value(req.duration))
        .col_expr(calendar::Column::Buffer, Expr::value(req.buffer))
        .exec(db)
        .await?;
    
    Ok(())
}

pub async fn delete_calendar(db: &DatabaseConnection, calendar_id: &str, user_id: &str) -> Result<(), DbErr> {
    // Delete working hours first
    CalendarWorkingHours::delete_many()
        .filter(calendar_working_hours::Column::CalendarId.eq(calendar_id))
        .exec(db)
        .await?;
    
    // Delete bookings
    Booking::delete_many()
        .filter(booking::Column::CalendarId.eq(calendar_id))
        .exec(db)
        .await?;
    
    // Delete calendar
    Calendar::delete_many()
        .filter(calendar::Column::Id.eq(calendar_id))
        .filter(calendar::Column::UserId.eq(user_id))
        .exec(db)
        .await?;
    
    Ok(())
}

// Working hours operations
pub async fn get_working_hours(db: &DatabaseConnection, calendar_id: &str) -> Result<Vec<calendar_working_hours::Model>, DbErr> {
    CalendarWorkingHours::find()
        .filter(calendar_working_hours::Column::CalendarId.eq(calendar_id))
        .order_by_asc(calendar_working_hours::Column::DayOfWeek)
        .all(db)
        .await
}

pub async fn update_working_hours(
    db: &DatabaseConnection,
    calendar_id: &str,
    user_id: &str,
    times: &[WorkingTime],
) -> Result<(), DbErr> {
    // Verify calendar ownership
    let _calendar = Calendar::find_by_id(calendar_id)
        .filter(calendar::Column::UserId.eq(user_id))
        .one(db)
        .await?
        .ok_or(DbErr::RecordNotFound("Calendar not found".to_string()))?;
    
    // Update each day
    for time in times {
        CalendarWorkingHours::update_many()
            .filter(calendar_working_hours::Column::CalendarId.eq(calendar_id))
            .filter(calendar_working_hours::Column::DayOfWeek.eq(time.day))
            .col_expr(calendar_working_hours::Column::StartTime, Expr::value(time.start.clone()))
            .col_expr(calendar_working_hours::Column::EndTime, Expr::value(time.end.clone()))
            .col_expr(calendar_working_hours::Column::Enabled, Expr::value(true))
            .exec(db)
            .await?;
    }
    
    Ok(())
}

// Booking operations
pub async fn get_available_slots(db: &DatabaseConnection, slug: &str, date: &str) -> Result<Vec<String>, DbErr> {
    // Get calendar
    let calendar = Calendar::find()
        .filter(calendar::Column::Slug.eq(slug))
        .one(db)
        .await?
        .ok_or(DbErr::RecordNotFound("Calendar not found".to_string()))?;
    
    // Get day of week
    let day_of_week = get_day_of_week(date).map_err(|e| DbErr::Custom(e))?;
    
    // Get working hours for this day
    let working_hours = CalendarWorkingHours::find()
        .filter(calendar_working_hours::Column::CalendarId.eq(&calendar.id))
        .filter(calendar_working_hours::Column::DayOfWeek.eq(day_of_week))
        .one(db)
        .await?;
    
    let Some(wh) = working_hours else {
        return Ok(Vec::new());
    };
    
    if !wh.enabled {
        return Ok(Vec::new());
    }
    
    // Get existing bookings for this date
    let date_start = format!("{}T00:00:00Z", date);
    let date_end = format!("{}T23:59:59Z", date);
    
    let bookings = Booking::find()
        .filter(booking::Column::CalendarId.eq(&calendar.id))
        .filter(booking::Column::StartTime.gte(chrono::DateTime::parse_from_rfc3339(&date_start).unwrap().with_timezone(&chrono::Utc)))
        .filter(booking::Column::StartTime.lt(chrono::DateTime::parse_from_rfc3339(&date_end).unwrap().with_timezone(&chrono::Utc)))
        .all(db)
        .await?;
    
    let booked_times: Vec<String> = bookings
        .into_iter()
        .map(|b| b.start_time.format("%H:%M").to_string())
        .collect();
    
    // Generate available slots using shared logic
    let slots = generate_time_slots(
        &wh.start_time,
        &wh.end_time,
        calendar.duration,
        calendar.buffer,
        date,
        &booked_times,
    );
    
    Ok(slots)
}

pub async fn create_booking(
    db: &DatabaseConnection,
    slug: &str,
    req: &BookingRequest,
) -> Result<booking::Model, DbErr> {
    // Get calendar
    let calendar = Calendar::find()
        .filter(calendar::Column::Slug.eq(slug))
        .one(db)
        .await?
        .ok_or(DbErr::RecordNotFound("Calendar not found".to_string()))?;
    
    // Parse slot time
    let (start_time_str, end_time_str) = parse_booking_slot(&req.slot, calendar.duration)
        .map_err(|e| DbErr::Custom(e))?;
    
    let start_time = chrono::DateTime::parse_from_rfc3339(&format!("{}Z", start_time_str))
        .map_err(|e| DbErr::Custom(format!("Invalid start time: {}", e)))?
        .with_timezone(&chrono::Utc);
    
    let end_time = chrono::DateTime::parse_from_rfc3339(&format!("{}Z", end_time_str))
        .map_err(|e| DbErr::Custom(format!("Invalid end time: {}", e)))?
        .with_timezone(&chrono::Utc);
    
    // Check if slot is still available
    let existing = Booking::find()
        .filter(booking::Column::CalendarId.eq(&calendar.id))
        .filter(booking::Column::StartTime.eq(start_time))
        .one(db)
        .await?;
    
    if existing.is_some() {
        return Err(DbErr::RecordNotFound("Slot already booked".to_string()));
    }
    
    let booking_id = uuid::Uuid::new_v4().to_string();
    
    // Create booking
    let booking = booking::ActiveModel {
        id: Set(booking_id.clone()),
        calendar_id: Set(calendar.id),
        name: Set(req.name.clone()),
        email: Set(req.email.clone()),
        notes: Set(req.notes.clone()),
        start_time: Set(start_time),
        end_time: Set(end_time),
        created_at: Set(chrono::Utc::now()),
    };
    
    Booking::insert(booking).exec(db).await?;
    
    Booking::find_by_id(&booking_id)
        .one(db)
        .await?
        .ok_or(DbErr::RecordNotFound("Booking not found after creation".to_string()))
}