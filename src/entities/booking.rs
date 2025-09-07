use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "bookings")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Text")]
    pub id: String,
    #[sea_orm(column_type = "Text")]
    pub calendar_id: String,
    #[sea_orm(column_type = "Text")]
    pub name: String,
    #[sea_orm(column_type = "Text")]
    pub email: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub notes: Option<String>,
    pub start_time: ChronoDateTimeUtc,
    pub end_time: ChronoDateTimeUtc,
    pub created_at: ChronoDateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "super::calendar::Entity", from = "Column::CalendarId", to = "super::calendar::Column::Id")]
    Calendar,
}

impl Related<super::calendar::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Calendar.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}