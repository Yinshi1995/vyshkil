use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TrainingKind::Table)
                    .if_not_exists()
                    .col(pk_auto(TrainingKind::Id))
                    // code — вільний текст, не CHECK: "розширюваний" за 01 §2 (нові види
                    // підготовки не повинні вимагати міграції типу).
                    .col(string_uniq(TrainingKind::Code))
                    .col(string(TrainingKind::Name))
                    .col(
                        ColumnDef::new(TrainingKind::DeletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(TrainingKind::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub enum TrainingKind {
    Table,
    Id,
    Code,
    Name,
    DeletedAt,
}
