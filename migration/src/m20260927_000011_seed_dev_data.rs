use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

// Dev-сід Етапу 1: командні органи, частини з реальними номерами (docs/source-analysis.md),
// сценарій переходу 17 АК → 7 КШР і подвійного підпорядкування 110 омбр (докладно — 01-domain-model.md),
// приклад "152 нц (А4896)" з docs/spec/02-input-forms-ux.md §3. Джерело — вже витягнуті в docs/
// фрагменти specи, НЕ сирі файли source_files/ (ДСК, поза доступом). Це не Етап 6 (archive_seed) —
// повний перенос архіву робиться пізніше окремим конвеєром через submission.
const SEED_STATEMENTS: &[&str] = &[
    // --- командні органи ---
    r#"INSERT INTO org (kind, short_name, echelon) VALUES ('command', 'УВ(с) "Південь"', 'ок')"#,
    r#"INSERT INTO org (kind, short_name, echelon) VALUES ('command', '17 АК', 'ак')"#,
    r#"INSERT INTO org (kind, short_name, echelon) VALUES ('command', '20 АК', 'ак')"#,
    r#"INSERT INTO org (kind, short_name, echelon) VALUES ('command', '30 КМП', 'кмп')"#,
    r#"INSERT INTO org (kind, short_name, echelon) VALUES ('command', '7 КШР', 'кшр')"#,
    r#"INSERT INTO org (kind, short_name, echelon) VALUES ('command', 'ОТУ "Одеса"', 'оту')"#,
    r#"INSERT INTO org (kind, short_name) VALUES ('virtual_group', 'ЧБП')"#,
    // --- частини з реальними номерами (docs/source-analysis.md, "Терміни" фікстура) ---
    r#"INSERT INTO org (kind, short_name, number_kind, number) VALUES
        ('military_unit', '241 обр ТрО', 'A', '4076'),
        ('military_unit', '128 овмбр', 'A', '7384'),
        ('military_unit', '128 огшбр', 'A', '1556'),
        ('military_unit', '253 ошп', 'A', '4706'),
        ('military_unit', '153 омбр', 'A', '4955'),
        ('military_unit', '65 омбр', 'A', '7013'),
        ('military_unit', '118 омбр', 'A', '4712'),
        ('military_unit', '260 обр ТрО', 'A', '7038'),
        ('military_unit', '7 орб', 'A', '7091'),
        ('military_unit', '151 омбр', 'A', '4941'),
        ('military_unit', '110 омбр', 'A', '4007'),
        ('military_unit', '422 оп БпС', 'A', '5047'),
        ('military_unit', '101 оптб', 'A', '5219'),
        ('military_unit', '54 оабр', 'A', '5183'),
        ('military_unit', '124 обз', 'A', '5243'),
        ('military_unit', '1223 обп', 'A', '4796'),
        ('military_unit', '186 обмз', 'A', '5184'),
        ('military_unit', '514 орвб', 'A', '5188'),
        ('military_unit', '529 обоо', 'A', '5148'),
        ('military_unit', '1065 сфпз', 'A', '7269'),
        ('military_unit', '1172 озрдн', 'A', '5265'),
        ('military_unit', '93 оптб', 'A', '5021'),
        ('military_unit', '152 нц', 'A', '4896')"#,
    // --- частини для сценарію переходу (номери невідомі з наявних джерел) ---
    r#"INSERT INTO org (kind, short_name) VALUES
        ('military_unit', '142 омбр'),
        ('military_unit', '154 омбр'),
        ('military_unit', '61 омбр'),
        ('military_unit', '5 омбр'),
        ('military_unit', '92 ошбр'),
        ('military_unit', '225 ошп'),
        ('military_unit', '44 оабр'),
        ('military_unit', '67 омбр'),
        ('military_unit', '423 обБпС'),
        ('military_unit', '433 обБпС'),
        ('military_unit', '13 кппо')"#,
    r#"INSERT INTO org (kind, short_name, country) VALUES ('foreign_state', 'Республіка Польща', 'Польща')"#,
    // --- місце навчання 152 нц (02-input-forms-ux.md §3) ---
    r#"INSERT INTO training_site (org_id, locality)
        SELECT id, 'м. Верхньодніпровськ' FROM org WHERE short_name = '152 нц'"#,
    // --- підпорядкування: командний ланцюг під УВ(с) (штатне, з 2022) ---
    r#"INSERT INTO subordination (child_org_id, parent_org_id, axis, valid_from)
        SELECT (SELECT id FROM org WHERE short_name = c.name),
               (SELECT id FROM org WHERE short_name = 'УВ(с) "Південь"'), 'staff', '2022-01-01'
        FROM (VALUES ('17 АК'), ('20 АК'), ('30 КМП'), ('7 КШР'), ('ОТУ "Одеса"'), ('ЧБП')) AS c(name)"#,
    // --- частини з реальними номерами → 17 АК (штатне, з 2022, без змін) ---
    r#"INSERT INTO subordination (child_org_id, parent_org_id, axis, valid_from)
        SELECT o.id, (SELECT id FROM org WHERE short_name = '17 АК'), 'staff', '2022-01-01'
        FROM org o WHERE o.number_kind = 'A' AND o.number IN
            ('4076','7384','1556','4706','4955','7013','4712','7038','7091','4941',
             '4007','5047','5219','5183','5243','4796','5184','5188','5148','7269','5265','5021')"#,
    // --- 110 омбр: одночасно оперативно підлегла 20 АК (окрім штатного 17 АК вище) ---
    r#"INSERT INTO subordination (child_org_id, parent_org_id, axis, valid_from) VALUES
        ((SELECT id FROM org WHERE short_name = '110 омбр'),
         (SELECT id FROM org WHERE short_name = '20 АК'), 'operational', '2022-01-01')"#,
    // --- сценарій переходу: 6 частин 17 АК → 7 КШР з 01.08.2026 (штатне) ---
    r#"INSERT INTO subordination (child_org_id, parent_org_id, axis, valid_from, valid_to)
        SELECT o.id, (SELECT id FROM org WHERE short_name = '17 АК'), 'staff', '2022-01-01', '2026-08-01'
        FROM org o WHERE o.short_name IN ('142 омбр','154 омбр','61 омбр','5 омбр','92 ошбр','225 ошп','44 оабр')"#,
    r#"INSERT INTO subordination (child_org_id, parent_org_id, axis, valid_from)
        SELECT o.id, (SELECT id FROM org WHERE short_name = '7 КШР'), 'staff', '2026-08-01'
        FROM org o WHERE o.short_name IN ('142 омбр','154 омбр','61 омбр','5 омбр','92 ошбр','225 ошп','44 оабр')"#,
    // --- 67 омбр: 20 АК → 17 АК з початку серпня 2026 (штатне) ---
    r#"INSERT INTO subordination (child_org_id, parent_org_id, axis, valid_from, valid_to) VALUES
        ((SELECT id FROM org WHERE short_name = '67 омбр'),
         (SELECT id FROM org WHERE short_name = '20 АК'), 'staff', '2022-01-01', '2026-08-01')"#,
    r#"INSERT INTO subordination (child_org_id, parent_org_id, axis, valid_from) VALUES
        ((SELECT id FROM org WHERE short_name = '67 омбр'),
         (SELECT id FROM org WHERE short_name = '17 АК'), 'staff', '2026-08-01')"#,
    // --- alias: базовий рядок на кожну org (власна назва) і на кожен номер ---
    r#"INSERT INTO alias (target_type, target_id, raw, norm, source)
        SELECT 'org', id, short_name, lower(regexp_replace(short_name, '["«»]', '', 'g')), 'seed'
        FROM org"#,
    r#"INSERT INTO alias (target_type, target_id, raw, norm, source)
        SELECT 'org', id, number, lower(number), 'seed'
        FROM org WHERE number IS NOT NULL"#,
    // --- alias: варіанти написання й тестові приклади з роадмапу (06-roadmap.md, Етап 1) ---
    r#"INSERT INTO alias (target_type, target_id, raw, norm, source) VALUES
        ('org', (SELECT id FROM org WHERE short_name = '423 обБпС'), '423 опБпС', lower('423 опБпС'), 'seed'),
        ('org', (SELECT id FROM org WHERE short_name = '433 обБпС'), '433 опБпС', lower('433 опБпС'), 'seed'),
        ('org', (SELECT id FROM org WHERE short_name = '13 кппо'), '13 кпппо', lower('13 кпппо'), 'seed'),
        ('org', (SELECT id FROM org WHERE short_name = 'Республіка Польща'), 'польша', 'польша', 'seed'),
        ('org', (SELECT id FROM org WHERE short_name = '152 нц'), 'а4896', 'а4896', 'seed'),
        ('org', (SELECT id FROM org WHERE short_name = '152 нц'), '152НЦ', '152нц', 'seed')"#,
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for stmt in SEED_STATEMENTS {
            db.execute_unprepared(stmt).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        // Порядок зворотний FK: спершу залежні, потім org.
        db.execute_unprepared("DELETE FROM alias WHERE source = 'seed'")
            .await?;
        db.execute_unprepared(
            "DELETE FROM subordination WHERE child_org_id IN (SELECT id FROM org WHERE short_name IN ( \
                'УВ(с) \"Південь\"','17 АК','20 АК','30 КМП','7 КШР','ОТУ \"Одеса\"','ЧБП', \
                '241 обр ТрО','128 овмбр','128 огшбр','253 ошп','153 омбр','65 омбр','118 омбр', \
                '260 обр ТрО','7 орб','151 омбр','110 омбр','422 оп БпС','101 оптб','54 оабр', \
                '124 обз','1223 обп','186 обмз','514 орвб','529 обоо','1065 сфпз','1172 озрдн', \
                '93 оптб','152 нц','142 омбр','154 омбр','61 омбр','5 омбр','92 ошбр','225 ошп', \
                '44 оабр','67 омбр'))",
        )
        .await?;
        db.execute_unprepared(
            "DELETE FROM training_site WHERE org_id = (SELECT id FROM org WHERE short_name = '152 нц')",
        )
        .await?;
        db.execute_unprepared(
            "DELETE FROM org WHERE short_name IN ( \
                'УВ(с) \"Південь\"','17 АК','20 АК','30 КМП','7 КШР','ОТУ \"Одеса\"','ЧБП', \
                '241 обр ТрО','128 овмбр','128 огшбр','253 ошп','153 омбр','65 омбр','118 омбр', \
                '260 обр ТрО','7 орб','151 омбр','110 омбр','422 оп БпС','101 оптб','54 оабр', \
                '124 обз','1223 обп','186 обмз','514 орвб','529 обоо','1065 сфпз','1172 озрдн', \
                '93 оптб','152 нц','142 омбр','154 омбр','61 омбр','5 омбр','92 ошбр','225 ошп', \
                '44 оабр','67 омбр','423 обБпС','433 обБпС','13 кппо','Республіка Польща')",
        )
        .await
        .map(|_| ())
    }
}
