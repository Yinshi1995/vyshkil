use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // 1. Delete events/compositions/reported_groups of duplicate groups (keep lowest id per key).
        db.execute_unprepared(
            "WITH keepers AS (
                SELECT MIN(id) AS id
                FROM training_group
                GROUP BY sender_org_id, training_kind_id, site_id, planned_start, planned_end,
                         COALESCE(vos_id, 0), COALESCE(position_id, 0),
                         COALESCE(course_id, 0), COALESCE(bzvp_program_id, 0)
            ),
            dupes AS (
                SELECT g.id FROM training_group g
                LEFT JOIN keepers k ON g.id = k.id
                WHERE k.id IS NULL
                  AND EXISTS (
                    SELECT 1 FROM training_group g2
                    WHERE g2.id < g.id
                      AND g2.sender_org_id = g.sender_org_id
                      AND g2.training_kind_id = g.training_kind_id
                      AND g2.site_id = g.site_id
                      AND g2.planned_start = g.planned_start
                      AND g2.planned_end = g.planned_end
                      AND COALESCE(g2.vos_id, 0) = COALESCE(g.vos_id, 0)
                      AND COALESCE(g2.position_id, 0) = COALESCE(g.position_id, 0)
                      AND COALESCE(g2.course_id, 0) = COALESCE(g.course_id, 0)
                      AND COALESCE(g2.bzvp_program_id, 0) = COALESCE(g.bzvp_program_id, 0)
                  )
            )
            DELETE FROM group_event WHERE group_id IN (SELECT id FROM dupes)",
        )
        .await?;

        db.execute_unprepared(
            "WITH keepers AS (
                SELECT MIN(id) AS id
                FROM training_group
                GROUP BY sender_org_id, training_kind_id, site_id, planned_start, planned_end,
                         COALESCE(vos_id, 0), COALESCE(position_id, 0),
                         COALESCE(course_id, 0), COALESCE(bzvp_program_id, 0)
            ),
            dupes AS (
                SELECT g.id FROM training_group g
                LEFT JOIN keepers k ON g.id = k.id
                WHERE k.id IS NULL
                  AND EXISTS (
                    SELECT 1 FROM training_group g2
                    WHERE g2.id < g.id
                      AND g2.sender_org_id = g.sender_org_id
                      AND g2.training_kind_id = g.training_kind_id
                      AND g2.site_id = g.site_id
                      AND g2.planned_start = g.planned_start
                      AND g2.planned_end = g.planned_end
                      AND COALESCE(g2.vos_id, 0) = COALESCE(g.vos_id, 0)
                      AND COALESCE(g2.position_id, 0) = COALESCE(g.position_id, 0)
                      AND COALESCE(g2.course_id, 0) = COALESCE(g.course_id, 0)
                      AND COALESCE(g2.bzvp_program_id, 0) = COALESCE(g.bzvp_program_id, 0)
                  )
            )
            DELETE FROM group_composition WHERE group_id IN (SELECT id FROM dupes)",
        )
        .await?;

        db.execute_unprepared(
            "WITH keepers AS (
                SELECT MIN(id) AS id
                FROM training_group
                GROUP BY sender_org_id, training_kind_id, site_id, planned_start, planned_end,
                         COALESCE(vos_id, 0), COALESCE(position_id, 0),
                         COALESCE(course_id, 0), COALESCE(bzvp_program_id, 0)
            ),
            dupes AS (
                SELECT g.id FROM training_group g
                LEFT JOIN keepers k ON g.id = k.id
                WHERE k.id IS NULL
                  AND EXISTS (
                    SELECT 1 FROM training_group g2
                    WHERE g2.id < g.id
                      AND g2.sender_org_id = g.sender_org_id
                      AND g2.training_kind_id = g.training_kind_id
                      AND g2.site_id = g.site_id
                      AND g2.planned_start = g.planned_start
                      AND g2.planned_end = g.planned_end
                      AND COALESCE(g2.vos_id, 0) = COALESCE(g.vos_id, 0)
                      AND COALESCE(g2.position_id, 0) = COALESCE(g.position_id, 0)
                      AND COALESCE(g2.course_id, 0) = COALESCE(g.course_id, 0)
                      AND COALESCE(g2.bzvp_program_id, 0) = COALESCE(g.bzvp_program_id, 0)
                  )
            )
            UPDATE reported_group SET matched_group_id = (
                SELECT MIN(g2.id) FROM training_group g2
                WHERE g2.sender_org_id = (SELECT sender_org_id FROM training_group WHERE id = reported_group.matched_group_id)
                  AND g2.training_kind_id = (SELECT training_kind_id FROM training_group WHERE id = reported_group.matched_group_id)
                  AND g2.site_id = (SELECT site_id FROM training_group WHERE id = reported_group.matched_group_id)
                  AND g2.planned_start = (SELECT planned_start FROM training_group WHERE id = reported_group.matched_group_id)
                  AND g2.planned_end = (SELECT planned_end FROM training_group WHERE id = reported_group.matched_group_id)
            )
            WHERE matched_group_id IN (SELECT id FROM dupes)",
        )
        .await?;

        db.execute_unprepared(
            "WITH keepers AS (
                SELECT MIN(id) AS id
                FROM training_group
                GROUP BY sender_org_id, training_kind_id, site_id, planned_start, planned_end,
                         COALESCE(vos_id, 0), COALESCE(position_id, 0),
                         COALESCE(course_id, 0), COALESCE(bzvp_program_id, 0)
            ),
            dupes AS (
                SELECT g.id FROM training_group g
                LEFT JOIN keepers k ON g.id = k.id
                WHERE k.id IS NULL
                  AND EXISTS (
                    SELECT 1 FROM training_group g2
                    WHERE g2.id < g.id
                      AND g2.sender_org_id = g.sender_org_id
                      AND g2.training_kind_id = g.training_kind_id
                      AND g2.site_id = g.site_id
                      AND g2.planned_start = g.planned_start
                      AND g2.planned_end = g.planned_end
                      AND COALESCE(g2.vos_id, 0) = COALESCE(g.vos_id, 0)
                      AND COALESCE(g2.position_id, 0) = COALESCE(g.position_id, 0)
                      AND COALESCE(g2.course_id, 0) = COALESCE(g.course_id, 0)
                      AND COALESCE(g2.bzvp_program_id, 0) = COALESCE(g.bzvp_program_id, 0)
                  )
            )
            UPDATE discrepancy SET group_id = (
                SELECT MIN(g2.id) FROM training_group g2
                WHERE g2.sender_org_id = (SELECT sender_org_id FROM training_group WHERE id = discrepancy.group_id)
                  AND g2.training_kind_id = (SELECT training_kind_id FROM training_group WHERE id = discrepancy.group_id)
                  AND g2.site_id = (SELECT site_id FROM training_group WHERE id = discrepancy.group_id)
                  AND g2.planned_start = (SELECT planned_start FROM training_group WHERE id = discrepancy.group_id)
                  AND g2.planned_end = (SELECT planned_end FROM training_group WHERE id = discrepancy.group_id)
            )
            WHERE group_id IN (SELECT id FROM dupes)",
        )
        .await?;

        // 2. Delete the duplicate training_group rows themselves.
        db.execute_unprepared(
            "WITH keepers AS (
                SELECT MIN(id) AS id
                FROM training_group
                GROUP BY sender_org_id, training_kind_id, site_id, planned_start, planned_end,
                         COALESCE(vos_id, 0), COALESCE(position_id, 0),
                         COALESCE(course_id, 0), COALESCE(bzvp_program_id, 0)
            )
            DELETE FROM training_group
            WHERE id NOT IN (SELECT id FROM keepers)",
        )
        .await?;

        // 3. Add UNIQUE constraint.
        db.execute_unprepared(
            "CREATE UNIQUE INDEX idx_training_group_natural_key \
             ON training_group ( \
                 sender_org_id, training_kind_id, site_id, \
                 planned_start, planned_end, \
                 COALESCE(vos_id, 0), COALESCE(position_id, 0), \
                 COALESCE(course_id, 0), COALESCE(bzvp_program_id, 0) \
             )",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP INDEX IF EXISTS idx_training_group_natural_key")
            .await?;
        Ok(())
    }
}
