use sea_orm_migration::{prelude::*, schema::*};

use crate::m20260927_000002_create_org_table::Org;
use crate::m20260928_000028_create_submission_table::Submission;

#[derive(DeriveMigrationName)]
pub struct Migration;

// Укомплектованість (01 §4, Етап 5 -- КВід/ІВС потребують цю схему, якої досі не було в жодному
// попередньому етапі). staffing_snapshot -- один "зріз" укомплектованості org на дату; staffing_
// metric -- окремі числа в ньому (text+CHECK, не Postgres ENUM -- migration/CLAUDE.md). Відсотки
// НЕ зберігаються (01 §4: "рахуються при виведенні") -- лише сирі числа.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(StaffingSnapshot::Table)
                    .if_not_exists()
                    .col(pk_auto(StaffingSnapshot::Id))
                    .col(integer(StaffingSnapshot::OrgId))
                    .col(ColumnDef::new(StaffingSnapshot::AsOf).date().not_null())
                    .col(
                        ColumnDef::new(StaffingSnapshot::Category)
                            .string()
                            .not_null()
                            .check(Expr::col(StaffingSnapshot::Category).is_in([
                                "squad_leaders",
                                "instructors",
                            ])),
                    )
                    .col(integer_null(StaffingSnapshot::SubmissionId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-staffing_snapshot-org_id")
                            .from(StaffingSnapshot::Table, StaffingSnapshot::OrgId)
                            .to(Org::Table, Org::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-staffing_snapshot-submission_id")
                            .from(StaffingSnapshot::Table, StaffingSnapshot::SubmissionId)
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
                    .name("idx-staffing_snapshot-org_id-category-as_of")
                    .table(StaffingSnapshot::Table)
                    .col(StaffingSnapshot::OrgId)
                    .col(StaffingSnapshot::Category)
                    .col(StaffingSnapshot::AsOf)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(StaffingMetric::Table)
                    .if_not_exists()
                    .col(pk_auto(StaffingMetric::Id))
                    .col(integer(StaffingMetric::SnapshotId))
                    .col(
                        ColumnDef::new(StaffingMetric::Metric)
                            .string()
                            .not_null()
                            .check(Expr::col(StaffingMetric::Metric).is_in([
                                "by_tos",
                                "by_list",
                                "present",
                                "trained_sergeant",
                                "trained_kibr",
                                "in_training",
                                "planned_next_month",
                                "need_training",
                            ])),
                    )
                    .col(integer(StaffingMetric::Value))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-staffing_metric-snapshot_id")
                            .from(StaffingMetric::Table, StaffingMetric::SnapshotId)
                            .to(StaffingSnapshot::Table, StaffingSnapshot::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-staffing_metric-snapshot_id")
                    .table(StaffingMetric::Table)
                    .col(StaffingMetric::SnapshotId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(StaffingMetric::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(StaffingSnapshot::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub enum StaffingSnapshot {
    Table,
    Id,
    OrgId,
    AsOf,
    Category,
    SubmissionId,
}

#[derive(DeriveIden)]
pub enum StaffingMetric {
    Table,
    Id,
    SnapshotId,
    Metric,
    Value,
}
