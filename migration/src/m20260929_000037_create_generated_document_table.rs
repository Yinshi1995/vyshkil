use sea_orm_migration::{prelude::*, schema::*};

use crate::m20260927_000002_create_org_table::Org;

// Мінімальний зріз під Етап 7/D1 (05 §вступ: "кожен згенерований документ зберігається в
// generated_document") -- не повна схема під усі D1-D6 одразу (той самий принцип, що Етап 3/4:
// мінімум під критерій, розширити колонками/значеннями kind, коли дійде черга D2/D3/D4). Файл --
// на диску (01 §5 "оригінали на диску", не BYTEA); без audit-тригера -- рядки лише створюються,
// ніколи не редагуються (append-only лог генерацій, UPDATE тут не станеться).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(GeneratedDocument::Table)
                    .if_not_exists()
                    .col(pk_auto(GeneratedDocument::Id))
                    .col(
                        ColumnDef::new(GeneratedDocument::Kind)
                            .string()
                            .not_null()
                            .check(Expr::col(GeneratedDocument::Kind).is_in(["d1"])),
                    )
                    .col(integer(GeneratedDocument::OrgId))
                    .col(ColumnDef::new(GeneratedDocument::AsOfDate).date().not_null())
                    .col(string(GeneratedDocument::FilePath))
                    .col(
                        ColumnDef::new(GeneratedDocument::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-generated_document-org_id")
                            .from(GeneratedDocument::Table, GeneratedDocument::OrgId)
                            .to(Org::Table, Org::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-generated_document-org_id-as_of_date")
                    .table(GeneratedDocument::Table)
                    .col(GeneratedDocument::OrgId)
                    .col(GeneratedDocument::AsOfDate)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(GeneratedDocument::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
pub enum GeneratedDocument {
    Table,
    Id,
    Kind,
    OrgId,
    AsOfDate,
    FilePath,
    CreatedAt,
}
