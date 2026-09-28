use sea_orm_migration::{prelude::*, schema::*};

use crate::m20260928_000026_create_group_event_table::GroupEvent;
use crate::m20260928_000028_create_submission_table::Submission;

#[derive(DeriveMigrationName)]
pub struct Migration;

// group_event.submission_id (01 §3) -- nullable: seed/archive-перенесені події не завжди мають
// подання (Етап 6); події з форми (Етап 4) -- завжди мають.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(GroupEvent::Table)
                    .add_column(integer_null(GroupEvent::SubmissionId))
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk-group_event-submission_id")
                    .from(GroupEvent::Table, GroupEvent::SubmissionId)
                    .to(Submission::Table, Submission::Id)
                    .on_delete(ForeignKeyAction::Restrict)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk-group_event-submission_id")
                    .table(GroupEvent::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(GroupEvent::Table)
                    .drop_column(GroupEvent::SubmissionId)
                    .to_owned(),
            )
            .await
    }
}
