use sea_orm_migration::{prelude::*, schema::*};

use crate::m20260927_000002_create_org_table::Org;
use crate::m20260928_000025_create_training_group_table::TrainingGroup;

#[derive(DeriveMigrationName)]
pub struct Migration;

// Етап 8, зріз 1 (04 §4). Повний набір статусів зі спеки одразу (дешево додати зараз, дорого
// мігрувати CHECK пізніше — той самий урок, що generated_document.kind, m20260930_000038) --
// але УЖИВАЄМО зараз лише open/resolved (.claude/decisions/
// etap8-horizontal-reconciliation-first-slice.md, "workflow розбіжностей" відкладено).
// group_id nullable -- майбутні kind (vertical/temporal) можуть стосуватись діапазону груп
// органу, не однієї конкретної.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Discrepancy::Table)
                    .if_not_exists()
                    .col(pk_auto(Discrepancy::Id))
                    .col(
                        ColumnDef::new(Discrepancy::Kind)
                            .string()
                            .not_null()
                            .check(Expr::col(Discrepancy::Kind).is_in([
                                "horizontal",
                                "vertical",
                                "temporal",
                                "duplicate",
                                "data_quality",
                            ])),
                    )
                    .col(integer(Discrepancy::OrgId))
                    .col(integer_null(Discrepancy::GroupId))
                    .col(ColumnDef::new(Discrepancy::AsOf).date().not_null())
                    .col(string(Discrepancy::Metric))
                    .col(ColumnDef::new(Discrepancy::Values).json_binary().not_null())
                    .col(
                        ColumnDef::new(Discrepancy::Status)
                            .string()
                            .not_null()
                            .default("open")
                            .check(Expr::col(Discrepancy::Status).is_in([
                                "open",
                                "notified",
                                "in_progress",
                                "resolved",
                                "dismissed",
                            ])),
                    )
                    .col(text_null(Discrepancy::ResolutionNote))
                    .col(
                        ColumnDef::new(Discrepancy::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Discrepancy::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-discrepancy-org_id")
                            .from(Discrepancy::Table, Discrepancy::OrgId)
                            .to(Org::Table, Org::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-discrepancy-group_id")
                            .from(Discrepancy::Table, Discrepancy::GroupId)
                            .to(TrainingGroup::Table, TrainingGroup::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-discrepancy-org_id-status")
                    .table(Discrepancy::Table)
                    .col(Discrepancy::OrgId)
                    .col(Discrepancy::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-discrepancy-group_id-kind")
                    .table(Discrepancy::Table)
                    .col(Discrepancy::GroupId)
                    .col(Discrepancy::Kind)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Discrepancy::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub enum Discrepancy {
    Table,
    Id,
    Kind,
    OrgId,
    GroupId,
    AsOf,
    Metric,
    Values,
    Status,
    ResolutionNote,
    CreatedAt,
    UpdatedAt,
}
