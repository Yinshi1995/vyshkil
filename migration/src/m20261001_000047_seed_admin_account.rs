use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, db: &SchemaManager) -> Result<(), DbErr> {
        // Dev seed: admin/admin123 — ТІЛЬКИ для розробки.
        // У проді INITIAL_ADMIN_PASSWORD задається env-змінною, seed перезаписує хеш.
        // argon2id hash of "admin123" (precomputed, NOT from env — env-based rehash at server start)
        let hash = "$argon2id$v=19$m=19456,t=2,p=1$YWRtaW5fc2VlZF9zYWx0AAAA$opUUnUXe8lkEuazVx7MTJpmX9blm7548lZcRYoJsoJ4";

        db.get_connection()
            .execute_unprepared(&format!(
                "INSERT INTO user_account (login, password_hash, display_name) \
                 VALUES ('admin', '{hash}', 'Адміністратор') \
                 ON CONFLICT (login) DO NOTHING"
            ))
            .await?;

        // Admin role for ALL existing orgs (in dev seed, org_id=1 is УВ(с) Південь root)
        db.get_connection()
            .execute_unprepared(
                "INSERT INTO user_role (user_id, org_id, role) \
                 SELECT ua.id, o.id, 'admin' \
                 FROM user_account ua, org o \
                 WHERE ua.login = 'admin' \
                 ON CONFLICT DO NOTHING"
            )
            .await?;

        Ok(())
    }

    async fn down(&self, db: &SchemaManager) -> Result<(), DbErr> {
        db.get_connection()
            .execute_unprepared(
                "DELETE FROM user_role WHERE user_id IN (SELECT id FROM user_account WHERE login = 'admin')"
            )
            .await?;
        db.get_connection()
            .execute_unprepared("DELETE FROM user_account WHERE login = 'admin'")
            .await?;
        Ok(())
    }
}
