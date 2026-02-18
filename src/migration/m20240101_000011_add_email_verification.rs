use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            "ALTER TABLE users ADD COLUMN email_verified BOOLEAN NOT NULL DEFAULT FALSE",
        )
        .await?;

        db.execute_unprepared(
            "ALTER TABLE users ADD COLUMN email_verification_token VARCHAR(255) NULL",
        )
        .await?;

        db.execute_unprepared(
            "ALTER TABLE users ADD COLUMN email_verification_expires TIMESTAMP NULL",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared("ALTER TABLE users DROP COLUMN IF EXISTS email_verified")
            .await?;
        db.execute_unprepared("ALTER TABLE users DROP COLUMN IF EXISTS email_verification_token")
            .await?;
        db.execute_unprepared("ALTER TABLE users DROP COLUMN IF EXISTS email_verification_expires")
            .await?;

        Ok(())
    }
}
