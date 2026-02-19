use sea_orm_migration::prelude::*;

mod m20240101_000001_create_users_table;
mod m20240101_000002_create_forums_table;
mod m20240101_000003_create_posts_table;
mod m20240101_000004_create_comments_table;
mod m20240101_000005_create_votes_table;
mod m20240101_000006_add_comment_parent_index;
mod m20240101_000007_add_search_index;
mod m20240101_000008_create_notifications_table;
mod m20240101_000009_create_reports_table;
mod m20240101_000010_add_hidden_columns;
mod m20240101_000011_add_email_verification;
mod m20240101_000012_create_bookmarks_table;
mod m20240101_000013_create_follows_table;
mod m20240101_000014_create_tags_tables;
mod m20240101_000015_add_password_reset;
mod m20240101_000016_create_refresh_tokens;
mod m20240101_000017_add_performance_indexes;
mod m20260219_000001_create_user_points_ledger;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_000001_create_users_table::Migration),
            Box::new(m20240101_000002_create_forums_table::Migration),
            Box::new(m20240101_000003_create_posts_table::Migration),
            Box::new(m20240101_000004_create_comments_table::Migration),
            Box::new(m20240101_000005_create_votes_table::Migration),
            Box::new(m20240101_000006_add_comment_parent_index::Migration),
            Box::new(m20240101_000007_add_search_index::Migration),
            Box::new(m20240101_000008_create_notifications_table::Migration),
            Box::new(m20240101_000009_create_reports_table::Migration),
            Box::new(m20240101_000010_add_hidden_columns::Migration),
            Box::new(m20240101_000011_add_email_verification::Migration),
            Box::new(m20240101_000012_create_bookmarks_table::Migration),
            Box::new(m20240101_000013_create_follows_table::Migration),
            Box::new(m20240101_000014_create_tags_tables::Migration),
            Box::new(m20240101_000015_add_password_reset::Migration),
            Box::new(m20240101_000016_create_refresh_tokens::Migration),
            Box::new(m20240101_000017_add_performance_indexes::Migration),
            Box::new(m20260219_000001_create_user_points_ledger::Migration),
        ]
    }
}
