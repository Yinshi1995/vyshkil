use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, db: &SchemaManager) -> Result<(), DbErr> {
        db.get_connection()
            .execute_unprepared(
                "ALTER TABLE user_account ADD COLUMN must_change_password boolean NOT NULL DEFAULT false",
            )
            .await?;
        db.get_connection()
            .execute_unprepared(
                "UPDATE user_account SET must_change_password = true WHERE login != 'admin'",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, db: &SchemaManager) -> Result<(), DbErr> {
        db.get_connection()
            .execute_unprepared("ALTER TABLE user_account DROP COLUMN must_change_password")
            .await?;
        Ok(())
    }
}
