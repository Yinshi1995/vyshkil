//! Інтеграційний тест агрегату "групи" (Етап 3) — окрема тестова БД, той самий підхід, що й
//! `app/tests/orgs.rs` (див. `common`). Лише під `ssr`.
//!
//! Арифметика воронки перевіряється юніт-тестами в `app/src/domain/counting.rs` (чиста функція,
//! без БД). Тут — наскрізна перевірка: реальний `training_group`/`group_event` у Postgres →
//! `backend::repo::groups::group_events` → та сама `domain::counting` — щоб зловити розсинхрон
//! між форматом дат у SQL (`to_char`) і тим, що очікує домен, а не лише саму арифметику.
#![cfg(feature = "ssr")]

mod common;

use app::backend::repo::groups::group_events;
use app::domain::counting::{finishing_on, in_training, EventType, Finishing};
use common::fresh_test_db;
use sea_orm::{ConnectionTrait, FromQueryResult, Statement};

#[tokio::test]
async fn training_group_funnel_end_to_end() {
    let Some(db) = fresh_test_db().await else { return };

    // Мінімальні FK: перша org і перший training_site з dev-сіду Етапу 1, training_kind "Фахова".
    #[derive(FromQueryResult)]
    struct IdRow {
        id: i32,
    }
    let org_id = IdRow::find_by_statement(Statement::from_string(
        db.get_database_backend(),
        "SELECT id FROM org ORDER BY id LIMIT 1",
    ))
    .one(&db)
    .await
    .unwrap()
    .expect("dev-сід має org")
    .id;
    let site_id = IdRow::find_by_statement(Statement::from_string(
        db.get_database_backend(),
        "SELECT id FROM training_site ORDER BY id LIMIT 1",
    ))
    .one(&db)
    .await
    .unwrap()
    .expect("dev-сід має training_site")
    .id;
    let training_kind_id = IdRow::find_by_statement(Statement::from_string(
        db.get_database_backend(),
        "SELECT id FROM training_kind WHERE code = 'special'",
    ))
    .one(&db)
    .await
    .unwrap()
    .expect("сід довідників має training_kind 'special'")
    .id;

    let group_id = IdRow::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO training_group \
            (sender_org_id, training_kind_id, site_id, planned_start, planned_end) \
         VALUES ($1, $2, $3, '2026-03-01', '2026-04-01') RETURNING id",
        [org_id.into(), training_kind_id.into(), site_id.into()],
    ))
    .one(&db)
    .await
    .unwrap()
    .expect("INSERT ... RETURNING id")
    .id;

    // Той самий приклад воронки, що й у юніт-тестах domain::counting: 20 started, +5 added,
    // -3 attrition (реально сталось 15.03, але ВНЕСЛИ 20.03 — бітемпоральна затримка подання).
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO group_event (group_id, event_type, count, occurred_on, recorded_at) VALUES \
            ($1, 'started', 20, '2026-03-05', '2026-03-05T09:00:00Z'), \
            ($1, 'added', 5, '2026-03-10', '2026-03-10T09:00:00Z'), \
            ($1, 'attrition', 3, '2026-03-15', '2026-03-20T09:00:00Z'), \
            ($1, 'completed', 12, '2026-04-01', '2026-04-01T09:00:00Z')",
        [group_id.into()],
    ))
    .await
    .unwrap();

    let events = group_events(&db, group_id).await.unwrap();
    assert_eq!(events.len(), 4, "{events:?}");

    // --- 06-roadmap.md, Етап 3, "Готово, коли": формули воронки на прикладі ---
    assert_eq!(in_training(&events, "2026-03-04", None), 0);
    assert_eq!(in_training(&events, "2026-03-05", None), 20);
    assert_eq!(in_training(&events, "2026-03-10", None), 25);
    assert_eq!(in_training(&events, "2026-03-18", None), 22, "started+added-attrition");
    assert_eq!(in_training(&events, "2026-04-01", None), 10, "-12 completed");

    assert_eq!(
        finishing_on(&events, "2026-04-01", "2026-04-01", None),
        Finishing::Actual(12),
        "є факт completed на planned_end"
    );

    // --- бітемпоральний тест: "як ми знали на момент T" (01 §3), тепер через реальний SQL-шлях ---
    assert_eq!(
        in_training(&events, "2026-03-18", Some("2026-03-18T00:00:00")),
        25,
        "на момент T=18.03 ще не знали про attrition від 15.03 (внесли лише 20.03)"
    );
    assert_eq!(
        in_training(&events, "2026-03-18", Some("2026-03-21T00:00:00")),
        22,
        "на момент T=21.03 вже знали про attrition від 15.03"
    );

    // events_on-подібна перевірка через сам список подій.
    let attrition_events: i64 =
        events.iter().filter(|e| e.event_type == EventType::Attrition).map(|e| i64::from(e.count)).sum();
    assert_eq!(attrition_events, 3);
}
