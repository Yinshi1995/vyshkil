//! Резолюція сирих рядків "Фах" (`backend::import::fah::RawFahRow`) у `GroupFormRow` (та сама
//! сітка, що й ручне введення, 03 §6: "Таблиця як у формі введення (02)") — org/vos/посада/місце
//! шукаються в довідниках; нерозпізнане лишається текстом з `*_id = None`, і сітка сама підсвітить
//! як помилку при спробі зафіксувати (той самий шлях, що й "не вказано частину-відправника").

use sea_orm::DatabaseConnection;

use crate::backend::import::fah::RawFahRow;
use crate::backend::repo::{dictionaries, groups, orgs};
use crate::types::submission::GroupFormRow;

/// `code='special'` — Фах завжди фахова підготовка (`training_kind`, Стадія 2 сід).
const TRAINING_KIND_CODE: &str = "special";

pub async fn resolve_rows(
    db: &DatabaseConnection,
    raw_rows: Vec<RawFahRow>,
) -> Result<Vec<GroupFormRow>, sea_orm::DbErr> {
    let training_kind_id = dictionaries::training_kind_id_by_code(db, TRAINING_KIND_CODE).await?;

    let mut out = Vec::with_capacity(raw_rows.len());
    for raw in raw_rows {
        let sender = orgs::resolve_org(db, &raw.org_raw).await?;
        let (sender_org_id, sender_org_label) = match sender {
            Some(r) => (Some(r.org_id), r.label),
            None => (None, raw.org_raw.clone()),
        };

        let (site_id, site_label) = match orgs::resolve_org(db, &raw.site_raw).await? {
            Some(r) => {
                let site_id = groups::find_or_create_training_site(db, r.org_id).await?;
                (Some(site_id), r.label)
            }
            None => (None, raw.site_raw.clone()),
        };

        let (vos_id, vos_label) = match dictionaries::resolve_vos_by_code(db, &raw.vos_raw).await? {
            Some((id, label)) => (Some(id), label),
            None => (None, String::new()),
        };
        let (position_id, position_label) =
            match dictionaries::resolve_position(db, &raw.position_raw).await? {
                Some((id, label)) => (Some(id), label),
                None => (None, raw.position_raw.clone()),
            };
        // Одне поле "ВОС / посада / курс" у сітці (02 §3) — Фах дає їх окремими колонками,
        // склеюємо в той самий вигляд, що показує автокомпліт грід (`code — title`, або посада).
        let vos_position_course_label = if !vos_label.is_empty() {
            vos_label
        } else {
            position_label
        };

        let count: i64 = raw.count_raw.trim().parse().unwrap_or(0);

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
            planned_count: count,
            arrived_count: count,
            in_training_count: count,
            composition: Vec::new(),
            organizer_org_id: None,
            organizer_org_label: String::new(),
            basis_doc_number: String::new(),
            basis_doc_date_raw: String::new(),
            note: format!("Імпорт: {} {}", raw.sheet, raw.row_number),
        });
    }
    Ok(out)
}
