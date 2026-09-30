use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TRIGGER trg_audit_discrepancy AFTER INSERT OR UPDATE OR DELETE \
                 ON discrepancy FOR EACH ROW EXECUTE FUNCTION audit_log_trigger()",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TRIGGER IF EXISTS trg_audit_discrepancy ON discrepancy")
            .await?;
        Ok(())
    }
}
