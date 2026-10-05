//! Резолюція сирих рядків архіву ВЧ (`backend::import::vch_archive::RawVchArchiveRow`) у
//! `GroupFormRow` (та сама сітка, що й Фах/БпС/ІВС/Терміни, Етап 6 — лише інше джерело подання:
//! `source_type='archive_seed'` замість `'table'`, встановлюється в `pages/archive_import/server.rs`,
//! не тут). "Місце проведення" тут — вільний географічний текст, не назва частини/майданчика —
//! `orgs::resolve_org` на ньому часто дасть `None`, і це очікувано (03 §5).

use sea_orm::DatabaseConnection;

use crate::backend::import::vch_archive::RawVchArchiveRow;
use crate::backend::repo::{dictionaries, groups, orgs};
use crate::types::submission::GroupFormRow;

/// `code='special'` — фахова підготовка/спеціальність (той самий вид, що й "Фах", Стадія 2 сід).
const TRAINING_KIND_CODE: &str = "special";

pub async fn resolve_rows(
    db: &DatabaseConnection,
    raw_rows: Vec<RawVchArchiveRow>,
) -> Result<Vec<GroupFormRow>, sea_orm::DbErr> {
    let training_kind_id = dictionaries::training_kind_id_by_code(db, TRAINING_KIND_CODE).await?;

    let mut out = Vec::with_capacity(raw_rows.len());
    for raw in raw_rows {
        let (sender_org_id, sender_org_label) = match orgs::resolve_org(db, &raw.org_raw).await? {
            Some(r) => (Some(r.org_id), r.label),
            None => (None, raw.org_raw.clone()),
        };

        let (site_id, site_label) = match orgs::resolve_org(db, &raw.place_raw).await? {
            Some(r) => {
                let site_id = groups::find_or_create_training_site(db, r.org_id).await?;
                (Some(site_id), r.label)
            }
            None => (None, raw.place_raw.clone()),
        };

        let (vos_id, vos_label) = match dictionaries::resolve_vos_by_code(db, &raw.vos_raw).await? {
            Some((id, label)) => (Some(id), label),
            None => (None, raw.vos_raw.clone()),
        };
        let (position_id, position_label) =
            match dictionaries::resolve_position(db, &raw.specialty_raw).await? {
                Some((id, label)) => (Some(id), label),
                None => (None, raw.specialty_raw.clone()),
            };
        let vos_position_course_label = if !vos_label.is_empty() { vos_label } else { position_label };

        let planned: i64 = raw.planned_raw.trim().parse().unwrap_or(0);
        let actual: i64 = raw.actual_raw.trim().parse().unwrap_or(0);

        out.push(GroupFormRow {
            sender_org_id,
            sender_org_label,
            training_kind_id,
            training_kind_label: "Фахова".to_string(),
            bzvp_program_id: None,
            vos_id,
            position_id,
            course_id: None,
            vos_position_course_label,
            equipment_text: raw.equipment_raw,
            site_id,
            site_label,
            planned_start_raw: raw.start_raw.map(|d| d.format("%d.%m.%Y").to_string()).unwrap_or_default(),
            planned_end_raw: raw.end_raw.map(|d| d.format("%d.%m.%Y").to_string()).unwrap_or_default(),
            planned_count: planned,
            arrived_count: actual,
            in_training_count: actual,
            composition: Vec::new(),
            organizer_org_id: None,
            organizer_org_label: String::new(),
            basis_doc_number: String::new(),
            basis_doc_date_raw: String::new(),
            note: format!("Імпорт: Архів {}", raw.row_number),
        });
    }
    Ok(out)
}
