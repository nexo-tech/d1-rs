use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "calendars")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Text")]
    pub id: String,
    #[sea_orm(column_type = "Text")]
    pub user_id: String,
    #[sea_orm(column_type = "Text")]
    pub name: String,
    #[sea_orm(column_type = "Text", unique)]
    pub slug: String,
    #[sea_orm(column_type = "Text")]
    pub timezone: String,
    pub duration: i32,
    pub buffer: i32,
    pub created_at: ChronoDateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "super::user::Entity", from = "Column::UserId", to = "super::user::Column::Id")]
    User,
    #[sea_orm(has_many = "super::calendar_working_hours::Entity")]
    CalendarWorkingHours,
    #[sea_orm(has_many = "super::booking::Entity")]
    Booking,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::calendar_working_hours::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CalendarWorkingHours.def()
    }
}

impl Related<super::booking::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Booking.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}