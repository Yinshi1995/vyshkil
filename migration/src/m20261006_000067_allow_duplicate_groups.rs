use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared("DROP INDEX IF EXISTS idx_training_group_natural_key").await?;
        db.execute_unprepared(
            "CREATE INDEX idx_training_group_natural_key \
             ON training_group ( \
                 sender_org_id, training_kind_id, \
                 planned_start, planned_end, \
                 COALESCE(vos_id, 0), COALESCE(course_id, 0), \
                 COALESCE(bzvp_program_id, 0), \
                 COALESCE(venue_type, ''), \
                 COALESCE(training_venue_id, 0), \
                 COALESCE(city_id, 0) \
             )",
        ).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared("DROP INDEX IF EXISTS idx_training_group_natural_key").await?;
        db.execute_unprepared(
            "CREATE UNIQUE INDEX idx_training_group_natural_key \
             ON training_group ( \
                 sender_org_id, training_kind_id, site_id, \
                 planned_start, planned_end, \
                 COALESCE(vos_id, 0), COALESCE(position_id, 0), \
                 COALESCE(course_id, 0), COALESCE(bzvp_program_id, 0) \
             )",
        ).await?;
        Ok(())
    }
}
