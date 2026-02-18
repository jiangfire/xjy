use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Forums {
    Table,
    Id,
    Name,
    Description,
    Slug,
    SortOrder,
    IconUrl,
    CreatedAt,
    UpdatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Forums::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Forums::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Forums::Name)
                            .string_len(100)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Forums::Description)
                            .string_len(500)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Forums::Slug)
                            .string_len(100)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Forums::SortOrder)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Forums::IconUrl).string_len(500).null())
                    .col(
                        ColumnDef::new(Forums::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Forums::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Forums::Table).to_owned())
            .await
    }
}
