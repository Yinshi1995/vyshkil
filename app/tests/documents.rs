//! Інтеграційний тест генерації D1 (Етап 7, 05 §D1) — окрема тестова БД, той самий підхід, що й
//! `app/tests/groups.rs` (див. `common`). Золотий тест ПОСЛАБЛЕНО (рішення користувача,
//! `.claude/plans/effervescent-dazzling-sunset.md`): перевіряється СТРУКТУРА (правило групування,
//! колонки, формула SUM, примітки), а не точний числовий збіг з реальним еталоном — у БД зараз
//! нема повного реального покриття всіх підрозділів 17 АК на конкретну дату.
#![cfg(feature = "ssr")]

mod common;

use app::backend::documents::d1::build_day_sheet;
use app::backend::repo::documents::daily_training_rollup;
use calamine::{Data, Reader, Xlsx};
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
async fn daily_rollup_applies_grouping_rule_and_sums_events() {
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
}
