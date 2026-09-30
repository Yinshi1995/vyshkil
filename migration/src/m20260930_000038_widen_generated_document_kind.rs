use sea_orm_migration::prelude::*;

// Етап 7, D2: "Контролька" — той самий generated_document, що D1, лише нове значення kind.
// Нова міграція (не редагувати вже застосовану m20260929_000037) — Postgres CHECK не можна
// ALTER, лише DROP+ADD.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            "ALTER TABLE generated_document DROP CONSTRAINT \"generated_document_kind_check\"",
        )
        .await?;
        db.execute_unprepared(
            "ALTER TABLE generated_document ADD CONSTRAINT \"generated_document_kind_check\" \
             CHECK (kind IN ('d1', 'd2'))",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            "ALTER TABLE generated_document DROP CONSTRAINT \"generated_document_kind_check\"",
        )
        .await?;
        db.execute_unprepared(
            "ALTER TABLE generated_document ADD CONSTRAINT \"generated_document_kind_check\" \
             CHECK (kind IN ('d1'))",
        )
        .await?;
        Ok(())
    }
}
