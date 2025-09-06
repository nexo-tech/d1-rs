use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WorkingHours::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WorkingHours::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(WorkingHours::StartTime).string().not_null())
                    .col(ColumnDef::new(WorkingHours::EndTime).string().not_null())
                    .col(ColumnDef::new(WorkingHours::Timezone).string().not_null())
                    .col(ColumnDef::new(WorkingHours::WorkingDays).string().not_null())
                    .col(
                        ColumnDef::new(WorkingHours::CreatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(WorkingHours::UpdatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WorkingHours::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum WorkingHours {
    Table,
    Id,
    StartTime,
    EndTime,
    Timezone,
    WorkingDays,
    CreatedAt,
    UpdatedAt,
}