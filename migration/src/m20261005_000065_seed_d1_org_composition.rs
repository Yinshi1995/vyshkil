use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

// Повний склад органів на 26.09.2026 — з щоденних зведень D1 (39 тиждень, аркуш "26.09") по
// 17 АК / 20 АК / 30 КМП / 7 КШР / ОТУ "Одеса" / ЧБП (docs/source-analysis.md §2: "Склад органів
// на 26.09"). Узято лише назви частин і належність до органу (нормалізовані похідні, без
// показників). Ідемпотентно: вставляється тільки відсутнє — однаково лягає на dev-БД і на прод.
//
// Частини з блоку "у штатному підпорядкуванні, але не виконують бойові завдання в смузі
// УВ(с)" — так само штатне підпорядкування органу. 110 омбр у списку 20 АК — оперативне
// (штатне 17 АК вже є з 000011) — повторно не вставляється.
// Дата початку невідома з джерела — '2022-01-01', як у 000011; basis фіксує джерело.
//
// Лапки — лише латинські ("), не «» (вимога замовника); заодно виправлено вже наявні дані.

const ORGANS: &[(&str, &[&str])] = &[
    ("17 АК", &[
        "128 огшбр", "128 овмбр", "65 омбр", "118 омбр", "422 оп БпС", "54 оабр", "101 оптб",
        "241 обр ТрО", "153 омбр", "260 обр ТрО", "253 ошп", "7 орб", "529 обоо", "124 обз",
        "1223 обп", "186 обмз", "514 орвб", "1065 сфпз", "1172 зрдн", "93 оптб", "148 цзодт",
        "165 крп", "12 кп ППО", "110 омбр", "151 омбр",
    ]),
    ("20 АК", &[
        "23 омбр", "31 омбр", "37 обрмп", "141 омбр", "110 омбр", "160 омбр", "23 орб", "60 оабр",
        "98 оптб", "123 обз", "128 обмз", "169 крп", "423 опБпС", "433 опБпС", "519 орвб",
        "533 обоо", "1201 озрдн", "1226 обп", "118 обр ТрО", "154 цзодт", "13 кппо", "298 вфпз",
        "77 пусз", "центр ППП 20 АК", "33 омбр", "17 овмбр",
    ]),
    ("30 КМП", &[
        "42 обрмп", "162 омбр", "34 обрмп", "102 обр ТрО", "121 обр ТрО", "406 оабр", "32 оабр",
        "15 опп", "140 орб", "145 крп", "67 обл", "310 опРЕБ", "101 озрдн", "80 обу",
        "426 опБпС", "18 оптб",
    ]),
    ("7 КШР", &[
        "154 омбр", "142 омбр", "225 ошп", "92 ошбр", "5 ошбр", "61 омбр", "44 оабр",
        "237 обБпС", "87 обу", "231 обл",
    ]),
    ("ОТУ \"Одеса\"", &[
        "122 обр ТрО", "123 обр БпС", "108 обр ТрО", "11 бр НГУ", "2 прикз", "17 прикз",
        "25 прикз", "26 прикз", "34 об НГУ", "СО \"Південноукраїнськ\"", "18 об НГУ",
    ]),
    ("ЧБП", &["16 обрп", "131 орб", "306 оп РЕБ", "363 обоо", "38 зрп", "7 опз"]),
];

// Не військові частини в строгому сенсі — kind 'other'.
const OTHER_KIND: &[&str] = &["центр ППП 20 АК", "СО \"Південноукраїнськ\""];

// Варіанти написання з джерела, що вказують на вже наявні org (без перейменування — історія
// назв не переписується, лише alias для розпізнавання).
const VARIANTS: &[(&str, &str)] = &[
    ("1172 зрдн", "1172 озрдн"),
    ("5 ошбр", "5 омбр"),
    ("426 опббс", "426 опБпС"),
    ("310 опреб", "310 опРЕБ"),
    ("237 оббпс", "237 обБпС"),
    ("118 обр тро", "118 обр ТрО"),
    ("центр ППП", "центр ППП 20 АК"),
];

const BASIS: &str = "D1, 39 тиждень (21-27.09.2026), склад органу станом на 26.09.2026";

fn q(s: &str) -> String {
    s.replace('\'', "''")
}

/// SQL-вираз: id org за назвою або alias-ом (норма — нижній регістр без лапок, як у 000011).
fn org_id(name: &str) -> String {
    let n = q(name);
    format!(
        "(SELECT id FROM (SELECT o.id, 0 AS pri FROM org o WHERE o.deleted_at IS NULL AND lower(o.short_name) = lower('{n}') \
          UNION ALL SELECT a.target_id, 1 FROM alias a JOIN org o ON o.id = a.target_id AND o.deleted_at IS NULL \
          WHERE a.target_type = 'org' AND a.norm = lower(regexp_replace('{n}', '[\"«»“”]', '', 'g'))) x ORDER BY pri LIMIT 1)"
    )
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // 0. Лапки «» / “” → " у вже наявних довідникових даних.
        db.execute_unprepared(
            "UPDATE training_site SET locality = translate(locality, '«»“”', '\"\"\"\"') WHERE locality ~ '[«»“”]'",
        )
        .await?;
        db.execute_unprepared(
            "UPDATE org SET short_name = translate(short_name, '«»“”', '\"\"\"\"'), \
             full_name = translate(full_name, '«»“”', '\"\"\"\"') \
             WHERE short_name ~ '[«»“”]' OR full_name ~ '[«»“”]'",
        )
        .await?;

        // 1. ЧБП — діючий орган за зведеннями 26.09 (на dev був м'яко видалений при тестуванні CRUD).
        db.execute_unprepared(
            "UPDATE org SET deleted_at = NULL, updated_at = now() WHERE short_name = 'ЧБП' AND deleted_at IS NOT NULL",
        )
        .await?;
        db.execute_unprepared(
            "INSERT INTO org (kind, short_name) SELECT 'virtual_group', 'ЧБП' \
             WHERE NOT EXISTS (SELECT 1 FROM org WHERE short_name = 'ЧБП' AND deleted_at IS NULL)",
        )
        .await?;
        db.execute_unprepared(&format!(
            "INSERT INTO subordination (child_org_id, parent_org_id, axis, valid_from, basis) \
             SELECT {chbp}, {uvs}, 'staff', '2022-01-01', '{basis}' \
             WHERE NOT EXISTS (SELECT 1 FROM subordination WHERE child_org_id = {chbp} AND axis = 'staff' AND valid_to IS NULL)",
            chbp = org_id("ЧБП"),
            uvs = org_id("УВ(с) \"Південь\""),
            basis = q(BASIS),
        ))
        .await?;

        // 2. Alias-и варіантів написання для наявних org (лише якщо ціль існує).
        for (raw, target) in VARIANTS {
            db.execute_unprepared(&format!(
                "INSERT INTO alias (target_type, target_id, raw, norm, source) \
                 SELECT 'org', t.id, '{r}', lower(regexp_replace('{r}', '[\"«»“”]', '', 'g')), 'seed' \
                 FROM (SELECT {tid} AS id) t \
                 WHERE t.id IS NOT NULL AND NOT EXISTS (SELECT 1 FROM alias a WHERE a.target_type = 'org' \
                   AND a.norm = lower(regexp_replace('{r}', '[\"«»“”]', '', 'g')))",
                r = q(raw),
                tid = org_id(target),
            ))
            .await?;
        }

        // 3. Нові частини + базовий alias + штатне підпорядкування органу.
        for (organ, units) in ORGANS {
            for unit in *units {
                let kind = if OTHER_KIND.contains(unit) { "other" } else { "military_unit" };
                let u = q(unit);
                db.execute_unprepared(&format!(
                    "INSERT INTO org (kind, short_name) SELECT '{kind}', '{u}' WHERE {id} IS NULL",
                    id = org_id(unit),
                ))
                .await?;
                db.execute_unprepared(&format!(
                    "INSERT INTO alias (target_type, target_id, raw, norm, source) \
                     SELECT 'org', {id}, '{u}', lower(regexp_replace('{u}', '[\"«»“”]', '', 'g')), 'seed' \
                     WHERE NOT EXISTS (SELECT 1 FROM alias a WHERE a.target_type = 'org' \
                       AND a.norm = lower(regexp_replace('{u}', '[\"«»“”]', '', 'g')))",
                    id = org_id(unit),
                ))
                .await?;
                db.execute_unprepared(&format!(
                    "INSERT INTO subordination (child_org_id, parent_org_id, axis, valid_from, basis) \
                     SELECT c.id, p.id, 'staff', '2022-01-01', '{basis}' \
                     FROM (SELECT {cid} AS id) c, (SELECT {pid} AS id) p \
                     WHERE c.id IS NOT NULL AND p.id IS NOT NULL AND NOT EXISTS ( \
                       SELECT 1 FROM subordination s WHERE s.child_org_id = c.id AND s.valid_to IS NULL)",
                    cid = org_id(unit),
                    pid = org_id(organ),
                    basis = q(BASIS),
                ))
                .await?;
            }
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Довідникові дані, на які далі посилаються подання/групи — відкат не видаляє
        // (нічого не перезаписується без історії; прибирати — через адмінку з soft delete).
        Ok(())
    }
}
