use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // 1. Add created_by column to group_event
        db.execute_unprepared(
            "ALTER TABLE group_event ADD COLUMN created_by integer REFERENCES user_account(id)",
        )
        .await?;

        // 2. Seed user profiles
        db.execute_unprepared(
            "UPDATE user_account SET \
                first_name = 'Андрій', last_name = 'Ковальчук', rank = 'майор', \
                phone = '+380671234567', callsign = 'Сокіл', delta_nick = 'sokil_17' \
             WHERE login = 'editor_17ak'"
        ).await?;
        db.execute_unprepared(
            "UPDATE user_account SET \
                first_name = 'Ірина', last_name = 'Шевченко', rank = 'старший лейтенант', \
                phone = '+380931112233', callsign = 'Берегиня', delta_nick = 'bereginya_20' \
             WHERE login = 'editor_20ak'"
        ).await?;
        db.execute_unprepared(
            "UPDATE user_account SET \
                first_name = 'Дмитро', last_name = 'Бондаренко', rank = 'підполковник', \
                phone = '+380502223344', callsign = 'Чорний', delta_nick = 'chorniy_uvs' \
             WHERE login = 'viewer_pivden'"
        ).await?;
        db.execute_unprepared(
            "UPDATE user_account SET \
                first_name = 'Тарас', last_name = 'Мельник', rank = 'капітан', \
                phone = '+380663334455', callsign = 'Граніт', delta_nick = 'granit_152' \
             WHERE login = 'editor_152nc'"
        ).await?;
        db.execute_unprepared(
            "UPDATE user_account SET \
                first_name = 'Олена', last_name = 'Литвиненко', rank = 'лейтенант', \
                phone = '+380734445566', callsign = 'Ластівка', delta_nick = 'lastivka_30' \
             WHERE login = 'viewer_30kmp'"
        ).await?;

        // 3. Clean up duplicate events for group 3 (import duplications)
        db.execute_unprepared(
            "DELETE FROM group_event WHERE group_id = 3 AND id NOT IN ( \
                SELECT DISTINCT ON (event_type, occurred_on) id FROM group_event \
                WHERE group_id = 3 ORDER BY event_type, occurred_on, id \
             )"
        ).await?;

        // 4. Assign created_by to existing events based on org ownership
        // Events for groups owned by orgs under 17 АК → editor_17ak (id=7)
        db.execute_unprepared(
            "UPDATE group_event ge SET created_by = 7 \
             FROM training_group tg \
             WHERE ge.group_id = tg.id AND tg.sender_org_id IN ( \
                SELECT descendant_id FROM subordination_closure WHERE ancestor_id = 2 \
                UNION SELECT 2 \
             ) AND ge.created_by IS NULL"
        ).await?;
        // Events for groups owned by orgs under 20 АК → editor_20ak (id=8)
        db.execute_unprepared(
            "UPDATE group_event ge SET created_by = 8 \
             FROM training_group tg \
             WHERE ge.group_id = tg.id AND tg.sender_org_id IN ( \
                SELECT descendant_id FROM subordination_closure WHERE ancestor_id = 3 \
                UNION SELECT 3 \
             ) AND ge.created_by IS NULL"
        ).await?;
        // Events for 152 НЦ groups → editor_152nc (id=10)
        db.execute_unprepared(
            "UPDATE group_event ge SET created_by = 10 \
             FROM training_group tg \
             WHERE ge.group_id = tg.id AND tg.sender_org_id = 30 AND ge.created_by IS NULL"
        ).await?;
        // Remaining events → admin (id=1)
        db.execute_unprepared(
            "UPDATE group_event SET created_by = 1 WHERE created_by IS NULL"
        ).await?;

        // 5. Vary recorded_at to show realistic creation times
        db.execute_unprepared(
            "UPDATE group_event SET recorded_at = occurred_on::timestamp + \
             (id % 14)::int * interval '1 day' + \
             ((id * 7 + 3) % 12)::int * interval '1 hour' + \
             ((id * 13 + 5) % 59)::int * interval '1 minute' \
             WHERE recorded_at = '2026-10-02 08:22:46.806657+00'"
        ).await?;
        db.execute_unprepared(
            "UPDATE group_event SET recorded_at = occurred_on::timestamp + \
             interval '1 day' + \
             ((id * 11 + 2) % 10)::int * interval '1 hour' + \
             ((id * 17 + 7) % 59)::int * interval '1 minute' \
             WHERE recorded_at = '2026-10-01 14:22:25.873733+00' \
                OR recorded_at = '2026-10-01 15:11:12.034729+00' \
                OR recorded_at = '2026-10-01 15:21:43.067104+00' \
                OR recorded_at = '2026-10-01 15:42:27.86829+00' \
                OR recorded_at = '2026-10-01 19:05:04.313369+00' \
                OR recorded_at = '2026-10-02 05:39:44.338177+00' \
                OR recorded_at = '2026-10-02 05:40:43.39901+00' \
                OR recorded_at = '2026-10-02 05:55:40.851683+00'"
        ).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared("ALTER TABLE group_event DROP COLUMN IF EXISTS created_by").await?;
        Ok(())
    }
}
