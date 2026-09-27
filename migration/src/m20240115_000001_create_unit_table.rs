use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Unit::Table)
                    .if_not_exists()
                    .col(pk_auto(Unit::Id))
                    .col(integer_null(Unit::ParentUnitId))
                    .col(string(Unit::Name))
                    .col(string(Unit::Code))
                    // echelon зберігаємо рядком (brigade/battalion/company/platoon), а не enum-типом БД,
                    // щоб додавання нового рівня ієрархії не вимагало міграції типу в Postgres.
                    .col(string(Unit::Echelon))
                    .col(string_null(Unit::CommanderFullName))
                    .col(
                        ColumnDef::new(Unit::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Unit::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    // Самопосилання: batальйон -> бригада, рота -> батальйон і т.д. в одній таблиці.
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-unit-parent_unit_id")
                            .from(Unit::Table, Unit::ParentUnitId)
                            .to(Unit::Table, Unit::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-unit-code")
                    .table(Unit::Table)
                    .col(Unit::Code)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Unit::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Unit {
    Table,
    Id,
    ParentUnitId,
    Name,
    Code,
    Echelon,
    CommanderFullName,
    CreatedAt,
    UpdatedAt,
}
