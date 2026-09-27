use sea_orm_migration::{prelude::*, schema::*};

use crate::m20240115_000001_create_unit_table::Unit;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Personnel::Table)
                    .if_not_exists()
                    .col(pk_auto(Personnel::Id))
                    .col(integer(Personnel::UnitId))
                    .col(string(Personnel::FullName))
                    .col(string(Personnel::Rank))
                    .col(string_null(Personnel::Position))
                    .col(string(Personnel::ServiceNumber))
                    // status рядком (active/wounded/kia/missing/discharged) з дефолтом — той самий підхід, що й echelon.
                    .col(string(Personnel::Status).default("active"))
                    .col(
                        ColumnDef::new(Personnel::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Personnel::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-personnel-unit_id")
                            .from(Personnel::Table, Personnel::UnitId)
                            .to(Unit::Table, Unit::Id)
                            // Restrict, а не Cascade: розформування підрозділу не має мовчки стирати особовий склад.
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-personnel-service_number")
                    .table(Personnel::Table)
                    .col(Personnel::ServiceNumber)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-personnel-unit_id")
                    .table(Personnel::Table)
                    .col(Personnel::UnitId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Personnel::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Personnel {
    Table,
    Id,
    UnitId,
    FullName,
    Rank,
    Position,
    ServiceNumber,
    Status,
    CreatedAt,
    UpdatedAt,
}
