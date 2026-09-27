use sea_orm_migration::{prelude::*, schema::*};

use crate::m20260927_000002_create_org_table::Org;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Subordination::Table)
                    .if_not_exists()
                    .col(pk_auto(Subordination::Id))
                    .col(integer(Subordination::ChildOrgId))
                    .col(integer(Subordination::ParentOrgId))
                    .col(
                        ColumnDef::new(Subordination::Axis)
                            .string()
                            .not_null()
                            .check(Expr::col(Subordination::Axis).is_in(["staff", "operational"])),
                    )
                    .col(ColumnDef::new(Subordination::ValidFrom).date().not_null())
                    .col(ColumnDef::new(Subordination::ValidTo).date().null())
                    .col(text_null(Subordination::Basis))
                    // Без FK: таблиця submission з'явиться на Етапі 3/5 — FK додамо окремою міграцією тоді.
                    .col(integer_null(Subordination::CreatedBySubmissionId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-subordination-child_org_id")
                            .from(Subordination::Table, Subordination::ChildOrgId)
                            .to(Org::Table, Org::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-subordination-parent_org_id")
                            .from(Subordination::Table, Subordination::ParentOrgId)
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
                    .name("idx-subordination-parent_org_id-axis")
                    .table(Subordination::Table)
                    .col(Subordination::ParentOrgId)
                    .col(Subordination::Axis)
                    .to_owned(),
            )
            .await?;

        // Один і той самий (child_org_id, axis) не може мати два періоди, що перетинаються —
        // саме це не дає 110 омбр одночасно "штатно" підпорядковуватись двом органам (01 §1).
        // Потребує btree_gist (m20260927_000001) для GiST-порівняння bigint/text на рівність.
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE subordination ADD CONSTRAINT excl_subordination_no_overlap \
                 EXCLUDE USING gist ( \
                     child_org_id WITH =, \
                     axis WITH =, \
                     daterange(valid_from, valid_to, '[)') WITH && \
                 )",
            )
            .await
            .map(|_| ())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Subordination::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Subordination {
    Table,
    Id,
    ChildOrgId,
    ParentOrgId,
    Axis,
    ValidFrom,
    ValidTo,
    Basis,
    CreatedBySubmissionId,
}
