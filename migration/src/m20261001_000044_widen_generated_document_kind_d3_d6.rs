use sea_orm_migration::prelude::*;

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
             CHECK (kind IN ('d1', 'd2', 'd3', 'd4', 'd5_fah', 'd5_bps', 'd5_kvid', 'd5_ivs', 'd5_terminy', 'd6'))",
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
             CHECK (kind IN ('d1', 'd2'))",
        )
        .await?;
        Ok(())
    }
}
