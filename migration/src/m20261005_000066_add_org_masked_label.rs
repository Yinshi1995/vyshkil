use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            "ALTER TABLE org ADD COLUMN masked_label text \
             GENERATED ALWAYS AS (\
               COALESCE(\
                 CASE number_kind WHEN 'A' THEN 'А' WHEN 'T' THEN 'Т' ELSE number_kind END || number,\
                 short_name\
               )\
             ) STORED"
        ).await?;
        db.execute_unprepared(
            "CREATE INDEX idx_org_masked_label_trgm ON org USING gin (masked_label gin_trgm_ops)"
        ).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared("DROP INDEX IF EXISTS idx_org_masked_label_trgm").await?;
        db.execute_unprepared("ALTER TABLE org DROP COLUMN IF EXISTS masked_label").await?;
        Ok(())
    }
}
