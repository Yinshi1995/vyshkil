//! Інтеграційний тест Етапу 8, зріз 1 (04 §2-4, `.claude/decisions/
//! etap8-horizontal-reconciliation-first-slice.md`) — реальна Postgres, той самий підхід, що
//! `app/tests/groups.rs`. Один `#[tokio::test]` (проєктна конвенція — race на `fresh_test_db`,
//! `app/tests/groups.rs` пояснює чому).
//!
//! Чиста логіка "чи є незгода" — `app/src/domain/reconciliation.rs` (юніт-тести там же). Тут —
//! наскрізна перевірка: `commit_group_rows` зіставляє повторне подання з ІСНУЮЧОЮ канонічною
//! групою (не створює дублікат), і `refresh_horizontal` відкриває/автозакриває `discrepancy`.
#![cfg(feature = "ssr")]

mod common;

use app::backend::repo::groups::{commit_group_rows, ValidatedRow};
use app::backend::repo::reconciliation::list_discrepancies;
use app::types::submission::GroupFormRow;
use chrono::NaiveDate;
use common::fresh_test_db;
use sea_orm::{ConnectionTrait, FromQueryResult, Statement};

#[tokio::test]
async fn repeated_submission_matches_existing_group_and_tracks_horizontal_discrepancy() {
    let Some(db) = fresh_test_db().await else { return };

    #[derive(FromQueryResult)]
    struct IdRow {
        id: i32,
    }
    let org_ids: Vec<i32> = IdRow::find_by_statement(Statement::from_string(
        db.get_database_backend(),
        "SELECT id FROM org ORDER BY id LIMIT 3",
    ))
    .all(&db)
    .await
    .unwrap()
    .into_iter()
    .map(|r| r.id)
    .collect();
    assert!(org_ids.len() >= 3, "dev-сід має щонайменше 3 org");
    let sender_org_id = org_ids[0];
    // Дві РІЗНІ reporting_org (04 §5: "хто РЕАЛЬНО подав" — корпус за підлеглого чи сама
    // частина) — той самий sender_org_id (ключ зіставлення це вимагає), різне джерело подання.
    let reporting_org_a = org_ids[1];
    let reporting_org_b = org_ids[2];

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

    let make_submission = |reporting_org_id: i32| {
        let db = &db;
        async move {
            IdRow::find_by_statement(Statement::from_sql_and_values(
                db.get_database_backend(),
                "INSERT INTO submission (source_type, reporting_org_id, as_of_date, status) \
                 VALUES ('form', $1, '2026-09-28', 'draft') RETURNING id",
                [reporting_org_id.into()],
            ))
            .one(db)
            .await
            .unwrap()
            .expect("INSERT submission ... RETURNING id")
            .id
        }
    };

    let row = |planned: i64, arrived: i64, in_training: i64| GroupFormRow {
        sender_org_id: Some(sender_org_id),
        training_kind_id: Some(training_kind_id),
        site_id: Some(site_id),
        planned_count: planned,
        arrived_count: arrived,
        in_training_count: in_training,
        ..Default::default()
    };
    let dates = || ValidatedRow {
        start: NaiveDate::from_ymd_opt(2026, 8, 18).unwrap(),
        end: NaiveDate::from_ymd_opt(2026, 10, 9).unwrap(),
        basis_doc_date: None,
    };

    // --- Подання №1 (reporting_org_a): план 20, прибуло 18 ---
    let submission_1 = make_submission(reporting_org_a).await;
    let group_ids_1 =
        commit_group_rows(&db, &[(row(20, 18, 15), dates())], submission_1).await.unwrap();
    assert_eq!(group_ids_1.len(), 1);
    let group_id = group_ids_1[0];

    // --- Подання №2 (reporting_org_b, той самий ключ: sender+вид+місце+дата ±3 дні): прибуло 20 ---
    let submission_2 = make_submission(reporting_org_b).await;
    let group_ids_2 =
        commit_group_rows(&db, &[(row(20, 20, 15), dates())], submission_2).await.unwrap();
    assert_eq!(
        group_ids_2, group_ids_1,
        "той самий ключ зіставлення -- ПЕРЕВИКОРИСТАНА канонічна група, не дублікат"
    );

    let open = list_discrepancies(&db, Some("open")).await.unwrap();
    let arrived_discrepancy = open
        .iter()
        .find(|d| d.group_id == Some(group_id) && d.metric == "arrived_count")
        .expect("горизонтальна розбіжність на 'прибуло' має бути відкрита");
    assert_eq!(arrived_discrepancy.status, "open");
    assert_eq!(arrived_discrepancy.values.len(), 2, "{:?}", arrived_discrepancy.values);

    // --- Подання №3 (reporting_org_a знову -- ОСТАННЄ слово цього джерела): виправляє на 20 ---
    let submission_3 = make_submission(reporting_org_a).await;
    let group_ids_3 =
        commit_group_rows(&db, &[(row(20, 20, 15), dates())], submission_3).await.unwrap();
    assert_eq!(group_ids_3, group_ids_1, "усе ще та сама канонічна група");

    let after = list_discrepancies(&db, None).await.unwrap();
    let resolved = after
        .iter()
        .find(|d| d.group_id == Some(group_id) && d.metric == "arrived_count")
        .expect("рядок лишається в історії (04 §4), не видаляється");
    assert_eq!(resolved.status, "resolved", "останнє слово reporting_org_a тепер узгоджується з reporting_org_b");
}
