use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Position::Table)
                    .if_not_exists()
                    .col(pk_auto(Position::Id))
                    .col(string_uniq(Position::Name))
                    .col(ColumnDef::new(Position::DeletedAt).timestamp_with_time_zone().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Position::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub enum Position {
    Table,
    Id,
    Name,
    DeletedAt,
}
