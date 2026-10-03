use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, db: &SchemaManager) -> Result<(), DbErr> {
        db.get_connection()
            .execute_unprepared(
                "INSERT INTO notification_type (code, name, scope) VALUES
                    ('group_event_added', 'Подія групи підготовки', 'org')
                 ON CONFLICT (code) DO NOTHING",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, db: &SchemaManager) -> Result<(), DbErr> {
        db.get_connection()
            .execute_unprepared(
                "DELETE FROM notification_type WHERE code = 'group_event_added'",
            )
            .await?;
        Ok(())
    }
}
