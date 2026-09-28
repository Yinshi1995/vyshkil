use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Equipment::Table)
                    .if_not_exists()
                    .col(pk_auto(Equipment::Id))
                    .col(string_uniq(Equipment::Name))
                    // category — відкритий список ("…" у 01 §2), вільний текст, не CHECK.
                    .col(string_null(Equipment::Category))
                    .col(ColumnDef::new(Equipment::DeletedAt).timestamp_with_time_zone().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Equipment::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub enum Equipment {
    Table,
    Id,
    Name,
    Category,
    DeletedAt,
}
