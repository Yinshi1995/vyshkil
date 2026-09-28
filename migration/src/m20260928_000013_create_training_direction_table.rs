use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TrainingDirection::Table)
                    .if_not_exists()
                    .col(pk_auto(TrainingDirection::Id))
                    // Теги "розширювані" (01 §2, "…") — вільний текст, не CHECK.
                    .col(string_uniq(TrainingDirection::Code))
                    .col(string(TrainingDirection::Name))
                    .col(
                        ColumnDef::new(TrainingDirection::DeletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(TrainingDirection::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub enum TrainingDirection {
    Table,
    Id,
    Code,
    Name,
    DeletedAt,
}
