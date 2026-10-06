//! SQL для агрегату "групи на навчанні" (01 §3): читання подій (воронка), пошук ВОС/посади/курсу
//! й майданчиків для сітки (02 §3), фіксація рядків сітки в `training_group`/`group_event`
//! (02 §5 — уся сітка зберігається одним усе-або-нічого записом).
//! Сама арифметика воронки — в `domain::counting` (чиста, без БД); тут лише SQL і перетворення типів.

use super::reconciliation::{refresh_horizontal, refresh_temporal, refresh_vertical};
use crate::domain::counting::{EventType, GroupEventRecord};
use crate::domain::dates::{parse_date, parse_end_date, parse_maybe_range, validate_period, DateError};
use crate::domain::normalize::normalize;
use crate::domain::validation::{validate_funnel_order, CountError};
use crate::types::submission::{
    CompositionRow, GroupFormRow, TrainingSiteOption, VosPositionCourseHint, VosPositionCourseKind,
};
use chrono::NaiveDate;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement};
use std::collections::BTreeSet;

/// Усі події групи, найстаріша перша — досить для будь-якої функції `domain::counting`
/// (`in_training`/`events_on`/`finishing_on` самі фільтрують за датою/`known_at`).
pub async fn group_events(
    db: &DatabaseConnection,
    group_id: i32,
) -> Result<Vec<GroupEventRecord>, DbErr> {
    #[derive(FromQueryResult)]
    struct Row {
        event_type: String,
        count: i32,
        occurred_on: String,
        recorded_at: String,
    }

    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        r#"
        SELECT
            event_type,
            count,
            to_char(occurred_on, 'YYYY-MM-DD') AS occurred_on,
            to_char(recorded_at, 'YYYY-MM-DD"T"HH24:MI:SS') AS recorded_at
        FROM group_event
        WHERE group_id = $1
        ORDER BY occurred_on, id
        "#,
        [group_id.into()],
    );

    let rows = Row::find_by_statement(stmt).all(db).await?;

    Ok(rows
        .into_iter()
        .filter_map(|r| {
            EventType::parse(&r.event_type).map(|event_type| GroupEventRecord {
                event_type,
                count: r.count,
                occurred_on: r.occurred_on,
                recorded_at: r.recorded_at,
            })
        })
        .collect())
}

/// Одне поле "ВОС / посада / курс" (02 §3): прямі збіги по `alias` (vos/position/course) +
/// непрямі через ОВТ (`equipment_vos`, той самий шлях, що й `dictionaries::equipment_vos_hint`,
/// тут — з обов'язковим поясненням "бо …", 02 §3: "«вамп» → ВОС 218 · бо «Vampire» → 218").
pub async fn search_vos_position_course(
    db: &DatabaseConnection,
    query: &str,
) -> Result<Vec<VosPositionCourseHint>, DbErr> {
    let norm_query = normalize(query);
    if norm_query.is_empty() {
        return default_vos_position_course_listing(db).await;
    }

    #[derive(FromQueryResult)]
    struct Row {
        kind: String,
        id: i32,
        label: String,
        matched_raw: String,
        is_exact: bool,
        why: Option<String>,
    }

    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        r#"
        WITH direct AS (
            SELECT
                a.target_type AS kind,
                a.target_id AS id,
                CASE a.target_type
                    WHEN 'vos' THEN (SELECT code || ' — ' || title FROM vos WHERE id = a.target_id)
                    WHEN 'position' THEN (SELECT name FROM "position" WHERE id = a.target_id)
                    WHEN 'course' THEN (SELECT name FROM course WHERE id = a.target_id)
                END AS label,
                a.raw AS matched_raw,
                a.uses_count AS weight,
                (a.norm = $1) AS is_exact,
                similarity(a.norm, $1) AS sim,
                NULL::text AS why
            FROM alias a
            WHERE a.target_type IN ('vos', 'position', 'course')
              AND (a.norm = $1 OR a.norm % $1)
        ),
        via_equipment AS (
            SELECT
                'vos' AS kind,
                v.id,
                v.code || ' — ' || v.title AS label,
                a.raw AS matched_raw,
                ev.weight,
                (a.norm = $1) AS is_exact,
                similarity(a.norm, $1) AS sim,
                ('бо "' || e.name || '" → ' || v.code) AS why
            FROM alias a
            JOIN equipment e ON e.id = a.target_id AND a.target_type = 'equipment'
            JOIN equipment_vos ev ON ev.equipment_id = e.id
            JOIN vos v ON v.id = ev.vos_id
            WHERE a.norm = $1 OR a.norm % $1
        ),
        by_code_or_title AS (
            SELECT 'vos' AS kind, v.id,
                   v.code || ' — ' || v.title AS label,
                   v.code AS matched_raw, 0 AS weight,
                   (v.code = $1) AS is_exact,
                   GREATEST(similarity(v.code, $1), similarity(v.title, $1)) AS sim,
                   NULL::text AS why
            FROM vos v
            WHERE v.code = $1 OR v.code ILIKE ($1 || '%')
                  OR v.title ILIKE ('%' || $1 || '%')
            UNION ALL
            SELECT 'position' AS kind, p.id,
                   p.name AS label,
                   p.name AS matched_raw, 0 AS weight,
                   false AS is_exact,
                   similarity(p.name, $1) AS sim,
                   NULL::text AS why
            FROM "position" p
            WHERE p.name ILIKE ('%' || $1 || '%')
            UNION ALL
            SELECT 'course' AS kind, c.id,
                   c.name AS label,
                   c.name AS matched_raw, 0 AS weight,
                   false AS is_exact,
                   similarity(c.name, $1) AS sim,
                   NULL::text AS why
            FROM course c
            WHERE c.name ILIKE ('%' || $1 || '%')
        ),
        combined AS (
            SELECT * FROM direct
            UNION ALL SELECT * FROM via_equipment
            UNION ALL SELECT * FROM by_code_or_title
        ),
        ranked AS (
            SELECT
                *,
                ROW_NUMBER() OVER (
                    PARTITION BY kind, id
                    ORDER BY is_exact DESC, weight DESC, sim DESC
                ) AS rn
            FROM combined
            WHERE label IS NOT NULL
        )
        SELECT kind, id, label, matched_raw, is_exact, why
        FROM ranked
        WHERE rn = 1
        ORDER BY is_exact DESC, sim DESC
        LIMIT 10
        "#,
        [norm_query.into()],
    );

    let rows = Row::find_by_statement(stmt).all(db).await?;

    Ok(rows
        .into_iter()
        .filter_map(|r| {
            let kind = match r.kind.as_str() {
                "vos" => VosPositionCourseKind::Vos,
                "position" => VosPositionCourseKind::Position,
                "course" => VosPositionCourseKind::Course,
                _ => return None,
            };
            Some(VosPositionCourseHint {
                kind,
                id: r.id,
                label: r.label,
                matched_raw: r.matched_raw,
                is_exact: r.is_exact,
                why: r.why,
            })
        })
        .collect())
}

/// "Весь довідник" для порожнього запиту (grid-interaction.md §2, той самий принцип, що
/// `orgs::default_org_listing`) — прості "перші N за кодом/назвою" з кожного з трьох довідників,
/// не спроба глобальної "найчастіші"-статистики (свідоме спрощення).
async fn default_vos_position_course_listing(
    db: &DatabaseConnection,
) -> Result<Vec<VosPositionCourseHint>, DbErr> {
    #[derive(FromQueryResult)]
    struct Row {
        kind: String,
        id: i32,
        label: String,
    }
    let stmt = Statement::from_string(
        db.get_database_backend(),
        r#"
        (SELECT 'vos' AS kind, id, code || ' — ' || title AS label FROM vos ORDER BY code LIMIT 15)
        UNION ALL
        (SELECT 'position' AS kind, id, name AS label FROM "position" ORDER BY name LIMIT 10)
        UNION ALL
        (SELECT 'course' AS kind, id, name AS label FROM course ORDER BY name LIMIT 10)
        "#,
    );
    let rows = Row::find_by_statement(stmt).all(db).await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| {
            let kind = match r.kind.as_str() {
                "vos" => VosPositionCourseKind::Vos,
                "position" => VosPositionCourseKind::Position,
                "course" => VosPositionCourseKind::Course,
                _ => return None,
            };
            Some(VosPositionCourseHint {
                kind,
                id: r.id,
                label: r.label,
                matched_raw: String::new(),
                is_exact: false,
                why: None,
            })
        })
        .collect())
}

/// Майданчики навчання конкретної частини (02 §1 колонка 5, після вибору частини) — `training_site`.
pub async fn training_site_options(
    db: &DatabaseConnection,
    org_id: i32,
) -> Result<Vec<TrainingSiteOption>, DbErr> {
    #[derive(FromQueryResult)]
    struct Row {
        id: i32,
        locality: Option<String>,
    }

    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id, locality FROM training_site WHERE org_id = $1 ORDER BY locality NULLS FIRST",
        [org_id.into()],
    );
    let rows = Row::find_by_statement(stmt).all(db).await?;

    Ok(rows
        .into_iter()
        .map(|r| TrainingSiteOption {
            site_id: r.id,
            label: r.locality.unwrap_or_else(|| "на базі частини".to_string()),
        })
        .collect())
}

/// Пошук майданчиків навчання по тексту (Data Workspace — створення групи).
#[derive(FromQueryResult, serde::Serialize)]
pub struct SiteSearchRow {
    pub id: i32,
    pub label: String,
}

pub async fn search_training_sites(
    db: &DatabaseConnection,
    query: &str,
    limit: i64,
) -> Result<Vec<SiteSearchRow>, DbErr> {
    let pattern = format!("%{}%", query.to_lowercase());
    SiteSearchRow::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT ts.id, \
                CASE WHEN ts.locality IS NOT NULL \
                     THEN ts.locality || ' (' || o.short_name || ')' \
                     ELSE o.short_name \
                END AS label \
         FROM training_site ts \
         JOIN org o ON o.id = ts.org_id \
         WHERE LOWER(COALESCE(ts.locality, '')) LIKE $1 \
            OR LOWER(o.short_name) LIKE $1 \
         ORDER BY o.short_name LIMIT $2",
        [pattern.into(), limit.into()],
    ))
    .all(db)
    .await
}

/// Розібрані дати одного рядка сітки — те, що `validate_row` віддає, а `commit_group_rows` бере
/// (щоб не парсити двічі: сітка валідується заздалегідь, коміт довіряє результату).
pub struct ValidatedRow {
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub basis_doc_date: Option<NaiveDate>,
}

/// Майданчик навчання для org_id без нас. пункту (locality=NULL) — знайти або створити (Етап 5:
/// імпорт резолвить "місце" як організацію, локальність поки не розбираємо окремо, задокументоване
/// спрощення). Унікальний частковий індекс на `(org_id) WHERE locality IS NULL` (01 §1) гарантує
/// не більше одного такого рядка на org — знайти-або-створити безпечний навіть під конкурентним
/// імпортом (ON CONFLICT DO NOTHING + повторний SELECT).
pub async fn find_or_create_training_site(
    db: &impl ConnectionTrait,
    org_id: i32,
) -> Result<i32, DbErr> {
    #[derive(FromQueryResult)]
    struct Row {
        id: i32,
    }
    if let Some(row) = Row::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id FROM training_site WHERE org_id = $1 AND locality IS NULL",
        [org_id.into()],
    ))
    .one(db)
    .await?
    {
        return Ok(row.id);
    }

    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO training_site (org_id, locality) VALUES ($1, NULL) ON CONFLICT DO NOTHING",
        [org_id.into()],
    ))
    .await?;

    let row = Row::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id FROM training_site WHERE org_id = $1 AND locality IS NULL",
        [org_id.into()],
    ))
    .one(db)
    .await?
    .ok_or_else(|| DbErr::Custom("find_or_create_training_site: рядок не з'явився".into()))?;
    Ok(row.id)
}

/// Одна помилка валідації рядка сітки: (назва ПОЛЯ СІТКИ українською — той самий підпис, що й
/// заголовок колонки в `widgets::group_grid::Grid`, не ім'я поля `GroupFormRow` в Rust-коді, —
/// показується користувачу напряму) для `CommitOutcome::ValidationFailed` (02 §5). Перевіряє
/// тільки те, що `domain` вміє без БД (дати, порядок воронки, обов'язкові поля); биту зовнішню
/// посилальну цілісність (неіснуючий vos_id тощо) ловить FK-обмеження при INSERT.
pub fn validate_row(row: &GroupFormRow, as_of: NaiveDate) -> Result<ValidatedRow, (String, String)> {
    let Some(_) = row.sender_org_id else {
        return Err(("Частина".into(), "не вказано частину-відправника".into()));
    };
    let Some(_) = row.training_kind_id else {
        return Err(("Вид підготовки".into(), "не вказано вид підготовки".into()));
    };
    let Some(_) = row.site_id else {
        return Err(("Місце".into(), "не вказано місце проведення".into()));
    };

    let map_date_err = |field: &str, e: DateError| (field.to_string(), e.message());

    let (start, range_end) = parse_maybe_range(&row.planned_start_raw, as_of)
        .map_err(|e| map_date_err("З", e))?;

    let end = if row.planned_end_raw.trim().is_empty() {
        range_end.ok_or_else(|| ("По".to_string(), "не вказано термін \"по\"".to_string()))?
    } else {
        parse_end_date(&row.planned_end_raw, start).map_err(|e| map_date_err("По", e))?
    };
    validate_period(start, end).map_err(|e| map_date_err("По", e))?;

    let map_count_err = |field: &str, e: CountError| {
        let message = match e {
            CountError::Negative => "кількість не може бути від'ємною".to_string(),
            CountError::FunnelOrderViolated => {
                "порушено порядок воронки: план ≥ прибуло ≥ навчаються".to_string()
            }
            CountError::BalanceExceeded { available, requested, event_type } => {
                format!("неможливо {event_type} {requested}: у групі лише {available} осіб")
            }
        };
        (field.to_string(), message)
    };
    validate_funnel_order(row.planned_count, row.arrived_count, row.in_training_count)
        .map_err(|e| map_count_err("План", e))?;

    let basis_doc_date = if row.basis_doc_date_raw.trim().is_empty() {
        None
    } else {
        Some(
            parse_date(&row.basis_doc_date_raw, as_of)
                .map_err(|e| map_date_err("Дата розпорядження", e))?,
        )
    };

    Ok(ValidatedRow { start, end, basis_doc_date })
}

/// Допуск зіставлення за датою початку (04 §2: "±3 дні (допуск налаштовуваний)") — константа
/// зараз, майбутній адмін-налаштований поріг (Етап 8, наступний зріз) не зачепить виклики.
const MATCH_DATE_TOLERANCE_DAYS: i64 = 3;

/// Шукає ІСНУЮЧУ канонічну `training_group` за ключем зіставлення (04 §2): відправник + вид +
/// (ВОС|посада|курс|програма БЗВП — `IS NOT DISTINCT FROM`, бо для БЗВП/курсів частина цих полів
/// NULL) + місце + `planned_start` ±`MATCH_DATE_TOLERANCE_DAYS`. РІВНО один кандидат → Some
/// (перевикористати); 0 або 2+ → None (нова група) — 2+ навмисно, не "перший-ліпший"
/// (`.claude/decisions/etap8-horizontal-reconciliation-first-slice.md`: неоднозначність
/// відкладена на UI-крок дизамбігуації, безпечний дефолт зараз — той самий, що сьогоднішня
/// поведінка без зіставлення взагалі).
async fn find_matching_group(
    db: &impl ConnectionTrait,
    row: &GroupFormRow,
    start: NaiveDate,
) -> Result<Option<i32>, DbErr> {
    #[derive(FromQueryResult)]
    struct Row {
        id: i32,
    }

    let lo = (start - chrono::Duration::days(MATCH_DATE_TOLERANCE_DAYS)).format("%Y-%m-%d").to_string();
    let hi = (start + chrono::Duration::days(MATCH_DATE_TOLERANCE_DAYS)).format("%Y-%m-%d").to_string();

    // sender_org_id/training_kind_id/site_id гарантовано Some -- `validate_row` це перевіряє
    // ДО того, як рядок узагалі потрапляє сюди (той самий інваріант, що вже спирається
    // решта цієї функції, напр. INSERT нижче теж бере ці поля напряму).
    let candidates = Row::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id FROM training_group \
         WHERE sender_org_id = $1 AND training_kind_id = $2 AND site_id = $3 \
           AND vos_id IS NOT DISTINCT FROM $4 \
           AND position_id IS NOT DISTINCT FROM $5 \
           AND course_id IS NOT DISTINCT FROM $6 \
           AND bzvp_program_id IS NOT DISTINCT FROM $7 \
           AND planned_start BETWEEN $8::date AND $9::date",
        [
            row.sender_org_id.expect("validate_row гарантує Some").into(),
            row.training_kind_id.expect("validate_row гарантує Some").into(),
            row.site_id.expect("validate_row гарантує Some").into(),
            row.vos_id.into(),
            row.position_id.into(),
            row.course_id.into(),
            row.bzvp_program_id.into(),
            lo.into(),
            hi.into(),
        ],
    ))
    .all(db)
    .await?;

    Ok(match candidates.len() {
        1 => Some(candidates[0].id),
        _ => None,
    })
}

async fn insert_reported_group(
    db: &impl ConnectionTrait,
    row: &GroupFormRow,
    dates: &ValidatedRow,
    submission_id: i32,
    matched_group_id: i32,
) -> Result<(), DbErr> {
    let basis_doc_date = dates.basis_doc_date.map(|d| d.format("%Y-%m-%d").to_string());
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO reported_group \
            (submission_id, sender_org_id, training_kind_id, bzvp_program_id, vos_id, \
             position_id, course_id, site_id, organizer_org_id, planned_start, planned_end, \
             equipment_text, basis_doc_number, basis_doc_date, note, \
             planned_count, arrived_count, in_training_count, matched_group_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::date, $11::date, $12, $13, $14::date, \
                 $15, $16, $17, $18, $19)",
        [
            submission_id.into(),
            row.sender_org_id.into(),
            row.training_kind_id.into(),
            row.bzvp_program_id.into(),
            row.vos_id.into(),
            row.position_id.into(),
            row.course_id.into(),
            row.site_id.into(),
            row.organizer_org_id.into(),
            dates.start.format("%Y-%m-%d").to_string().into(),
            dates.end.format("%Y-%m-%d").to_string().into(),
            non_empty(&row.equipment_text).into(),
            non_empty(&row.basis_doc_number).into(),
            basis_doc_date.into(),
            non_empty(&row.note).into(),
            (row.planned_count as i32).into(),
            (row.arrived_count as i32).into(),
            (row.in_training_count as i32).into(),
            matched_group_id.into(),
        ],
    ))
    .await?;
    Ok(())
}

/// Фіксація сітки (02 §5, `Ctrl+Enter`) — усе-або-нічого: викликач обгортає одну транзакцію,
/// валідує (`validate_row`) заздалегідь усі рядки, і лише тоді викликає цю функцію.
/// Події воронки для щойно внесеної групи спрощено записуються на дату початку (`planned_start`):
/// форма не збирає окремих дат "викликали"/"прибули"/"розпочали" (02 §1 колонка 7: "для нової
/// групи часто рівні").
///
/// **Етап 8, зріз 1** (04 §2-4, `.claude/decisions/etap8-horizontal-reconciliation-first-slice.md`):
/// кожен рядок спершу ЗІСТАВЛЯЄТЬСЯ з існуючою канонічною групою (`find_matching_group`) — при
/// збігу перевикористовує її `id` (події пишуться на той самий group_id, що й раніше — та сама
/// логіка `insert_event`, лише вже не завжди на щойно вставлений рядок), інакше створює нову, як
/// і раніше. Кожен рядок ТАКОЖ незмінно зберігається в `reported_group` (04 §2 — "що саме
/// подала ця сесія"), а після коміту всіх рядків — для кожної зачепленої канонічної групи
/// перераховується горизонтальна звірка (`repo::reconciliation::refresh_horizontal`).
pub async fn commit_group_rows(
    db: &impl ConnectionTrait,
    rows: &[(GroupFormRow, ValidatedRow)],
    submission_id: i32,
) -> Result<Vec<i32>, DbErr> {
    #[derive(FromQueryResult)]
    struct NewId {
        id: i32,
    }

    let mut group_ids = Vec::with_capacity(rows.len());
    let mut affected_groups = BTreeSet::new();

    for (row, dates) in rows {
        let matched = find_matching_group(db, row, dates.start).await?;

        let group_id = match matched {
            Some(existing_id) => existing_id,
            None => {
                let basis_doc_date = dates.basis_doc_date.map(|d| d.format("%Y-%m-%d").to_string());
                let group = NewId::find_by_statement(Statement::from_sql_and_values(
                    db.get_database_backend(),
                    "INSERT INTO training_group \
                        (sender_org_id, training_kind_id, bzvp_program_id, vos_id, position_id, \
                         course_id, equipment_text, site_id, organizer_org_id, planned_start, \
                         planned_end, basis_doc_number, basis_doc_date, note) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::date, $11::date, $12, \
                             $13::date, $14) \
                     RETURNING id",
                    [
                        row.sender_org_id.into(),
                        row.training_kind_id.into(),
                        row.bzvp_program_id.into(),
                        row.vos_id.into(),
                        row.position_id.into(),
                        row.course_id.into(),
                        non_empty(&row.equipment_text).into(),
                        row.site_id.into(),
                        row.organizer_org_id.into(),
                        dates.start.format("%Y-%m-%d").to_string().into(),
                        dates.end.format("%Y-%m-%d").to_string().into(),
                        non_empty(&row.basis_doc_number).into(),
                        basis_doc_date.into(),
                        non_empty(&row.note).into(),
                    ],
                ))
                .one(db)
                .await?
                .ok_or_else(|| DbErr::Custom("INSERT training_group не повернув id".into()))?;
                group.id
            }
        };

        insert_reported_group(db, row, dates, submission_id, group_id).await?;

        for composition in &row.composition {
            insert_composition(db, group_id, composition).await?;
        }

        let start_str = dates.start.format("%Y-%m-%d").to_string();
        insert_event(db, group_id, "planned", row.planned_count as i32, &start_str, submission_id)
            .await?;
        if row.arrived_count > 0 {
            insert_event(db, group_id, "arrived", row.arrived_count as i32, &start_str, submission_id)
                .await?;
        }
        if row.in_training_count > 0 {
            insert_event(
                db,
                group_id,
                "started",
                row.in_training_count as i32,
                &start_str,
                submission_id,
            )
            .await?;
        }

        group_ids.push(group_id);
        affected_groups.insert(group_id);
    }

    for group_id in &affected_groups {
        refresh_horizontal(db, *group_id).await?;
        refresh_temporal(db, *group_id).await?;
    }

    // Вертикальна звірка — на рівні (org, training_kind, date), не per-group.
    #[derive(FromQueryResult)]
    struct SubDate {
        as_of: String,
    }
    if let Some(sub) = SubDate::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT to_char(as_of_date, 'YYYY-MM-DD') AS as_of FROM submission WHERE id = $1",
        [submission_id.into()],
    ))
    .one(db)
    .await?
    {
        let mut vertical_keys = BTreeSet::new();
        for (row, _) in rows {
            if let (Some(org), Some(kind)) = (row.sender_org_id, row.training_kind_id) {
                vertical_keys.insert((org, kind));
            }
        }
        for (org_id, kind_id) in vertical_keys {
            refresh_vertical(db, org_id, kind_id, &sub.as_of).await?;
        }
    }

    Ok(group_ids)
}

fn non_empty(s: &str) -> Option<&str> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

async fn insert_composition(
    db: &impl ConnectionTrait,
    group_id: i32,
    composition: &CompositionRow,
) -> Result<(), DbErr> {
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO group_composition (group_id, subunit_org_id, subunit_label, count) \
         VALUES ($1, $2, $3, $4)",
        [
            group_id.into(),
            composition.subunit_org_id.into(),
            non_empty(&composition.subunit_label).into(),
            (composition.count as i32).into(),
        ],
    ))
    .await?;
    Ok(())
}

async fn insert_event(
    db: &impl ConnectionTrait,
    group_id: i32,
    event_type: &str,
    count: i32,
    occurred_on: &str,
    submission_id: i32,
) -> Result<(), DbErr> {
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO group_event (group_id, event_type, count, occurred_on, submission_id) \
         VALUES ($1, $2, $3, $4::date, $5)",
        [group_id.into(), event_type.into(), count.into(), occurred_on.into(), submission_id.into()],
    ))
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Data workspace: extended CRUD for the data management UI
// ---------------------------------------------------------------------------

#[derive(FromQueryResult, serde::Serialize)]
pub struct DataGroupRow {
    pub id: i32,
    pub sender_org_id: i32,
    pub org_label: String,
    #[serde(skip_serializing)]
    pub org_masked_label: String,
    pub training_kind_id: i32,
    pub training_kind: String,
    pub bzvp_program_id: Option<i32>,
    pub vos_id: Option<i32>,
    pub vos_label: String,
    pub position_id: Option<i32>,
    pub course_id: Option<i32>,
    pub site_id: Option<i32>,
    pub venue_type: Option<String>,
    pub training_venue_id: Option<i32>,
    pub city_id: Option<i32>,
    pub site_label: String,
    pub city_label: String,
    pub organizer_org_id: Option<i32>,
    pub organizer_label: String,
    #[serde(skip_serializing)]
    pub organizer_masked_label: String,
    pub planned_start: String,
    pub planned_end: String,
    pub equipment_text: String,
    pub basis_doc_number: String,
    pub basis_doc_date: String,
    pub note: String,
    pub planned_count: i32,
    pub arrived_count: i32,
    pub in_training_count: i32,
    pub completed_count: i32,
    pub attrition_count: i32,
    pub discrepancy_count: i32,
}

pub async fn list_groups_extended(
    db: &DatabaseConnection,
    org_ids: Option<&[i32]>,
) -> Result<Vec<DataGroupRow>, DbErr> {
    let base = "\
        SELECT tg.id, \
        tg.sender_org_id, \
        COALESCE(o.short_name, 'org#' || tg.sender_org_id::text) AS org_label, \
        COALESCE(o.masked_label, o.short_name, 'org#' || tg.sender_org_id::text) AS org_masked_label, \
        tg.training_kind_id, \
        COALESCE(tk.name, '') AS training_kind, \
        tg.bzvp_program_id, \
        tg.vos_id, \
        COALESCE(v.code || ' — ' || v.title, p.name, c.name, '') AS vos_label, \
        tg.position_id, \
        tg.course_id, \
        tg.site_id, \
        tg.venue_type, \
        tg.training_venue_id, \
        tg.city_id, \
        COALESCE(tv.name, ts.locality, so.short_name, '') AS site_label, \
        COALESCE(ct.name, '') AS city_label, \
        tg.organizer_org_id, \
        COALESCE(oo.short_name, '') AS organizer_label, \
        COALESCE(oo.masked_label, oo.short_name, '') AS organizer_masked_label, \
        COALESCE(to_char(tg.planned_start, 'DD.MM.YYYY'), '') AS planned_start, \
        COALESCE(to_char(tg.planned_end, 'DD.MM.YYYY'), '') AS planned_end, \
        COALESCE(tg.equipment_text, '') AS equipment_text, \
        COALESCE(tg.basis_doc_number, '') AS basis_doc_number, \
        COALESCE(to_char(tg.basis_doc_date, 'DD.MM.YYYY'), '') AS basis_doc_date, \
        COALESCE(tg.note, '') AS note, \
        COALESCE((SELECT SUM(ge.count) FROM group_event ge WHERE ge.group_id = tg.id AND ge.event_type = 'planned'), 0)::int AS planned_count, \
        COALESCE((SELECT SUM(ge.count) FROM group_event ge WHERE ge.group_id = tg.id AND ge.event_type = 'arrived'), 0)::int AS arrived_count, \
        ( COALESCE((SELECT SUM(ge.count) FROM group_event ge WHERE ge.group_id = tg.id AND ge.event_type IN ('started','added')), 0) \
        - COALESCE((SELECT SUM(ge.count) FROM group_event ge WHERE ge.group_id = tg.id AND ge.event_type IN ('attrition','completed')), 0) \
        )::int AS in_training_count, \
        COALESCE((SELECT SUM(ge.count) FROM group_event ge WHERE ge.group_id = tg.id AND ge.event_type = 'completed'), 0)::int AS completed_count, \
        COALESCE((SELECT SUM(ge.count) FROM group_event ge WHERE ge.group_id = tg.id AND ge.event_type = 'attrition'), 0)::int AS attrition_count, \
        COALESCE((SELECT COUNT(*) FROM discrepancy d WHERE d.group_id = tg.id AND d.status IN ('open','notified','in_progress')), 0)::int AS discrepancy_count \
        FROM training_group tg \
        LEFT JOIN org o ON o.id = tg.sender_org_id \
        LEFT JOIN training_kind tk ON tk.id = tg.training_kind_id \
        LEFT JOIN vos v ON v.id = tg.vos_id \
        LEFT JOIN \"position\" p ON p.id = tg.position_id \
        LEFT JOIN course c ON c.id = tg.course_id \
        LEFT JOIN training_venue tv ON tv.id = tg.training_venue_id \
        LEFT JOIN city ct ON ct.id = tg.city_id \
        LEFT JOIN training_site ts ON ts.id = tg.site_id \
        LEFT JOIN org so ON so.id = ts.org_id \
        LEFT JOIN org oo ON oo.id = tg.organizer_org_id";

    let (sql, params): (String, Vec<sea_orm::Value>) = match org_ids {
        Some(ids) if !ids.is_empty() => {
            let placeholders: Vec<String> =
                ids.iter().enumerate().map(|(i, _)| format!("${}", i + 1)).collect();
            let sql = format!(
                "{base} WHERE tg.sender_org_id IN ({}) \
                 ORDER BY tg.planned_start DESC NULLS LAST, tg.id DESC LIMIT 1000",
                placeholders.join(", ")
            );
            let params: Vec<sea_orm::Value> = ids.iter().map(|&id| id.into()).collect();
            (sql, params)
        }
        _ => {
            let sql = format!(
                "{base} ORDER BY tg.planned_start DESC NULLS LAST, tg.id DESC LIMIT 1000"
            );
            (sql, vec![])
        }
    };

    DataGroupRow::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        &sql,
        params,
    ))
    .all(db)
    .await
}

pub async fn update_group_field(
    db: &DatabaseConnection,
    group_id: i32,
    field: &str,
    value: &str,
) -> Result<bool, DbErr> {
    let allowed = [
        "sender_org_id", "training_kind_id", "vos_id", "position_id", "course_id",
        "site_id", "organizer_org_id", "bzvp_program_id",
        "planned_start", "planned_end", "equipment_text", "basis_doc_number",
        "basis_doc_date", "note",
        "venue_type", "training_venue_id", "city_id",
    ];
    if !allowed.contains(&field) {
        return Ok(false);
    }

    let sql = if field.ends_with("_id") {
        if value.is_empty() || value == "null" {
            format!("UPDATE training_group SET {field} = NULL WHERE id = $1")
        } else {
            format!("UPDATE training_group SET {field} = $2::int WHERE id = $1")
        }
    } else if field.contains("date") || field.contains("start") || field.contains("end") {
        format!("UPDATE training_group SET {field} = $2::date WHERE id = $1")
    } else {
        format!("UPDATE training_group SET {field} = $2 WHERE id = $1")
    };

    let params: Vec<sea_orm::Value> = if (field.ends_with("_id")) && (value.is_empty() || value == "null") {
        vec![group_id.into()]
    } else {
        vec![group_id.into(), value.into()]
    };

    let result = db
        .execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            &sql,
            params,
        ))
        .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn delete_group(db: &DatabaseConnection, group_id: i32) -> Result<bool, DbErr> {
    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "DELETE FROM group_event WHERE group_id = $1",
        [group_id.into()],
    ))
    .await?;
    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "DELETE FROM group_composition WHERE group_id = $1",
        [group_id.into()],
    ))
    .await?;
    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "DELETE FROM reported_group WHERE matched_group_id = $1",
        [group_id.into()],
    ))
    .await?;
    let result = db
        .execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "DELETE FROM training_group WHERE id = $1",
            [group_id.into()],
        ))
        .await?;
    Ok(result.rows_affected() > 0)
}

#[derive(FromQueryResult, serde::Serialize)]
pub struct GroupEventRow {
    pub id: i32,
    pub group_id: i32,
    pub event_type: String,
    pub count: i32,
    pub occurred_on: String,
    pub recorded_at: String,
    pub reason_label: Option<String>,
    pub note: Option<String>,
    pub source_label: Option<String>,
    pub created_by_label: Option<String>,
    pub created_by_id: Option<i32>,
    pub created_by_org: Option<String>,
    pub created_by_phone: Option<String>,
    pub created_by_rank: Option<String>,
    pub created_by_delta: Option<String>,
    pub created_by_active: Option<bool>,
}

pub async fn list_group_events(
    db: &DatabaseConnection,
    group_id: i32,
) -> Result<Vec<GroupEventRow>, DbErr> {
    GroupEventRow::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT ge.id, ge.group_id, ge.event_type, ge.count, \
         to_char(ge.occurred_on, 'DD.MM.YYYY') AS occurred_on, \
         to_char(ge.recorded_at, 'DD.MM.YYYY HH24:MI') AS recorded_at, \
         ar.name AS reason_label, ge.note, \
         CASE WHEN s.id IS NOT NULL THEN \
             COALESCE(src_org.short_name, 'org#' || s.reporting_org_id::text) || ' · ' || \
             CASE s.source_type \
                 WHEN 'form' THEN 'Форма' \
                 WHEN 'table' THEN 'Таблиця' \
                 WHEN 'official_letter' THEN 'Офіц. лист' \
                 ELSE s.source_type \
             END \
         WHEN al.actor IS NOT NULL AND al.actor <> '' THEN \
             COALESCE(actor_org.short_name, 'org#' || split_part(al.actor, ':', 1)) \
         END AS source_label, \
         COALESCE(cb.callsign, cb.display_name, cb.login) AS created_by_label, \
         cb.id AS created_by_id, \
         cb_org.short_name AS created_by_org, \
         cb.phone AS created_by_phone, \
         cb.rank AS created_by_rank, \
         cb.delta_nick AS created_by_delta, \
         cb.is_active AS created_by_active \
         FROM group_event ge \
         LEFT JOIN attrition_reason ar ON ar.id = ge.reason_id \
         LEFT JOIN submission s ON s.id = ge.submission_id \
         LEFT JOIN org src_org ON src_org.id = s.reporting_org_id \
         LEFT JOIN LATERAL ( \
             SELECT actor FROM audit_log \
             WHERE table_name = 'group_event' AND row_id = ge.id AND action = 'insert' \
             LIMIT 1 \
         ) al ON TRUE \
         LEFT JOIN org actor_org ON al.actor IS NOT NULL AND al.actor <> '' \
             AND actor_org.id = split_part(al.actor, ':', 1)::int \
         LEFT JOIN user_account cb ON cb.id = ge.created_by \
         LEFT JOIN LATERAL ( \
             SELECT o.short_name FROM user_role ur \
             JOIN org o ON o.id = ur.org_id \
             WHERE ur.user_id = cb.id \
             ORDER BY ur.id LIMIT 1 \
         ) cb_org ON cb.id IS NOT NULL \
         WHERE ge.group_id = $1 \
         ORDER BY ge.occurred_on, ge.id",
        [group_id.into()],
    ))
    .all(db)
    .await
}

pub async fn group_org_id(db: &DatabaseConnection, group_id: i32) -> Result<Option<i32>, DbErr> {
    #[derive(FromQueryResult)]
    struct Row {
        sender_org_id: i32,
    }
    let row = Row::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT sender_org_id FROM training_group WHERE id = $1",
        [group_id.into()],
    ))
    .one(db)
    .await?;
    Ok(row.map(|r| r.sender_org_id))
}

pub async fn add_group_event(
    db: &(impl ConnectionTrait + Send),
    group_id: i32,
    event_type: &str,
    count: i32,
    occurred_on: &str,
    reason_id: Option<i32>,
    note: Option<&str>,
    created_by: Option<i32>,
) -> Result<i32, DbErr> {
    if matches!(event_type, "attrition" | "completed") {
        #[derive(FromQueryResult)]
        struct Balance {
            total_in: i64,
            total_out: i64,
        }
        let bal = Balance::find_by_statement(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT \
                COALESCE(SUM(CASE WHEN event_type IN ('arrived','added') THEN count ELSE 0 END), 0) AS total_in, \
                COALESCE(SUM(CASE WHEN event_type IN ('attrition','completed') THEN count ELSE 0 END), 0) AS total_out \
             FROM group_event WHERE group_id = $1",
            [group_id.into()],
        ))
        .one(db)
        .await?
        .unwrap_or(Balance { total_in: 0, total_out: 0 });

        crate::domain::validation::validate_event_balance(
            bal.total_in, bal.total_out, count as i64, event_type,
        )
        .map_err(|e| match e {
            crate::domain::validation::CountError::BalanceExceeded { available, requested, event_type: label } => {
                DbErr::Custom(format!("Неможливо {label} {requested}: у групі лише {available} осіб"))
            }
            _ => DbErr::Custom("Помилка валідації балансу".into()),
        })?;
    }

    #[derive(FromQueryResult)]
    struct NewId {
        id: i32,
    }
    let row = NewId::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "INSERT INTO group_event (group_id, event_type, count, occurred_on, reason_id, note, created_by) \
         VALUES ($1, $2, $3, $4::date, $5, $6, $7) RETURNING id",
        [
            group_id.into(),
            event_type.into(),
            count.into(),
            occurred_on.into(),
            reason_id.into(),
            note.into(),
            created_by.into(),
        ],
    ))
    .one(db)
    .await?
    .ok_or_else(|| DbErr::Custom("INSERT group_event did not return id".into()))?;
    Ok(row.id)
}

pub async fn delete_group_event(db: &(impl ConnectionTrait + Send), event_id: i32) -> Result<bool, DbErr> {
    let result = db
        .execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "DELETE FROM group_event WHERE id = $1",
            [event_id.into()],
        ))
        .await?;
    Ok(result.rows_affected() > 0)
}

#[derive(FromQueryResult, serde::Serialize)]
pub struct TrainingKindOption {
    pub id: i32,
    pub code: String,
    pub name: String,
}

pub async fn list_training_kinds(db: &DatabaseConnection) -> Result<Vec<TrainingKindOption>, DbErr> {
    TrainingKindOption::find_by_statement(Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT id, code, name FROM training_kind ORDER BY id",
    ))
    .all(db)
    .await
}

#[derive(FromQueryResult, serde::Serialize)]
pub struct AttritionReasonOption {
    pub id: i32,
    pub name: String,
}

pub async fn list_attrition_reasons(db: &DatabaseConnection) -> Result<Vec<AttritionReasonOption>, DbErr> {
    AttritionReasonOption::find_by_statement(Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT id, name FROM attrition_reason WHERE deleted_at IS NULL ORDER BY id",
    ))
    .all(db)
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn create_group_with_events(
    db: &DatabaseConnection,
    sender_org_id: i32,
    training_kind_id: i32,
    site_id: Option<i32>,
    vos_id: Option<i32>,
    position_id: Option<i32>,
    course_id: Option<i32>,
    bzvp_program_id: Option<i32>,
    organizer_org_id: Option<i32>,
    planned_start: &str,
    planned_end: &str,
    equipment_text: Option<&str>,
    basis_doc_number: Option<&str>,
    basis_doc_date: Option<&str>,
    note: Option<&str>,
    planned_count: i32,
    arrived_count: i32,
    in_training_count: i32,
    venue_type: Option<&str>,
    training_venue_id: Option<i32>,
    city_id: Option<i32>,
    submission_id: Option<i32>,
    force: bool,
) -> Result<CreateGroupResult, DbErr> {
    #[derive(FromQueryResult)]
    struct NewId {
        id: i32,
    }

    if !force {
        let similar = SimilarGroup::find_by_statement(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT tg.id, \
                    COALESCE(v.code || ' — ' || v.name, '') AS vos_label, \
                    COALESCE(tv.name, c.name, '') AS site_label, \
                    tg.planned_count \
             FROM training_group tg \
             LEFT JOIN vos v ON v.id = tg.vos_id \
             LEFT JOIN training_venue tv ON tv.id = tg.training_venue_id \
             LEFT JOIN city c ON c.id = tg.city_id \
             WHERE tg.sender_org_id = $1 AND tg.training_kind_id = $2 \
               AND tg.planned_start = $3::date AND tg.planned_end = $4::date \
             LIMIT 5",
            [
                sender_org_id.into(),
                training_kind_id.into(),
                planned_start.into(),
                planned_end.into(),
            ],
        ))
        .all(db)
        .await?;

        if !similar.is_empty() {
            let items: Vec<DuplicateInfo> = similar
                .into_iter()
                .map(|s| DuplicateInfo {
                    id: s.id,
                    vos_label: s.vos_label,
                    site_label: s.site_label,
                    planned_count: s.planned_count,
                })
                .collect();
            return Ok(CreateGroupResult::Duplicates(items));
        }
    }

    let basis_date_val: sea_orm::Value = basis_doc_date
        .filter(|s| !s.is_empty())
        .map(sea_orm::Value::from)
        .unwrap_or(sea_orm::Value::from(None::<String>));

    let row = NewId::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "INSERT INTO training_group \
            (sender_org_id, training_kind_id, bzvp_program_id, vos_id, position_id, \
             course_id, equipment_text, site_id, organizer_org_id, planned_start, \
             planned_end, basis_doc_number, basis_doc_date, note, \
             venue_type, training_venue_id, city_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::date, $11::date, $12, $13::date, $14, \
                 $15, $16, $17) \
         RETURNING id",
        [
            sender_org_id.into(),
            training_kind_id.into(),
            bzvp_program_id.into(),
            vos_id.into(),
            position_id.into(),
            course_id.into(),
            equipment_text.into(),
            site_id.into(),
            organizer_org_id.into(),
            planned_start.into(),
            planned_end.into(),
            basis_doc_number.into(),
            basis_date_val,
            note.into(),
            venue_type.into(),
            training_venue_id.into(),
            city_id.into(),
        ],
    ))
    .one(db)
    .await?
    .ok_or_else(|| DbErr::Custom("INSERT training_group did not return id".into()))?;

    let gid = row.id;
    if planned_count > 0 {
        db.execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "INSERT INTO group_event (group_id, event_type, count, occurred_on, submission_id) \
             VALUES ($1, 'planned', $2, $3::date, $4)",
            [gid.into(), planned_count.into(), planned_start.into(), submission_id.into()],
        ))
        .await?;
    }
    if arrived_count > 0 {
        db.execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "INSERT INTO group_event (group_id, event_type, count, occurred_on, submission_id) \
             VALUES ($1, 'arrived', $2, $3::date, $4)",
            [gid.into(), arrived_count.into(), planned_start.into(), submission_id.into()],
        ))
        .await?;
    }
    if in_training_count > 0 {
        db.execute(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "INSERT INTO group_event (group_id, event_type, count, occurred_on, submission_id) \
             VALUES ($1, 'started', $2, $3::date, $4)",
            [gid.into(), in_training_count.into(), planned_start.into(), submission_id.into()],
        ))
        .await?;
    }
    Ok(CreateGroupResult::Created(gid))
}

#[derive(FromQueryResult)]
struct SimilarGroup {
    id: i32,
    vos_label: String,
    site_label: String,
    planned_count: i32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DuplicateInfo {
    pub id: i32,
    pub vos_label: String,
    pub site_label: String,
    pub planned_count: i32,
}

pub enum CreateGroupResult {
    Created(i32),
    Duplicates(Vec<DuplicateInfo>),
}
