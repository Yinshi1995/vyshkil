//! Інтеграційні тести проти реальної (не-мокованої) БД для агрегату "організація" — окрема
//! тестова база в тому ж контейнері Postgres, міграції (включно з dev-сідом) з нуля на кожен
//! прогін ("Як вести розробку далі" §3, docs/spec/00-agent-brief.md). Лише під `ssr` — без цієї
//! фічі `app::backend` не існує.
#![cfg(feature = "ssr")]

use app::backend::repo::orgs::{org_detail, search_orgs, subordination_tree};
use app::types::org::OrgTreeRow;
use migration::MigratorTrait;
use sea_orm::{ConnectionTrait, DatabaseConnection};

/// Потребують `TEST_DATABASE_URL` (напр. `postgres://taktoblik:taktoblik@localhost:5432/taktoblik_test`).
/// Якщо не задано — тест пропускається (той самий підхід, що й для `source_files/`-тестів:
/// "тести з ними — skip за відсутності", 00-agent-brief.md).
async fn fresh_test_db() -> Option<DatabaseConnection> {
    let Ok(test_url) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!(
            "TEST_DATABASE_URL не задано — інтеграційний тест app/tests/orgs.rs пропущено \
             (див. .claude/memory/MEMORY.md)"
        );
        return None;
    };

    let slash = test_url
        .rfind('/')
        .expect("TEST_DATABASE_URL має бути виду postgres://.../ім'я_бази");
    let db_name = &test_url[slash + 1..];
    assert!(
        !db_name.is_empty() && db_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "ім'я тестової бази має містити лише [a-zA-Z0-9_]: {db_name:?}"
    );
    let admin_url = format!("{}/postgres", &test_url[..slash]);

    let admin_db = sea_orm::Database::connect(&admin_url)
        .await
        .expect("не вдалось з'єднатись з maintenance-базою 'postgres' для перестворення тестової БД");
    admin_db
        .execute_unprepared(&format!(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity \
             WHERE datname = '{db_name}' AND pid <> pg_backend_pid()"
        ))
        .await
        .expect("не вдалось розірвати старі з'єднання з тестовою базою");
    admin_db
        .execute_unprepared(&format!("DROP DATABASE IF EXISTS {db_name}"))
        .await
        .expect("не вдалось видалити стару тестову базу");
    admin_db
        .execute_unprepared(&format!("CREATE DATABASE {db_name}"))
        .await
        .expect("не вдалось створити тестову базу");

    let db = sea_orm::Database::connect(&test_url)
        .await
        .expect("не вдалось з'єднатись зі свіжою тестовою базою");
    migration::Migrator::up(&db, None)
        .await
        .expect("не вдалось прогнати міграції на тестовій базі");

    Some(db)
}

/// Один тест на всі три Stage-1-сценарії — щоб не ганяти дороге перестворення БД
/// (DROP+CREATE+міграції) кілька разів і не ловити гонки паралельних `cargo test`.
#[tokio::test]
async fn stage1_readiness_scenarios() {
    let Some(db) = fresh_test_db().await else { return };

    // --- 06-roadmap.md: "пошук 152НЦ/а4896/польша знаходить канонічні організації" ---
    let by_152nc = search_orgs(&db, "152НЦ").await.unwrap();
    assert!(
        by_152nc.iter().any(|r| r.label.starts_with("152 нц") && r.is_exact),
        "«152НЦ» має точно знайти 152 нц: {by_152nc:?}"
    );

    let by_a4896 = search_orgs(&db, "а4896").await.unwrap();
    assert!(
        by_a4896.iter().any(|r| r.label.starts_with("152 нц")),
        "«а4896» має знайти 152 нц: {by_a4896:?}"
    );

    let by_polsha = search_orgs(&db, "польша").await.unwrap();
    assert!(
        by_polsha.iter().any(|r| r.label == "Республіка Польща" && r.is_exact),
        "«польша» має точно знайти Республіка Польща: {by_polsha:?}"
    );

    // --- транзитивна перевірка: "неіснуючий" запит не знаходить нічого ---
    let nothing = search_orgs(&db, "жжжнеіснуєжжж").await.unwrap();
    assert!(nothing.is_empty(), "вигаданий запит не має нічого знаходити: {nothing:?}");

    // --- 06-roadmap.md: "підлеглі 17 АК на 2026-07-20 і на 2026-09-20 дають різні набори
    // (142/154/61/5/92/225 пішли в 7 КШР)" ---
    let before = subordination_tree(&db, "2026-07-20", "staff").await.unwrap();
    let after = subordination_tree(&db, "2026-09-20", "staff").await.unwrap();

    let seventeenth_ak_id = before
        .iter()
        .find(|r| r.label == "17 АК")
        .expect("17 АК має бути в дереві")
        .id;
    let seventh_kshr_id = before
        .iter()
        .find(|r| r.label == "7 КШР")
        .expect("7 КШР має бути в дереві")
        .id;

    let children_of = |rows: &[OrgTreeRow], parent: i32| -> Vec<String> {
        let mut v: Vec<String> =
            rows.iter().filter(|r| r.parent_id == Some(parent)).map(|r| r.label.clone()).collect();
        v.sort();
        v
    };

    let ak_before = children_of(&before, seventeenth_ak_id);
    let ak_after = children_of(&after, seventeenth_ak_id);
    assert_eq!(ak_before.len(), 29, "17 АК на 2026-07-20 мала 29 підлеглих: {ak_before:?}");
    assert_eq!(ak_after.len(), 23, "17 АК на 2026-09-20 має 23 підлеглих: {ak_after:?}");

    let kshr_after = children_of(&after, seventh_kshr_id);
    for moved in ["142 омбр", "154 омбр", "61 омбр", "5 омбр", "92 ошбр", "225 ошп", "44 оабр"] {
        assert!(
            kshr_after.iter().any(|s| s.starts_with(moved)),
            "{moved} має бути під 7 КШР на 2026-09-20: {kshr_after:?}"
        );
        assert!(
            !ak_after.iter().any(|s| s.starts_with(moved)),
            "{moved} НЕ має лишатись під 17 АК на 2026-09-20: {ak_after:?}"
        );
    }

    // --- 01 §"subordination": 110 омбр одночасно штатно в 17 АК і оперативно в 20 АК ---
    let staff_110 = after
        .iter()
        .find(|r| r.label.starts_with("110 омбр"))
        .expect("110 омбр має бути в дереві (staff)")
        .parent_id;
    let operational = subordination_tree(&db, "2026-09-20", "operational").await.unwrap();
    let operational_110 = operational
        .iter()
        .find(|r| r.label.starts_with("110 омбр"))
        .expect("110 омбр має бути в дереві (operational)")
        .parent_id;
    let twentieth_ak_id =
        operational.iter().find(|r| r.label == "20 АК").expect("20 АК має бути в дереві").id;

    assert_eq!(staff_110, Some(seventeenth_ak_id), "110 омбр штатно підпорядкована 17 АК");
    assert_eq!(operational_110, Some(twentieth_ak_id), "110 омбр оперативно підпорядкована 20 АК");
    assert_ne!(
        staff_110, operational_110,
        "110 омбр має різних батьків на різних осях одночасно (01, dual-axis)"
    );

    // --- org_detail: існуюча org повертає Some (порожня історія — легітимно для dev-сіду, де
    // ще не було жодного перейменування/зміни статусу), неіснуюча — None ---
    let some_org_id = before.first().expect("дерево не порожнє").id;
    let detail = org_detail(&db, some_org_id).await.unwrap();
    assert!(detail.is_some(), "org_detail(існуючий id) має повернути Some");

    let missing = org_detail(&db, -1).await.unwrap();
    assert!(missing.is_none(), "org_detail(-1) має повернути None");
}
