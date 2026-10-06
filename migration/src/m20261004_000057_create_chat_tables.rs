use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20261004_000057_create_chat_tables"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // ── chat_room ──
        db.execute_unprepared(
            "CREATE TABLE chat_room (
                id          SERIAL PRIMARY KEY,
                name        TEXT    NOT NULL,
                kind        TEXT    NOT NULL DEFAULT 'general'
                            CHECK (kind IN ('general','org','direct')),
                org_id      INTEGER REFERENCES org(id),
                emoji       TEXT,
                created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
                is_active   BOOLEAN     NOT NULL DEFAULT TRUE
            )",
        )
        .await?;

        // ── chat_message ──
        db.execute_unprepared(
            "CREATE TABLE chat_message (
                id                 SERIAL PRIMARY KEY,
                room_id            INTEGER NOT NULL REFERENCES chat_room(id),
                sender_id          INTEGER NOT NULL REFERENCES user_account(id),
                kind               TEXT    NOT NULL DEFAULT 'text'
                                   CHECK (kind IN ('text','image','video','voice','file','system')),
                body               TEXT    NOT NULL DEFAULT '',
                media_url          TEXT,
                media_mime         TEXT,
                media_duration_sec REAL,
                reply_to_id        INTEGER REFERENCES chat_message(id),
                created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
                updated_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
                deleted_at         TIMESTAMPTZ
            )",
        )
        .await?;

        db.execute_unprepared(
            "CREATE INDEX idx_chat_message_room_ts ON chat_message(room_id, created_at DESC)"
        ).await?;
        db.execute_unprepared(
            "CREATE INDEX idx_chat_message_sender ON chat_message(sender_id)"
        ).await?;

        // ── chat_room_member (who can see / last-read tracking) ──
        db.execute_unprepared(
            "CREATE TABLE chat_room_member (
                room_id     INTEGER NOT NULL REFERENCES chat_room(id),
                user_id     INTEGER NOT NULL REFERENCES user_account(id),
                last_read_at TIMESTAMPTZ,
                joined_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
                PRIMARY KEY (room_id, user_id)
            )",
        )
        .await?;

        // ── Seed rooms ──
        // General room for everyone
        db.execute_unprepared(
            "INSERT INTO chat_room (name, kind, emoji) VALUES ('Загальний', 'general', '📢')"
        ).await?;

        // Org-based rooms for top-level command structures
        db.execute_unprepared(
            "INSERT INTO chat_room (name, kind, org_id, emoji)
             SELECT short_name, 'org', id, '🏛️'
             FROM org
             WHERE id IN (
                 SELECT DISTINCT ancestor_id FROM subordination_closure
                 WHERE depth = 1
                 UNION
                 SELECT id FROM org WHERE id NOT IN (SELECT descendant_id FROM subordination_closure WHERE depth > 0)
             )
             ORDER BY id
             LIMIT 10"
        ).await?;

        // Add all active users to the general room
        db.execute_unprepared(
            "INSERT INTO chat_room_member (room_id, user_id)
             SELECT
                 (SELECT id FROM chat_room WHERE kind = 'general' ORDER BY id LIMIT 1),
                 id
             FROM user_account
             WHERE is_active = TRUE"
        ).await?;

        // Add users to their org rooms
        db.execute_unprepared(
            "INSERT INTO chat_room_member (room_id, user_id)
             SELECT cr.id, ur.user_id
             FROM chat_room cr
             JOIN user_role ur ON ur.org_id = cr.org_id
             JOIN user_account ua ON ua.id = ur.user_id AND ua.is_active = TRUE
             WHERE cr.kind = 'org'
             ON CONFLICT DO NOTHING"
        ).await?;

        // ── Seed messages ──
        // Some initial messages in the general room to make it look alive
        let general_room = "(SELECT id FROM chat_room WHERE kind = 'general' ORDER BY id LIMIT 1)";

        let seed_msgs = [
            ("admin", "text", "Вітаю всіх у загальному чаті Вишколу! Тут можна обговорювати поточні питання підготовки.", "2026-10-01 08:00:00+03"),
            ("admin", "text", "Нагадую: дані за поточний тиждень потрібно внести до п'ятниці 18:00. Графік — delta:doc:Графік-БЗВП-жовтень", "2026-10-01 09:15:00+03"),
            ("editor_17ak", "text", "Добрий день! Підтверджую отримання даних від підрозділів 17 АК.", "2026-10-01 10:30:00+03"),
            ("editor_20ak", "text", "Дані по 20 АК теж готові. Деталі в delta:table:Зведення-20АК", "2026-10-01 10:45:00+03"),
            ("admin", "text", "Дякую. Хто ще не подав — прошу не затягувати. delta:Граніт — як справи з терміновими?", "2026-10-01 11:00:00+03"),
            ("editor_152nc", "text", "Працюю над цим. Оновив delta:wiki:Порядок-подання-даних — додав нові кроки 👍", "2026-10-01 11:20:00+03"),
            ("editor_17ak", "text", "До речі, подивіться /discrepancies — є кілька нових розбіжностей по БЗВП.", "2026-10-02 08:30:00+03"),
            ("admin", "text", "Побачив. Створив delta:board:Розбіжності-жовтень щоб трекати. Усім дякую за оперативність!", "2026-10-02 09:00:00+03"),
            ("editor_20ak", "text", "Завантажив нові дані. Перевірте /data — все коректно?", "2026-10-03 14:00:00+03"),
            ("editor_17ak", "text", "Перевірив, все ок. Тільки по 128 ОВМБР є питання — напишу в delta:Сокіл. Протокол наради — delta:doc:Протокол-03-10", "2026-10-03 14:30:00+03"),
            ("admin", "system", "📋 Нова розбіжність виявлена: ФАГ vs Терміни по групі БЗВП #11", "2026-10-03 15:00:00+03"),
            ("editor_152nc", "text", "Підтверджую дані по 152 НЦ. Все звірено з /training. Заповнив delta:form:Звіт-за-тиждень", "2026-10-04 09:00:00+03"),
            ("admin", "text", "Нарада delta:cal:Нарада-штабу завтра о 10:00. Порядок денний — delta:wiki:Порядок-денний-нарада-05-10. Готуємо delta:doc:Доповідь-БЗВП", "2026-10-04 10:00:00+03"),
        ];

        // Відправник — за login, не літеральним id: на чистій БД (прод) seed_comprehensive видає
        // інші id, ніж на dev (зловлено на першому прод-запуску, як і в 000056).
        for (sender_login, kind, body, ts) in &seed_msgs {
            db.execute_unprepared(&format!(
                "INSERT INTO chat_message (room_id, sender_id, kind, body, created_at, updated_at) \
                 VALUES ({general_room}, (SELECT id FROM user_account WHERE login = '{sender_login}'), '{kind}', $str${body}$str$, '{ts}'::timestamptz, '{ts}'::timestamptz)"
            )).await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared("DROP TABLE IF EXISTS chat_room_member CASCADE").await?;
        db.execute_unprepared("DROP TABLE IF EXISTS chat_message CASCADE").await?;
        db.execute_unprepared("DROP TABLE IF EXISTS chat_room CASCADE").await?;
        Ok(())
    }
}
