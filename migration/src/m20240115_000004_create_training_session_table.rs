use sea_orm_migration::{prelude::*, schema::*};

use crate::m20240115_000001_create_unit_table::Unit;
use crate::m20240115_000003_create_exercise_table::Exercise;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TrainingSession::Table)
                    .if_not_exists()
                    .col(pk_auto(TrainingSession::Id))
                    .col(integer(TrainingSession::ExerciseId))
                    .col(integer(TrainingSession::UnitId))
                    .col(
                        ColumnDef::new(TrainingSession::ScheduledAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TrainingSession::ConductedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(string_null(TrainingSession::Location))
                    // status: planned/in_progress/completed/cancelled — життєвий цикл конкретного заняття/навчання.
                    .col(string(TrainingSession::Status).default("planned"))
                    .col(
                        ColumnDef::new(TrainingSession::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(TrainingSession::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-training_session-exercise_id")
                            .from(TrainingSession::Table, TrainingSession::ExerciseId)
                            .to(Exercise::Table, Exercise::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-training_session-unit_id")
                            .from(TrainingSession::Table, TrainingSession::UnitId)
                            .to(Unit::Table, Unit::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-training_session-unit_id-scheduled_at")
                    .table(TrainingSession::Table)
                    .col(TrainingSession::UnitId)
                    .col(TrainingSession::ScheduledAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(TrainingSession::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum TrainingSession {
    Table,
    Id,
    ExerciseId,
    UnitId,
    ScheduledAt,
    ConductedAt,
    Location,
    Status,
    CreatedAt,
    UpdatedAt,
}
