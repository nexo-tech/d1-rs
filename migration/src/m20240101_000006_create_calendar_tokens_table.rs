use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CalendarTokens::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CalendarTokens::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CalendarTokens::CalendarId).integer().not_null())
                    .col(ColumnDef::new(CalendarTokens::Provider).string().not_null())
                    .col(ColumnDef::new(CalendarTokens::ProviderEmail).string().not_null())
                    .col(ColumnDef::new(CalendarTokens::AccessToken).text().not_null())
                    .col(ColumnDef::new(CalendarTokens::RefreshToken).text().not_null())
                    .col(ColumnDef::new(CalendarTokens::TokenType).string().not_null())
                    .col(ColumnDef::new(CalendarTokens::Expiry).date_time().not_null())
                    .col(ColumnDef::new(CalendarTokens::Scopes).text())
                    .col(
                        ColumnDef::new(CalendarTokens::CreatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(CalendarTokens::UpdatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_calendar_tokens_calendar_id")
                            .from(CalendarTokens::Table, CalendarTokens::CalendarId)
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
            .drop_table(Table::drop().table(CalendarTokens::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum CalendarTokens {
    Table,
    Id,
    CalendarId,
    Provider,
    ProviderEmail,
    AccessToken,
    RefreshToken,
    TokenType,
    Expiry,
    Scopes,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Calendars {
    Table,
    Id,
}