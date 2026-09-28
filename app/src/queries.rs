//! DTO-и й (лише на сервері) реалізації запитів пошуку/дерева/картки організації.
//!
//! DTO-и компілюються завжди (клієнт отримує їх від `#[server]`-функцій у `app::app` як
//! JSON), а самі запити до Postgres — лише під `feature = "ssr"`, і беруть `&DatabaseConnection`
//! явним аргументом (без `expect_context`), щоб їх можна було викликати напряму з інтеграційних
//! тестів без Leptos-рантайму й без сервера.

use serde::{Deserialize, Serialize};

/// Один результат нечіткого пошуку організацій: канонічна форма + який саме синонім збігся.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrgSearchResult {
    pub org_id: i32,
    pub label: String,
    pub matched_raw: String,
    pub is_exact: bool,
}

/// Один вузол дерева підпорядкування: пряма (`depth = 1`) ланка з `subordination_closure` на дату.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrgTreeRow {
    pub id: i32,
    pub label: String,
    pub parent_id: Option<i32>,
}

/// Картка частини з історією назв і статусів.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrgDetail {
    pub id: i32,
    pub short_name: String,
    pub full_name: Option<String>,
    pub number: Option<String>,
    pub kind: String,
    pub is_active: bool,
    pub name_history: Vec<(String, String, Option<String>)>,
    pub status_history: Vec<(String, String, Option<String>, Option<String>)>,
}

#[cfg(feature = "ssr")]
mod ssr {
    use super::{OrgDetail, OrgSearchResult, OrgTreeRow};
    use crate::normalize::normalize;
    use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement};

    /// Нечіткий пошук організацій по `alias.norm` (02 §3, критерій готовності Етапу 1:
    /// "152НЦ"/"а4896"/"польша" знаходять канонічні організації).
    /// Запит нормалізується тією ж функцією, що й alias.norm при сіді/введенні (01 §"alias") —
    /// інакше "152НЦ" (з великими літерами) не збігся б з засіяним норм-рядком "152нц".
    /// Ранжування — за 02 §3: точний збіг синоніма → частота використання цією організацією →
    /// схожість (pg_trgm).
    pub async fn search_orgs(
        db: &DatabaseConnection,
        query: &str,
    ) -> Result<Vec<OrgSearchResult>, DbErr> {
        let norm_query = normalize(query);
        if norm_query.is_empty() {
            return Ok(Vec::new());
        }

        #[derive(FromQueryResult)]
        struct Row {
            org_id: i32,
            short_name: String,
            number: Option<String>,
            matched_raw: String,
            is_exact: bool,
        }

        let stmt = Statement::from_sql_and_values(
            db.get_database_backend(),
            r#"
            WITH matches AS (
                SELECT
                    o.id AS org_id,
                    o.short_name,
                    o.number,
                    a.raw AS matched_raw,
                    a.uses_count,
                    (a.norm = $1) AS is_exact,
                    similarity(a.norm, $1) AS sim
                FROM alias a
                JOIN org o ON o.id = a.target_id AND a.target_type = 'org'
                WHERE o.deleted_at IS NULL
                  AND (a.norm = $1 OR a.norm % $1)
            ),
            ranked AS (
                SELECT
                    *,
                    ROW_NUMBER() OVER (
                        PARTITION BY org_id
                        ORDER BY is_exact DESC, uses_count DESC, sim DESC
                    ) AS rn
                FROM matches
            )
            SELECT org_id, short_name, number, matched_raw, is_exact
            FROM ranked
            WHERE rn = 1
            ORDER BY is_exact DESC, uses_count DESC, sim DESC
            LIMIT 10
            "#,
            [norm_query.into()],
        );

        let rows = Row::find_by_statement(stmt).all(db).await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let label = match r.number {
                    Some(n) => format!("{} ({n})", r.short_name),
                    None => r.short_name,
                };
                OrgSearchResult {
                    org_id: r.org_id,
                    label,
                    matched_raw: r.matched_raw,
                    is_exact: r.is_exact,
                }
            })
            .collect())
    }

    /// Дерево підпорядкування на дату (06-roadmap.md, Етап 1): перемикач осі штатне/оперативне.
    /// Гарячий запит — через `subordination_closure` (`depth = 1`, матеріалізоване замикання),
    /// **не** рекурсивний CTE (server/CLAUDE.md).
    pub async fn subordination_tree(
        db: &DatabaseConnection,
        as_of: &str,
        axis: &str,
    ) -> Result<Vec<OrgTreeRow>, DbErr> {
        #[derive(FromQueryResult)]
        struct Row {
            id: i32,
            short_name: String,
            number: Option<String>,
            parent_id: Option<i32>,
        }

        let stmt = Statement::from_sql_and_values(
            db.get_database_backend(),
            r#"
            SELECT o.id, o.short_name, o.number, sc.ancestor_id AS parent_id
            FROM org o
            LEFT JOIN subordination_closure sc
                ON sc.descendant_id = o.id
               AND sc.axis = $1
               AND sc.depth = 1
               AND daterange(sc.valid_from, sc.valid_to, '[)') @> $2::date
            WHERE o.deleted_at IS NULL
            ORDER BY o.short_name
            "#,
            [axis.into(), as_of.into()],
        );

        let rows = Row::find_by_statement(stmt).all(db).await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let label = match r.number {
                    Some(n) => format!("{} ({n})", r.short_name),
                    None => r.short_name,
                };
                OrgTreeRow { id: r.id, label, parent_id: r.parent_id }
            })
            .collect())
    }

    /// Картка частини: поточні дані + історія назв/статусів. `None` — org_id не знайдено
    /// (видалено або не існує), щоб виклик міг сам вирішити, як це подати (404 тощо).
    pub async fn org_detail(
        db: &DatabaseConnection,
        org_id: i32,
    ) -> Result<Option<OrgDetail>, DbErr> {
        #[derive(FromQueryResult)]
        struct OrgRow {
            id: i32,
            short_name: String,
            full_name: Option<String>,
            number: Option<String>,
            kind: String,
            is_active: bool,
        }
        #[derive(FromQueryResult)]
        struct NameHistoryRow {
            short_name: String,
            valid_from: String,
            valid_to: Option<String>,
        }
        #[derive(FromQueryResult)]
        struct StatusHistoryRow {
            status: String,
            valid_from: String,
            valid_to: Option<String>,
            note: Option<String>,
        }

        let backend = db.get_database_backend();

        let org = OrgRow::find_by_statement(Statement::from_sql_and_values(
            backend,
            "SELECT id, short_name, full_name, number, kind, is_active \
             FROM org WHERE id = $1 AND deleted_at IS NULL",
            [org_id.into()],
        ))
        .one(db)
        .await?;

        let Some(org) = org else {
            return Ok(None);
        };

        let name_history = NameHistoryRow::find_by_statement(Statement::from_sql_and_values(
            backend,
            "SELECT short_name, to_char(valid_from, 'YYYY-MM-DD') AS valid_from, \
                    to_char(valid_to, 'YYYY-MM-DD') AS valid_to \
             FROM org_name_history WHERE org_id = $1 ORDER BY valid_from",
            [org_id.into()],
        ))
        .all(db)
        .await?;

        let status_history = StatusHistoryRow::find_by_statement(Statement::from_sql_and_values(
            backend,
            "SELECT status, to_char(valid_from, 'YYYY-MM-DD') AS valid_from, \
                    to_char(valid_to, 'YYYY-MM-DD') AS valid_to, note \
             FROM org_status WHERE org_id = $1 ORDER BY valid_from",
            [org_id.into()],
        ))
        .all(db)
        .await?;

        Ok(Some(OrgDetail {
            id: org.id,
            short_name: org.short_name,
            full_name: org.full_name,
            number: org.number,
            kind: org.kind,
            is_active: org.is_active,
            name_history: name_history
                .into_iter()
                .map(|r| (r.short_name, r.valid_from, r.valid_to))
                .collect(),
            status_history: status_history
                .into_iter()
                .map(|r| (r.status, r.valid_from, r.valid_to, r.note))
                .collect(),
        }))
    }

    /// Інтеграційні тести проти реальної (не-мокованої) БД — окрема тестова база в тому ж
    /// контейнері Postgres, міграції (включно з dev-сідом) з нуля на кожен прогін
    /// ("Як вести розробку далі" §3, docs/spec/00-agent-brief.md).
    ///
    /// Потребують `TEST_DATABASE_URL` (напр. `postgres://taktoblik:taktoblik@localhost:5432/taktoblik_test`).
    /// Якщо не задано — тест пропускається (той самий підхід, що й для `source_files/`-тестів:
    /// "тести з ними — skip за відсутності", 00-agent-brief.md).
    #[cfg(test)]
    mod tests {
        use super::*;
        use migration::MigratorTrait;

        /// Перестворює тестову БД з нуля (DROP+CREATE DATABASE) і прожене всі міграції
        /// (включно з dev-сідом m…_000011) — щоб тест завжди бачив ту саму, детерміновану
        /// вихідну точку, а не залишки попереднього прогону чи ручних експериментів у psql.
        async fn fresh_test_db() -> Option<DatabaseConnection> {
            let Ok(test_url) = std::env::var("TEST_DATABASE_URL") else {
                eprintln!(
                    "TEST_DATABASE_URL не задано — інтеграційний тест queries::tests пропущено \
                     (див. .claude/memory/MEMORY.md)"
                );
                return None;
            };

            let slash = test_url
                .rfind('/')
                .expect("TEST_DATABASE_URL має бути виду postgres://.../ім'я_бази");
            let db_name = &test_url[slash + 1..];
            assert!(
                !db_name.is_empty()
                    && db_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
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
                let mut v: Vec<String> = rows
                    .iter()
                    .filter(|r| r.parent_id == Some(parent))
                    .map(|r| r.label.clone())
                    .collect();
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
            assert_eq!(
                operational_110,
                Some(twentieth_ak_id),
                "110 омбр оперативно підпорядкована 20 АК"
            );
            assert_ne!(
                staff_110, operational_110,
                "110 омбр має різних батьків на різних осях одночасно (01, dual-axis)"
            );

            // --- get_org_detail: існуюча org повертає Some (порожня історія — легітимно для
            // dev-сіду, де ще не було жодного перейменування/зміни статусу), неіснуюча — None ---
            let some_org_id = before.first().expect("дерево не порожнє").id;
            let detail = org_detail(&db, some_org_id).await.unwrap();
            assert!(detail.is_some(), "org_detail(існуючий id) має повернути Some");

            let missing = org_detail(&db, -1).await.unwrap();
            assert!(missing.is_none(), "org_detail(-1) має повернути None");
        }
    }
}

#[cfg(feature = "ssr")]
pub use ssr::*;
