use app::domain::normalize::normalize;
use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::Statement;

#[derive(DeriveMigrationName)]
pub struct Migration;

// Джерело кожного блоку — docs/spec/01-domain-model.md §2 (training_kind/training_direction/
// bzvp_program/course/attrition_reason/position — назви вже узгоджені із замовником) і
// docs/source-analysis.md §3 "Сід словника «ОВТ → ВОС» (перевірено на живих даних)" — 17 пар,
// НЕ з сирих source_files/ (ДСК). Повний перелік ВОС (91) і посад (226) замовник надасть пізніше
// (source-analysis.md: "Решту — зібрати скриптом... status = draft").
const SEED_STATEMENTS: &[&str] = &[
    r#"INSERT INTO training_kind (code, name) VALUES
        ('bzvp', 'БЗВП'),
        ('special', 'Фахова'),
        ('adaptation', 'Адаптація'),
        ('internship', 'Стажування')"#,
    r#"INSERT INTO training_direction (code, name) VALUES
        ('kvid', 'Командири відділень'),
        ('instructors', 'Інструктори'),
        ('bps', 'Безпілотні системи')"#,
    r#"INSERT INTO bzvp_program (name) VALUES
        ('БЗВП-3'), ('БЗВП-6'), ('УДЗ'), ('КТЗ')"#,
    r#"INSERT INTO course (name) VALUES
        ('КІБР'), ('КІПР'), ('КПК'), ('СККВ'),
        ('курс лідерства середнього рівня'), ('КПОШ')"#,
    r#"INSERT INTO attrition_reason (name, requires_note) VALUES
        ('СЗЧ', false),
        ('лікування/шпиталь', false),
        ('відрахований за станом здоров''я', false),
        ('не пройшов вхідний контроль', false),
        ('не пройшов проміжний контроль', false),
        ('не склав іспит', false),
        ('не прибув до НЦ', false),
        ('переведення', false),
        ('смерть', false),
        ('помилка внесення/задвоєння', false),
        ('інше', true)"#,
    r#"INSERT INTO "position" (name) VALUES
        ('зовнішній пілот (оператор) БпЛА'),
        ('бойовий медик взводу')"#,
    // vos.title для більшості кодів невідомий (немає в дозволених джерелах) — чесно позначено
    // status='draft' і плейсхолдером замість вигаданої назви спеціальності. 218 — офіційно
    // підтверджений приклад з docs/spec/02-input-forms-ux.md §3.
    r#"INSERT INTO vos (code, title, status, source) VALUES
        ('218', 'зовнішній пілот (оператор) БпЛА', 'official', 'seed'),
        ('219', 'назва уточнюється (офіційний перелік — пізніше)', 'draft', 'seed'),
        ('217', 'назва уточнюється (офіційний перелік — пізніше)', 'draft', 'seed'),
        ('216', 'назва уточнюється (офіційний перелік — пізніше)', 'draft', 'seed'),
        ('129', 'назва уточнюється (офіційний перелік — пізніше)', 'draft', 'seed'),
        ('240', 'назва уточнюється (офіційний перелік — пізніше)', 'draft', 'seed'),
        ('225', 'назва уточнюється (офіційний перелік — пізніше)', 'draft', 'seed'),
        ('714', 'назва уточнюється (офіційний перелік — пізніше)', 'draft', 'seed'),
        ('420', 'назва уточнюється (офіційний перелік — пізніше)', 'draft', 'seed'),
        ('403', 'назва уточнюється (офіційний перелік — пізніше)', 'draft', 'seed'),
        ('117', 'назва уточнюється (офіційний перелік — пізніше)', 'draft', 'seed'),
        ('121', 'назва уточнюється (офіційний перелік — пізніше)', 'draft', 'seed'),
        ('104', 'назва уточнюється (офіційний перелік — пізніше)', 'draft', 'seed'),
        ('545', 'назва уточнюється (офіційний перелік — пізніше)', 'draft', 'seed'),
        ('140', 'назва уточнюється (офіційний перелік — пізніше)', 'draft', 'seed'),
        ('515', 'назва уточнюється (офіційний перелік — пізніше)', 'draft', 'seed'),
        ('993', 'назва уточнюється (офіційний перелік — пізніше)', 'draft', 'seed')"#,
    r#"INSERT INTO vos_position (vos_id, position_id, weight)
        SELECT (SELECT id FROM vos WHERE code = '218'),
               (SELECT id FROM "position" WHERE name = 'зовнішній пілот (оператор) БпЛА'), 1"#,
    // equipment: один рядок на "концепт" з таблиці source-analysis.md §3; equipment.name — перший
    // (основний) термін клітинки. Синоніми з тієї ж клітинки йдуть в alias (нижче), а не сюди.
    r#"INSERT INTO equipment (name) VALUES
        ('FPV'), ('Vampire'), ('Mavic'), ('Darts'), ('НРК'),
        ('Боєприпаси до БпЛА'), ('ремонт/експлуатація БпАК'), ('майстер НРК'),
        ('Р-145БМ'), ('ТК1-4'), ('Т-72'), ('БМП-1'), ('ПЗРК'),
        ('РЕБ ближньої дії'), ('БМ-21'), ('СБР-3'), ('Ай-Петрі')"#,
    r#"INSERT INTO equipment_vos (equipment_id, vos_id, weight, source)
        SELECT e.id, v.id, 1, 'seed' FROM equipment e JOIN vos v ON true
        WHERE (e.name, v.code) IN (
            ('FPV', '219'), ('Vampire', '218'), ('Mavic', '217'), ('Darts', '216'),
            ('НРК', '129'), ('Боєприпаси до БпЛА', '240'),
            ('ремонт/експлуатація БпАК', '225'), ('майстер НРК', '714'),
            ('Р-145БМ', '420'), ('ТК1-4', '403'), ('Т-72', '117'), ('БМП-1', '121'),
            ('ПЗРК', '104'), ('РЕБ ближньої дії', '545'), ('БМ-21', '140'),
            ('СБР-3', '515'), ('Ай-Петрі', '993')
        )"#,
];

/// (equipment.name, синонім) — та сама пара, що й у попередньому SQL-блоці, але norm рахуємо
/// через `app::domain::normalize` у Rust (не SQL `lower()`): equipment-терміни здебільшого
/// латиницею ("Mavic", "FPV"), а `normalize()` транслітерує окремі латинські літери в кириличні
/// (задумано для номерів ВЧ на кшталт "A4076") — якщо norm при сіді й norm при пошуку рахувати
/// РІЗНИМИ способами (SQL lower() тут, Rust normalize() у backend::repo::dictionaries), пошук
/// латиницею ("mavic") ніколи не збіжиться із засіяним рядком. `alias.norm` мусить рахуватись
/// однією функцією скрізь (01 §"alias", рішення [[alias-normalization-in-app]]).
const EQUIPMENT_ALIASES: &[(&str, &str)] = &[
    ("FPV", "FPV"),
    ("FPV", "БпЛА мультироторного типу ударні FPV"),
    ("FPV", "FPV ППО"),
    ("FPV", "перехоплювачі"),
    ("Vampire", "Vampire"),
    ("Vampire", "Вампір"),
    ("Vampire", "ВАМПІР"),
    ("Vampire", "Nemesis BHM"),
    ("Vampire", "Heavy Shot"),
    ("Vampire", "Pegasus Arms-25"),
    ("Vampire", "Скиди"),
    ("Vampire", "важкі бомбери"),
    ("Mavic", "Mavic"),
    ("Mavic", "DJI Mavic 3"),
    ("Mavic", "DJI Mavic 3T"),
    ("Mavic", "Matrice 4T"),
    ("Mavic", "Autel"),
    ("Mavic", "Мавік"),
    ("Mavic", "Шмавік"),
    ("Mavic", "БпЛА мультироторного типу"),
    ("Darts", "Darts"),
    ("Darts", "ДАРТС"),
    ("Darts", "Shark-D"),
    ("Darts", "А1-СМ Фурія"),
    ("Darts", "Чаклун"),
    ("Darts", "Блискавка"),
    ("НРК", "НРК"),
    ("НРК", "РНК"),
    ("НРК", "БпНРК"),
    ("НРК", "ЛНРК"),
    ("НРК", "Воля-Е"),
    ("НРК", "Moroz"),
    ("НРК", "Термит"),
    ("НРК", "Рись"),
    ("Боєприпаси до БпЛА", "майстер боєприпасів"),
    ("Боєприпаси до БпЛА", "інженер боєприпасів"),
    ("ремонт/експлуатація БпАК", "механік БпЛА"),
    ("ремонт/експлуатація БпАК", "майстер БпЛА"),
    ("майстер НРК", "майстер НРК"),
    ("Р-145БМ", "Р-145БМ"),
    ("Р-145БМ", "Р-161А2М"),
    ("Р-145БМ", "Mototrbo"),
    ("Р-145БМ", "Hytera"),
    ("Р-145БМ", "Harris"),
    ("Р-145БМ", "Aselsan"),
    ("Р-145БМ", "Tooway"),
    ("Р-145БМ", "Starlink"),
    ("Р-145БМ", "радіотелефоніст"),
    ("ТК1-4", "ТК1-4"),
    ("ТК1-4", "П-257-24К"),
    ("ТК1-4", "лінійний наглядач"),
    ("ТК1-4", "телефоніст"),
    ("Т-72", "Т-72"),
    ("Т-72", "Т-80"),
    ("Т-72", "Т-64"),
    ("Т-72", "навідник"),
    ("Т-72", "командир танка"),
    ("БМП-1", "БМП-1"),
    ("БМП-1", "БМП-2"),
    ("БМП-1", "механік-водій"),
    ("ПЗРК", "ПЗРК"),
    ("ПЗРК", "Ігла"),
    ("ПЗРК", "Stinger"),
    ("ПЗРК", "Piorun"),
    ("РЕБ ближньої дії", "Буковель-АД"),
    ("РЕБ ближньої дії", "Дамба"),
    ("РЕБ ближньої дії", "антидронові рушниці"),
    ("БМ-21", "БМ-21"),
    ("БМ-21", "Град"),
    ("СБР-3", "СБР-3"),
    ("СБР-3", "ПСНР-5"),
    ("СБР-3", "GO-12"),
    ("Ай-Петрі", "Нота"),
    ("Ай-Петрі", "Ай-Петрі"),
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for stmt in SEED_STATEMENTS {
            db.execute_unprepared(stmt).await?;
        }

        for (equipment_name, raw) in EQUIPMENT_ALIASES {
            let norm = normalize(raw);
            db.execute(Statement::from_sql_and_values(
                db.get_database_backend(),
                "INSERT INTO alias (target_type, target_id, raw, norm, source) \
                 SELECT 'equipment', id, $1, $2, 'seed' FROM equipment WHERE name = $3",
                [(*raw).into(), norm.into(), (*equipment_name).into()],
            ))
            .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared("DELETE FROM alias WHERE target_type = 'equipment' AND source = 'seed'")
            .await?;
        db.execute_unprepared("DELETE FROM equipment_vos WHERE source = 'seed'").await?;
        db.execute_unprepared("DELETE FROM vos_position").await?;
        db.execute_unprepared("DELETE FROM equipment").await?;
        db.execute_unprepared("DELETE FROM vos WHERE source = 'seed'").await?;
        db.execute_unprepared("DELETE FROM \"position\"").await?;
        db.execute_unprepared("DELETE FROM attrition_reason").await?;
        db.execute_unprepared("DELETE FROM course").await?;
        db.execute_unprepared("DELETE FROM bzvp_program").await?;
        db.execute_unprepared("DELETE FROM training_direction").await?;
        db.execute_unprepared("DELETE FROM training_kind").await?;
        Ok(())
    }
}
