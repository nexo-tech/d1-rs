use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub email: String,
    pub password_hash: String,
    pub full_name: String,
    pub company_name: Option<String>,
    pub is_active: bool,
    pub is_verified: bool,
    pub verification_token: Option<String>,
    pub reset_token: Option<String>,
    pub reset_token_expires: Option<DateTime>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::calendar::Entity")]
    Calendars,
    #[sea_orm(has_many = "super::session::Entity")]
    Sessions,
}

impl Related<super::calendar::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Calendars.def()
    }
}

impl Related<super::session::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Sessions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        use sea_orm::ActiveValue::Set;
        let now = chrono::Utc::now().naive_utc();
        Self {
            is_active: Set(true),
            is_verified: Set(false),
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