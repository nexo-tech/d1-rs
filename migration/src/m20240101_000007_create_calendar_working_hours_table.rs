use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CalendarWorkingHours::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CalendarWorkingHours::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CalendarWorkingHours::CalendarId).integer().not_null())
                    .col(ColumnDef::new(CalendarWorkingHours::DayOfWeek).integer().not_null())
                    .col(ColumnDef::new(CalendarWorkingHours::StartTime).string().not_null())
                    .col(ColumnDef::new(CalendarWorkingHours::EndTime).string().not_null())
                    .col(ColumnDef::new(CalendarWorkingHours::IsActive).boolean().not_null().default(true))
                    .col(
                        ColumnDef::new(CalendarWorkingHours::CreatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(CalendarWorkingHours::UpdatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_calendar_working_hours_calendar_id")
                            .from(CalendarWorkingHours::Table, CalendarWorkingHours::CalendarId)
                            .to(Calendars::Table, Calendars::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CalendarWorkingHours::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum CalendarWorkingHours {
    Table,
    Id,
    CalendarId,
    DayOfWeek,
    StartTime,
    EndTime,
    IsActive,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Calendars {
    Table,
    Id,
}