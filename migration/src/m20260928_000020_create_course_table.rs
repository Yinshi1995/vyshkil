use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Course::Table)
                    .if_not_exists()
                    .col(pk_auto(Course::Id))
                    .col(string_uniq(Course::Name))
                    .col(ColumnDef::new(Course::DeletedAt).timestamp_with_time_zone().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Course::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub enum Course {
    Table,
    Id,
    Name,
    DeletedAt,
}
