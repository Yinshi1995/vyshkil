use sea_orm_migration::{prelude::*, schema::*};

use crate::m20260928_000015_create_vos_table::Vos;
use crate::m20260928_000016_create_position_table::Position;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(VosPosition::Table)
                    .if_not_exists()
                    .col(pk_auto(VosPosition::Id))
                    .col(integer(VosPosition::VosId))
                    .col(integer(VosPosition::PositionId))
                    // weight = кількість спостережень (01 §2): ВОС ≠ посада 1:1, ранжуємо підказки.
                    .col(integer(VosPosition::Weight).default(1))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-vos_position-vos_id")
                            .from(VosPosition::Table, VosPosition::VosId)
                            .to(Vos::Table, Vos::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-vos_position-position_id")
                            .from(VosPosition::Table, VosPosition::PositionId)
                            .to(Position::Table, Position::Id)
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
                    .name("idx-vos_position-vos_id-position_id")
                    .table(VosPosition::Table)
                    .col(VosPosition::VosId)
                    .col(VosPosition::PositionId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(VosPosition::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub enum VosPosition {
    Table,
    Id,
    VosId,
    PositionId,
    Weight,
}
