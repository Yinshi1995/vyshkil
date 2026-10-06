use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20261005_000065_create_training_venue_model"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // 1. city dictionary
        db.execute_unprepared(
            "CREATE TABLE city (
                id serial PRIMARY KEY,
                name varchar NOT NULL,
                created_at timestamptz NOT NULL DEFAULT now(),
                CONSTRAINT uq_city_name UNIQUE (name)
            )"
        ).await?;

        db.execute_unprepared(
            "CREATE INDEX idx_city_name_trgm ON city USING gin (name gin_trgm_ops)"
        ).await?;

        // 2. training_venue — stationary training locations (НЦ, ВВНЗ)
        db.execute_unprepared(
            "CREATE TABLE training_venue (
                id serial PRIMARY KEY,
                kind varchar NOT NULL CHECK (kind IN ('training_center', 'vvnz')),
                name varchar NOT NULL,
                short_name varchar,
                military_number varchar,
                city_id integer NOT NULL REFERENCES city(id),
                org_id integer REFERENCES org(id),
                is_active boolean NOT NULL DEFAULT true,
                created_at timestamptz NOT NULL DEFAULT now(),
                updated_at timestamptz NOT NULL DEFAULT now()
            )"
        ).await?;

        db.execute_unprepared(
            "CREATE INDEX idx_training_venue_name_trgm ON training_venue USING gin (name gin_trgm_ops)"
        ).await?;

        db.execute_unprepared(
            "CREATE INDEX idx_training_venue_kind ON training_venue (kind)"
        ).await?;

        // 3. org.current_city_id
        db.execute_unprepared(
            "ALTER TABLE org ADD COLUMN current_city_id integer REFERENCES city(id)"
        ).await?;

        // 4. training_group: add new venue columns, make site_id nullable
        db.execute_unprepared(
            "ALTER TABLE training_group
                ADD COLUMN venue_type varchar,
                ADD COLUMN training_venue_id integer REFERENCES training_venue(id),
                ADD COLUMN city_id integer REFERENCES city(id)"
        ).await?;

        db.execute_unprepared(
            "ALTER TABLE training_group ALTER COLUMN site_id DROP NOT NULL"
        ).await?;

        // 5. reported_group: same changes
        db.execute_unprepared(
            "ALTER TABLE reported_group
                ADD COLUMN venue_type varchar,
                ADD COLUMN training_venue_id integer REFERENCES training_venue(id),
                ADD COLUMN city_id integer REFERENCES city(id)"
        ).await?;

        db.execute_unprepared(
            "ALTER TABLE reported_group ALTER COLUMN site_id DROP NOT NULL"
        ).await?;

        // 6. Migrate existing data: training_site → city + training_venue

        // Extract cities from training_site localities
        // "НЦ «Десна»" / "НЦ \"Десна\"" → "Десна" — лапки обох видів: 000065_seed_d1_org_composition
        // (виконується раніше) переводить «» у латинські.
        db.execute_unprepared(
            "INSERT INTO city (name)
             SELECT DISTINCT
                regexp_replace(
                    regexp_replace(locality, '^(НЦ|ПП|НП)\\s*[«\"]', ''),
                    '[»\"]$', ''
                )
             FROM training_site
             WHERE locality IS NOT NULL
             ON CONFLICT (name) DO NOTHING"
        ).await?;

        // Create training_venue for each training_site with locality
        db.execute_unprepared(
            "INSERT INTO training_venue (kind, name, short_name, org_id, city_id)
             SELECT
                'training_center',
                ts.locality,
                ts.locality,
                ts.org_id,
                c.id
             FROM training_site ts
             JOIN city c ON c.name = regexp_replace(
                regexp_replace(ts.locality, '^(НЦ|ПП|НП)\\s*[«\"]', ''),
                '[»\"]$', ''
             )
             WHERE ts.locality IS NOT NULL"
        ).await?;

        // 7. Migrate training_group records
        db.execute_unprepared(
            "UPDATE training_group tg SET
                venue_type = CASE
                    WHEN ts.locality IS NOT NULL THEN 'training_center'
                    ELSE 'unit_base'
                END,
                training_venue_id = tv.id,
                city_id = COALESCE(tv.city_id, NULL)
             FROM training_site ts
             LEFT JOIN training_venue tv ON tv.org_id = ts.org_id AND tv.name = ts.locality
             WHERE tg.site_id = ts.id"
        ).await?;

        // 8. Migrate reported_group records
        db.execute_unprepared(
            "UPDATE reported_group rg SET
                venue_type = CASE
                    WHEN ts.locality IS NOT NULL THEN 'training_center'
                    ELSE 'unit_base'
                END,
                training_venue_id = tv.id,
                city_id = COALESCE(tv.city_id, NULL)
             FROM training_site ts
             LEFT JOIN training_venue tv ON tv.org_id = ts.org_id AND tv.name = ts.locality
             WHERE rg.site_id = ts.id"
        ).await?;

        // 9. Audit triggers
        db.execute_unprepared(
            "CREATE TRIGGER trg_audit_city
             AFTER INSERT OR UPDATE OR DELETE ON city
             FOR EACH ROW EXECUTE FUNCTION audit_log_trigger()"
        ).await?;

        db.execute_unprepared(
            "CREATE TRIGGER trg_audit_training_venue
             AFTER INSERT OR UPDATE OR DELETE ON training_venue
             FOR EACH ROW EXECUTE FUNCTION audit_log_trigger()"
        ).await?;

        // 10. Update natural key index on training_group to include venue columns
        // Drop old index first
        db.execute_unprepared(
            "DROP INDEX IF EXISTS idx_training_group_natural_key"
        ).await?;

        db.execute_unprepared(
            "CREATE UNIQUE INDEX idx_training_group_natural_key
             ON training_group (
                sender_org_id, training_kind_id, planned_start, planned_end,
                COALESCE(vos_id, 0), COALESCE(position_id, 0),
                COALESCE(course_id, 0), COALESCE(bzvp_program_id, 0),
                COALESCE(venue_type, ''), COALESCE(training_venue_id, 0), COALESCE(city_id, 0)
             )"
        ).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared("DROP INDEX IF EXISTS idx_training_group_natural_key").await?;
        db.execute_unprepared(
            "CREATE UNIQUE INDEX idx_training_group_natural_key
             ON training_group (
                sender_org_id, training_kind_id, site_id, planned_start, planned_end,
                COALESCE(vos_id, 0), COALESCE(position_id, 0),
                COALESCE(course_id, 0), COALESCE(bzvp_program_id, 0)
             )"
        ).await?;

        db.execute_unprepared("ALTER TABLE training_group ALTER COLUMN site_id SET NOT NULL").await?;
        db.execute_unprepared("ALTER TABLE training_group DROP COLUMN IF EXISTS venue_type, DROP COLUMN IF EXISTS training_venue_id, DROP COLUMN IF EXISTS city_id").await?;

        db.execute_unprepared("ALTER TABLE reported_group ALTER COLUMN site_id SET NOT NULL").await?;
        db.execute_unprepared("ALTER TABLE reported_group DROP COLUMN IF EXISTS venue_type, DROP COLUMN IF EXISTS training_venue_id, DROP COLUMN IF EXISTS city_id").await?;

        db.execute_unprepared("ALTER TABLE org DROP COLUMN IF EXISTS current_city_id").await?;

        db.execute_unprepared("DROP TABLE IF EXISTS training_venue").await?;
        db.execute_unprepared("DROP TABLE IF EXISTS city").await?;

        Ok(())
    }
}
