use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum UserPointsLedger {
    Table,
    Id,
    UserId,
    Delta,
    Reason,
    RefType,
    RefId,
    ActorUserId,
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
                    .table(UserPointsLedger::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserPointsLedger::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(UserPointsLedger::UserId).integer().not_null())
                    .col(ColumnDef::new(UserPointsLedger::Delta).integer().not_null())
                    .col(ColumnDef::new(UserPointsLedger::Reason).string_len(64).not_null())
                    .col(ColumnDef::new(UserPointsLedger::RefType).string_len(20).not_null())
                    .col(ColumnDef::new(UserPointsLedger::RefId).integer().not_null())
                    .col(ColumnDef::new(UserPointsLedger::ActorUserId).integer().not_null())
                    .col(
                        ColumnDef::new(UserPointsLedger::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_points_ledger_user_id")
                            .from(UserPointsLedger::Table, UserPointsLedger::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_points_ledger_actor_user_id")
                            .from(UserPointsLedger::Table, UserPointsLedger::ActorUserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_user_points_ledger_user_created_at")
                    .table(UserPointsLedger::Table)
                    .col(UserPointsLedger::UserId)
                    .col(UserPointsLedger::CreatedAt)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_user_points_ledger_ref")
                    .table(UserPointsLedger::Table)
                    .col(UserPointsLedger::RefType)
                    .col(UserPointsLedger::RefId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserPointsLedger::Table).to_owned())
            .await
    }
}

