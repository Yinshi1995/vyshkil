//! Звірка подань (04 §3-4): горизонтальна (зріз 1), часова + вертикальна (зріз 2),
//! workflow розбіжностей (зріз 3).
//! Чиста логіка ("чи є незгода") — `domain::reconciliation`; тут лише SQL + запис `discrepancy`.

use crate::backend::repo::{notifications, outbox, whatsapp_routing};
use crate::domain::reconciliation::{
    detect_horizontal, detect_vertical, AggregatedCounts, ReportedValues,
};
use crate::types::reconciliation::{
    DiscrepancyComparison, DiscrepancyRow, DiscrepancyValue, ReportedGroupSnapshot, SubmissionDetail,
};
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
        "source_format" => "Формат джерела",
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

async fn notify_discrepancy_opened(
    db: &impl ConnectionTrait,
    org_id: i32,
    kind: &str,
    metric_label_str: &str,
) -> Result<(), DbErr> {
    let title = format!("Нова розбіжність: {metric_label_str}");
    let body = format!("{} розбіжність — перевірте розділ \"Розбіжності\"", kind_label_ua(kind));
    notifications::insert(db, org_id, "discrepancy", &title, Some(&body), Some("/discrepancies")).await
}

fn kind_label_ua(kind: &str) -> &'static str {
    match kind {
        "horizontal" => "Горизонтальна",
        "vertical" => "Вертикальна",
        "temporal" => "Часова",
        _ => "Нова",
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
            whatsapp_routing::dispatch_wa_notifications(
                db, ctx.sender_org_id, "discrepancy_resolved", contracts::NotifyTemplate::DiscrepancyResolved,
            ).await?;
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
            notify_discrepancy_opened(db, ctx.sender_org_id, "horizontal", metric_label(d.metric))
                .await?;
            whatsapp_routing::dispatch_wa_notifications(
                db, ctx.sender_org_id, "discrepancy_opened", contracts::NotifyTemplate::DiscrepancyDetected,
            ).await?;
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
                notify_discrepancy_opened(db, ctx.sender_org_id, "temporal", metric_label("total"))
                    .await?;
                whatsapp_routing::dispatch_wa_notifications(
                    db, ctx.sender_org_id, "discrepancy_opened", contracts::NotifyTemplate::DiscrepancyDetected,
                ).await?;
            }
        }
    }

    if !found_temporal {
        auto_close_temporal(db, group_id).await?;
    }

    Ok(())
}

/// Вертикальна звірка (04 §3, decision §3): для кожного органу, згаданого у поданні, перевіряє,
/// чи є "чуже" подання (від батьківського органу) з іншими агрегатними числами. Виклик —
/// після commit_group_rows, для кожного зачепленого `sender_org_id` + `training_kind_id`.
pub async fn refresh_vertical(
    db: &impl ConnectionTrait,
    sender_org_id: i32,
    training_kind_id: i32,
    as_of: &str,
) -> Result<(), DbErr> {
    #[derive(FromQueryResult)]
    struct Agg {
        reporting_org_id: i32,
        total: i64,
        finishing: i64,
        started: i64,
    }

    let rows = Agg::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT s.reporting_org_id, \
                COALESCE(SUM(rg.in_training_count), 0)::bigint AS total, \
                0::bigint AS finishing, \
                0::bigint AS started \
         FROM reported_group rg \
         JOIN submission s ON s.id = rg.submission_id \
         WHERE rg.sender_org_id = $1 \
           AND rg.training_kind_id = $2 \
           AND s.as_of_date = $3::date \
           AND s.status = 'committed' \
         GROUP BY s.reporting_org_id",
        [sender_org_id.into(), training_kind_id.into(), as_of.into()],
    ))
    .all(db)
    .await?;

    let own_report = rows.iter().find(|r| r.reporting_org_id == sender_org_id);
    let parent_reports: Vec<_> = rows.iter().filter(|r| r.reporting_org_id != sender_org_id).collect();

    if parent_reports.is_empty() {
        auto_close_vertical(db, sender_org_id, training_kind_id, as_of).await?;
        return Ok(());
    }

    let Some(own) = own_report else {
        return Ok(());
    };

    let own_counts = AggregatedCounts { total: own.total, finishing: own.finishing, started: own.started };

    for parent in &parent_reports {
        let parent_counts =
            AggregatedCounts { total: parent.total, finishing: parent.finishing, started: parent.started };

        let discrepancies = detect_vertical(
            &format!("подання від org #{}", parent.reporting_org_id),
            &parent_counts,
            &format!("власне подання org #{sender_org_id}"),
            &own_counts,
        );

        if discrepancies.is_empty() {
            auto_close_vertical(db, sender_org_id, training_kind_id, as_of).await?;
            continue;
        }

        for d in &discrepancies {
            let values_json = serde_json::to_string(&vec![
                (parent.reporting_org_id, d.parent.total.to_string()),
                (sender_org_id, d.children_sum.total.to_string()),
            ])
            .map_err(|e| DbErr::Custom(format!("серіалізація vertical values: {e}")))?;

            #[derive(FromQueryResult)]
            struct Existing {
                id: i32,
            }
            let existing = Existing::find_by_statement(Statement::from_sql_and_values(
                db.get_database_backend(),
                "SELECT id FROM discrepancy \
                 WHERE org_id = $1 AND kind = 'vertical' AND metric = $2 AND status = 'open' \
                   AND as_of = $3::date",
                [sender_org_id.into(), d.metric.into(), as_of.into()],
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
                     VALUES ('vertical', $1, NULL, $2::date, $3, $4::jsonb, 'open') RETURNING id",
                    [
                        sender_org_id.into(),
                        as_of.into(),
                        d.metric.into(),
                        values_json.into(),
                    ],
                ))
                .one(db)
                .await?
                .ok_or_else(|| DbErr::Custom("INSERT vertical discrepancy не повернув id".into()))?;
                publish_event(
                    db,
                    subjects::DISCREPANCY_OPENED_V1,
                    "vyshkil.discrepancy.opened.v1",
                    DiscrepancyOpened {
                        discrepancy_id: new_row.id,
                        org_id: sender_org_id,
                        group_id: None,
                        metric: match d.metric {
                            "total" => DiscrepancyMetric::InTrainingCount,
                            "finishing" => DiscrepancyMetric::PlannedEnd,
                            _ => DiscrepancyMetric::InTrainingCount,
                        },
                    },
                )
                .await?;
                notify_discrepancy_opened(db, sender_org_id, "vertical", metric_label(d.metric))
                    .await?;
                whatsapp_routing::dispatch_wa_notifications(
                    db, sender_org_id, "discrepancy_opened", contracts::NotifyTemplate::DiscrepancyDetected,
                ).await?;
            }
        }
    }

    Ok(())
}

async fn auto_close_vertical(
    db: &impl ConnectionTrait,
    org_id: i32,
    _training_kind_id: i32,
    as_of: &str,
) -> Result<(), DbErr> {
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "UPDATE discrepancy SET status = 'resolved', \
            resolution_note = 'автоматично: вертикальну розбіжність усунуто', updated_at = now() \
         WHERE org_id = $1 AND kind = 'vertical' AND status = 'open' AND as_of = $2::date",
        [org_id.into(), as_of.into()],
    ))
    .await?;
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

/// Зміна статусу розбіжності (04 §4, зріз 3 — workflow). Повертає org_id розбіжності
/// (для перевірки прав викликачем) або None, якщо id не знайдено.
pub async fn update_discrepancy_status(
    db: &impl ConnectionTrait,
    id: i32,
    new_status: &str,
    resolution_note: Option<&str>,
) -> Result<Option<i32>, DbErr> {
    #[derive(FromQueryResult)]
    struct OrgRow {
        org_id: i32,
    }
    let Some(row) = OrgRow::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT org_id FROM discrepancy WHERE id = $1",
        [id.into()],
    ))
    .one(db)
    .await?
    else {
        return Ok(None);
    };

    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "UPDATE discrepancy SET status = $1, resolution_note = COALESCE($2, resolution_note), \
            updated_at = now() \
         WHERE id = $3",
        [new_status.into(), resolution_note.into(), id.into()],
    ))
    .await?;

    if new_status == "resolved" {
        whatsapp_routing::dispatch_wa_notifications(
            db, row.org_id, "discrepancy_resolved", contracts::NotifyTemplate::DiscrepancyResolved,
        ).await?;
    }

    Ok(Some(row.org_id))
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

    // Collect all submission IDs to enrich in one query
    let parsed: Vec<(Row, Vec<(i32, String)>)> = rows
        .into_iter()
        .map(|r| {
            let vals: Vec<(i32, String)> = serde_json::from_str(&r.values).unwrap_or_default();
            (r, vals)
        })
        .collect();

    let all_sub_ids: Vec<i32> = parsed
        .iter()
        .flat_map(|(_, vals)| vals.iter().map(|(id, _)| *id))
        .collect();

    // Fetch submission labels: "65 омбр · Форма"
    #[derive(FromQueryResult)]
    struct SubLabel {
        id: i32,
        label: String,
    }

    let sub_labels: std::collections::HashMap<i32, String> = if all_sub_ids.is_empty() {
        Default::default()
    } else {
        let ids_csv = all_sub_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let sub_sql = format!(
            "SELECT s.id, \
                 COALESCE(o.short_name, 'org#' || s.reporting_org_id::text) || ' · ' || \
                 CASE s.source_type \
                     WHEN 'form' THEN 'Форма' \
                     WHEN 'table' THEN 'Таблиця' \
                     WHEN 'official_letter' THEN 'Офіц. лист' \
                     ELSE s.source_type \
                 END AS label \
             FROM submission s \
             JOIN org o ON o.id = s.reporting_org_id \
             WHERE s.id IN ({ids_csv})"
        );
        SubLabel::find_by_statement(Statement::from_string(
            db.get_database_backend(),
            sub_sql,
        ))
        .all(db)
        .await?
        .into_iter()
        .map(|sl| (sl.id, sl.label))
        .collect()
    };

    Ok(parsed
        .into_iter()
        .map(|(r, vals)| {
            let values = vals
                .into_iter()
                .map(|(sub_id, val)| DiscrepancyValue {
                    source_label: sub_labels
                        .get(&sub_id)
                        .cloned()
                        .unwrap_or_else(|| format!("#{sub_id}")),
                    submission_id: sub_id,
                    value: val,
                })
                .collect();
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

/// Порівняння подань для однієї розбіжності: повертає `reported_group` рядки з лейблами
/// для кожного `submission_id`, що фігурує у `values` цієї розбіжності.
/// Повертає `None`, якщо `disc_id` не знайдено.
pub async fn compare_discrepancy(
    db: &DatabaseConnection,
    disc_id: i32,
) -> Result<Option<DiscrepancyComparison>, DbErr> {
    #[derive(FromQueryResult)]
    struct DiscMeta {
        metric: String,
        group_id: Option<i32>,
        values: String,
    }
    let Some(meta) = DiscMeta::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT metric, group_id, values::text AS values FROM discrepancy WHERE id = $1",
        [disc_id.into()],
    ))
    .one(db)
    .await?
    else {
        return Ok(None);
    };

    let sub_ids: Vec<i32> = {
        let pairs: Vec<(i32, String)> = serde_json::from_str(&meta.values).unwrap_or_default();
        pairs.into_iter().map(|(id, _)| id).collect()
    };

    if sub_ids.is_empty() {
        return Ok(Some(DiscrepancyComparison {
            metric: meta.metric.clone(),
            metric_label: metric_label(&meta.metric).to_string(),
            rows: vec![],
        }));
    }

    let ids_csv = sub_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");

    #[derive(FromQueryResult)]
    struct RgRow {
        submission_id: i32,
        source_label: String,
        training_kind: String,
        vos_label: Option<String>,
        position_label: Option<String>,
        course_label: Option<String>,
        site_label: String,
        organizer_label: Option<String>,
        planned_start: String,
        planned_end: String,
        equipment_text: Option<String>,
        basis_doc_number: Option<String>,
        note: Option<String>,
        planned_count: i32,
        arrived_count: i32,
        in_training_count: i32,
    }

    let sql = format!(
        "SELECT rg.submission_id, \
             COALESCE(src_org.short_name, 'org#' || s.reporting_org_id::text) || ' · ' || \
             CASE s.source_type \
                 WHEN 'form' THEN 'Форма' \
                 WHEN 'table' THEN 'Таблиця' \
                 WHEN 'official_letter' THEN 'Офіц. лист' \
                 ELSE s.source_type \
             END AS source_label, \
             tk.name AS training_kind, \
             v.code AS vos_label, \
             pos.name AS position_label, \
             crs.name AS course_label, \
             ts_org.short_name || COALESCE(' ' || ts.locality, '') AS site_label, \
             org_o.short_name AS organizer_label, \
             to_char(rg.planned_start, 'DD.MM.YYYY') AS planned_start, \
             to_char(rg.planned_end, 'DD.MM.YYYY') AS planned_end, \
             rg.equipment_text, \
             rg.basis_doc_number, \
             rg.note, \
             rg.planned_count, \
             rg.arrived_count, \
             rg.in_training_count \
         FROM reported_group rg \
         JOIN submission s ON s.id = rg.submission_id \
         JOIN org src_org ON src_org.id = s.reporting_org_id \
         JOIN training_kind tk ON tk.id = rg.training_kind_id \
         JOIN training_site ts ON ts.id = rg.site_id \
         JOIN org ts_org ON ts_org.id = ts.org_id \
         LEFT JOIN vos v ON v.id = rg.vos_id \
         LEFT JOIN \"position\" pos ON pos.id = rg.position_id \
         LEFT JOIN course crs ON crs.id = rg.course_id \
         LEFT JOIN org org_o ON org_o.id = rg.organizer_org_id \
         WHERE rg.submission_id IN ({ids_csv}) \
           AND ({matched_filter}) \
         ORDER BY rg.submission_id",
        ids_csv = ids_csv,
        matched_filter = if let Some(gid) = meta.group_id {
            format!("rg.matched_group_id = {gid}")
        } else {
            "TRUE".to_string()
        },
    );

    let rows = RgRow::find_by_statement(Statement::from_string(db.get_database_backend(), sql))
        .all(db)
        .await?;

    let snapshots = rows
        .into_iter()
        .map(|r| ReportedGroupSnapshot {
            submission_id: r.submission_id,
            source_label: r.source_label,
            training_kind: r.training_kind,
            vos_label: r.vos_label,
            position_label: r.position_label,
            course_label: r.course_label,
            site_label: r.site_label,
            organizer_label: r.organizer_label,
            planned_start: r.planned_start,
            planned_end: r.planned_end,
            equipment_text: r.equipment_text,
            basis_doc_number: r.basis_doc_number,
            note: r.note,
            planned_count: r.planned_count,
            arrived_count: r.arrived_count,
            in_training_count: r.in_training_count,
        })
        .collect();

    Ok(Some(DiscrepancyComparison {
        metric_label: metric_label(&meta.metric).to_string(),
        metric: meta.metric,
        rows: snapshots,
    }))
}
