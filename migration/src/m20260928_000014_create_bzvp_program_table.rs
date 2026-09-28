use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(BzvpProgram::Table)
                    .if_not_exists()
                    .col(pk_auto(BzvpProgram::Id))
                    .col(string_uniq(BzvpProgram::Name))
                    .col(ColumnDef::new(BzvpProgram::DeletedAt).timestamp_with_time_zone().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(BzvpProgram::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub enum BzvpProgram {
    Table,
    Id,
    Name,
    DeletedAt,
}
