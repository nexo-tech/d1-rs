pub use sea_orm_migration::prelude::*;

pub struct Migrator;

mod m20240101_000001_create_tokens_table;
mod m20240101_000002_create_working_hours_table;
mod m20240101_000003_create_users_table;
mod m20240101_000004_create_sessions_table;
mod m20240101_000005_create_calendars_table;
mod m20240101_000006_create_calendar_tokens_table;
mod m20240101_000007_create_calendar_working_hours_table;
mod m20240101_000008_create_bookings_table;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_000001_create_tokens_table::Migration),
            Box::new(m20240101_000002_create_working_hours_table::Migration),
            Box::new(m20240101_000003_create_users_table::Migration),
            Box::new(m20240101_000004_create_sessions_table::Migration),
            Box::new(m20240101_000005_create_calendars_table::Migration),
            Box::new(m20240101_000006_create_calendar_tokens_table::Migration),
            Box::new(m20240101_000007_create_calendar_working_hours_table::Migration),
            Box::new(m20240101_000008_create_bookings_table::Migration),
        ]
    }
}