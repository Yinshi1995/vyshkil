//! SQL/SeaORM для агрегату "організація": `org`, `alias`, `subordination_closure`,
//! `org_name_history`, `org_status`. Без Leptos-контексту (бере `&DatabaseConnection` явним
//! аргументом), щоб бути тестованим напряму інтеграційними тестами (`app/tests/orgs.rs`).

use crate::domain::normalize::normalize;
use crate::types::org::{OrgDetail, OrgSearchResult, OrgTreeRow};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement};

/// Список організацій для перемикача актора й лічильника на головній: (id, "назва (номер)").
/// Прямий SQL, без entity — Stage 1 ще не заводить повноцінні sea-orm entity для org.
pub async fn list_orgs(db: &DatabaseConnection) -> Result<Vec<(i32, String)>, DbErr> {
    #[derive(FromQueryResult)]
    struct OrgRow {
        id: i32,
        short_name: String,
        number: Option<String>,
    }

    let stmt = Statement::from_string(
        db.get_database_backend(),
        "SELECT id, short_name, number FROM org WHERE deleted_at IS NULL ORDER BY short_name",
    );
    let rows = OrgRow::find_by_statement(stmt).all(db).await?;

    Ok(rows
        .into_iter()
        .map(|r| {
            let label = match r.number {
                Some(n) => format!("{} ({n})", r.short_name),
                None => r.short_name,
            };
            (r.id, label)
        })
        .collect())
}

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

/// Найкращий кандидат для сирого тексту (Етап 5, 03 §4: "номер у тексті має пріоритет над назвою"
/// — уже забезпечено ранжуванням `search_orgs`, тут просто беремо перший). `None` — "частина не
/// розпізнана" (03 §5, помилка блокує фіксацію рядка).
pub async fn resolve_org(db: &DatabaseConnection, raw: &str) -> Result<Option<OrgSearchResult>, DbErr> {
    Ok(search_orgs(db, raw).await?.into_iter().next())
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
pub async fn org_detail(db: &DatabaseConnection, org_id: i32) -> Result<Option<OrgDetail>, DbErr> {
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
