use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "bookings")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub calendar_id: i32,
    pub external_event_id: Option<String>, // ID from Google Calendar, etc.
    pub guest_email: String,
    pub guest_name: String,
    pub title: String,
    pub description: Option<String>,
    pub start_time: DateTime,
    pub end_time: DateTime,
    pub status: String, // "pending", "confirmed", "cancelled"
    pub meeting_link: Option<String>,
    pub cancelled_at: Option<DateTime>,
    pub cancellation_reason: Option<String>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::calendar::Entity",
        from = "Column::CalendarId",
        to = "super::calendar::Column::Id"
    )]
    Calendar,
}

impl Related<super::calendar::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Calendar.def()
    }
}

impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        use sea_orm::ActiveValue::Set;
        let now = chrono::Utc::now().naive_utc();
        Self {
            status: Set("pending".to_string()),
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