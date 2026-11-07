use sea_orm_migration::prelude::*;

/// Модуль `personnel` (рішення `.claude/decisions/personnel-named-persons.md`): особи поіменно до подій групи.
/// Створює лише ПОРОЖНІ таблиці — дані з'являються, лише коли сервер запущено з `PERSONNEL_ENABLED=1`
/// (за замовчуванням вимкнено до юридичної перевірки).
///
/// ВАЖЛИВО: на `person` НЕ вішаємо generic-тригер `audit_log` — він пише `to_jsonb(NEW)` з усіма полями,
/// тобто ПІБ потрапили б в аудит. Замість цього — `personnel_access_log` лише з ідентифікаторами.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, db: &SchemaManager) -> Result<(), DbErr> {
        let c = db.get_connection();
        c.execute_unprepared(
            "CREATE TABLE person (
                id          serial PRIMARY KEY,
                rank        text,
                last_name   text NOT NULL,
                first_name  text NOT NULL,
                birth_date  date,
                -- майбутні поля (по батькові, жетон, посада…) без нової міграції на кожне
                extra       jsonb NOT NULL DEFAULT '{}'::jsonb,
                -- нормалізований ключ 'прізвище|ім'я|дата' для пошуку дублів (див. domain::normalize)
                norm_key    text NOT NULL,
                created_by  integer REFERENCES user_account(id),
                created_at  timestamptz NOT NULL DEFAULT now(),
                deleted_at  timestamptz
            )",
        )
        .await?;
        c.execute_unprepared("CREATE INDEX idx_person_norm_key ON person (norm_key) WHERE deleted_at IS NULL").await?;
        c.execute_unprepared(
            "CREATE TABLE group_event_person (
                event_id   integer NOT NULL REFERENCES group_event(id) ON DELETE CASCADE,
                person_id  integer NOT NULL REFERENCES person(id) ON DELETE RESTRICT,
                linked_at  timestamptz NOT NULL DEFAULT now(),
                PRIMARY KEY (event_id, person_id)
            )",
        )
        .await?;
        c.execute_unprepared("CREATE INDEX idx_group_event_person_person ON group_event_person (person_id)").await?;
        c.execute_unprepared(
            "CREATE TABLE personnel_access_log (
                id         bigserial PRIMARY KEY,
                action     text NOT NULL CHECK (action IN ('read', 'create', 'link', 'unlink')),
                person_id  integer,
                event_id   integer,
                -- лише 'org_id:role' актора (як audit_log.actor), без ПІБ
                actor      text,
                at         timestamptz NOT NULL DEFAULT now()
            )",
        )
        .await?;
        c.execute_unprepared("CREATE INDEX idx_personnel_access_log_person ON personnel_access_log (person_id, at)").await?;
        Ok(())
    }

    async fn down(&self, db: &SchemaManager) -> Result<(), DbErr> {
        let c = db.get_connection();
        c.execute_unprepared("DROP TABLE IF EXISTS personnel_access_log").await?;
        c.execute_unprepared("DROP TABLE IF EXISTS group_event_person").await?;
        c.execute_unprepared("DROP TABLE IF EXISTS person").await?;
        Ok(())
    }
}
