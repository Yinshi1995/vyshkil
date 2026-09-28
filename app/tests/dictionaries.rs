//! Інтеграційний тест агрегату "довідники" (Етап 2) — окрема тестова БД, той самий підхід,
//! що й `app/tests/orgs.rs` (див. `common`). Лише під `ssr`.
#![cfg(feature = "ssr")]

mod common;

use app::backend::repo::dictionaries::equipment_vos_hint;
use common::fresh_test_db;

/// 06-roadmap.md, Етап 2, "Готово, коли": запит підказок "вамп" → 218, "mavic" → 217,
/// "fpv" → 219, "нрк" → 129, "darts" → 216 з поясненням "бо …".
#[tokio::test]
async fn equipment_vos_hints_match_roadmap_examples() {
    let Some(db) = fresh_test_db().await else { return };

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
}
