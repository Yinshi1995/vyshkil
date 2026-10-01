//! Горизонтальна звірка (04 §3-4, Етап 8 зріз 1) — SQL навколо `discrepancy`/`reported_group`.
//! Сама логіка "чи є незгода" — `domain::reconciliation::detect_horizontal` (чиста); тут лише
//! читання подань для однієї канонічної групи й запис/автозакриття `discrepancy`.

use crate::backend::repo::outbox;
use crate::domain::reconciliation::{detect_horizontal, ReportedValues};
use crate::types::reconciliation::DiscrepancyRow;
use contracts::{subjects, DiscrepancyMetric, DiscrepancyOpened, DiscrepancyResolved, Envelope};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement};
use uuid::Uuid;

fn metric_label(metric: &str) -> &'static str {
    match metric {
        "planned_count" => "План",
        "arrived_count" => "Прибуло",
        "in_training_count" => "Навчаються",
        "planned_start" => "Термін з",
        "planned_end" => "Термін по",
        "site_id" => "Місце",
        "total" => "Всього (навчаються)",
        "finishing" => "Закінчують",
        "started" => "Почали",
        _ => "?",
    }
}

/// `discrepancy.metric` (текстовий стовпець, `text`+`CHECK`, не Postgres ENUM — migration/CLAUDE.md)
/// → `contracts::DiscrepancyMetric` (`enum`, приватність 09 §3.5). Панікує на невідомому рядку —
/// той самий стовпець, що `metric_label` уже вичерпно матчить вище; розсинхрон між ними був би
/// багом коду, не даними з БД.
fn metric_to_contract(metric: &str) -> DiscrepancyMetric {
    match metric {
        "planned_count" => DiscrepancyMetric::PlannedCount,
        "arrived_count" => DiscrepancyMetric::ArrivedCount,
        "in_training_count" => DiscrepancyMetric::InTrainingCount,
        "planned_start" => DiscrepancyMetric::PlannedStart,
        "planned_end" => DiscrepancyMetric::PlannedEnd,
        "site_id" => DiscrepancyMetric::SiteId,
        other => unreachable!("невідома discrepancy.metric {other:?} — розсинхрон із metric_label"),
    }
}

/// Загортає `payload` в `Envelope` і пише в `outbox` (09-messaging.md §3.1) — той самий `db`, що
/// доменний запис вище, тому та сама транзакція: відкат домену відкочує й подію, коміту без
/// публікації не станеться (relay, Фаза 1, публікує в NATS окремо, поза цією транзакцією).
/// Продюсер тут не веде наскрізного `correlation_id` через увесь ланцюг виклику (спостережуваність
/// із §5 "Експлуатація" — поза обсягом Фази 1) — кожна подія отримує власний.
async fn publish_event<T: serde::Serialize>(
    db: &impl ConnectionTrait,
    subject: &str,
    type_: &str,
    payload: T,
) -> Result<(), DbErr> {
    let envelope =
        Envelope::new(type_, 1, "app::backend::repo::reconciliation", Uuid::now_v7(), None, payload);
    let payload_json = serde_json::to_string(&envelope)
        .map_err(|e| DbErr::Custom(format!("серіалізація події {subject}: {e}")))?;
    outbox::insert(db, subject, &payload_json, "{}").await?;
    Ok(())
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
            publish_event(
                db,
                subjects::DISCREPANCY_RESOLVED_V1,
                "vyshkil.discrepancy.resolved.v1",
                DiscrepancyResolved {
                    discrepancy_id: row.id,
                    org_id: ctx.sender_org_id,
                    group_id: Some(group_id),
                    metric: metric_to_contract(&row.metric),
                },
            )
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
            #[derive(FromQueryResult)]
            struct NewId {
                id: i32,
            }
            let new_row = NewId::find_by_statement(Statement::from_sql_and_values(
                db.get_database_backend(),
                "INSERT INTO discrepancy (kind, org_id, group_id, as_of, metric, values, status) \
                 VALUES ('horizontal', $1, $2, $3::date, $4, $5::jsonb, 'open') RETURNING id",
                [
                    ctx.sender_org_id.into(),
                    group_id.into(),
                    ctx.planned_start.clone().into(),
                    d.metric.into(),
                    values_json.into(),
                ],
            ))
            .one(db)
            .await?
            .ok_or_else(|| DbErr::Custom("INSERT discrepancy не повернув id".into()))?;
            publish_event(
                db,
                subjects::DISCREPANCY_OPENED_V1,
                "vyshkil.discrepancy.opened.v1",
                DiscrepancyOpened {
                    discrepancy_id: new_row.id,
                    org_id: ctx.sender_org_id,
                    group_id: Some(group_id),
                    metric: metric_to_contract(d.metric),
                },
            )
            .await?;
        }
    }

    Ok(())
}

/// Часова звірка (04 §3, decision §4): нове подання суперечить попередньому від того самого
/// джерела без події-пояснення. Викликається для кожної зачепленої `group_id` після фіксації
/// подання (той самий цикл, що й `refresh_horizontal`).
pub async fn refresh_temporal(db: &impl ConnectionTrait, group_id: i32) -> Result<(), DbErr> {
    use crate::domain::reconciliation::{detect_temporal, TemporalSnapshot};

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

    #[derive(FromQueryResult)]
    struct SubmissionRow {
        submission_id: i32,
        as_of: String,
        total: i64,
    }
    let snapshots = SubmissionRow::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT DISTINCT ON (s.reporting_org_id) \
                rg.submission_id, \
                to_char(s.as_of_date, 'YYYY-MM-DD') AS as_of, \
                rg.in_training_count::bigint AS total \
         FROM reported_group rg \
         JOIN submission s ON s.id = rg.submission_id \
         WHERE rg.matched_group_id = $1 AND s.status = 'committed' \
         ORDER BY s.reporting_org_id, rg.created_at DESC, rg.id DESC",
        [group_id.into()],
    ))
    .all(db)
    .await?;

    if snapshots.len() < 2 {
        auto_close_temporal(db, group_id).await?;
        return Ok(());
    }

    let mut sorted: Vec<_> = snapshots
        .into_iter()
        .map(|s| TemporalSnapshot {
            submission_id: s.submission_id,
            as_of: s.as_of,
            total: s.total,
        })
        .collect();
    sorted.sort_by(|a, b| a.as_of.cmp(&b.as_of));

    let mut found_temporal = false;
    for pair in sorted.windows(2) {
        let prev = &pair[0];
        let curr = &pair[1];

        #[derive(FromQueryResult)]
        struct EventSum {
            total_decrease: i64,
        }
        let explained = EventSum::find_by_statement(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT COALESCE(SUM(ge.count), 0)::bigint AS total_decrease \
             FROM group_event ge \
             WHERE ge.group_id = $1 \
               AND ge.event_type IN ('attrition', 'completed') \
               AND ge.occurred_on > $2::date AND ge.occurred_on <= $3::date",
            [group_id.into(), prev.as_of.clone().into(), curr.as_of.clone().into()],
        ))
        .one(db)
        .await?
        .map(|e| e.total_decrease)
        .unwrap_or(0);

        if let Some(_disc) = detect_temporal(prev, curr, explained) {
            found_temporal = true;
            let values_json = serde_json::to_string(&vec![
                (prev.submission_id, prev.total.to_string()),
                (curr.submission_id, curr.total.to_string()),
            ])
            .map_err(|e| DbErr::Custom(format!("серіалізація temporal values: {e}")))?;

            #[derive(FromQueryResult)]
            struct Existing {
                id: i32,
            }
            let existing = Existing::find_by_statement(Statement::from_sql_and_values(
                db.get_database_backend(),
                "SELECT id FROM discrepancy \
                 WHERE group_id = $1 AND kind = 'temporal' AND status = 'open'",
                [group_id.into()],
            ))
            .one(db)
            .await?;

            if let Some(row) = existing {
                db.execute(Statement::from_sql_and_values(
                    db.get_database_backend(),
                    "UPDATE discrepancy SET values = $1::jsonb, updated_at = now() WHERE id = $2",
                    [values_json.into(), row.id.into()],
                ))
                .await?;
            } else {
                #[derive(FromQueryResult)]
                struct NewId {
                    id: i32,
                }
                let new_row = NewId::find_by_statement(Statement::from_sql_and_values(
                    db.get_database_backend(),
                    "INSERT INTO discrepancy (kind, org_id, group_id, as_of, metric, values, status) \
                     VALUES ('temporal', $1, $2, $3::date, 'total', $4::jsonb, 'open') RETURNING id",
                    [
                        ctx.sender_org_id.into(),
                        group_id.into(),
                        ctx.planned_start.clone().into(),
                        values_json.into(),
                    ],
                ))
                .one(db)
                .await?
                .ok_or_else(|| DbErr::Custom("INSERT temporal discrepancy не повернув id".into()))?;
                publish_event(
                    db,
                    subjects::DISCREPANCY_OPENED_V1,
                    "vyshkil.discrepancy.opened.v1",
                    DiscrepancyOpened {
                        discrepancy_id: new_row.id,
                        org_id: ctx.sender_org_id,
                        group_id: Some(group_id),
                        metric: DiscrepancyMetric::InTrainingCount,
                    },
                )
                .await?;
            }
        }
    }

    if !found_temporal {
        auto_close_temporal(db, group_id).await?;
    }

    Ok(())
}

async fn auto_close_temporal(db: &impl ConnectionTrait, group_id: i32) -> Result<(), DbErr> {
    #[derive(FromQueryResult)]
    struct OpenRow {
        id: i32,
    }
    let open_now = OpenRow::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id FROM discrepancy \
         WHERE group_id = $1 AND kind = 'temporal' AND status = 'open'",
        [group_id.into()],
    ))
    .all(db)
    .await?;

    for row in open_now {
        db.execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            "UPDATE discrepancy SET status = 'resolved', \
                resolution_note = 'автоматично: часову розбіжність усунуто', updated_at = now() \
             WHERE id = $1",
            [row.id.into()],
        ))
        .await?;
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
