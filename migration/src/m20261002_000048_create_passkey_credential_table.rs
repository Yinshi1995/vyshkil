use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, db: &SchemaManager) -> Result<(), DbErr> {
        db.get_connection()
            .execute_unprepared(
                "CREATE TABLE passkey_credential (
                    id SERIAL PRIMARY KEY,
                    user_id INT NOT NULL REFERENCES user_account(id),
                    credential_id BYTEA UNIQUE NOT NULL,
                    public_key BYTEA NOT NULL,
                    sign_count BIGINT NOT NULL DEFAULT 0,
                    name TEXT,
                    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
                )",
            )
            .await?;

        db.get_connection()
            .execute_unprepared(
                "CREATE INDEX idx_passkey_credential_user_id ON passkey_credential (user_id)",
            )
            .await?;

        Ok(())
    }

    async fn down(&self, db: &SchemaManager) -> Result<(), DbErr> {
        db.get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS passkey_credential")
            .await?;
        Ok(())
    }
}
