use sea_orm_migration::{prelude::*, schema::*};

use crate::m20260927_000002_create_org_table::Org;
use crate::m20260927_000004_create_training_site_table::TrainingSite;
use crate::m20260928_000012_create_training_kind_table::TrainingKind;
use crate::m20260928_000014_create_bzvp_program_table::BzvpProgram;
use crate::m20260928_000015_create_vos_table::Vos;
use crate::m20260928_000016_create_position_table::Position;
use crate::m20260928_000020_create_course_table::Course;

#[derive(DeriveMigrationName)]
pub struct Migration;

// Мінімальний зріз Етапу 3 (01 §3): лише поля, потрібні, щоб мати канонічну групу й рахувати
// воронку. Без submission/reported_*/canonical_from (04, зіставлення подань) -- окремий крок
// пізніше; без group_composition/group_equipment/basis_doc/inflow_source -- не потрібні для
// критерію готовності (формули воронки + бітемпоральний тест).
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TrainingGroup::Table)
                    .if_not_exists()
                    .col(pk_auto(TrainingGroup::Id))
                    .col(integer(TrainingGroup::SenderOrgId))
                    .col(integer(TrainingGroup::TrainingKindId))
                    .col(integer_null(TrainingGroup::BzvpProgramId))
                    // vos/position/course -- усі nullable: інструкторські курси без ВОС,
                    // БЗВП без ВОС (01 §3).
                    .col(integer_null(TrainingGroup::VosId))
                    .col(integer_null(TrainingGroup::PositionId))
                    .col(integer_null(TrainingGroup::CourseId))
                    .col(integer(TrainingGroup::SiteId))
                    .col(integer_null(TrainingGroup::OrganizerOrgId))
                    .col(ColumnDef::new(TrainingGroup::PlannedStart).date().not_null())
                    .col(ColumnDef::new(TrainingGroup::PlannedEnd).date().not_null())
                    .col(text_null(TrainingGroup::Note))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-training_group-sender_org_id")
                            .from(TrainingGroup::Table, TrainingGroup::SenderOrgId)
                            .to(Org::Table, Org::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-training_group-training_kind_id")
                            .from(TrainingGroup::Table, TrainingGroup::TrainingKindId)
                            .to(TrainingKind::Table, TrainingKind::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-training_group-bzvp_program_id")
                            .from(TrainingGroup::Table, TrainingGroup::BzvpProgramId)
                            .to(BzvpProgram::Table, BzvpProgram::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-training_group-vos_id")
                            .from(TrainingGroup::Table, TrainingGroup::VosId)
                            .to(Vos::Table, Vos::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-training_group-position_id")
                            .from(TrainingGroup::Table, TrainingGroup::PositionId)
                            .to(Position::Table, Position::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-training_group-course_id")
                            .from(TrainingGroup::Table, TrainingGroup::CourseId)
                            .to(Course::Table, Course::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-training_group-site_id")
                            .from(TrainingGroup::Table, TrainingGroup::SiteId)
                            .to(TrainingSite::Table, TrainingSite::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-training_group-organizer_org_id")
                            .from(TrainingGroup::Table, TrainingGroup::OrganizerOrgId)
                            .to(Org::Table, Org::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-training_group-sender_org_id")
                    .table(TrainingGroup::Table)
                    .col(TrainingGroup::SenderOrgId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(TrainingGroup::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub enum TrainingGroup {
    Table,
    Id,
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
    Note,
}
