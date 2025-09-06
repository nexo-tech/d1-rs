use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "calendars")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub slug: String, // Unique URL slug for public calendar access
    pub description: Option<String>,
    pub timezone: String,
    pub is_active: bool,
    pub booking_buffer_minutes: i32, // Minutes between bookings
    pub max_booking_days_ahead: i32, // How far in advance can people book
    pub min_booking_notice_hours: i32, // Minimum notice required for bookings
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
    #[sea_orm(has_many = "super::calendar_token::Entity")]
    CalendarTokens,
    #[sea_orm(has_many = "super::calendar_working_hours::Entity")]
    WorkingHours,
    #[sea_orm(has_many = "super::booking::Entity")]
    Bookings,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::calendar_token::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CalendarTokens.def()
    }
}

impl Related<super::calendar_working_hours::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkingHours.def()
    }
}

impl Related<super::booking::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Bookings.def()
    }
}

impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        use sea_orm::ActiveValue::Set;
        let now = chrono::Utc::now().naive_utc();
        Self {
            is_active: Set(true),
            booking_buffer_minutes: Set(15),
            max_booking_days_ahead: Set(60),
            min_booking_notice_hours: Set(1),
            created_at: Set(now),
            updated_at: Set(now),
            ..::std::default::Default::default()
        }
    }

    fn before_save(mut self, _insert: bool) -> Result<Self, DbErr> {
        use sea_orm::ActiveValue::Set;
        self.updated_at = Set(chrono::Utc::now().naive_utc());
        Ok(self)
    }
}