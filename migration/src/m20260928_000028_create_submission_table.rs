use sea_orm_migration::{prelude::*, schema::*};

use crate::m20260927_000002_create_org_table::Org;

#[derive(DeriveMigrationName)]
pub struct Migration;

// Мінімальний зріз для Етапу 4 (01 §5, 02 §6): лише те, що потрібно для чернетки форми
// (source_type='form') і фіксації в group_event. reported_group/reported_event (нормалізовані
// рядки "як подали", зіставлення) -- Етап 5, ще не існують. Реквізити документа/file_id --
// для table/official_letter/scan/archive_seed, для form лишаються null.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Submission::Table)
                    .if_not_exists()
                    .col(pk_auto(Submission::Id))
                    .col(
                        ColumnDef::new(Submission::SourceType)
                            .string()
                            .not_null()
                            .check(Expr::col(Submission::SourceType).is_in([
                                "form",
                                "table",
                                "official_letter",
                                "scan",
                                "archive_seed",
                            ])),
                    )
                    .col(integer(Submission::ReportingOrgId))
                    .col(ColumnDef::new(Submission::AsOfDate).date().not_null())
                    .col(
                        ColumnDef::new(Submission::Status)
                            .string()
                            .not_null()
                            .default("draft")
                            .check(Expr::col(Submission::Status).is_in([
                                "draft",
                                "parsed",
                                "validated",
                                "committed",
                                "rejected",
                            ])),
                    )
                    // Буфер автозбереження форми (02 §6) -- сирі рядки сітки до розбору/валідації.
                    // Не читається жодним звітом; очищається при переході в 'committed'.
                    .col(ColumnDef::new(Submission::DraftPayload).json_binary().null())
                    .col(integer_null(Submission::SupersedesId))
                    .col(
                        ColumnDef::new(Submission::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Submission::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-submission-reporting_org_id")
                            .from(Submission::Table, Submission::ReportingOrgId)
                            .to(Org::Table, Org::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-submission-supersedes_id")
                            .from(Submission::Table, Submission::SupersedesId)
                            .to(Submission::Table, Submission::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-submission-reporting_org_id-status")
                    .table(Submission::Table)
                    .col(Submission::ReportingOrgId)
                    .col(Submission::Status)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Submission::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub enum Submission {
    Table,
    Id,
    SourceType,
    ReportingOrgId,
    AsOfDate,
    Status,
    DraftPayload,
    SupersedesId,
    CreatedAt,
    UpdatedAt,
}
