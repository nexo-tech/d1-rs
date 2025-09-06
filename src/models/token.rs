use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "tokens")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub user_email: String,
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expiry: DateTime,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        use sea_orm::ActiveValue::Set;
        let now = chrono::Utc::now().naive_utc();
        Self {
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