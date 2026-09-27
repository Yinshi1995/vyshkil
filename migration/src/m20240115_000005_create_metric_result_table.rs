use sea_orm_migration::{prelude::*, schema::*};

use crate::m20240115_000002_create_personnel_table::Personnel;
use crate::m20240115_000004_create_training_session_table::TrainingSession;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(MetricResult::Table)
                    .if_not_exists()
                    .col(pk_auto(MetricResult::Id))
                    .col(integer(MetricResult::TrainingSessionId))
                    .col(integer(MetricResult::PersonnelId))
                    // metric_name лишаємо вільним рядком (не FK на довідник нормативів):
                    // перелік показників (норматив, час, влучність...) надто різнорідний під різні вправи для жорсткої схеми.
                    .col(string(MetricResult::MetricName))
                    .col(decimal_null(MetricResult::Value))
                    .col(string_null(MetricResult::UnitOfMeasure))
                    .col(boolean_null(MetricResult::PassFail))
                    .col(text_null(MetricResult::Notes))
                    .col(
                        ColumnDef::new(MetricResult::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-metric_result-training_session_id")
                            .from(MetricResult::Table, MetricResult::TrainingSessionId)
                            .to(TrainingSession::Table, TrainingSession::Id)
                            // Cascade: видалення заняття прибирає й виміряні по ньому результати — сирітських рядків бути не повинно.
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-metric_result-personnel_id")
                            .from(MetricResult::Table, MetricResult::PersonnelId)
                            .to(Personnel::Table, Personnel::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-metric_result-training_session_id")
                    .table(MetricResult::Table)
                    .col(MetricResult::TrainingSessionId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-metric_result-personnel_id")
                    .table(MetricResult::Table)
                    .col(MetricResult::PersonnelId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MetricResult::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum MetricResult {
    Table,
    Id,
    TrainingSessionId,
    PersonnelId,
    MetricName,
    Value,
    UnitOfMeasure,
    PassFail,
    Notes,
    CreatedAt,
}
