use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const AUDITED_TABLES: [&str; 2] = ["training_group", "group_event"];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for table in AUDITED_TABLES {
            db.execute_unprepared(&format!(
                "CREATE TRIGGER trg_audit_{table} AFTER INSERT OR UPDATE OR DELETE ON {table} \
                 FOR EACH ROW EXECUTE FUNCTION audit_log_trigger()"
            ))
            .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for table in AUDITED_TABLES {
            db.execute_unprepared(&format!("DROP TRIGGER IF EXISTS trg_audit_{table} ON {table}"))
                .await?;
        }
        Ok(())
    }
}
