use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Bookings::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Bookings::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Bookings::CalendarId).integer().not_null())
                    .col(ColumnDef::new(Bookings::ExternalEventId).string())
                    .col(ColumnDef::new(Bookings::GuestEmail).string().not_null())
                    .col(ColumnDef::new(Bookings::GuestName).string().not_null())
                    .col(ColumnDef::new(Bookings::Title).string().not_null())
                    .col(ColumnDef::new(Bookings::Description).text())
                    .col(ColumnDef::new(Bookings::StartTime).date_time().not_null())
                    .col(ColumnDef::new(Bookings::EndTime).date_time().not_null())
                    .col(ColumnDef::new(Bookings::Status).string().not_null().default("pending"))
                    .col(ColumnDef::new(Bookings::MeetingLink).string())
                    .col(ColumnDef::new(Bookings::CancelledAt).date_time())
                    .col(ColumnDef::new(Bookings::CancellationReason).text())
                    .col(
                        ColumnDef::new(Bookings::CreatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Bookings::UpdatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_bookings_calendar_id")
                            .from(Bookings::Table, Bookings::CalendarId)
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
            .drop_table(Table::drop().table(Bookings::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Bookings {
    Table,
    Id,
    CalendarId,
    ExternalEventId,
    GuestEmail,
    GuestName,
    Title,
    Description,
    StartTime,
    EndTime,
    Status,
    MeetingLink,
    CancelledAt,
    CancellationReason,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Calendars {
    Table,
    Id,
}