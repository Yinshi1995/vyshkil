use sea_orm_migration::{prelude::*, schema::*};

use crate::m20260928_000015_create_vos_table::Vos;
use crate::m20260928_000018_create_equipment_table::Equipment;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(EquipmentVos::Table)
                    .if_not_exists()
                    .col(pk_auto(EquipmentVos::Id))
                    .col(integer(EquipmentVos::EquipmentId))
                    .col(integer(EquipmentVos::VosId))
                    .col(integer(EquipmentVos::Weight).default(1))
                    .col(
                        ColumnDef::new(EquipmentVos::Source)
                            .string()
                            .not_null()
                            .check(Expr::col(EquipmentVos::Source).is_in([
                                "seed", "manual", "learned",
                            ])),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-equipment_vos-equipment_id")
                            .from(EquipmentVos::Table, EquipmentVos::EquipmentId)
                            .to(Equipment::Table, Equipment::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-equipment_vos-vos_id")
                            .from(EquipmentVos::Table, EquipmentVos::VosId)
                            .to(Vos::Table, Vos::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .unique()
                    .name("idx-equipment_vos-equipment_id-vos_id")
                    .table(EquipmentVos::Table)
                    .col(EquipmentVos::EquipmentId)
                    .col(EquipmentVos::VosId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(EquipmentVos::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub enum EquipmentVos {
    Table,
    Id,
    EquipmentId,
    VosId,
    Weight,
    Source,
}
