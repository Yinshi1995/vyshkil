//! Резолюція сирих рядків "КВід" (`backend::import::kvid::RawKvidRow`) у `StaffingRow` — лише
//! організація потребує резолюції (числа вже структуровані).

use sea_orm::DatabaseConnection;

use crate::backend::import::kvid::RawKvidRow;
use crate::backend::repo::orgs;
use crate::types::staffing::StaffingRow;

fn parse(raw: &str) -> i64 {
    raw.trim().parse().unwrap_or(0)
}

pub async fn resolve_rows(
    db: &DatabaseConnection,
    raw_rows: Vec<RawKvidRow>,
) -> Result<Vec<StaffingRow>, sea_orm::DbErr> {
    let mut out = Vec::with_capacity(raw_rows.len());
    for raw in raw_rows {
        let (org_id, org_label) = match orgs::resolve_org(db, &raw.org_raw).await? {
            Some(r) => (Some(r.org_id), r.label),
            None => (None, raw.org_raw.clone()),
        };
        out.push(StaffingRow {
            org_id,
            org_label,
            by_tos: parse(&raw.by_tos_raw),
            by_list: parse(&raw.by_list_raw),
            present: parse(&raw.present_raw),
            trained_sergeant: parse(&raw.trained_sergeant_raw),
            in_training: parse(&raw.in_training_raw),
            planned_next_month: parse(&raw.planned_next_month_raw),
            need_training: parse(&raw.need_training_raw),
        });
    }
    Ok(out)
}
