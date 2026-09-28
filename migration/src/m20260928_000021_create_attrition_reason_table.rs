use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AttritionReason::Table)
                    .if_not_exists()
                    .col(pk_auto(AttritionReason::Id))
                    .col(string_uniq(AttritionReason::Name))
                    // "інше (+текст)" (01 §2) — єдина причина, що вимагає супровідного тексту.
                    .col(boolean(AttritionReason::RequiresNote).default(false))
                    .col(ColumnDef::new(AttritionReason::DeletedAt).timestamp_with_time_zone().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(AttritionReason::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub enum AttritionReason {
    Table,
    Id,
    Name,
    RequiresNote,
    DeletedAt,
}
