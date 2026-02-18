use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Notifications {
    Table,
    Id,
    UserId,
    Kind,
    ActorId,
    TargetType,
    TargetId,
    Message,
    IsRead,
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
                    .table(Notifications::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Notifications::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Notifications::UserId).integer().not_null())
                    .col(
                        ColumnDef::new(Notifications::Kind)
                            .string_len(50)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Notifications::ActorId).integer().not_null())
                    .col(
                        ColumnDef::new(Notifications::TargetType)
                            .string_len(20)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Notifications::TargetId).integer().not_null())
                    .col(ColumnDef::new(Notifications::Message).text().not_null())
                    .col(
                        ColumnDef::new(Notifications::IsRead)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Notifications::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_notifications_user_id")
                            .from(Notifications::Table, Notifications::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_notifications_actor_id")
                            .from(Notifications::Table, Notifications::ActorId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_notifications_user_id")
                    .table(Notifications::Table)
                    .col(Notifications::UserId)
                    .to_owned(),
            )
            .await?;

        // Partial index for unread notifications
        let db = manager.get_connection();
        db.execute_unprepared(
            "CREATE INDEX idx_notifications_unread ON notifications (user_id, is_read) WHERE is_read = FALSE",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Notifications::Table).to_owned())
            .await
    }
}
