use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Calendars::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Calendars::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Calendars::UserId).integer().not_null())
                    .col(ColumnDef::new(Calendars::Name).string().not_null())
                    .col(
                        ColumnDef::new(Calendars::Slug)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Calendars::Description).text())
                    .col(ColumnDef::new(Calendars::Timezone).string().not_null())
                    .col(ColumnDef::new(Calendars::IsActive).boolean().not_null().default(true))
                    .col(ColumnDef::new(Calendars::BookingBufferMinutes).integer().not_null().default(15))
                    .col(ColumnDef::new(Calendars::MaxBookingDaysAhead).integer().not_null().default(60))
                    .col(ColumnDef::new(Calendars::MinBookingNoticeHours).integer().not_null().default(1))
                    .col(
                        ColumnDef::new(Calendars::CreatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Calendars::UpdatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_calendars_user_id")
                            .from(Calendars::Table, Calendars::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Calendars::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Calendars {
    Table,
    Id,
    UserId,
    Name,
    Slug,
    Description,
    Timezone,
    IsActive,
    BookingBufferMinutes,
    MaxBookingDaysAhead,
    MinBookingNoticeHours,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}