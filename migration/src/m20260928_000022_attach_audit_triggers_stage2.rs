use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

// Довідники Етапу 2 — теж під generic тригер audit_log (migration/CLAUDE.md: "кожна доменна
// таблиця — під generic тригер"), функція вже створена в m20260927_000009. vos_position/
// equipment_vos — вагові зв'язки-підказки, не історичні факти, але теж редагуються вручну
// (learned), тож теж під аудит.
const AUDITED_TABLES: [&str; 10] = [
    "training_kind",
    "training_direction",
    "bzvp_program",
    "vos",
    "position",
    "vos_position",
    "equipment",
    "equipment_vos",
    "course",
    "attrition_reason",
];

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
