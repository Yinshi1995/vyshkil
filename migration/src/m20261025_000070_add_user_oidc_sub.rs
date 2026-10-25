use sea_orm_migration::prelude::*;

/// Вхід через ІКС (12-auth.md §2.5): `oidc_sub` — незмінний ідентифікатор профілю в Pocket ID
/// (claim `sub`). Мапінг іде за логіном (`preferred_username`), а `sub` фіксується при першому
/// вході — щоб перейменований/перестворений профіль ІКС не "успадкував" чужий обліковий запис.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, db: &SchemaManager) -> Result<(), DbErr> {
        db.get_connection()
            .execute_unprepared("ALTER TABLE user_account ADD COLUMN oidc_sub text UNIQUE")
            .await?;
        Ok(())
    }

    async fn down(&self, db: &SchemaManager) -> Result<(), DbErr> {
        db.get_connection()
            .execute_unprepared("ALTER TABLE user_account DROP COLUMN IF EXISTS oidc_sub")
            .await?;
        Ok(())
    }
}
