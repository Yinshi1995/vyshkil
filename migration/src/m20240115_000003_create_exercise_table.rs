use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Exercise::Table)
                    .if_not_exists()
                    .col(pk_auto(Exercise::Id))
                    .col(string(Exercise::Code))
                    .col(string(Exercise::Name))
                    // category: normative/tactical/live_fire/staff — довідник вправ бойової підготовки, а не жорсткий enum,
                    // бо перелік типів вправ регулярно поповнюється новими наказами/програмами підготовки.
                    .col(string(Exercise::Category))
                    .col(text_null(Exercise::Description))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-exercise-code")
                    .table(Exercise::Table)
                    .col(Exercise::Code)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Exercise::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Exercise {
    Table,
    Id,
    Code,
    Name,
    Category,
    Description,
}
