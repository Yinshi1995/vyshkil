use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, db: &SchemaManager) -> Result<(), DbErr> {
        db.get_connection()
            .execute_unprepared(
                "ALTER TABLE whatsapp_destination DROP CONSTRAINT whatsapp_destination_kind_check",
            )
            .await?;
        db.get_connection()
            .execute_unprepared(
                "ALTER TABLE whatsapp_destination ADD CONSTRAINT whatsapp_destination_kind_check \
                 CHECK (kind IN ('personal', 'bot', 'group'))",
            )
            .await?;
        db.get_connection()
            .execute_unprepared(
                "ALTER TABLE whatsapp_destination ADD COLUMN group_id TEXT",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, db: &SchemaManager) -> Result<(), DbErr> {
        db.get_connection()
            .execute_unprepared(
                "ALTER TABLE whatsapp_destination DROP COLUMN IF EXISTS group_id",
            )
            .await?;
        db.get_connection()
            .execute_unprepared(
                "DELETE FROM whatsapp_destination WHERE kind = 'group'",
            )
            .await?;
        db.get_connection()
            .execute_unprepared(
                "ALTER TABLE whatsapp_destination DROP CONSTRAINT whatsapp_destination_kind_check",
            )
            .await?;
        db.get_connection()
            .execute_unprepared(
                "ALTER TABLE whatsapp_destination ADD CONSTRAINT whatsapp_destination_kind_check \
                 CHECK (kind IN ('personal', 'bot'))",
            )
            .await?;
        Ok(())
    }
}
