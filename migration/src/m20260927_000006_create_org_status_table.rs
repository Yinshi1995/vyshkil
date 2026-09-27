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
                    .table(OrgStatus::Table)
                    .if_not_exists()
                    .col(pk_auto(OrgStatus::Id))
                    .col(integer(OrgStatus::OrgId))
                    .col(
                        ColumnDef::new(OrgStatus::Status)
                            .string()
                            .not_null()
                            .check(Expr::col(OrgStatus::Status).is_in([
                                "in_zone",
                                "out_of_zone",
                                "transferred",
                                "disbanded",
                                "other",
                            ])),
                    )
                    .col(ColumnDef::new(OrgStatus::ValidFrom).date().not_null())
                    .col(ColumnDef::new(OrgStatus::ValidTo).date().null())
                    .col(integer_null(OrgStatus::CounterpartOrgId))
                    .col(text_null(OrgStatus::Note))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-org_status-org_id")
                            .from(OrgStatus::Table, OrgStatus::OrgId)
                            .to(Org::Table, Org::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-org_status-counterpart_org_id")
                            .from(OrgStatus::Table, OrgStatus::CounterpartOrgId)
                            .to(Org::Table, Org::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-org_status-org_id")
                    .table(OrgStatus::Table)
                    .col(OrgStatus::OrgId)
                    .to_owned(),
            )
            .await?;

        // Один org не може мати два одночасно активні статуси — інакше зведення по 01 §1
        // ("кожна частина рахується рівно один раз") стає неоднозначним на конкретну дату.
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE org_status ADD CONSTRAINT excl_org_status_no_overlap \
                 EXCLUDE USING gist ( \
                     org_id WITH =, \
                     daterange(valid_from, valid_to, '[)') WITH && \
                 )",
            )
            .await
            .map(|_| ())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(OrgStatus::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum OrgStatus {
    Table,
    Id,
    OrgId,
    Status,
    ValidFrom,
    ValidTo,
    CounterpartOrgId,
    Note,
}
