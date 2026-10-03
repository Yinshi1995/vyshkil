use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, db: &SchemaManager) -> Result<(), DbErr> {
        db.get_connection()
            .execute_unprepared(
                "CREATE TABLE whatsapp_destination (
                    id SERIAL PRIMARY KEY,
                    user_id INT REFERENCES user_account(id),
                    org_id INT NOT NULL REFERENCES org(id),
                    kind TEXT NOT NULL CHECK (kind IN ('personal', 'bot')),
                    phone_masked TEXT NOT NULL,
                    is_active BOOLEAN NOT NULL DEFAULT true,
                    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
                )",
            )
            .await?;

        db.get_connection()
            .execute_unprepared(
                "CREATE INDEX idx_whatsapp_destination_org_id ON whatsapp_destination (org_id)",
            )
            .await?;

        db.get_connection()
            .execute_unprepared(
                "CREATE INDEX idx_whatsapp_destination_user_id ON whatsapp_destination (user_id)",
            )
            .await?;

        db.get_connection()
            .execute_unprepared(
                "CREATE TABLE notification_type (
                    code TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    scope TEXT NOT NULL CHECK (scope IN ('personal', 'org'))
                )",
            )
            .await?;

        db.get_connection()
            .execute_unprepared(
                "INSERT INTO notification_type (code, name, scope) VALUES
                    ('discrepancy_opened', 'Розбіжність відкрита', 'org'),
                    ('discrepancy_resolved', 'Розбіжність закрита', 'org'),
                    ('submission_committed', 'Подання зафіксовано', 'org'),
                    ('import_completed', 'Імпорт завершено', 'org'),
                    ('system', 'Системне повідомлення', 'personal')",
            )
            .await?;

        db.get_connection()
            .execute_unprepared(
                "CREATE TABLE notification_subscription (
                    id SERIAL PRIMARY KEY,
                    destination_id INT NOT NULL REFERENCES whatsapp_destination(id) ON DELETE CASCADE,
                    notification_type_code TEXT NOT NULL REFERENCES notification_type(code) ON DELETE CASCADE,
                    is_active BOOLEAN NOT NULL DEFAULT true,
                    UNIQUE (destination_id, notification_type_code)
                )",
            )
            .await?;

        db.get_connection()
            .execute_unprepared(
                "CREATE TABLE notification_group (
                    id SERIAL PRIMARY KEY,
                    org_id INT NOT NULL REFERENCES org(id),
                    name TEXT NOT NULL,
                    created_by INT NOT NULL REFERENCES user_account(id),
                    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                    deleted_at TIMESTAMPTZ
                )",
            )
            .await?;

        db.get_connection()
            .execute_unprepared(
                "CREATE INDEX idx_notification_group_org_id ON notification_group (org_id)",
            )
            .await?;

        db.get_connection()
            .execute_unprepared(
                "CREATE TABLE notification_group_member (
                    group_id INT NOT NULL REFERENCES notification_group(id) ON DELETE CASCADE,
                    destination_id INT NOT NULL REFERENCES whatsapp_destination(id) ON DELETE CASCADE,
                    PRIMARY KEY (group_id, destination_id)
                )",
            )
            .await?;

        Ok(())
    }

    async fn down(&self, db: &SchemaManager) -> Result<(), DbErr> {
        db.get_connection().execute_unprepared("DROP TABLE IF EXISTS notification_group_member").await?;
        db.get_connection().execute_unprepared("DROP TABLE IF EXISTS notification_group").await?;
        db.get_connection().execute_unprepared("DROP TABLE IF EXISTS notification_subscription").await?;
        db.get_connection().execute_unprepared("DROP TABLE IF EXISTS notification_type").await?;
        db.get_connection().execute_unprepared("DROP TABLE IF EXISTS whatsapp_destination").await?;
        Ok(())
    }
}
