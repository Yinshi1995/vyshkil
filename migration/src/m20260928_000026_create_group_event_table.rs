use sea_orm_migration::{prelude::*, schema::*};

use crate::m20260928_000021_create_attrition_reason_table::AttritionReason;
use crate::m20260928_000025_create_training_group_table::TrainingGroup;

#[derive(DeriveMigrationName)]
pub struct Migration;

// Event sourcing (01 §3): кількості НЕ зберігаються полями групи -- лише подіями. Воронка:
// planned -> arrived -> started (+added) -> (-attrition) -> completed -> vos_awarded/vos_not_awarded.
// submission_id навмисно відсутній зараз (submission -- окремий крок, 04); додасться колонкою
// пізніше, коли з'явиться таблиця submission.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(GroupEvent::Table)
                    .if_not_exists()
                    .col(pk_auto(GroupEvent::Id))
                    .col(integer(GroupEvent::GroupId))
                    .col(
                        ColumnDef::new(GroupEvent::EventType)
                            .string()
                            .not_null()
                            .check(Expr::col(GroupEvent::EventType).is_in([
                                "planned",
                                "arrived",
                                "started",
                                "added",
                                "attrition",
                                "completed",
                                "vos_awarded",
                                "vos_not_awarded",
                                "correction",
                            ])),
                    )
                    // count > 0 для всіх типів, окрім correction (там знак значущий -- 01 §3).
                    .col(
                        ColumnDef::new(GroupEvent::Count)
                            .integer()
                            .not_null()
                            .check(
                                Expr::col(GroupEvent::EventType)
                                    .eq("correction")
                                    .or(Expr::col(GroupEvent::Count).gt(0)),
                            ),
                    )
                    .col(integer_null(GroupEvent::ReasonId))
                    .col(ColumnDef::new(GroupEvent::OccurredOn).date().not_null())
                    // recorded_at -- бітемпоральність ("як ми знали на момент T", 01 §3).
                    .col(
                        ColumnDef::new(GroupEvent::RecordedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(string_null(GroupEvent::AwardOrderNumber))
                    .col(ColumnDef::new(GroupEvent::AwardOrderDate).date().null())
                    // "по скороченню" -- стосується лише completed (01 §3).
                    .col(boolean(GroupEvent::Shortened).default(false))
                    .col(text_null(GroupEvent::Note))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-group_event-group_id")
                            .from(GroupEvent::Table, GroupEvent::GroupId)
                            .to(TrainingGroup::Table, TrainingGroup::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-group_event-reason_id")
                            .from(GroupEvent::Table, GroupEvent::ReasonId)
                            .to(AttritionReason::Table, AttritionReason::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-group_event-group_id-occurred_on")
                    .table(GroupEvent::Table)
                    .col(GroupEvent::GroupId)
                    .col(GroupEvent::OccurredOn)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(GroupEvent::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub enum GroupEvent {
    Table,
    Id,
    GroupId,
    EventType,
    Count,
    ReasonId,
    OccurredOn,
    RecordedAt,
    AwardOrderNumber,
    AwardOrderDate,
    Shortened,
    Note,
}
