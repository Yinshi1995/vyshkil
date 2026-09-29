//! SQL для укомплектованості (`staffing_snapshot`/`staffing_metric`, 01 §4, Етап 5).

use sea_orm::{ConnectionTrait, DbErr, FromQueryResult, Statement};

use crate::types::staffing::{InstructorStaffingRow, StaffingRow};

type MetricGetter = fn(&StaffingRow) -> i64;

const METRICS: &[(&str, MetricGetter)] = &[
    ("by_tos", |r| r.by_tos),
    ("by_list", |r| r.by_list),
    ("present", |r| r.present),
    ("trained_sergeant", |r| r.trained_sergeant),
    ("in_training", |r| r.in_training),
    ("planned_next_month", |r| r.planned_next_month),
    ("need_training", |r| r.need_training),
];

type InstructorMetricGetter = fn(&InstructorStaffingRow) -> i64;

const INSTRUCTOR_METRICS: &[(&str, InstructorMetricGetter)] = &[
    ("by_tos", |r| r.by_tos),
    ("by_list", |r| r.by_list),
    ("trained_sergeant", |r| r.trained_sergeant),
    ("trained_kibr", |r| r.trained_kibr),
];

/// Записує один рядок (org + 7 чисел) як `staffing_snapshot` + його `staffing_metric`-и. Викликач
/// відповідає за перевірку `policy::can_edit_org(row.org_id)` ДО виклику (той самий порядок, що
/// й `repo::groups::commit_group_rows`) і за транзакцію/`SET LOCAL app.actor`.
pub async fn insert_snapshot(
    db: &impl ConnectionTrait,
    org_id: i32,
    as_of: &str,
    category: &str,
    submission_id: i32,
    row: &StaffingRow,
) -> Result<i32, DbErr> {
    #[derive(FromQueryResult)]
    struct NewId {
        id: i32,
    }
    let snapshot = NewId::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO staffing_snapshot (org_id, as_of, category, submission_id) \
         VALUES ($1, $2::date, $3, $4) RETURNING id",
        [org_id.into(), as_of.into(), category.into(), submission_id.into()],
    ))
    .one(db)
    .await?
    .ok_or_else(|| DbErr::Custom("INSERT staffing_snapshot не повернув id".into()))?;

    for (metric, get) in METRICS {
        db.execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            "INSERT INTO staffing_metric (snapshot_id, metric, value) VALUES ($1, $2, $3)",
            [snapshot.id.into(), (*metric).into(), (get(row) as i32).into()],
        ))
        .await?;
    }

    Ok(snapshot.id)
}

/// Те саме для ІВС (`category='instructors'`) — інший набір метрик (`INSTRUCTOR_METRICS`), тому
/// окрема функція, не узагальнення через generic/trait (два виклики — не варто абстракції, 07 §1).
pub async fn insert_instructor_snapshot(
    db: &impl ConnectionTrait,
    org_id: i32,
    as_of: &str,
    submission_id: i32,
    row: &InstructorStaffingRow,
) -> Result<i32, DbErr> {
    #[derive(FromQueryResult)]
    struct NewId {
        id: i32,
    }
    let snapshot = NewId::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO staffing_snapshot (org_id, as_of, category, submission_id) \
         VALUES ($1, $2::date, 'instructors', $3) RETURNING id",
        [org_id.into(), as_of.into(), submission_id.into()],
    ))
    .one(db)
    .await?
    .ok_or_else(|| DbErr::Custom("INSERT staffing_snapshot не повернув id".into()))?;

    for (metric, get) in INSTRUCTOR_METRICS {
        db.execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            "INSERT INTO staffing_metric (snapshot_id, metric, value) VALUES ($1, $2, $3)",
            [snapshot.id.into(), (*metric).into(), (get(row) as i32).into()],
        ))
        .await?;
    }

    Ok(snapshot.id)
}
