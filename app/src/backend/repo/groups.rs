//! SQL для агрегату "групи на навчанні" (01 §3): читання подій (воронка), пошук ВОС/посади/курсу
//! й майданчиків для сітки (02 §3), фіксація рядків сітки в `training_group`/`group_event`
//! (02 §5 — уся сітка зберігається одним усе-або-нічого записом).
//! Сама арифметика воронки — в `domain::counting` (чиста, без БД); тут лише SQL і перетворення типів.

use crate::domain::counting::{EventType, GroupEventRecord};
use crate::domain::dates::{parse_date, parse_end_date, parse_maybe_range, validate_period, DateError};
use crate::domain::normalize::normalize;
use crate::domain::validation::{validate_funnel_order, CountError};
use crate::types::submission::{
    CompositionRow, GroupFormRow, TrainingSiteOption, VosPositionCourseHint, VosPositionCourseKind,
};
use chrono::NaiveDate;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement};

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
        return Ok(Vec::new());
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
        combined AS (SELECT * FROM direct UNION ALL SELECT * FROM via_equipment),
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
        range_end.ok_or_else(|| ("По".to_string(), "не вказано термін «по»".to_string()))?
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

/// Фіксація сітки (02 §5, `Ctrl+Enter`) — усе-або-нічого: викликач обгортає одну транзакцію,
/// валідує (`validate_row`) заздалегідь усі рядки, і лише тоді викликає цю функцію.
/// Події воронки для щойно внесеної групи спрощено записуються на дату початку (`planned_start`):
/// форма не збирає окремих дат "викликали"/"прибули"/"розпочали" (02 §1 колонка 7: "для нової
/// групи часто рівні").
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

    for (row, dates) in rows {
        let basis_doc_date =
            dates.basis_doc_date.map(|d| d.format("%Y-%m-%d").to_string());

        let group = NewId::find_by_statement(Statement::from_sql_and_values(
            db.get_database_backend(),
            "INSERT INTO training_group \
                (sender_org_id, training_kind_id, bzvp_program_id, vos_id, position_id, course_id, \
                 equipment_text, site_id, organizer_org_id, planned_start, planned_end, \
                 basis_doc_number, basis_doc_date, note) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::date, $11::date, $12, $13::date, $14) \
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

        for composition in &row.composition {
            insert_composition(db, group.id, composition).await?;
        }

        let start_str = dates.start.format("%Y-%m-%d").to_string();
        insert_event(db, group.id, "planned", row.planned_count as i32, &start_str, submission_id)
            .await?;
        if row.arrived_count > 0 {
            insert_event(db, group.id, "arrived", row.arrived_count as i32, &start_str, submission_id)
                .await?;
        }
        if row.in_training_count > 0 {
            insert_event(
                db,
                group.id,
                "started",
                row.in_training_count as i32,
                &start_str,
                submission_id,
            )
            .await?;
        }

        group_ids.push(group.id);
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
