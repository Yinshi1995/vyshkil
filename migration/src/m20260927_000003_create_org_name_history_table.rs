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
                    .table(OrgNameHistory::Table)
                    .if_not_exists()
                    .col(pk_auto(OrgNameHistory::Id))
                    .col(integer(OrgNameHistory::OrgId))
                    .col(string(OrgNameHistory::ShortName))
                    .col(ColumnDef::new(OrgNameHistory::ValidFrom).date().not_null())
                    .col(ColumnDef::new(OrgNameHistory::ValidTo).date().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-org_name_history-org_id")
                            .from(OrgNameHistory::Table, OrgNameHistory::OrgId)
                            .to(Org::Table, Org::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-org_name_history-org_id")
                    .table(OrgNameHistory::Table)
                    .col(OrgNameHistory::OrgId)
                    .to_owned(),
            )
            .await?;

        // Періоди перейменування для одного org_id не мають перетинатись (migration/CLAUDE.md) —
        // інакше "поточна назва на дату" стає неоднозначною. Потребує btree_gist (000001).
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE org_name_history ADD CONSTRAINT excl_org_name_history_no_overlap \
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
            .drop_table(Table::drop().table(OrgNameHistory::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum OrgNameHistory {
    Table,
    Id,
    OrgId,
    ShortName,
    ValidFrom,
    ValidTo,
}
