use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "calendar_working_hours")]
pub struct Model {
    #[sea_orm(primary_key, column_type = "Text")]
    pub id: String,
    #[sea_orm(column_type = "Text")]
    pub calendar_id: String,
    pub day_of_week: i32, // 1=Monday, 7=Sunday
    #[sea_orm(column_type = "Text")]
    pub start_time: String, // "09:00"
    #[sea_orm(column_type = "Text")]
    pub end_time: String, // "17:00"
    pub enabled: bool,
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