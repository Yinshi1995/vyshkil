use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, db: &SchemaManager) -> Result<(), DbErr> {
        db.get_connection()
            .execute_unprepared(
                "CREATE INDEX IF NOT EXISTS idx_org_name_history_org_id ON org_name_history (org_id)",
            )
            .await?;

        db.get_connection()
            .execute_unprepared(
                "CREATE INDEX IF NOT EXISTS idx_subordination_child_id ON subordination (child_id)",
            )
            .await?;

        db.get_connection()
            .execute_unprepared(
                "CREATE INDEX IF NOT EXISTS idx_training_group_org_id ON training_group (org_id)",
            )
            .await?;

        db.get_connection()
            .execute_unprepared(
                "CREATE INDEX IF NOT EXISTS idx_submission_reporting_org_id ON submission (reporting_org_id)",
            )
            .await?;

        db.get_connection()
            .execute_unprepared(
                "CREATE INDEX IF NOT EXISTS idx_submission_status ON submission (status)",
            )
            .await?;

        db.get_connection()
            .execute_unprepared(
                "CREATE INDEX IF NOT EXISTS idx_discrepancy_status ON discrepancy (status)",
            )
            .await?;

        db.get_connection()
            .execute_unprepared(
                "CREATE INDEX IF NOT EXISTS idx_notification_org_id_is_read ON notification (org_id, is_read)",
            )
            .await?;

        db.get_connection()
            .execute_unprepared(
                "CREATE INDEX IF NOT EXISTS idx_group_event_group_id ON group_event (group_id)",
            )
            .await?;

        db.get_connection()
            .execute_unprepared(
                "CREATE INDEX IF NOT EXISTS idx_vos_position_vos_id ON vos_position (vos_id)",
            )
            .await?;

        db.get_connection()
            .execute_unprepared(
                "CREATE INDEX IF NOT EXISTS idx_equipment_vos_equipment_id ON equipment_vos (equipment_id)",
            )
            .await?;

        db.get_connection()
            .execute_unprepared(
                "CREATE INDEX IF NOT EXISTS idx_equipment_vos_vos_id ON equipment_vos (vos_id)",
            )
            .await?;

        Ok(())
    }

    async fn down(&self, db: &SchemaManager) -> Result<(), DbErr> {
        for idx in [
            "idx_org_name_history_org_id",
            "idx_subordination_child_id",
            "idx_training_group_org_id",
            "idx_submission_reporting_org_id",
            "idx_submission_status",
            "idx_discrepancy_status",
            "idx_notification_org_id_is_read",
            "idx_group_event_group_id",
            "idx_vos_position_vos_id",
            "idx_equipment_vos_equipment_id",
            "idx_equipment_vos_vos_id",
        ] {
            db.get_connection()
                .execute_unprepared(&format!("DROP INDEX IF EXISTS {idx}"))
                .await?;
        }
        Ok(())
    }
}
