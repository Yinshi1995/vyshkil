use sea_orm_migration::{prelude::*, schema::*};

use crate::m20260927_000002_create_org_table::Org;
use crate::m20260928_000025_create_training_group_table::TrainingGroup;

#[derive(DeriveMigrationName)]
pub struct Migration;

// Розподіл групи за підрозділами (01 §3, 02 §1 колонка 8) -- опційний розкривний підрядок у
// сітці форми. subunit_org_id nullable: підрозділ (org.kind='subunit') створюється лише за
// потреби, до того часу зберігаємо як вільний текст ("1б тро-2") у subunit_label.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(GroupComposition::Table)
                    .if_not_exists()
                    .col(pk_auto(GroupComposition::Id))
                    .col(integer(GroupComposition::GroupId))
                    .col(integer_null(GroupComposition::SubunitOrgId))
                    .col(string_null(GroupComposition::SubunitLabel))
                    .col(integer(GroupComposition::Count))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-group_composition-group_id")
                            .from(GroupComposition::Table, GroupComposition::GroupId)
                            .to(TrainingGroup::Table, TrainingGroup::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-group_composition-subunit_org_id")
                            .from(GroupComposition::Table, GroupComposition::SubunitOrgId)
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
                    .name("idx-group_composition-group_id")
                    .table(GroupComposition::Table)
                    .col(GroupComposition::GroupId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(GroupComposition::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub enum GroupComposition {
    Table,
    Id,
    GroupId,
    SubunitOrgId,
    SubunitLabel,
    Count,
}
