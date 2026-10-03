use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

// Dev-сід: комплексні дані для тестування всіх аспектів застосунку — кілька користувачів із
// різними ролями, групи підготовки різних видів на різних етапах воронки, подання, розбіжності,
// сповіщення, укомплектованість, WhatsApp-адреси та підписки. Усі паролі — "admin123" (той самий
// precomputed argon2id hash, що й у m20261001_000047 — ТІЛЬКИ для розробки).
const ADMIN_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$YWRtaW5fc2VlZF9zYWx0AAAA$opUUnUXe8lkEuazVx7MTJpmX9blm7548lZcRYoJsoJ4";

const SEED_STATEMENTS: &[&str] = &[
    // ── Навчальні центри ──
    r#"INSERT INTO org (kind, short_name) VALUES
        ('edu_institution', '184 нц'),
        ('edu_institution', '197 нц'),
        ('edu_institution', '214 нц'),
        ('edu_institution', '169 нц')"#,
    r#"INSERT INTO subordination (child_org_id, parent_org_id, axis, valid_from)
        SELECT o.id, (SELECT id FROM org WHERE short_name = 'ЧБП'), 'staff', '2022-01-01'
        FROM org o WHERE o.short_name IN ('184 нц', '197 нц', '214 нц', '169 нц')"#,

    // ── Додаткові місця навчання (належать НЦ, не бойовим частинам) ──
    r#"INSERT INTO training_site (org_id, locality) VALUES
        ((SELECT id FROM org WHERE short_name = '184 нц'), 'ПП «Рівне»'),
        ((SELECT id FROM org WHERE short_name = '197 нц'), 'НЦ «Запоріжжя»'),
        ((SELECT id FROM org WHERE short_name = '214 нц'), 'ПП «Черкаське»'),
        ((SELECT id FROM org WHERE short_name = '169 нц'), 'ПП «Широкий Лан»'),
        ((SELECT id FROM org WHERE short_name = '152 нц'), 'ПП «Покровськ»')"#,

    // ── Групи підготовки (7 груп різних видів і стадій) ──

    // Група 1: 128 овмбр, БЗВП БЗВП-3, завершена (минула)
    r#"INSERT INTO training_group (sender_org_id, training_kind_id, bzvp_program_id, site_id,
        planned_start, planned_end, note)
        SELECT
            (SELECT id FROM org WHERE short_name = '128 овмбр'),
            (SELECT id FROM training_kind WHERE code = 'bzvp'),
            (SELECT id FROM bzvp_program WHERE name = 'БЗВП-3'),
            (SELECT id FROM training_site WHERE locality = 'ПП «Рівне»' LIMIT 1),
            '2026-07-01', '2026-08-15', 'Завершена група БЗВП-3'"#,

    // Група 2: 65 омбр, фахова (ВОС 218 — пілоти БпЛА), поточна
    r#"INSERT INTO training_group (sender_org_id, training_kind_id, vos_id,
        position_id, site_id, planned_start, planned_end, note)
        SELECT
            (SELECT id FROM org WHERE short_name = '65 омбр'),
            (SELECT id FROM training_kind WHERE code = 'special'),
            (SELECT id FROM vos WHERE code = '218'),
            (SELECT id FROM "position" WHERE name = 'зовнішній пілот (оператор) БпЛА'),
            (SELECT id FROM training_site WHERE locality = 'НЦ «Запоріжжя»' LIMIT 1),
            '2026-09-15', '2026-11-01', 'Фахова: пілоти Vampire'"#,

    // Група 3: 118 омбр, БЗВП БЗВП-6, запланована (майбутня)
    r#"INSERT INTO training_group (sender_org_id, training_kind_id, bzvp_program_id, site_id,
        planned_start, planned_end)
        SELECT
            (SELECT id FROM org WHERE short_name = '118 омбр'),
            (SELECT id FROM training_kind WHERE code = 'bzvp'),
            (SELECT id FROM bzvp_program WHERE name = 'БЗВП-6'),
            (SELECT id FROM training_site WHERE locality = 'ПП «Черкаське»' LIMIT 1),
            '2026-11-01', '2026-12-20'"#,

    // Група 4: 153 омбр, адаптація, поточна
    r#"INSERT INTO training_group (sender_org_id, training_kind_id, site_id,
        planned_start, planned_end, note)
        SELECT
            (SELECT id FROM org WHERE short_name = '153 омбр'),
            (SELECT id FROM training_kind WHERE code = 'adaptation'),
            (SELECT id FROM training_site WHERE locality = 'ПП «Широкий Лан»' LIMIT 1),
            '2026-09-20', '2026-10-10', 'Адаптація новоприбулих'"#,

    // Група 5: 110 омбр, фахова (ВОС 219 — FPV), завершена
    r#"INSERT INTO training_group (sender_org_id, training_kind_id, vos_id, site_id,
        planned_start, planned_end)
        SELECT
            (SELECT id FROM org WHERE short_name = '110 омбр'),
            (SELECT id FROM training_kind WHERE code = 'special'),
            (SELECT id FROM vos WHERE code = '219'),
            (SELECT id FROM training_site WHERE locality = 'ПП «Покровськ»' LIMIT 1),
            '2026-06-01', '2026-07-30'"#,

    // Група 6: 260 обр ТрО, стажування, щойно розпочата
    r#"INSERT INTO training_group (sender_org_id, training_kind_id, site_id,
        planned_start, planned_end)
        SELECT
            (SELECT id FROM org WHERE short_name = '260 обр ТрО'),
            (SELECT id FROM training_kind WHERE code = 'internship'),
            (SELECT id FROM training_site WHERE locality = 'НЦ «Десна»' LIMIT 1),
            '2026-09-28', '2026-10-30'"#,

    // Група 7: 241 обр ТрО, БЗВП КТЗ, запланована
    r#"INSERT INTO training_group (sender_org_id, training_kind_id, bzvp_program_id, site_id,
        planned_start, planned_end)
        SELECT
            (SELECT id FROM org WHERE short_name = '241 обр ТрО'),
            (SELECT id FROM training_kind WHERE code = 'bzvp'),
            (SELECT id FROM bzvp_program WHERE name = 'КТЗ'),
            (SELECT id FROM training_site WHERE locality = 'НЦ «Десна»' LIMIT 1),
            '2026-11-15', '2026-12-30'"#,

    // ── Воронка подій для завершеної групи 1 (128 овмбр, БЗВП-3) ──
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on)
        SELECT tg.id, 'planned', 30, '2026-07-01'
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '128 овмбр' AND tg.planned_start = '2026-07-01'"#,
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on)
        SELECT tg.id, 'arrived', 28, '2026-07-01'
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '128 овмбр' AND tg.planned_start = '2026-07-01'"#,
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on)
        SELECT tg.id, 'started', 28, '2026-07-02'
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '128 овмбр' AND tg.planned_start = '2026-07-01'"#,
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on, reason_id)
        SELECT tg.id, 'attrition', 3, '2026-07-20',
            (SELECT id FROM attrition_reason WHERE name = 'СЗЧ')
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '128 овмбр' AND tg.planned_start = '2026-07-01'"#,
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on, reason_id)
        SELECT tg.id, 'attrition', 1, '2026-08-01',
            (SELECT id FROM attrition_reason WHERE name = 'лікування/шпиталь')
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '128 овмбр' AND tg.planned_start = '2026-07-01'"#,
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on)
        SELECT tg.id, 'completed', 24, '2026-08-15'
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '128 овмбр' AND tg.planned_start = '2026-07-01'"#,
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on,
        award_order_number, award_order_date)
        SELECT tg.id, 'vos_awarded', 22, '2026-08-20', 'Н-128/43', '2026-08-20'
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '128 овмбр' AND tg.planned_start = '2026-07-01'"#,
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on)
        SELECT tg.id, 'vos_not_awarded', 2, '2026-08-20'
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '128 овмбр' AND tg.planned_start = '2026-07-01'"#,

    // ── Воронка подій для поточної групи 2 (65 омбр, фахова) ──
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on)
        SELECT tg.id, 'planned', 15, '2026-09-15'
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '65 омбр' AND tg.planned_start = '2026-09-15'"#,
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on)
        SELECT tg.id, 'arrived', 14, '2026-09-15'
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '65 омбр' AND tg.planned_start = '2026-09-15'"#,
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on)
        SELECT tg.id, 'started', 14, '2026-09-16'
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '65 омбр' AND tg.planned_start = '2026-09-15'"#,
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on, reason_id)
        SELECT tg.id, 'attrition', 1, '2026-09-25',
            (SELECT id FROM attrition_reason WHERE name = 'не пройшов проміжний контроль')
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '65 омбр' AND tg.planned_start = '2026-09-15'"#,

    // ── Воронка подій для запланованої групи 3 (118 омбр, БЗВП-6) — лише planned ──
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on)
        SELECT tg.id, 'planned', 25, '2026-10-01'
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '118 омбр' AND tg.planned_start = '2026-11-01'"#,

    // ── Воронка подій для поточної групи 4 (153 омбр, адаптація) ──
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on)
        SELECT tg.id, 'planned', 40, '2026-09-20'
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '153 омбр' AND tg.planned_start = '2026-09-20'"#,
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on)
        SELECT tg.id, 'arrived', 38, '2026-09-20'
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '153 омбр' AND tg.planned_start = '2026-09-20'"#,
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on)
        SELECT tg.id, 'started', 37, '2026-09-21'
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '153 омбр' AND tg.planned_start = '2026-09-20'"#,
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on)
        SELECT tg.id, 'added', 5, '2026-09-25'
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '153 омбр' AND tg.planned_start = '2026-09-20'"#,
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on, reason_id)
        SELECT tg.id, 'attrition', 2, '2026-09-28',
            (SELECT id FROM attrition_reason WHERE name = 'не прибув до НЦ')
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '153 омбр' AND tg.planned_start = '2026-09-20'"#,

    // ── Воронка подій для завершеної групи 5 (110 омбр, FPV) ──
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on)
        SELECT tg.id, 'planned', 20, '2026-06-01'
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '110 омбр' AND tg.planned_start = '2026-06-01'"#,
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on)
        SELECT tg.id, 'arrived', 19, '2026-06-01'
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '110 омбр' AND tg.planned_start = '2026-06-01'"#,
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on)
        SELECT tg.id, 'started', 19, '2026-06-02'
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '110 омбр' AND tg.planned_start = '2026-06-01'"#,
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on, reason_id)
        SELECT tg.id, 'attrition', 2, '2026-06-15',
            (SELECT id FROM attrition_reason WHERE name = 'відрахований за станом здоров''я')
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '110 омбр' AND tg.planned_start = '2026-06-01'"#,
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on, reason_id)
        SELECT tg.id, 'attrition', 1, '2026-07-10',
            (SELECT id FROM attrition_reason WHERE name = 'не склав іспит')
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '110 омбр' AND tg.planned_start = '2026-06-01'"#,
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on)
        SELECT tg.id, 'completed', 16, '2026-07-30'
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '110 омбр' AND tg.planned_start = '2026-06-01'"#,
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on,
        award_order_number, award_order_date)
        SELECT tg.id, 'vos_awarded', 15, '2026-08-05', 'Ф-110/21', '2026-08-05'
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '110 омбр' AND tg.planned_start = '2026-06-01'"#,
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on)
        SELECT tg.id, 'vos_not_awarded', 1, '2026-08-05'
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '110 омбр' AND tg.planned_start = '2026-06-01'"#,

    // ── Воронка подій для щойно розпочатої групи 6 (260 обр ТрО, стажування) ──
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on)
        SELECT tg.id, 'planned', 12, '2026-09-28'
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '260 обр ТрО' AND tg.planned_start = '2026-09-28'"#,
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on)
        SELECT tg.id, 'arrived', 11, '2026-09-28'
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '260 обр ТрО' AND tg.planned_start = '2026-09-28'"#,
    r#"INSERT INTO group_event (group_id, event_type, count, occurred_on)
        SELECT tg.id, 'started', 11, '2026-09-29'
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '260 обр ТрО' AND tg.planned_start = '2026-09-28'"#,

    // ── Подання (різні статуси) ──

    // Подання 1: 128 овмбр, committed (для завершеної групи)
    r#"INSERT INTO submission (source_type, reporting_org_id, as_of_date, status)
        SELECT 'form', id, '2026-07-01', 'committed'
        FROM org WHERE short_name = '128 овмбр'"#,
    // Подання 2: 17 АК, committed (табличний імпорт)
    r#"INSERT INTO submission (source_type, reporting_org_id, as_of_date, status)
        SELECT 'table', id, '2026-07-01', 'committed'
        FROM org WHERE short_name = '17 АК'"#,
    // Подання 3: 65 омбр, committed (для поточної групи)
    r#"INSERT INTO submission (source_type, reporting_org_id, as_of_date, status)
        SELECT 'form', id, '2026-09-15', 'committed'
        FROM org WHERE short_name = '65 омбр'"#,
    // Подання 4: 20 АК, committed (агрегат)
    r#"INSERT INTO submission (source_type, reporting_org_id, as_of_date, status)
        SELECT 'table', id, '2026-09-15', 'committed'
        FROM org WHERE short_name = '20 АК'"#,
    // Подання 5: 153 омбр, committed (перше подання)
    r#"INSERT INTO submission (source_type, reporting_org_id, as_of_date, status)
        SELECT 'form', id, '2026-09-20', 'committed'
        FROM org WHERE short_name = '153 омбр'"#,
    // Подання 5b: 153 омбр, committed (тиждень пізніше — для часової розбіжності)
    r#"INSERT INTO submission (source_type, reporting_org_id, as_of_date, status)
        SELECT 'form', id, '2026-09-28', 'committed'
        FROM org WHERE short_name = '153 омбр'"#,
    // Подання 6: 118 омбр, draft (чернетка)
    r#"INSERT INTO submission (source_type, reporting_org_id, as_of_date, status,
        draft_payload)
        SELECT 'form', id, '2026-10-01', 'draft',
            '{"rows":[{"planned":25,"kind":"bzvp"}]}'::jsonb
        FROM org WHERE short_name = '118 омбр'"#,
    // Подання 7: 110 омбр, committed (для завершеної групи)
    r#"INSERT INTO submission (source_type, reporting_org_id, as_of_date, status)
        SELECT 'form', id, '2026-06-01', 'committed'
        FROM org WHERE short_name = '110 омбр'"#,
    // Подання 8: 260 обр ТрО, committed (для розпочатої групи)
    r#"INSERT INTO submission (source_type, reporting_org_id, as_of_date, status)
        SELECT 'form', id, '2026-09-28', 'committed'
        FROM org WHERE short_name = '260 обр ТрО'"#,
    // Подання 9: 241 обр ТрО, rejected (помилкове подання)
    r#"INSERT INTO submission (source_type, reporting_org_id, as_of_date, status)
        SELECT 'form', id, '2026-09-25', 'rejected'
        FROM org WHERE short_name = '241 обр ТрО'"#,
    // Подання 10: 110 омбр, committed (друге подання, як 17 АК подав агрегат)
    r#"INSERT INTO submission (source_type, reporting_org_id, as_of_date, status)
        SELECT 'official_letter', id, '2026-06-01', 'committed'
        FROM org WHERE short_name = '17 АК'"#,

    // ── Reported groups (для горизонтальної звірки) ──

    // Reported group від 128 овмбр (подання 1): planned=30, arrived=28, in_training=28
    r#"INSERT INTO reported_group (submission_id, sender_org_id, training_kind_id,
        bzvp_program_id, site_id, planned_start, planned_end,
        planned_count, arrived_count, in_training_count, matched_group_id)
        SELECT
            s.id,
            (SELECT id FROM org WHERE short_name = '128 овмбр'),
            (SELECT id FROM training_kind WHERE code = 'bzvp'),
            (SELECT id FROM bzvp_program WHERE name = 'БЗВП-3'),
            (SELECT id FROM training_site WHERE locality = 'ПП «Рівне»' LIMIT 1),
            '2026-07-01', '2026-08-15',
            30, 28, 28,
            tg.id
        FROM submission s
        JOIN org so ON s.reporting_org_id = so.id
        CROSS JOIN (
            SELECT tg2.id FROM training_group tg2
            JOIN org o2 ON tg2.sender_org_id = o2.id
            WHERE o2.short_name = '128 овмбр' AND tg2.planned_start = '2026-07-01'
        ) tg
        WHERE so.short_name = '128 овмбр' AND s.as_of_date = '2026-07-01' AND s.status = 'committed'"#,

    // Reported group від 17 АК (подання 2): planned=30, arrived=27, in_training=27
    // (розбіжність з поданням 128 овмбр: arrived 28 vs 27)
    r#"INSERT INTO reported_group (submission_id, sender_org_id, training_kind_id,
        bzvp_program_id, site_id, planned_start, planned_end,
        planned_count, arrived_count, in_training_count, matched_group_id)
        SELECT
            s.id,
            (SELECT id FROM org WHERE short_name = '128 овмбр'),
            (SELECT id FROM training_kind WHERE code = 'bzvp'),
            (SELECT id FROM bzvp_program WHERE name = 'БЗВП-3'),
            (SELECT id FROM training_site WHERE locality = 'ПП «Рівне»' LIMIT 1),
            '2026-07-01', '2026-08-15',
            30, 27, 27,
            tg.id
        FROM submission s
        JOIN org so ON s.reporting_org_id = so.id
        CROSS JOIN (
            SELECT tg2.id FROM training_group tg2
            JOIN org o2 ON tg2.sender_org_id = o2.id
            WHERE o2.short_name = '128 овмбр' AND tg2.planned_start = '2026-07-01'
        ) tg
        WHERE so.short_name = '17 АК' AND s.as_of_date = '2026-07-01'
            AND s.source_type = 'table' AND s.status = 'committed'"#,

    // Reported group від 65 омбр (подання 3)
    r#"INSERT INTO reported_group (submission_id, sender_org_id, training_kind_id,
        vos_id, position_id, site_id, planned_start, planned_end,
        planned_count, arrived_count, in_training_count, matched_group_id)
        SELECT
            s.id,
            (SELECT id FROM org WHERE short_name = '65 омбр'),
            (SELECT id FROM training_kind WHERE code = 'special'),
            (SELECT id FROM vos WHERE code = '218'),
            (SELECT id FROM "position" WHERE name = 'зовнішній пілот (оператор) БпЛА'),
            (SELECT id FROM training_site WHERE locality = 'НЦ «Запоріжжя»' LIMIT 1),
            '2026-09-15', '2026-11-01',
            15, 14, 14,
            tg.id
        FROM submission s
        JOIN org so ON s.reporting_org_id = so.id
        CROSS JOIN (
            SELECT tg2.id FROM training_group tg2
            JOIN org o2 ON tg2.sender_org_id = o2.id
            WHERE o2.short_name = '65 омбр' AND tg2.planned_start = '2026-09-15'
        ) tg
        WHERE so.short_name = '65 омбр' AND s.as_of_date = '2026-09-15' AND s.status = 'committed'"#,

    // Reported group від 20 АК (подання 4): planned=15, arrived=13, in_training=13
    // (розбіжність з поданням 65 омбр: arrived 14 vs 13)
    r#"INSERT INTO reported_group (submission_id, sender_org_id, training_kind_id,
        vos_id, position_id, site_id, planned_start, planned_end,
        planned_count, arrived_count, in_training_count, matched_group_id)
        SELECT
            s.id,
            (SELECT id FROM org WHERE short_name = '65 омбр'),
            (SELECT id FROM training_kind WHERE code = 'special'),
            (SELECT id FROM vos WHERE code = '218'),
            (SELECT id FROM "position" WHERE name = 'зовнішній пілот (оператор) БпЛА'),
            (SELECT id FROM training_site WHERE locality = 'НЦ «Запоріжжя»' LIMIT 1),
            '2026-09-15', '2026-11-01',
            15, 13, 13,
            tg.id
        FROM submission s
        JOIN org so ON s.reporting_org_id = so.id
        CROSS JOIN (
            SELECT tg2.id FROM training_group tg2
            JOIN org o2 ON tg2.sender_org_id = o2.id
            WHERE o2.short_name = '65 омбр' AND tg2.planned_start = '2026-09-15'
        ) tg
        WHERE so.short_name = '20 АК' AND s.as_of_date = '2026-09-15'
            AND s.source_type = 'table' AND s.status = 'committed'"#,

    // Reported group від 110 омбр (подання 7)
    r#"INSERT INTO reported_group (submission_id, sender_org_id, training_kind_id,
        vos_id, site_id, planned_start, planned_end,
        planned_count, arrived_count, in_training_count, matched_group_id)
        SELECT
            s.id,
            (SELECT id FROM org WHERE short_name = '110 омбр'),
            (SELECT id FROM training_kind WHERE code = 'special'),
            (SELECT id FROM vos WHERE code = '219'),
            (SELECT id FROM training_site WHERE locality = 'ПП «Покровськ»' LIMIT 1),
            '2026-06-01', '2026-07-30',
            20, 19, 19,
            tg.id
        FROM submission s
        JOIN org so ON s.reporting_org_id = so.id
        CROSS JOIN (
            SELECT tg2.id FROM training_group tg2
            JOIN org o2 ON tg2.sender_org_id = o2.id
            WHERE o2.short_name = '110 омбр' AND tg2.planned_start = '2026-06-01'
        ) tg
        WHERE so.short_name = '110 омбр' AND s.as_of_date = '2026-06-01' AND s.status = 'committed'"#,

    // Reported group від 17 АК (подання 10): planned=20, arrived=18, in_training=18
    // (розбіжність з поданням 110 омбр: arrived 19 vs 18)
    r#"INSERT INTO reported_group (submission_id, sender_org_id, training_kind_id,
        vos_id, site_id, planned_start, planned_end,
        planned_count, arrived_count, in_training_count, matched_group_id)
        SELECT
            s.id,
            (SELECT id FROM org WHERE short_name = '110 омбр'),
            (SELECT id FROM training_kind WHERE code = 'special'),
            (SELECT id FROM vos WHERE code = '219'),
            (SELECT id FROM training_site WHERE locality = 'ПП «Покровськ»' LIMIT 1),
            '2026-06-01', '2026-07-30',
            20, 18, 18,
            tg.id
        FROM submission s
        JOIN org so ON s.reporting_org_id = so.id
        CROSS JOIN (
            SELECT tg2.id FROM training_group tg2
            JOIN org o2 ON tg2.sender_org_id = o2.id
            WHERE o2.short_name = '110 омбр' AND tg2.planned_start = '2026-06-01'
        ) tg
        WHERE so.short_name = '17 АК' AND s.as_of_date = '2026-06-01'
            AND s.source_type = 'official_letter' AND s.status = 'committed'"#,

    // ── Розбіжності ──
    // Формат values — Vec<(i32, String)>: [[submission_id, "значення"], ...] — той самий,
    // що domain::reconciliation::HorizontalDiscrepancy.values і list_discrepancies десеріалізує.

    // Горизонтальна розбіжність 1: 128 овмбр vs 17 АК — arrived_count (28 vs 27), resolved
    r#"INSERT INTO discrepancy (kind, org_id, group_id, as_of, metric, values, status, resolution_note)
        SELECT 'horizontal',
            (SELECT id FROM org WHERE short_name = '128 овмбр'),
            tg.id, '2026-07-01', 'arrived_count',
            jsonb_build_array(
                jsonb_build_array(
                    (SELECT s.id FROM submission s JOIN org so ON s.reporting_org_id = so.id
                     WHERE so.short_name = '128 овмбр' AND s.as_of_date = '2026-07-01'
                       AND s.status = 'committed' AND s.source_type = 'form' LIMIT 1),
                    '28'),
                jsonb_build_array(
                    (SELECT s.id FROM submission s JOIN org so ON s.reporting_org_id = so.id
                     WHERE so.short_name = '17 АК' AND s.as_of_date = '2026-07-01'
                       AND s.source_type = 'table' AND s.status = 'committed' LIMIT 1),
                    '27')
            ),
            'resolved',
            'Уточнено: 1 о/с прибув пізніше, дані 128 овмбр коректні'
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '128 овмбр' AND tg.planned_start = '2026-07-01'"#,

    // Горизонтальна розбіжність 2: 65 омбр vs 20 АК — arrived_count (14 vs 13), open
    r#"INSERT INTO discrepancy (kind, org_id, group_id, as_of, metric, values, status)
        SELECT 'horizontal',
            (SELECT id FROM org WHERE short_name = '65 омбр'),
            tg.id, '2026-09-15', 'arrived_count',
            jsonb_build_array(
                jsonb_build_array(
                    (SELECT s.id FROM submission s JOIN org so ON s.reporting_org_id = so.id
                     WHERE so.short_name = '65 омбр' AND s.as_of_date = '2026-09-15'
                       AND s.status = 'committed' LIMIT 1),
                    '14'),
                jsonb_build_array(
                    (SELECT s.id FROM submission s JOIN org so ON s.reporting_org_id = so.id
                     WHERE so.short_name = '20 АК' AND s.as_of_date = '2026-09-15'
                       AND s.source_type = 'table' AND s.status = 'committed' LIMIT 1),
                    '13')
            ),
            'open'
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '65 омбр' AND tg.planned_start = '2026-09-15'"#,

    // Горизонтальна розбіжність 3: 110 омбр vs 17 АК — arrived_count (19 vs 18), notified
    r#"INSERT INTO discrepancy (kind, org_id, group_id, as_of, metric, values, status)
        SELECT 'horizontal',
            (SELECT id FROM org WHERE short_name = '110 омбр'),
            tg.id, '2026-06-01', 'arrived_count',
            jsonb_build_array(
                jsonb_build_array(
                    (SELECT s.id FROM submission s JOIN org so ON s.reporting_org_id = so.id
                     WHERE so.short_name = '110 омбр' AND s.as_of_date = '2026-06-01'
                       AND s.status = 'committed' AND s.source_type = 'form' LIMIT 1),
                    '19'),
                jsonb_build_array(
                    (SELECT s.id FROM submission s JOIN org so ON s.reporting_org_id = so.id
                     WHERE so.short_name = '17 АК' AND s.as_of_date = '2026-06-01'
                       AND s.source_type = 'official_letter' AND s.status = 'committed' LIMIT 1),
                    '18')
            ),
            'notified'
        FROM training_group tg
        JOIN org o ON tg.sender_org_id = o.id
        WHERE o.short_name = '110 омбр' AND tg.planned_start = '2026-06-01'"#,

    // Розбіжність data_quality: 241 обр ТрО — rejected подання (помилковий формат)
    r#"INSERT INTO discrepancy (kind, org_id, as_of, metric, values, status)
        SELECT 'data_quality',
            (SELECT id FROM org WHERE short_name = '241 обр ТрО'),
            '2026-09-25', 'source_format',
            jsonb_build_array(
                jsonb_build_array(s.id, 'некоректний формат дати')
            ),
            'dismissed'
        FROM submission s
        JOIN org o ON s.reporting_org_id = o.id
        WHERE o.short_name = '241 обр ТрО' AND s.as_of_date = '2026-09-25'
          AND s.status = 'rejected'"#,

    // Розбіжність temporal: 153 омбр — in_training змінилось між двома тижнями
    r#"INSERT INTO discrepancy (kind, org_id, as_of, metric, values, status)
        SELECT 'temporal',
            (SELECT id FROM org WHERE short_name = '153 омбр'),
            '2026-09-28', 'total',
            jsonb_build_array(
                jsonb_build_array(s1.id, '37'),
                jsonb_build_array(s2.id, '40')
            ),
            'in_progress'
        FROM (
            SELECT s.id FROM submission s JOIN org o ON s.reporting_org_id = o.id
            WHERE o.short_name = '153 омбр' AND s.as_of_date = '2026-09-20' LIMIT 1
        ) s1
        CROSS JOIN (
            SELECT s.id FROM submission s JOIN org o ON s.reporting_org_id = o.id
            WHERE o.short_name = '153 омбр' AND s.as_of_date = '2026-09-28' LIMIT 1
        ) s2"#,

    // ── Сповіщення (notification bell) ──

    // Непрочитані сповіщення
    r#"INSERT INTO notification (org_id, kind, title, body, link, is_read) VALUES
        ((SELECT id FROM org WHERE short_name = '65 омбр'),
         'discrepancy', 'Нова розбіжність', 'Розбіжність arrived: 65 омбр vs 20 АК',
         '/discrepancies', false),
        ((SELECT id FROM org WHERE short_name = '110 омбр'),
         'discrepancy', 'Розбіжність оповіщено', 'Горизонтальна розбіжність arrived: 110 омбр vs 17 АК',
         '/discrepancies', false),
        ((SELECT id FROM org WHERE short_name = '118 омбр'),
         'submission', 'Чернетка збережена', 'Чернетку подання на 2026-10-01 збережено',
         NULL, false),
        ((SELECT id FROM org WHERE short_name = '153 омбр'),
         'discrepancy', 'Часова розбіжність', 'in_training: зміна +3 за тиждень',
         '/discrepancies', false),
        ((SELECT id FROM org WHERE short_name = '260 обр ТрО'),
         'submission', 'Подання зафіксовано', 'Стажування: 12 запланов., 11 прибули',
         '/training-form', false)"#,

    // Прочитані сповіщення
    r#"INSERT INTO notification (org_id, kind, title, body, link, is_read, created_at) VALUES
        ((SELECT id FROM org WHERE short_name = '128 овмбр'),
         'discrepancy', 'Розбіжність вирішена', 'arrived: 128 овмбр vs 17 АК — вирішено',
         '/discrepancies', true, '2026-07-05 10:00:00+03'),
        ((SELECT id FROM org WHERE short_name = '128 овмбр'),
         'submission', 'Подання зафіксовано', 'БЗВП-3: подано через форму',
         '/training-form', true, '2026-07-01 14:00:00+03'),
        ((SELECT id FROM org WHERE short_name = '110 омбр'),
         'submission', 'Подання зафіксовано', 'Фахова FPV: 20 запланов.',
         '/training-form', true, '2026-06-01 09:00:00+03'),
        ((SELECT id FROM org WHERE short_name = '65 омбр'),
         'submission', 'Подання зафіксовано', 'Фахова пілоти: 15 запланов.',
         '/training-form', true, '2026-09-15 11:00:00+03'),
        ((SELECT id FROM org WHERE short_name = '241 обр ТрО'),
         'submission', 'Подання відхилено', 'Помилка формату, подання відхилено',
         NULL, true, '2026-09-25 16:00:00+03'),
        ((SELECT id FROM org WHERE short_name = '128 овмбр'),
         'training', 'Групу завершено', 'БЗВП-3: 24 осіб завершили',
         '/training-form', true, '2026-08-15 12:00:00+03'),
        ((SELECT id FROM org WHERE short_name = '110 омбр'),
         'training', 'Групу завершено', 'FPV: 16 осіб завершили',
         '/training-form', true, '2026-07-30 15:00:00+03'),
        ((SELECT id FROM org WHERE short_name = '110 омбр'),
         'training', 'ВОС присвоєно', 'Наказ Ф-110/21: 15 осіб отримали ВОС',
         '/training-form', true, '2026-08-05 10:00:00+03')"#,

    // ── Укомплектованість (staffing_snapshot + staffing_metric) ──

    // 128 овмбр: командири відділень
    r#"INSERT INTO staffing_snapshot (org_id, as_of, category)
        SELECT id, '2026-09-01', 'squad_leaders'
        FROM org WHERE short_name = '128 овмбр'"#,
    r#"INSERT INTO staffing_metric (snapshot_id, metric, value)
        SELECT currval('staffing_snapshot_id_seq'), m.metric, m.value
        FROM (VALUES
            ('by_tos', 120), ('by_list', 95), ('present', 88),
            ('trained_sergeant', 45), ('in_training', 28),
            ('planned_next_month', 10), ('need_training', 12)
        ) AS m(metric, value)"#,

    // 128 овмбр: інструктори
    r#"INSERT INTO staffing_snapshot (org_id, as_of, category)
        SELECT id, '2026-09-01', 'instructors'
        FROM org WHERE short_name = '128 овмбр'"#,
    r#"INSERT INTO staffing_metric (snapshot_id, metric, value)
        SELECT currval('staffing_snapshot_id_seq'), m.metric, m.value
        FROM (VALUES
            ('by_tos', 30), ('by_list', 22), ('present', 20),
            ('trained_kibr', 12), ('in_training', 5),
            ('planned_next_month', 3), ('need_training', 5)
        ) AS m(metric, value)"#,

    // 65 омбр: командири відділень
    r#"INSERT INTO staffing_snapshot (org_id, as_of, category)
        SELECT id, '2026-09-01', 'squad_leaders'
        FROM org WHERE short_name = '65 омбр'"#,
    r#"INSERT INTO staffing_metric (snapshot_id, metric, value)
        SELECT currval('staffing_snapshot_id_seq'), m.metric, m.value
        FROM (VALUES
            ('by_tos', 80), ('by_list', 65), ('present', 60),
            ('trained_sergeant', 30), ('in_training', 14),
            ('planned_next_month', 8), ('need_training', 13)
        ) AS m(metric, value)"#,

    // 110 омбр: командири відділень
    r#"INSERT INTO staffing_snapshot (org_id, as_of, category)
        SELECT id, '2026-09-01', 'squad_leaders'
        FROM org WHERE short_name = '110 омбр'"#,
    r#"INSERT INTO staffing_metric (snapshot_id, metric, value)
        SELECT currval('staffing_snapshot_id_seq'), m.metric, m.value
        FROM (VALUES
            ('by_tos', 100), ('by_list', 82), ('present', 75),
            ('trained_sergeant', 40), ('in_training', 16),
            ('planned_next_month', 5), ('need_training', 14)
        ) AS m(metric, value)"#,

    // 153 омбр: командири відділень
    r#"INSERT INTO staffing_snapshot (org_id, as_of, category)
        SELECT id, '2026-09-01', 'squad_leaders'
        FROM org WHERE short_name = '153 омбр'"#,
    r#"INSERT INTO staffing_metric (snapshot_id, metric, value)
        SELECT currval('staffing_snapshot_id_seq'), m.metric, m.value
        FROM (VALUES
            ('by_tos', 90), ('by_list', 70), ('present', 65),
            ('trained_sergeant', 28), ('in_training', 37),
            ('planned_next_month', 0), ('need_training', 0)
        ) AS m(metric, value)"#,
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for stmt in SEED_STATEMENTS {
            db.execute_unprepared(stmt).await?;
        }

        // ── Користувачі (display_name — роль, не ПІБ; пароль = "admin123" для всіх) ──
        for (login, display) in [
            ("editor_17ak", "Редактор 17 АК"),
            ("editor_20ak", "Редактор 20 АК"),
            ("viewer_pivden", "Спостерігач УВ(с)"),
            ("editor_152nc", "Редактор 152 НЦ"),
            ("viewer_30kmp", "Спостерігач 30 КМП"),
        ] {
            db.execute_unprepared(&format!(
                "INSERT INTO user_account (login, password_hash, display_name) \
                 VALUES ('{login}', '{ADMIN_HASH}', '{display}') \
                 ON CONFLICT (login) DO NOTHING"
            ))
            .await?;
        }

        // ── Ролі користувачів ──

        // editor_17ak: org_editor для 17 АК і всіх його підлеглих частин
        db.execute_unprepared(
            "INSERT INTO user_role (user_id, org_id, role)
             SELECT ua.id, o.id, 'org_editor'
             FROM user_account ua, org o
             WHERE ua.login = 'editor_17ak'
               AND o.short_name IN ('17 АК', '128 овмбр', '128 огшбр', '253 ошп',
                   '153 омбр', '65 омбр', '118 омбр', '260 обр ТрО', '7 орб', '151 омбр',
                   '110 омбр', '241 обр ТрО')
             ON CONFLICT DO NOTHING",
        )
        .await?;

        // editor_20ak: org_editor для 20 АК і підлеглих
        db.execute_unprepared(
            "INSERT INTO user_role (user_id, org_id, role)
             SELECT ua.id, o.id, 'org_editor'
             FROM user_account ua, org o
             WHERE ua.login = 'editor_20ak'
               AND o.short_name IN ('20 АК', '67 омбр')
             ON CONFLICT DO NOTHING",
        )
        .await?;

        // viewer_pivden: viewer для УВ(с) і всіх
        db.execute_unprepared(
            "INSERT INTO user_role (user_id, org_id, role)
             SELECT ua.id, o.id, 'viewer'
             FROM user_account ua, org o
             WHERE ua.login = 'viewer_pivden'
             ON CONFLICT DO NOTHING",
        )
        .await?;

        // editor_152nc: org_editor лише для 152 нц
        db.execute_unprepared(
            "INSERT INTO user_role (user_id, org_id, role)
             SELECT ua.id, o.id, 'org_editor'
             FROM user_account ua, org o
             WHERE ua.login = 'editor_152nc' AND o.short_name = '152 нц'
             ON CONFLICT DO NOTHING",
        )
        .await?;

        // viewer_30kmp: viewer для 30 КМП
        db.execute_unprepared(
            "INSERT INTO user_role (user_id, org_id, role)
             SELECT ua.id, o.id, 'viewer'
             FROM user_account ua, org o
             WHERE ua.login = 'viewer_30kmp' AND o.short_name = '30 КМП'
             ON CONFLICT DO NOTHING",
        )
        .await?;

        // ── WhatsApp-адреси ──

        // Бот для УВ(с) "Південь" (створений адміном)
        db.execute_unprepared(
            "INSERT INTO whatsapp_destination (user_id, org_id, kind, phone_masked)
             SELECT
                 (SELECT id FROM user_account WHERE login = 'admin'),
                 (SELECT id FROM org WHERE short_name = 'УВ(с) \"Південь\"'),
                 'bot', '+380*****0001'",
        )
        .await?;

        // Особистий номер адміна
        db.execute_unprepared(
            "INSERT INTO whatsapp_destination (user_id, org_id, kind, phone_masked)
             SELECT
                 (SELECT id FROM user_account WHERE login = 'admin'),
                 (SELECT id FROM org WHERE short_name = 'УВ(с) \"Південь\"'),
                 'personal', '+380*****1234'",
        )
        .await?;

        // Особистий номер editor_17ak
        db.execute_unprepared(
            "INSERT INTO whatsapp_destination (user_id, org_id, kind, phone_masked)
             SELECT
                 (SELECT id FROM user_account WHERE login = 'editor_17ak'),
                 (SELECT id FROM org WHERE short_name = '17 АК'),
                 'personal', '+380*****5678'",
        )
        .await?;

        // Особистий номер editor_20ak
        db.execute_unprepared(
            "INSERT INTO whatsapp_destination (user_id, org_id, kind, phone_masked)
             SELECT
                 (SELECT id FROM user_account WHERE login = 'editor_20ak'),
                 (SELECT id FROM org WHERE short_name = '20 АК'),
                 'personal', '+380*****9012'",
        )
        .await?;

        // ── Підписки на сповіщення ──

        // Бот УВ(с) підписаний на всі org-рівня
        db.execute_unprepared(
            "INSERT INTO notification_subscription (destination_id, notification_type_code)
             SELECT wd.id, nt.code
             FROM whatsapp_destination wd, notification_type nt
             WHERE wd.phone_masked = '+380*****0001' AND wd.kind = 'bot'
               AND nt.scope = 'org'",
        )
        .await?;

        // Адмін підписаний на system (personal)
        db.execute_unprepared(
            "INSERT INTO notification_subscription (destination_id, notification_type_code)
             SELECT wd.id, 'system'
             FROM whatsapp_destination wd
             WHERE wd.phone_masked = '+380*****1234' AND wd.kind = 'personal'",
        )
        .await?;

        // editor_17ak підписаний на discrepancy_opened і submission_committed
        db.execute_unprepared(
            "INSERT INTO notification_subscription (destination_id, notification_type_code)
             SELECT wd.id, nt.code
             FROM whatsapp_destination wd, notification_type nt
             WHERE wd.phone_masked = '+380*****5678'
               AND nt.code IN ('discrepancy_opened', 'submission_committed')",
        )
        .await?;

        // ── Група розсилки ──

        db.execute_unprepared(
            "INSERT INTO notification_group (org_id, name, created_by)
             SELECT
                 (SELECT id FROM org WHERE short_name = 'УВ(с) \"Південь\"'),
                 'Оперативні сповіщення',
                 (SELECT id FROM user_account WHERE login = 'admin')",
        )
        .await?;

        // Учасники групи: бот УВ(с) + адмін
        db.execute_unprepared(
            "INSERT INTO notification_group_member (group_id, destination_id)
             SELECT ng.id, wd.id
             FROM notification_group ng, whatsapp_destination wd
             WHERE ng.name = 'Оперативні сповіщення'
               AND wd.phone_masked IN ('+380*****0001', '+380*****1234')",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared("DELETE FROM notification_group_member WHERE group_id IN (SELECT id FROM notification_group WHERE name = 'Оперативні сповіщення')").await?;
        db.execute_unprepared("DELETE FROM notification_group WHERE name = 'Оперативні сповіщення'").await?;
        db.execute_unprepared("DELETE FROM notification_subscription").await?;
        db.execute_unprepared("DELETE FROM whatsapp_destination").await?;

        db.execute_unprepared("DELETE FROM staffing_metric").await?;
        db.execute_unprepared("DELETE FROM staffing_snapshot").await?;

        db.execute_unprepared(
            "DELETE FROM notification WHERE title IN (
                'Нова розбіжність', 'Розбіжність оповіщено', 'Чернетка збережена',
                'Часова розбіжність', 'Подання зафіксовано', 'Розбіжність вирішена',
                'Подання відхилено', 'Групу завершено', 'ВОС присвоєно'
            )"
        ).await?;

        db.execute_unprepared("DELETE FROM discrepancy").await?;
        db.execute_unprepared("DELETE FROM reported_group").await?;

        // Submissions created by this seed (by date range not overlapping with older seeds)
        db.execute_unprepared(
            "DELETE FROM submission WHERE source_type != 'archive_seed' \
             AND reporting_org_id IN (SELECT id FROM org WHERE short_name IN ( \
                 '128 овмбр','17 АК','65 омбр','20 АК','153 омбр','118 омбр', \
                 '110 омбр','260 обр ТрО','241 обр ТрО'))",
        )
        .await?;

        // Events for groups created here (sender_org + planned_start combos unique to this seed)
        db.execute_unprepared(
            "DELETE FROM group_event WHERE group_id IN (
                SELECT tg.id FROM training_group tg
                JOIN org o ON tg.sender_org_id = o.id
                WHERE (o.short_name, tg.planned_start::text) IN (
                    ('128 овмбр','2026-07-01'), ('65 омбр','2026-09-15'),
                    ('118 омбр','2026-11-01'), ('153 омбр','2026-09-20'),
                    ('110 омбр','2026-06-01'), ('260 обр ТрО','2026-09-28'),
                    ('241 обр ТрО','2026-11-15')
                )
            )",
        )
        .await?;

        db.execute_unprepared(
            "DELETE FROM training_group WHERE sender_org_id IN (
                SELECT id FROM org WHERE short_name IN (
                    '128 овмбр','65 омбр','118 омбр','153 омбр',
                    '110 омбр','260 обр ТрО','241 обр ТрО'
                )
            ) AND id != 3", // preserve the original seed group (152 нц)
        )
        .await?;

        db.execute_unprepared(
            "DELETE FROM training_site WHERE locality IN (
                'ПП «Рівне»','НЦ «Запоріжжя»','ПП «Черкаське»','ПП «Широкий Лан»','ПП «Покровськ»'
            )",
        )
        .await?;

        db.execute_unprepared(
            "DELETE FROM subordination WHERE child_org_id IN (
                SELECT id FROM org WHERE short_name IN ('184 нц','197 нц','214 нц','169 нц')
            )",
        )
        .await?;
        db.execute_unprepared(
            "DELETE FROM org WHERE short_name IN ('184 нц','197 нц','214 нц','169 нц')",
        )
        .await?;

        db.execute_unprepared(
            "DELETE FROM user_role WHERE user_id IN (
                SELECT id FROM user_account WHERE login IN (
                    'editor_17ak','editor_20ak','viewer_pivden','editor_152nc','viewer_30kmp'
                )
            )",
        )
        .await?;
        db.execute_unprepared(
            "DELETE FROM user_account WHERE login IN (
                'editor_17ak','editor_20ak','viewer_pivden','editor_152nc','viewer_30kmp'
            )",
        )
        .await?;

        Ok(())
    }
}
