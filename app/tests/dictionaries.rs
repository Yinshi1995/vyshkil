//! Інтеграційний тест агрегату "довідники" (Етап 2) — окрема тестова БД, той самий підхід,
//! що й `app/tests/orgs.rs` (див. `common`). Лише під `ssr`.
//!
//! Один тест на весь агрегат (як і в orgs.rs/policy.rs) — щоб не ганяти дороге перестворення БД
//! кілька разів і не ловити гонки паралельних `cargo test` (кілька тестових функцій в одному
//! файлі виконуються паралельно за замовчуванням, а `fresh_test_db()` кожна робить DROP+CREATE
//! DATABASE — конкурентні виклики валяться з "duplicate key… pg_database").
#![cfg(feature = "ssr")]

mod common;

use app::backend::repo::dictionaries::{
    confirm_learned_alias, dictionaries_overview, equipment_vos_hint, learned_aliases,
    reject_learned_alias,
};
use common::fresh_test_db;
use sea_orm::{ConnectionTrait, FromQueryResult, Statement};

#[tokio::test]
async fn dictionaries_stage2_scenarios() {
    let Some(db) = fresh_test_db().await else { return };

    // --- 06-roadmap.md, Етап 2, "Готово, коли": підказки "вамп"→218, "mavic"→217, "fpv"→219,
    // "нрк"→129, "darts"→216, з поясненням "бо …" ---
    let cases: &[(&str, &str)] = &[
        ("вамп", "218"),
        ("mavic", "217"),
        ("fpv", "219"),
        ("нрк", "129"),
        ("darts", "216"),
    ];
    for (query, expected_code) in cases {
        let hints = equipment_vos_hint(&db, query).await.unwrap();
        assert!(
            hints.iter().any(|h| h.vos_code == *expected_code),
            "«{query}» має знайти ВОС {expected_code}: {hints:?}"
        );
        let hit = hints.iter().find(|h| h.vos_code == *expected_code).unwrap();
        assert!(!hit.matched_raw.is_empty(), "пояснення \"бо …\" не має бути порожнім: {hit:?}");
    }

    // "вамп" — точний seed-синонім "Вампір"/"ВАМПІР" (не просто випадкова схожість).
    let vamp = equipment_vos_hint(&db, "Вампір").await.unwrap();
    assert!(
        vamp.iter().any(|h| h.vos_code == "218" && h.is_exact),
        "\"Вампір\" (точна форма з сіду) має дати точний збіг: {vamp:?}"
    );

    let nothing = equipment_vos_hint(&db, "жжжнеіснуєжжж").await.unwrap();
    assert!(nothing.is_empty(), "вигаданий запит не має нічого знаходити: {nothing:?}");

    // --- адмінка довідників: усі "прості" словники непорожні після сіду ---
    let overview = dictionaries_overview(&db).await.unwrap();
    assert_eq!(overview.training_kinds.len(), 4, "{:?}", overview.training_kinds);
    assert_eq!(overview.training_directions.len(), 3, "{:?}", overview.training_directions);
    assert_eq!(overview.bzvp_programs.len(), 4, "{:?}", overview.bzvp_programs);
    assert_eq!(overview.vos.len(), 79, "{:?}", overview.vos.len());
    assert_eq!(overview.positions.len(), 48, "{:?}", overview.positions.len());
    assert_eq!(overview.equipment.len(), 17, "{:?}", overview.equipment);
    assert_eq!(overview.courses.len(), 6, "{:?}", overview.courses);
    assert_eq!(overview.attrition_reasons.len(), 11, "{:?}", overview.attrition_reasons);

    let inshe =
        overview.attrition_reasons.iter().find(|e| e.label == "інше").expect("«інше» має бути в списку");
    assert_eq!(inshe.extra.as_deref(), Some("потребує примітки"));

    // --- 01 §"Навчання": адмін підтверджує (-> source='manual', лишається в alias) або
    // відхиляє (-> видаляється) learned-синонім. Тестові рядки вставлені напряму SQL — учнівський
    // alias з'явиться в реальному UI лише зі Стадії 4 (форма введення), тут перевіряємо саму дію.
    let equipment_id: i32 = {
        #[derive(FromQueryResult)]
        struct Row {
            id: i32,
        }
        let row = Row::find_by_statement(Statement::from_string(
            db.get_database_backend(),
            "SELECT id FROM equipment ORDER BY id LIMIT 1",
        ))
        .one(&db)
        .await
        .unwrap()
        .expect("сід має містити хоча б один equipment");
        row.id
    };

    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO alias (target_type, target_id, raw, norm, source) \
         VALUES ('equipment', $1, 'testovyi_sinonim_confirm', 'testovyi_sinonim_confirm', 'learned'), \
                ('equipment', $1, 'testovyi_sinonim_reject', 'testovyi_sinonim_reject', 'learned')",
        [equipment_id.into()],
    ))
    .await
    .unwrap();

    let queue = learned_aliases(&db).await.unwrap();
    assert_eq!(queue.len(), 2, "{queue:?}");
    let confirm_id = queue.iter().find(|a| a.raw == "testovyi_sinonim_confirm").unwrap().id;
    let reject_id = queue.iter().find(|a| a.raw == "testovyi_sinonim_reject").unwrap().id;

    confirm_learned_alias(&db, confirm_id).await.unwrap();
    reject_learned_alias(&db, reject_id).await.unwrap();

    let queue_after = learned_aliases(&db).await.unwrap();
    assert!(queue_after.is_empty(), "обидва рядки мають зникнути з черги: {queue_after:?}");

    #[derive(FromQueryResult)]
    struct SourceRow {
        source: String,
    }
    let confirmed = SourceRow::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT source FROM alias WHERE id = $1",
        [confirm_id.into()],
    ))
    .one(&db)
    .await
    .unwrap();
    assert_eq!(confirmed.map(|r| r.source), Some("manual".to_string()), "підтверджений лишається, стає manual");

    let rejected = SourceRow::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT source FROM alias WHERE id = $1",
        [reject_id.into()],
    ))
    .one(&db)
    .await
    .unwrap();
    assert!(rejected.is_none(), "відхилений рядок має бути видалений");
}
