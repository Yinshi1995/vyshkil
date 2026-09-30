use sea_orm_migration::{prelude::*, schema::*};

use crate::m20260927_000002_create_org_table::Org;
use crate::m20260927_000004_create_training_site_table::TrainingSite;
use crate::m20260928_000012_create_training_kind_table::TrainingKind;
use crate::m20260928_000014_create_bzvp_program_table::BzvpProgram;
use crate::m20260928_000015_create_vos_table::Vos;
use crate::m20260928_000016_create_position_table::Position;
use crate::m20260928_000020_create_course_table::Course;
use crate::m20260928_000025_create_training_group_table::TrainingGroup;
use crate::m20260928_000028_create_submission_table::Submission;

#[derive(DeriveMigrationName)]
pub struct Migration;

// Етап 8, зріз 1 (04 §2, .claude/decisions/etap8-horizontal-reconciliation-first-slice.md):
// постійний НЕЗМІННИЙ знімок "що саме подала ця сесія" -- одна на кожен зафіксований рядок
// сітки (Ctrl+Enter), незалежно від того, з якою `training_group` (канонічною, дедупльованою)
// його зіставили. Без audit-тригера -- append-only лог подань (той самий привід, що
// `generated_document`, m20260929_000037): рядки лише створюються, ніколи не редагуються.
// Ті самі кількості (planned/arrived/in_training), яких НЕМА на training_group (канонічна група
// лишається "лише подіями" -- domain::counting), тут потрібні саме як РЯДОК: горизонтальна
// звірка порівнює числа МІЖ поданнями, не може читати їх назад із group_event (там усі числа
// вже змішані в один список подій без прив'язки "хто саме це сказав").
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ReportedGroup::Table)
                    .if_not_exists()
                    .col(pk_auto(ReportedGroup::Id))
                    .col(integer(ReportedGroup::SubmissionId))
                    .col(integer(ReportedGroup::SenderOrgId))
                    .col(integer(ReportedGroup::TrainingKindId))
                    .col(integer_null(ReportedGroup::BzvpProgramId))
                    .col(integer_null(ReportedGroup::VosId))
                    .col(integer_null(ReportedGroup::PositionId))
                    .col(integer_null(ReportedGroup::CourseId))
                    .col(integer(ReportedGroup::SiteId))
                    .col(integer_null(ReportedGroup::OrganizerOrgId))
                    .col(ColumnDef::new(ReportedGroup::PlannedStart).date().not_null())
                    .col(ColumnDef::new(ReportedGroup::PlannedEnd).date().not_null())
                    .col(text_null(ReportedGroup::EquipmentText))
                    .col(text_null(ReportedGroup::BasisDocNumber))
                    .col(ColumnDef::new(ReportedGroup::BasisDocDate).date().null())
                    .col(text_null(ReportedGroup::Note))
                    .col(integer(ReportedGroup::PlannedCount))
                    .col(integer(ReportedGroup::ArrivedCount))
                    .col(integer(ReportedGroup::InTrainingCount))
                    // NULL, поки зіставлення (Етап 8, зріз 1) не запущене на цьому рядку --
                    // насправді завжди заповнюється в тій самій транзакції, що й INSERT (репо
                    // пише обидва разом), nullable лише щоб не вигадувати "0 = ще нема групи".
                    .col(integer_null(ReportedGroup::MatchedGroupId))
                    .col(
                        ColumnDef::new(ReportedGroup::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-reported_group-submission_id")
                            .from(ReportedGroup::Table, ReportedGroup::SubmissionId)
                            .to(Submission::Table, Submission::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-reported_group-sender_org_id")
                            .from(ReportedGroup::Table, ReportedGroup::SenderOrgId)
                            .to(Org::Table, Org::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-reported_group-training_kind_id")
                            .from(ReportedGroup::Table, ReportedGroup::TrainingKindId)
                            .to(TrainingKind::Table, TrainingKind::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-reported_group-bzvp_program_id")
                            .from(ReportedGroup::Table, ReportedGroup::BzvpProgramId)
                            .to(BzvpProgram::Table, BzvpProgram::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-reported_group-vos_id")
                            .from(ReportedGroup::Table, ReportedGroup::VosId)
                            .to(Vos::Table, Vos::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-reported_group-position_id")
                            .from(ReportedGroup::Table, ReportedGroup::PositionId)
                            .to(Position::Table, Position::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-reported_group-course_id")
                            .from(ReportedGroup::Table, ReportedGroup::CourseId)
                            .to(Course::Table, Course::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-reported_group-site_id")
                            .from(ReportedGroup::Table, ReportedGroup::SiteId)
                            .to(TrainingSite::Table, TrainingSite::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-reported_group-organizer_org_id")
                            .from(ReportedGroup::Table, ReportedGroup::OrganizerOrgId)
                            .to(Org::Table, Org::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-reported_group-matched_group_id")
                            .from(ReportedGroup::Table, ReportedGroup::MatchedGroupId)
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
                    .name("idx-reported_group-matched_group_id")
                    .table(ReportedGroup::Table)
                    .col(ReportedGroup::MatchedGroupId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(ReportedGroup::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub enum ReportedGroup {
    Table,
    Id,
    SubmissionId,
    SenderOrgId,
    TrainingKindId,
    BzvpProgramId,
    VosId,
    PositionId,
    CourseId,
    SiteId,
    OrganizerOrgId,
    PlannedStart,
    PlannedEnd,
    EquipmentText,
    BasisDocNumber,
    BasisDocDate,
    Note,
    PlannedCount,
    ArrivedCount,
    InTrainingCount,
    MatchedGroupId,
    CreatedAt,
}
