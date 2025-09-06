use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Tokens::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Tokens::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Tokens::UserEmail)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Tokens::AccessToken).string().not_null())
                    .col(ColumnDef::new(Tokens::RefreshToken).string().not_null())
                    .col(ColumnDef::new(Tokens::TokenType).string().not_null())
                    .col(ColumnDef::new(Tokens::Expiry).date_time().not_null())
                    .col(
                        ColumnDef::new(Tokens::CreatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Tokens::UpdatedAt)
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
            .drop_table(Table::drop().table(Tokens::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Tokens {
    Table,
    Id,
    UserEmail,
    AccessToken,
    RefreshToken,
    TokenType,
    Expiry,
    CreatedAt,
    UpdatedAt,
}