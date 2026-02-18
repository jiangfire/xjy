use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_notifications_user_created
             ON notifications (user_id, created_at DESC)",
        )
        .await?;

        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_posts_forum_visible_created
             ON posts (forum_id, is_hidden, created_at DESC)",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared("DROP INDEX IF EXISTS idx_notifications_user_created")
            .await?;
        db.execute_unprepared("DROP INDEX IF EXISTS idx_posts_forum_visible_created")
            .await?;

        Ok(())
    }
}
