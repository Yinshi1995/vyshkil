//! Горизонтальна звірка (04 §3-4, Етап 8 зріз 1) — SQL навколо `discrepancy`/`reported_group`.
//! Сама логіка "чи є незгода" — `domain::reconciliation::detect_horizontal` (чиста); тут лише
//! читання подань для однієї канонічної групи й запис/автозакриття `discrepancy`.

use crate::domain::reconciliation::{detect_horizontal, ReportedValues};
use crate::types::reconciliation::DiscrepancyRow;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement};

fn metric_label(metric: &str) -> &'static str {
    match metric {
        "planned_count" => "План",
        "arrived_count" => "Прибуло",
        "in_training_count" => "Навчаються",
        "planned_start" => "Термін з",
        "planned_end" => "Термін по",
        "site_id" => "Місце",
        _ => "?",
    }
}

/// Перечитує ОСТАННЄ подання кожного джерела (`submission.reporting_org_id`), зіставлене з
/// `group_id`, рахує горизонтальну звірку
/// (`domain::reconciliation::detect_horizontal`) і синхронізує `discrepancy`: відкриває/оновлює
/// незгодні метрики, автозакриває ті, що вже узгодились (04 §4: "закривається автоматично...
/// лишається в історії" — `UPDATE status`, не `DELETE`). Викликається після кожної фіксації
/// сітки для кожної зачепленої групи (той самий принцип, що "звірка — не разова кнопка, а
/// перерахунок після кожної фіксації", 04 §3).
pub async fn refresh_horizontal(db: &impl ConnectionTrait, group_id: i32) -> Result<(), DbErr> {
    #[derive(FromQueryResult)]
    struct GroupCtx {
        sender_org_id: i32,
        planned_start: String,
    }
    let Some(ctx) = GroupCtx::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT sender_org_id, to_char(planned_start, 'YYYY-MM-DD') AS planned_start \
         FROM training_group WHERE id = $1",
        [group_id.into()],
    ))
    .one(db)
    .await?
    else {
        return Ok(());
    };

    // "Хто зараз що каже" — ОСТАННЄ подання КОЖНОЇ окремої `reporting_org` (`submission.
    // reporting_org_id`, не `reported_group.sender_org_id` — та сама частина завжди дорівнює
    // ключу зіставлення, бо це ЧАСТИНА ключа; хто РЕАЛЬНО подав — корпус за підлеглого чи сама
    // частина — це reporting_org). Без цього звірка порівнювала б УСЮ історію подань назавжди —
    // третє подання, що узгоджує перше й друге, ніколи не закрило б розбіжність (старий,
    // виправлений запис так і лишався б у порівнянні). `DISTINCT ON` — той самий бере ОСТАННІЙ
    // запис на джерело, як "останнє слово" цього джерела зараз.
    #[derive(FromQueryResult)]
    struct ReportRow {
        submission_id: i32,
        planned_count: i32,
        arrived_count: i32,
        in_training_count: i32,
        planned_start: String,
        planned_end: String,
        site_id: i32,
    }
    let reports = ReportRow::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT DISTINCT ON (s.reporting_org_id) \
                rg.submission_id, rg.planned_count, rg.arrived_count, rg.in_training_count, \
                to_char(rg.planned_start, 'YYYY-MM-DD') AS planned_start, \
                to_char(rg.planned_end, 'YYYY-MM-DD') AS planned_end, rg.site_id \
         FROM reported_group rg \
         JOIN submission s ON s.id = rg.submission_id \
         WHERE rg.matched_group_id = $1 \
         ORDER BY s.reporting_org_id, rg.created_at DESC, rg.id DESC",
        [group_id.into()],
    ))
    .all(db)
    .await?;

    let values: Vec<ReportedValues> = reports
        .into_iter()
        .map(|r| ReportedValues {
            submission_id: r.submission_id,
            planned_count: r.planned_count as i64,
            arrived_count: r.arrived_count as i64,
            in_training_count: r.in_training_count as i64,
            planned_start: r.planned_start,
            planned_end: r.planned_end,
            site_id: r.site_id,
        })
        .collect();

    let found = detect_horizontal(&values);
    let found_metrics: Vec<&str> = found.iter().map(|d| d.metric).collect();

    #[derive(FromQueryResult)]
    struct OpenRow {
        id: i32,
        metric: String,
    }
    let open_now = OpenRow::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id, metric FROM discrepancy \
         WHERE group_id = $1 AND kind = 'horizontal' AND status = 'open'",
        [group_id.into()],
    ))
    .all(db)
    .await?;

    // Автозакрити те, що вже не незгодне (04 §4).
    for row in &open_now {
        if !found_metrics.contains(&row.metric.as_str()) {
            db.execute(Statement::from_sql_and_values(
                db.get_database_backend(),
                "UPDATE discrepancy SET status = 'resolved', \
                    resolution_note = 'автоматично: подання узгодились', updated_at = now() \
                 WHERE id = $1",
                [row.id.into()],
            ))
            .await?;
        }
    }

    for d in &found {
        // JSON — рядком + `::jsonb`-каст, не `sea_orm::Value::Json` (той самий підхід, що
        // `repo::submissions::save_draft` — не тягнути sea-orm feature "with-json" заради
        // одного стовпця).
        let values_json = serde_json::to_string(&d.values)
            .map_err(|e| DbErr::Custom(format!("серіалізація values: {e}")))?;
        if let Some(existing) = open_now.iter().find(|r| r.metric == d.metric) {
            db.execute(Statement::from_sql_and_values(
                db.get_database_backend(),
                "UPDATE discrepancy SET values = $1::jsonb, updated_at = now() WHERE id = $2",
                [values_json.into(), existing.id.into()],
            ))
            .await?;
        } else {
            db.execute(Statement::from_sql_and_values(
                db.get_database_backend(),
                "INSERT INTO discrepancy (kind, org_id, group_id, as_of, metric, values, status) \
                 VALUES ('horizontal', $1, $2, $3::date, $4, $5::jsonb, 'open')",
                [
                    ctx.sender_org_id.into(),
                    group_id.into(),
                    ctx.planned_start.clone().into(),
                    d.metric.into(),
                    values_json.into(),
                ],
            ))
            .await?;
        }
    }

    Ok(())
}

/// Усі розбіжності (без фільтра прав — викликач звужує через `policy::visible_org_ids`, той
/// самий принцип, що `pages/home/server.rs::get_subordination_tree`). `status_filter=None` —
/// усі статуси.
pub async fn list_discrepancies(
    db: &DatabaseConnection,
    status_filter: Option<&str>,
) -> Result<Vec<DiscrepancyRow>, DbErr> {
    #[derive(FromQueryResult)]
    struct Row {
        id: i32,
        kind: String,
        org_id: i32,
        org_label: String,
        group_id: Option<i32>,
        group_label: Option<String>,
        as_of: String,
        metric: String,
        values: String,
        status: String,
        created_at: String,
    }

    let sql = "SELECT \
            d.id, d.kind, d.org_id, o.short_name AS org_label, d.group_id, \
            (CASE WHEN tg.id IS NULL THEN NULL ELSE \
                tk.name || ' · ' || COALESCE(v.code, '') || ' · з ' || \
                to_char(tg.planned_start, 'DD.MM.YYYY') \
             END) AS group_label, \
            to_char(d.as_of, 'YYYY-MM-DD') AS as_of, d.metric, d.values::text AS values, d.status, \
            to_char(d.created_at, 'YYYY-MM-DD\"T\"HH24:MI:SS') AS created_at \
         FROM discrepancy d \
         JOIN org o ON o.id = d.org_id \
         LEFT JOIN training_group tg ON tg.id = d.group_id \
         LEFT JOIN training_kind tk ON tk.id = tg.training_kind_id \
         LEFT JOIN vos v ON v.id = tg.vos_id \
         WHERE ($1::text IS NULL OR d.status = $1) \
         ORDER BY d.updated_at DESC";

    let rows = Row::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        sql,
        [status_filter.into()],
    ))
    .all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| {
            let values: Vec<(i32, String)> = serde_json::from_str(&r.values).unwrap_or_default();
            DiscrepancyRow {
                id: r.id,
                kind: r.kind,
                org_id: r.org_id,
                org_label: r.org_label,
                group_id: r.group_id,
                group_label: r.group_label,
                as_of: r.as_of,
                metric_label: metric_label(&r.metric).to_string(),
                metric: r.metric,
                values,
                status: r.status,
                created_at: r.created_at,
            }
        })
        .collect())
}
