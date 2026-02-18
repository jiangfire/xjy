use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Add generated tsvector column for full-text search
        db.execute_unprepared(
            "ALTER TABLE posts ADD COLUMN search_vector tsvector \
             GENERATED ALWAYS AS (\
                 to_tsvector('english', coalesce(title, '') || ' ' || coalesce(content, ''))\
             ) STORED",
        )
        .await?;

        // Create GIN index for fast full-text search
        db.execute_unprepared("CREATE INDEX idx_posts_search ON posts USING GIN (search_vector)")
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared("DROP INDEX IF EXISTS idx_posts_search")
            .await?;

        db.execute_unprepared("ALTER TABLE posts DROP COLUMN IF EXISTS search_vector")
            .await?;

        Ok(())
    }
}
