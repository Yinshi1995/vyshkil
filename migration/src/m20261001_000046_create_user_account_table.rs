use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, db: &SchemaManager) -> Result<(), DbErr> {
        db.get_connection()
            .execute_unprepared(
                "CREATE TABLE user_account (
                    id serial PRIMARY KEY,
                    login text UNIQUE NOT NULL,
                    password_hash text NOT NULL,
                    display_name text,
                    is_active boolean NOT NULL DEFAULT true,
                    created_at timestamptz NOT NULL DEFAULT now(),
                    updated_at timestamptz NOT NULL DEFAULT now()
                )",
            )
            .await?;

        db.get_connection()
            .execute_unprepared(
                "CREATE TABLE user_role (
                    id serial PRIMARY KEY,
                    user_id integer NOT NULL REFERENCES user_account(id),
                    org_id integer NOT NULL REFERENCES org(id),
                    role text NOT NULL CHECK (role IN ('admin', 'org_editor', 'viewer')),
                    UNIQUE (user_id, org_id, role)
                )",
            )
            .await?;

        db.get_connection()
            .execute_unprepared(
                "CREATE TABLE user_session (
                    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
                    user_id integer NOT NULL REFERENCES user_account(id),
                    active_org_id integer REFERENCES org(id),
                    active_role text,
                    created_at timestamptz NOT NULL DEFAULT now(),
                    expires_at timestamptz NOT NULL DEFAULT now() + interval '30 days'
                )",
            )
            .await?;

        db.get_connection()
            .execute_unprepared(
                "CREATE INDEX idx_user_session_user_id ON user_session (user_id)",
            )
            .await?;

        db.get_connection()
            .execute_unprepared(
                "CREATE INDEX idx_user_role_user_id ON user_role (user_id)",
            )
            .await?;

        Ok(())
    }

    async fn down(&self, db: &SchemaManager) -> Result<(), DbErr> {
        db.get_connection().execute_unprepared("DROP TABLE IF EXISTS user_session").await?;
        db.get_connection().execute_unprepared("DROP TABLE IF EXISTS user_role").await?;
        db.get_connection().execute_unprepared("DROP TABLE IF EXISTS user_account").await?;
        Ok(())
    }
}
