//! Інтеграційний тест генерації D1+D2 (Етап 7, 05 §D1/§D2) — окрема тестова БД, той самий підхід,
//! що й `app/tests/groups.rs` (див. `common`). Один `#[tokio::test]` на файл (як і решта
//! `app/tests/*.rs`) — паралельні тести в одному файлі гонялися б за DROP+CREATE тієї самої
//! тестової бази (`common::fresh_test_db` ділить ім'я з `TEST_DATABASE_URL`). Золотий тест
//! ПОСЛАБЛЕНО (рішення користувача, `.claude/plans/effervescent-dazzling-sunset.md`):
//! перевіряється СТРУКТУРА (правило групування, колонки, формули SUM/несуміжні суми, примітки),
//! а не точний числовий збіг з реальним еталоном — у БД зараз нема повного реального покриття
//! всіх підрозділів на конкретну дату.
#![cfg(feature = "ssr")]

mod common;

use app::backend::documents::d1::build_day_sheet;
use app::backend::documents::d2::{build_week_sheet, DayBlock};
use app::backend::repo::documents::daily_training_rollup;
use calamine::{Data, Reader, Xlsx};
use chrono::Datelike;
use common::fresh_test_db;
use sea_orm::{ConnectionTrait, FromQueryResult, Statement};
use std::io::Cursor;

#[derive(FromQueryResult)]
struct IdRow {
    id: i32,
}

async fn org_id(db: &impl ConnectionTrait, short_name: &str) -> i32 {
    IdRow::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id FROM org WHERE short_name = $1",
        [short_name.into()],
    ))
    .one(db)
    .await
    .unwrap()
    .unwrap_or_else(|| panic!("dev-сід має org {short_name:?}"))
    .id
}

async fn training_kind_id(db: &impl ConnectionTrait, code: &str) -> i32 {
    IdRow::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id FROM training_kind WHERE code = $1",
        [code.into()],
    ))
    .one(db)
    .await
    .unwrap()
    .unwrap_or_else(|| panic!("сід довідників має training_kind {code:?}"))
    .id
}

#[tokio::test]
async fn document_generation_covers_d1_daily_and_d2_weekly_rollups() {
    let Some(db) = fresh_test_db().await else { return };

    // "2026-07-20" — те саме "до переходу" (жоден з 6 частин 17 АК→7 КШР ще не пішов, найраніший
    // перехід — 67 омбр 03.08.2026), уже перевірений Етапом 1 (app/tests/orgs.rs).
    const AS_OF: &str = "2026-07-20";

    let ak17 = org_id(&db, "17 АК").await;
    // Лишається штатно й операційно під 17 АК увесь час — "основна" секція.
    let obr_tro = org_id(&db, "241 обр ТрО").await;
    // Штатно 17 АК, але оперативно 20 АК (постійне подвійне підпорядкування, 01 §1) —
    // "поза смугою" секція за визначенням, не транзитний перехід.
    let omb_110 = org_id(&db, "110 омбр").await;

    let site_id = IdRow::find_by_statement(Statement::from_string(
        db.get_database_backend(),
        "SELECT id FROM training_site ORDER BY id LIMIT 1",
    ))
    .one(&db)
    .await
    .unwrap()
    .expect("dev-сід має training_site")
    .id;

    let special = training_kind_id(&db, "special").await;
    let bzvp = training_kind_id(&db, "bzvp").await;
    let adaptation = training_kind_id(&db, "adaptation").await;

    // 241 обр ТрО, "Фахова": started 10 (15.07) -> ще в навчанні; completed 3 РІВНО на AS_OF
    // (тестує "Закінчують сьогодні"). Лишається 10-3=7 у навчанні.
    let g1 = IdRow::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO training_group (sender_org_id, training_kind_id, site_id, planned_start, planned_end) \
         VALUES ($1, $2, $3, '2026-07-15', '2026-08-15') RETURNING id",
        [obr_tro.into(), special.into(), site_id.into()],
    ))
    .one(&db)
    .await
    .unwrap()
    .unwrap()
    .id;
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO group_event (group_id, event_type, count, occurred_on, note) VALUES \
            ($1, 'started', 10, '2026-07-15', NULL), \
            ($1, 'completed', 3, $2::date, 'Фах - 3 вс завершили 20.07.2026')",
        [g1.into(), AS_OF.into()],
    ))
    .await
    .unwrap();

    // 241 обр ТрО, "БЗВП": started 4 РІВНО на AS_OF (тестує "почали сьогодні").
    let g2 = IdRow::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO training_group (sender_org_id, training_kind_id, site_id, planned_start, planned_end) \
         VALUES ($1, $2, $3, $4::date, '2026-08-20') RETURNING id",
        [obr_tro.into(), bzvp.into(), site_id.into(), AS_OF.into()],
    ))
    .one(&db)
    .await
    .unwrap()
    .unwrap()
    .id;
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO group_event (group_id, event_type, count, occurred_on) VALUES ($1, 'started', 4, $2::date)",
        [g2.into(), AS_OF.into()],
    ))
    .await
    .unwrap();

    // 110 омбр, "Адаптація": started 5 (01.07), ще триває на AS_OF.
    let g3 = IdRow::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO training_group (sender_org_id, training_kind_id, site_id, planned_start, planned_end) \
         VALUES ($1, $2, $3, '2026-07-01', '2026-07-25') RETURNING id",
        [omb_110.into(), adaptation.into(), site_id.into()],
    ))
    .one(&db)
    .await
    .unwrap()
    .unwrap()
    .id;
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO group_event (group_id, event_type, count, occurred_on) VALUES ($1, 'started', 5, '2026-07-01')",
        [g3.into()],
    ))
    .await
    .unwrap();

    let rollup = daily_training_rollup(&db, ak17, AS_OF).await.unwrap();

    // --- правило групування (01 §1) ---
    let main_ids: Vec<i32> = rollup.main.iter().map(|r| r.org_id).collect();
    let ooz_ids: Vec<i32> = rollup.out_of_zone.iter().map(|r| r.org_id).collect();
    assert!(main_ids.contains(&obr_tro), "241 обр ТрО — штатно й оперативно 17 АК, основна секція");
    assert!(
        ooz_ids.contains(&omb_110),
        "110 омбр — штатно 17 АК, оперативно 20 АК, поза смугою (не транзит)"
    );
    assert!(!main_ids.contains(&omb_110), "110 омбр НЕ в основній секції 17 АК");

    // --- суми по видах підготовки ---
    let obr_row = rollup.main.iter().find(|r| r.org_id == obr_tro).unwrap();
    assert_eq!(obr_row.special.total, 7, "10 started - 3 completed");
    assert_eq!(obr_row.special.finishing_today, 3);
    assert_eq!(obr_row.special.started_today, 0, "started було 15.07, не на AS_OF");
    assert_eq!(obr_row.bzvp.total, 4);
    assert_eq!(obr_row.bzvp.started_today, 4);
    assert_eq!(obr_row.bzvp.finishing_today, 0);
    assert_eq!(
        obr_row.note.as_deref(),
        Some("Фах - 3 вс завершили 20.07.2026"),
        "примітка дня — наявний group_event.note, не синтезований текст"
    );

    let omb_row = rollup.out_of_zone.iter().find(|r| r.org_id == omb_110).unwrap();
    assert_eq!(omb_row.adaptation.total, 5);

    // --- xlsx-структура (05 §D1): аркуш, заголовки, формула SUM, а не лише число ---
    let date = chrono::NaiveDate::parse_from_str(AS_OF, "%Y-%m-%d").unwrap();
    let mut workbook = rust_xlsxwriter::Workbook::new();
    build_day_sheet(&mut workbook, "17 АК", date, &rollup).unwrap();
    let bytes = workbook.save_to_buffer().unwrap();

    let mut xlsx: Xlsx<_> = Xlsx::new(Cursor::new(bytes)).unwrap();
    let sheet_name = xlsx.sheet_names()[0].clone();
    let range = xlsx.worksheet_range(&sheet_name).unwrap();
    let formulas = xlsx.worksheet_formula(&sheet_name).unwrap();

    assert_eq!(sheet_name, "20.07", "назва аркуша — день (dd.mm), поки лише один аркуш у файлі");
    assert_eq!(
        range.get_value((0, 0)),
        Some(&Data::String("17 АК. Зведена таблиця підготовки станом на 20.07.2026".to_string())),
        "заголовок з органом і датою"
    );
    assert_eq!(range.get_value((1, 1)), Some(&Data::String("БЗВП".to_string())));
    assert_eq!(range.get_value((2, 1)), Some(&Data::String("Всього".to_string())));

    // Рядок ВСЬОГО (row 3, 0-indexed) — формула, не готове число (05: "формули мають жити").
    let total_formula = formulas.get_value((3, 1)).cloned().unwrap_or_default();
    assert!(total_formula.starts_with("SUM("), "ВСЬОГО має бути формулою SUM(...): {total_formula:?}");

    // --- D2 ("Контролька", 05 §D2), той самий db/сід -- один тест на файл, як і решта app/tests/*
    // (спільна тестова БД по імені з TEST_DATABASE_URL, паралельні #[tokio::test] в одному файлі
    // гонялися б за DROP+CREATE тієї самої бази). На відміну від D1: підсумок дня й підсумок
    // тижня сумують НЕСУМІЖНІ рядки (рядки-підсумки корпусів розкидані поміж рядками підрозділів),
    // тому не можуть бути SUM(range) -- лише підсумок корпусу за день лишається суміжним
    // діапазоном, як у D1.
    let ak20 = org_id(&db, "20 АК").await;
    let omb_67 = org_id(&db, "67 омбр").await; // штатний підрозділ 20 АК до 01.08.2026
    // "adaptation" (не "special", як у D1-фікстурі вище) -- інакше рахунок 241 обр ТрО/special
    // цього ж дня об'єднав би обидві групи в один total, псуючи очікуване число нижче.
    let adaptation = training_kind_id(&db, "adaptation").await;

    const DAY0: &str = "2026-07-20"; // той самий AS_OF, що й D1-перевірка вище
    const DAY1: &str = "2026-07-21";

    // 241 обр ТрО (17 АК, той самий org/site, що й D1 вище), "Адаптація", день 0:
    // started 6, completed 2, attrition 1 -> total 3.
    let g1 = IdRow::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO training_group (sender_org_id, training_kind_id, site_id, planned_start, planned_end) \
         VALUES ($1, $2, $3, $4::date, '2026-08-15') RETURNING id",
        [obr_tro.into(), adaptation.into(), site_id.into(), DAY0.into()],
    ))
    .one(&db)
    .await
    .unwrap()
    .unwrap()
    .id;
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO group_event (group_id, event_type, count, occurred_on) VALUES \
            ($1, 'started', 6, $2::date), ($1, 'completed', 2, $2::date), ($1, 'attrition', 1, $2::date)",
        [g1.into(), DAY0.into()],
    ))
    .await
    .unwrap();

    // 67 омбр (20 АК), "БЗВП", день 1: started 9.
    let g2 = IdRow::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO training_group (sender_org_id, training_kind_id, site_id, planned_start, planned_end) \
         VALUES ($1, $2, $3, $4::date, '2026-08-20') RETURNING id",
        [omb_67.into(), bzvp.into(), site_id.into(), DAY1.into()],
    ))
    .one(&db)
    .await
    .unwrap()
    .unwrap()
    .id;
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO group_event (group_id, event_type, count, occurred_on) VALUES ($1, 'started', 9, $2::date)",
        [g2.into(), DAY1.into()],
    ))
    .await
    .unwrap();

    let day0 = chrono::NaiveDate::parse_from_str(DAY0, "%Y-%m-%d").unwrap();
    let day1 = chrono::NaiveDate::parse_from_str(DAY1, "%Y-%m-%d").unwrap();
    let week_start = day0 - chrono::Duration::days(day0.weekday().num_days_from_monday() as i64);

    let mut days = Vec::new();
    for date in [day0, day1] {
        let date_str = date.format("%Y-%m-%d").to_string();
        let mut corps = Vec::new();
        for (corps_id, label) in [(ak17, "17 АК"), (ak20, "20 АК")] {
            let rollup = daily_training_rollup(&db, corps_id, &date_str).await.unwrap();
            let mut orgs = rollup.main;
            orgs.extend(rollup.out_of_zone);
            corps.push((label.to_string(), orgs));
        }
        days.push(DayBlock { date, corps });
    }

    // "241 обр ТрО" на день 0, "Адаптація": 6 started - 2 completed - 1 attrition = 3 total.
    let obr_row_d0 = days[0]
        .corps
        .iter()
        .find(|(label, _)| label == "17 АК")
        .unwrap()
        .1
        .iter()
        .find(|r| r.org_id == obr_tro)
        .unwrap();
    assert_eq!(obr_row_d0.adaptation.total, 3);
    assert_eq!(obr_row_d0.adaptation.left_today, 1, "нова колонка 'вибули з різних причин' (05 §D2)");

    let mut workbook = rust_xlsxwriter::Workbook::new();
    build_week_sheet(&mut workbook, week_start, &days).unwrap();
    let bytes = workbook.save_to_buffer().unwrap();

    let mut xlsx: Xlsx<_> = Xlsx::new(Cursor::new(bytes)).unwrap();
    let sheet_name = xlsx.sheet_names()[0].clone();
    let range = xlsx.worksheet_range(&sheet_name).unwrap();
    let formulas = xlsx.worksheet_formula(&sheet_name).unwrap();

    // 4 підколонки на вид підготовки тепер (05 §D2 додає "вибули з різних причин").
    assert_eq!(
        range.get_value((2, 4)),
        Some(&Data::String("вибули з різних причин".to_string())),
        "четверта підколонка БЗВП — нова колонка D2"
    );

    let find_label_row = |label: &str| -> u32 {
        range
            .rows()
            .enumerate()
            .find(|(_, row)| row.first() == Some(&Data::String(label.to_string())))
            .map(|(i, _)| i as u32)
            .unwrap_or_else(|| panic!("рядок з міткою {label:?} не знайдено"))
    };

    let corps_row_d0 = find_label_row("17 АК");
    let week_row = find_label_row("Тиждень");
    let day0_row = find_label_row(&day0.format("%d.%m.%Y").to_string());

    // Підсумок корпусу за день — суміжний діапазон (як D1): формула SUM(...).
    let corps_formula = formulas.get_value((corps_row_d0, 1)).cloned().unwrap_or_default();
    assert!(
        corps_formula.starts_with("SUM("),
        "підсумок корпусу за день лишається SUM(суміжний_діапазон): {corps_formula:?}"
    );

    // Підсумок дня — сума КОНКРЕТНИХ (несуміжних) рядків-підсумків корпусів, не SUM(range).
    let day_formula = formulas.get_value((day0_row, 1)).cloned().unwrap_or_default();
    assert!(
        !day_formula.starts_with("SUM("),
        "підсумок дня — несуміжна сума клітинок, не SUM(range): {day_formula:?}"
    );
    assert!(day_formula.contains('+'), "має сумувати ≥2 корпуси: {day_formula:?}");

    // Підсумок тижня — так само несуміжна сума рядків-дат.
    let week_formula = formulas.get_value((week_row, 1)).cloned().unwrap_or_default();
    assert!(
        !week_formula.starts_with("SUM("),
        "підсумок тижня — несуміжна сума клітинок, не SUM(range): {week_formula:?}"
    );
    assert!(week_formula.contains('+'), "має сумувати 2 дні: {week_formula:?}");
}
