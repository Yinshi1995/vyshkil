use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Vos::Table)
                    .if_not_exists()
                    .col(pk_auto(Vos::Id))
                    // code — text, не integer: бувають варіанти "129 (202)", "240 (737)" (01 §2).
                    .col(string_uniq(Vos::Code))
                    .col(string(Vos::Title))
                    .col(
                        ColumnDef::new(Vos::Status)
                            .string()
                            .not_null()
                            .check(Expr::col(Vos::Status).is_in(["draft", "official"])),
                    )
                    .col(
                        ColumnDef::new(Vos::Source)
                            .string()
                            .not_null()
                            .check(Expr::col(Vos::Source).is_in(["seed", "manual", "learned"])),
                    )
                    .col(ColumnDef::new(Vos::DeletedAt).timestamp_with_time_zone().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Vos::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub enum Vos {
    Table,
    Id,
    Code,
    Title,
    Status,
    Source,
    DeletedAt,
}
