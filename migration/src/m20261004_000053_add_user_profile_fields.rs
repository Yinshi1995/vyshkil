use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, db: &SchemaManager) -> Result<(), DbErr> {
        db.get_connection()
            .execute_unprepared(
                "ALTER TABLE user_account \
                 ADD COLUMN phone text, \
                 ADD COLUMN callsign text",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, db: &SchemaManager) -> Result<(), DbErr> {
        db.get_connection()
            .execute_unprepared(
                "ALTER TABLE user_account \
                 DROP COLUMN IF EXISTS phone, \
                 DROP COLUMN IF EXISTS callsign",
            )
            .await?;
        Ok(())
    }
}
