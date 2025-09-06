use anyhow::Result;
use chrono::{DateTime, Duration, NaiveTime, Utc, Weekday};
use sea_orm::*;
use serde::{Deserialize, Serialize};

use crate::models::{
    booking, calendar, calendar_working_hours,
    Booking, Calendar, CalendarWorkingHours
};
use crate::server::{CalendarService, OAuthService};

#[derive(Debug, Clone)]
pub struct BookingService {
    db: DatabaseConnection,
    oauth_service: OAuthService,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateBookingRequest {
    pub guest_name: String,
    pub guest_email: String,
    pub title: String,
    pub description: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub timezone: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimeSlot {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_minutes: i32,
    pub available: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AvailabilityRequest {
    pub date: chrono::NaiveDate,
    pub timezone: Option<String>,
    pub duration_minutes: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BookingResponse {
    pub id: i32,
    pub external_event_id: Option<String>,
    pub guest_name: String,
    pub guest_email: String,
    pub title: String,
    pub description: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub status: String,
    pub meeting_link: Option<String>,
    pub created_at: chrono::NaiveDateTime,
}

impl BookingService {
    pub fn new(db: DatabaseConnection, oauth_service: OAuthService) -> Self {
        Self { db, oauth_service }
    }

    pub async fn create_booking(&self, calendar_slug: &str, req: CreateBookingRequest) -> Result<BookingResponse> {
        // Get calendar by slug
        let calendar = Calendar::find()
            .filter(calendar::Column::Slug.eq(calendar_slug))
            .filter(calendar::Column::IsActive.eq(true))
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Calendar not found or inactive"))?;

        // Validate booking time constraints
        self.validate_booking_constraints(&calendar, &req).await?;

        // Check if slot is available
        if !self.is_slot_available(calendar.id, req.start_time, req.end_time).await? {
            return Err(anyhow::anyhow!("Time slot is not available"));
        }

        // Create booking in database
        let booking_model = booking::ActiveModel {
            calendar_id: ActiveValue::Set(calendar.id),
            guest_name: ActiveValue::Set(req.guest_name),
            guest_email: ActiveValue::Set(req.guest_email),
            title: ActiveValue::Set(req.title),
            description: ActiveValue::Set(req.description),
            start_time: ActiveValue::Set(req.start_time.naive_utc()),
            end_time: ActiveValue::Set(req.end_time.naive_utc()),
            status: ActiveValue::Set("pending".to_string()),
            ..Default::default()
        };

        let booking_result = Booking::insert(booking_model).exec(&self.db).await?;
        let booking_id = booking_result.last_insert_id;

        // Try to create event in external calendar (Google Calendar)
        let mut external_event_id = None;
        if let Ok(event_id) = self.oauth_service.create_calendar_event(
            calendar.id,
            &req.title,
            req.description.as_deref(),
            req.start_time,
            req.end_time,
            &req.guest_email,
        ).await {
            external_event_id = Some(event_id);
        }

        // Update booking with external event ID and confirm status
        let mut booking: booking::ActiveModel = Booking::find_by_id(booking_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Booking not found after creation"))?
            .into();

        booking.external_event_id = ActiveValue::Set(external_event_id.clone());
        booking.status = ActiveValue::Set("confirmed".to_string());
        let updated_booking = booking.update(&self.db).await?;

        Ok(BookingResponse {
            id: updated_booking.id,
            external_event_id: updated_booking.external_event_id,
            guest_name: updated_booking.guest_name,
            guest_email: updated_booking.guest_email,
            title: updated_booking.title,
            description: updated_booking.description,
            start_time: DateTime::from_utc(updated_booking.start_time, Utc),
            end_time: DateTime::from_utc(updated_booking.end_time, Utc),
            status: updated_booking.status,
            meeting_link: updated_booking.meeting_link,
            created_at: updated_booking.created_at,
        })
    }

    pub async fn get_available_slots(&self, calendar_slug: &str, req: AvailabilityRequest) -> Result<Vec<TimeSlot>> {
        // Get calendar
        let calendar = Calendar::find()
            .filter(calendar::Column::Slug.eq(calendar_slug))
            .filter(calendar::Column::IsActive.eq(true))
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Calendar not found or inactive"))?;

        let duration_minutes = req.duration_minutes.unwrap_or(30);
        let date = req.date;

        // Check if the date is within booking constraints
        let now = Utc::now().date_naive();
        let days_ahead = (date - now).num_days();
        
        if days_ahead < 0 {
            return Ok(vec![]); // Can't book in the past
        }
        
        if days_ahead > calendar.max_booking_days_ahead as i64 {
            return Ok(vec![]); // Too far in advance
        }

        // Get working hours for this day
        let weekday = date.weekday();
        let day_of_week = match weekday {
            Weekday::Sun => 0,
            Weekday::Mon => 1,
            Weekday::Tue => 2,
            Weekday::Wed => 3,
            Weekday::Thu => 4,
            Weekday::Fri => 5,
            Weekday::Sat => 6,
        };

        let working_hours = CalendarWorkingHours::find()
            .filter(calendar_working_hours::Column::CalendarId.eq(calendar.id))
            .filter(calendar_working_hours::Column::DayOfWeek.eq(day_of_week))
            .filter(calendar_working_hours::Column::IsActive.eq(true))
            .all(&self.db)
            .await?;

        if working_hours.is_empty() {
            return Ok(vec![]); // No working hours for this day
        }

        // Parse timezone
        let tz = chrono_tz::Tz::from_str_insensitive(req.timezone.as_deref().unwrap_or(&calendar.timezone))
            .unwrap_or(chrono_tz::UTC);

        let mut all_slots = Vec::new();

        // Generate slots for each working hour period
        for wh in working_hours {
            let start_time = NaiveTime::parse_from_str(&wh.start_time, "%H:%M")?;
            let end_time = NaiveTime::parse_from_str(&wh.end_time, "%H:%M")?;

            let start_datetime = tz.from_local_datetime(&date.and_time(start_time)).single()
                .ok_or_else(|| anyhow::anyhow!("Invalid start datetime"))?;
            let end_datetime = tz.from_local_datetime(&date.and_time(end_time)).single()
                .ok_or_else(|| anyhow::anyhow!("Invalid end datetime"))?;

            let mut current = start_datetime;
            
            while current + Duration::minutes(duration_minutes as i64) <= end_datetime {
                let slot_end = current + Duration::minutes(duration_minutes as i64);
                
                // Check if this slot meets minimum booking notice
                let now = Utc::now();
                let hours_ahead = (current.with_timezone(&Utc) - now).num_hours();
                
                if hours_ahead >= calendar.min_booking_notice_hours as i64 {
                    let is_available = self.is_slot_available(
                        calendar.id,
                        current.with_timezone(&Utc),
                        slot_end.with_timezone(&Utc),
                    ).await.unwrap_or(false);

                    all_slots.push(TimeSlot {
                        start_time: current.with_timezone(&Utc),
                        end_time: slot_end.with_timezone(&Utc),
                        duration_minutes,
                        available: is_available,
                    });
                }

                current = current + Duration::minutes(30); // Move by 30-minute increments
            }
        }

        // Sort slots by start time
        all_slots.sort_by(|a, b| a.start_time.cmp(&b.start_time));

        Ok(all_slots)
    }

    pub async fn cancel_booking(&self, booking_id: i32, reason: Option<String>) -> Result<()> {
        let booking = Booking::find_by_id(booking_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Booking not found"))?;

        if booking.status == "cancelled" {
            return Err(anyhow::anyhow!("Booking is already cancelled"));
        }

        // TODO: Cancel event in external calendar if external_event_id exists

        // Update booking status
        let mut booking_active: booking::ActiveModel = booking.into();
        booking_active.status = ActiveValue::Set("cancelled".to_string());
        booking_active.cancelled_at = ActiveValue::Set(Some(Utc::now().naive_utc()));
        booking_active.cancellation_reason = ActiveValue::Set(reason);
        booking_active.update(&self.db).await?;

        Ok(())
    }

    pub async fn get_booking(&self, booking_id: i32) -> Result<BookingResponse> {
        let booking = Booking::find_by_id(booking_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Booking not found"))?;

        Ok(BookingResponse {
            id: booking.id,
            external_event_id: booking.external_event_id,
            guest_name: booking.guest_name,
            guest_email: booking.guest_email,
            title: booking.title,
            description: booking.description,
            start_time: DateTime::from_utc(booking.start_time, Utc),
            end_time: DateTime::from_utc(booking.end_time, Utc),
            status: booking.status,
            meeting_link: booking.meeting_link,
            created_at: booking.created_at,
        })
    }

    async fn validate_booking_constraints(&self, calendar: &calendar::Model, req: &CreateBookingRequest) -> Result<()> {
        let now = Utc::now();
        
        // Check minimum notice period
        let hours_ahead = (req.start_time - now).num_hours();
        if hours_ahead < calendar.min_booking_notice_hours as i64 {
            return Err(anyhow::anyhow!("Booking requires at least {} hours notice", calendar.min_booking_notice_hours));
        }

        // Check maximum days ahead
        let days_ahead = (req.start_time.date_naive() - now.date_naive()).num_days();
        if days_ahead > calendar.max_booking_days_ahead as i64 {
            return Err(anyhow::anyhow!("Cannot book more than {} days in advance", calendar.max_booking_days_ahead));
        }

        // Check if booking time is in the past
        if req.start_time < now {
            return Err(anyhow::anyhow!("Cannot book appointments in the past"));
        }

        // Check if end time is after start time
        if req.end_time <= req.start_time {
            return Err(anyhow::anyhow!("End time must be after start time"));
        }

        Ok(())
    }

    async fn is_slot_available(&self, calendar_id: i32, start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> Result<bool> {
        // Check for existing bookings that overlap
        let overlapping_bookings = Booking::find()
            .filter(booking::Column::CalendarId.eq(calendar_id))
            .filter(booking::Column::Status.ne("cancelled"))
            .filter(
                Condition::any()
                    .add(
                        Condition::all()
                            .add(booking::Column::StartTime.lte(start_time.naive_utc()))
                            .add(booking::Column::EndTime.gt(start_time.naive_utc()))
                    )
                    .add(
                        Condition::all()
                            .add(booking::Column::StartTime.lt(end_time.naive_utc()))
                            .add(booking::Column::EndTime.gte(end_time.naive_utc()))
                    )
                    .add(
                        Condition::all()
                            .add(booking::Column::StartTime.gte(start_time.naive_utc()))
                            .add(booking::Column::EndTime.lte(end_time.naive_utc()))
                    )
            )
            .count(&self.db)
            .await?;

        if overlapping_bookings > 0 {
            return Ok(false);
        }

        // Check external calendar events (Google Calendar)
        if let Ok(events) = self.oauth_service.get_calendar_events(calendar_id, start_time, end_time).await {
            for event in events {
                if let (Some(event_start), Some(event_end)) = (
                    event.start.date_time.as_ref(),
                    event.end.date_time.as_ref(),
                ) {
                    if let (Ok(event_start_dt), Ok(event_end_dt)) = (
                        DateTime::parse_from_rfc3339(event_start),
                        DateTime::parse_from_rfc3339(event_end),
                    ) {
                        let event_start_utc = event_start_dt.with_timezone(&Utc);
                        let event_end_utc = event_end_dt.with_timezone(&Utc);

                        // Check for overlap
                        if start_time < event_end_utc && end_time > event_start_utc {
                            return Ok(false);
                        }
                    }
                }
            }
        }

        Ok(true)
    }
}