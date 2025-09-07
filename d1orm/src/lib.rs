pub use d1orm_derive::*;

pub mod db;
pub mod query;
pub mod entity;
pub mod migrations;
pub mod types;

pub use db::*;
pub use query::*;
pub use entity::*;
pub use migrations::*;
pub use types::*;

pub use async_trait::async_trait;
use std::fmt;
use worker::d1::D1Database;

#[derive(Debug)]
pub enum D1OrmError {
    Database(String),
    NotFound,
    ValidationError(String),
    SerializationError(String),
}

impl fmt::Display for D1OrmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            D1OrmError::Database(msg) => write!(f, "Database error: {}", msg),
            D1OrmError::NotFound => write!(f, "Entity not found"),
            D1OrmError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            D1OrmError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
        }
    }
}

impl std::error::Error for D1OrmError {}

pub type Result<T> = std::result::Result<T, D1OrmError>;

pub trait Entity: Sized + serde::Serialize + serde::de::DeserializeOwned {
    type PrimaryKey: Clone + fmt::Debug + serde::Serialize + serde::de::DeserializeOwned;
    type QueryBuilder: QueryBuilder<Self>;
    type CreateBuilder: CreateBuilder<Self>;
    type UpdateBuilder: UpdateBuilder<Self>;

    const TABLE_NAME: &'static str;
    
    fn primary_key(&self) -> &Self::PrimaryKey;
    
    fn query() -> Self::QueryBuilder;
    fn create() -> Self::CreateBuilder;
    fn update(key: Self::PrimaryKey) -> Self::UpdateBuilder;
    
    async fn find(db: &D1Client, key: Self::PrimaryKey) -> Result<Option<Self>>;
    async fn delete(db: &D1Client, key: Self::PrimaryKey) -> Result<()>;
}

pub trait QueryBuilder<T: Entity> {
    async fn all(self, db: &D1Client) -> Result<Vec<T>>;
    async fn first(self, db: &D1Client) -> Result<Option<T>>;
    async fn count(self, db: &D1Client) -> Result<i64>;
}

pub trait CreateBuilder<T: Entity> {
    async fn save(self, db: &D1Client) -> Result<T>;
}

pub trait UpdateBuilder<T: Entity> {
    async fn save(self, db: &D1Client) -> Result<T>;
}