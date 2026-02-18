use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Votes {
    Table,
    Id,
    UserId,
    TargetType,
    TargetId,
    Value,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Votes::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Votes::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Votes::UserId).integer().not_null())
                    .col(ColumnDef::new(Votes::TargetType).string_len(20).not_null())
                    .col(ColumnDef::new(Votes::TargetId).integer().not_null())
                    .col(ColumnDef::new(Votes::Value).small_integer().not_null())
                    .col(
                        ColumnDef::new(Votes::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_votes_user_id")
                            .from(Votes::Table, Votes::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_votes_unique")
                    .table(Votes::Table)
                    .col(Votes::UserId)
                    .col(Votes::TargetType)
                    .col(Votes::TargetId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_votes_target")
                    .table(Votes::Table)
                    .col(Votes::TargetType)
                    .col(Votes::TargetId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Votes::Table).to_owned())
            .await
    }
}
