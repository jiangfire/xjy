use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Reports {
    Table,
    Id,
    ReporterId,
    TargetType,
    TargetId,
    Reason,
    Description,
    Status,
    ResolvedBy,
    ResolvedAt,
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
                    .table(Reports::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Reports::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Reports::ReporterId).integer().not_null())
                    .col(
                        ColumnDef::new(Reports::TargetType)
                            .string_len(20)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Reports::TargetId).integer().not_null())
                    .col(ColumnDef::new(Reports::Reason).string_len(50).not_null())
                    .col(ColumnDef::new(Reports::Description).text().null())
                    .col(
                        ColumnDef::new(Reports::Status)
                            .string_len(20)
                            .not_null()
                            .default("pending"),
                    )
                    .col(ColumnDef::new(Reports::ResolvedBy).integer().null())
                    .col(ColumnDef::new(Reports::ResolvedAt).timestamp().null())
                    .col(
                        ColumnDef::new(Reports::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_reports_reporter_id")
                            .from(Reports::Table, Reports::ReporterId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_reports_resolved_by")
                            .from(Reports::Table, Reports::ResolvedBy)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_reports_status")
                    .table(Reports::Table)
                    .col(Reports::Status)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_reports_target")
                    .table(Reports::Table)
                    .col(Reports::TargetType)
                    .col(Reports::TargetId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_reports_unique")
                    .table(Reports::Table)
                    .col(Reports::ReporterId)
                    .col(Reports::TargetType)
                    .col(Reports::TargetId)
                    .unique()
                    .if_not_exists()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Reports::Table).to_owned())
            .await
    }
}
