use anyhow::Result;
use sea_orm::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{
    calendar, calendar_working_hours, calendar_token, booking,
    Calendar, CalendarWorkingHours, CalendarToken, Booking
};

#[derive(Debug, Clone)]
pub struct CalendarService {
    db: DatabaseConnection,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCalendarRequest {
    pub name: String,
    pub description: Option<String>,
    pub timezone: String,
    pub booking_buffer_minutes: Option<i32>,
    pub max_booking_days_ahead: Option<i32>,
    pub min_booking_notice_hours: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateCalendarRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub timezone: Option<String>,
    pub is_active: Option<bool>,
    pub booking_buffer_minutes: Option<i32>,
    pub max_booking_days_ahead: Option<i32>,
    pub min_booking_notice_hours: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkingHoursRequest {
    pub day_of_week: i32, // 0 = Sunday, 6 = Saturday
    pub start_time: String, // "09:00"
    pub end_time: String,   // "17:00"
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CalendarResponse {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub timezone: String,
    pub is_active: bool,
    pub booking_buffer_minutes: i32,
    pub max_booking_days_ahead: i32,
    pub min_booking_notice_hours: i32,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub working_hours: Vec<WorkingHoursResponse>,
    pub connected_providers: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkingHoursResponse {
    pub id: i32,
    pub day_of_week: i32,
    pub start_time: String,
    pub end_time: String,
    pub is_active: bool,
}

impl CalendarService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn create_calendar(&self, user_id: i32, req: CreateCalendarRequest) -> Result<CalendarResponse> {
        let slug = self.generate_unique_slug(&req.name).await?;

        let calendar_active_model = calendar::ActiveModel {
            user_id: ActiveValue::Set(user_id),
            name: ActiveValue::Set(req.name),
            slug: ActiveValue::Set(slug),
            description: ActiveValue::Set(req.description),
            timezone: ActiveValue::Set(req.timezone),
            booking_buffer_minutes: ActiveValue::Set(req.booking_buffer_minutes.unwrap_or(15)),
            max_booking_days_ahead: ActiveValue::Set(req.max_booking_days_ahead.unwrap_or(60)),
            min_booking_notice_hours: ActiveValue::Set(req.min_booking_notice_hours.unwrap_or(1)),
            ..Default::default()
        };

        let calendar = Calendar::insert(calendar_active_model).exec(&self.db).await?;

        // Create default working hours (Monday to Friday, 9 AM to 5 PM)
        for day in 1..=5 {
            let working_hours = calendar_working_hours::ActiveModel {
                calendar_id: ActiveValue::Set(calendar.last_insert_id),
                day_of_week: ActiveValue::Set(day),
                start_time: ActiveValue::Set("09:00".to_string()),
                end_time: ActiveValue::Set("17:00".to_string()),
                ..Default::default()
            };
            CalendarWorkingHours::insert(working_hours).exec(&self.db).await?;
        }

        self.get_calendar_by_id(user_id, calendar.last_insert_id).await
    }

    pub async fn get_user_calendars(&self, user_id: i32) -> Result<Vec<CalendarResponse>> {
        let calendars = Calendar::find()
            .filter(calendar::Column::UserId.eq(user_id))
            .order_by_desc(calendar::Column::CreatedAt)
            .all(&self.db)
            .await?;

        let mut results = Vec::new();
        for calendar in calendars {
            let calendar_response = self.build_calendar_response(calendar).await?;
            results.push(calendar_response);
        }

        Ok(results)
    }

    pub async fn get_calendar_by_id(&self, user_id: i32, calendar_id: i32) -> Result<CalendarResponse> {
        let calendar = Calendar::find_by_id(calendar_id)
            .filter(calendar::Column::UserId.eq(user_id))
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Calendar not found"))?;

        self.build_calendar_response(calendar).await
    }

    pub async fn get_calendar_by_slug(&self, slug: &str) -> Result<CalendarResponse> {
        let calendar = Calendar::find()
            .filter(calendar::Column::Slug.eq(slug))
            .filter(calendar::Column::IsActive.eq(true))
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Calendar not found"))?;

        self.build_calendar_response(calendar).await
    }

    pub async fn update_calendar(&self, user_id: i32, calendar_id: i32, req: UpdateCalendarRequest) -> Result<CalendarResponse> {
        let mut calendar: calendar::ActiveModel = Calendar::find_by_id(calendar_id)
            .filter(calendar::Column::UserId.eq(user_id))
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Calendar not found"))?
            .into();

        if let Some(name) = req.name {
            calendar.name = ActiveValue::Set(name);
        }
        if let Some(description) = req.description {
            calendar.description = ActiveValue::Set(Some(description));
        }
        if let Some(timezone) = req.timezone {
            calendar.timezone = ActiveValue::Set(timezone);
        }
        if let Some(is_active) = req.is_active {
            calendar.is_active = ActiveValue::Set(is_active);
        }
        if let Some(buffer) = req.booking_buffer_minutes {
            calendar.booking_buffer_minutes = ActiveValue::Set(buffer);
        }
        if let Some(max_days) = req.max_booking_days_ahead {
            calendar.max_booking_days_ahead = ActiveValue::Set(max_days);
        }
        if let Some(min_hours) = req.min_booking_notice_hours {
            calendar.min_booking_notice_hours = ActiveValue::Set(min_hours);
        }

        calendar.update(&self.db).await?;
        self.get_calendar_by_id(user_id, calendar_id).await
    }

    pub async fn delete_calendar(&self, user_id: i32, calendar_id: i32) -> Result<()> {
        let calendar = Calendar::find_by_id(calendar_id)
            .filter(calendar::Column::UserId.eq(user_id))
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Calendar not found"))?;

        Calendar::delete_by_id(calendar.id).exec(&self.db).await?;
        Ok(())
    }

    pub async fn set_working_hours(&self, user_id: i32, calendar_id: i32, working_hours: Vec<WorkingHoursRequest>) -> Result<()> {
        // Verify calendar ownership
        Calendar::find_by_id(calendar_id)
            .filter(calendar::Column::UserId.eq(user_id))
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Calendar not found"))?;

        // Delete existing working hours
        CalendarWorkingHours::delete_many()
            .filter(calendar_working_hours::Column::CalendarId.eq(calendar_id))
            .exec(&self.db)
            .await?;

        // Insert new working hours
        for wh in working_hours {
            let working_hours_model = calendar_working_hours::ActiveModel {
                calendar_id: ActiveValue::Set(calendar_id),
                day_of_week: ActiveValue::Set(wh.day_of_week),
                start_time: ActiveValue::Set(wh.start_time),
                end_time: ActiveValue::Set(wh.end_time),
                is_active: ActiveValue::Set(wh.is_active),
                ..Default::default()
            };
            CalendarWorkingHours::insert(working_hours_model).exec(&self.db).await?;
        }

        Ok(())
    }

    pub async fn get_calendar_bookings(&self, user_id: i32, calendar_id: i32, limit: Option<u64>) -> Result<Vec<booking::Model>> {
        // Verify calendar ownership
        Calendar::find_by_id(calendar_id)
            .filter(calendar::Column::UserId.eq(user_id))
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Calendar not found"))?;

        let mut query = Booking::find()
            .filter(booking::Column::CalendarId.eq(calendar_id))
            .order_by_desc(booking::Column::StartTime);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        Ok(query.all(&self.db).await?)
    }

    async fn build_calendar_response(&self, calendar: calendar::Model) -> Result<CalendarResponse> {
        // Get working hours
        let working_hours = CalendarWorkingHours::find()
            .filter(calendar_working_hours::Column::CalendarId.eq(calendar.id))
            .order_by_asc(calendar_working_hours::Column::DayOfWeek)
            .all(&self.db)
            .await?
            .into_iter()
            .map(|wh| WorkingHoursResponse {
                id: wh.id,
                day_of_week: wh.day_of_week,
                start_time: wh.start_time,
                end_time: wh.end_time,
                is_active: wh.is_active,
            })
            .collect();

        // Get connected providers
        let providers = CalendarToken::find()
            .filter(calendar_token::Column::CalendarId.eq(calendar.id))
            .all(&self.db)
            .await?
            .into_iter()
            .map(|token| token.provider)
            .collect();

        Ok(CalendarResponse {
            id: calendar.id,
            name: calendar.name,
            slug: calendar.slug,
            description: calendar.description,
            timezone: calendar.timezone,
            is_active: calendar.is_active,
            booking_buffer_minutes: calendar.booking_buffer_minutes,
            max_booking_days_ahead: calendar.max_booking_days_ahead,
            min_booking_notice_hours: calendar.min_booking_notice_hours,
            created_at: calendar.created_at,
            updated_at: calendar.updated_at,
            working_hours,
            connected_providers: providers,
        })
    }

    async fn generate_unique_slug(&self, name: &str) -> Result<String> {
        let base_slug = name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string();

        let mut slug = base_slug.clone();
        let mut counter = 1;

        loop {
            let existing = Calendar::find()
                .filter(calendar::Column::Slug.eq(&slug))
                .one(&self.db)
                .await?;

            if existing.is_none() {
                break;
            }

            slug = format!("{}-{}", base_slug, counter);
            counter += 1;
        }

        Ok(slug)
    }
}