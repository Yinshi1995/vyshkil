use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, db: &SchemaManager) -> Result<(), DbErr> {
        db.get_connection()
            .execute_unprepared(
                "ALTER TABLE user_account \
                 ADD COLUMN first_name text, \
                 ADD COLUMN last_name text, \
                 ADD COLUMN rank text, \
                 ADD COLUMN avatar_path text",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, db: &SchemaManager) -> Result<(), DbErr> {
        db.get_connection()
            .execute_unprepared(
                "ALTER TABLE user_account \
                 DROP COLUMN IF EXISTS first_name, \
                 DROP COLUMN IF EXISTS last_name, \
                 DROP COLUMN IF EXISTS rank, \
                 DROP COLUMN IF EXISTS avatar_path",
            )
            .await?;
        Ok(())
    }
}
