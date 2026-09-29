//! Резолюція сирих рядків "БпС" (`backend::import::bps::RawBpsRow`) у `GroupFormRow` — та сама
//! ідея, що й `imports_fah.rs`, але частина/місце тут дають ОКРЕМУ колонку "номер" (`org.number`,
//! напр. "А7384") — резолюція через `resolve_org` на самому номері значно точніша за фах-текст.

use sea_orm::DatabaseConnection;

use crate::backend::import::bps::RawBpsRow;
use crate::backend::repo::{dictionaries, groups, orgs};
use crate::types::submission::GroupFormRow;

const TRAINING_KIND_CODE: &str = "special";

pub async fn resolve_rows(
    db: &DatabaseConnection,
    raw_rows: Vec<RawBpsRow>,
) -> Result<Vec<GroupFormRow>, sea_orm::DbErr> {
    let training_kind_id = dictionaries::training_kind_id_by_code(db, TRAINING_KIND_CODE).await?;

    let mut out = Vec::with_capacity(raw_rows.len());
    for raw in raw_rows {
        let sender = orgs::resolve_org(db, &raw.org_number_raw).await?;
        let (sender_org_id, sender_org_label) = match sender {
            Some(r) => (Some(r.org_id), r.label),
            None => (None, format!("{} {}", raw.org_number_raw, raw.org_name_raw).trim().to_string()),
        };

        let (site_id, site_label) = match orgs::resolve_org(db, &raw.site_number_raw).await? {
            Some(r) => {
                let site_id = groups::find_or_create_training_site(db, r.org_id).await?;
                (Some(site_id), r.label)
            }
            None => (None, format!("{} {}", raw.site_number_raw, raw.site_name_raw).trim().to_string()),
        };

        let (vos_id, vos_label) = match dictionaries::resolve_vos_by_code(db, &raw.vos_raw).await? {
            Some((id, label)) => (Some(id), label),
            None => (None, raw.vos_raw.clone()),
        };

        // Funnel: "Викликали" завжди -> planned; наступна стадія ("Прибуло до НЦ" на "Завершилась",
        // "Проходять" на "Навчаються") -> arrived; третя стадія, коли є ("Успішно завершило
        // навчання") -> in_training; коли третьої нема (аркуш "Навчаються") -- дублюємо
        // next_stage у in_training (та сама спрощена рівність, що й у ручному введенні, 02 §4).
        let planned: i64 = raw.called_raw.trim().parse().unwrap_or(0);
        let next_stage: i64 = raw.next_stage_raw.trim().parse().unwrap_or(0);
        let completed: Option<i64> = raw.completed_raw.as_deref().and_then(|s| s.trim().parse().ok());
        let in_training = completed.unwrap_or(next_stage);

        out.push(GroupFormRow {
            sender_org_id,
            sender_org_label,
            training_kind_id,
            training_kind_label: "Фахова".to_string(),
            bzvp_program_id: None,
            vos_id,
            position_id: None,
            course_id: None,
            vos_position_course_label: vos_label,
            equipment_text: raw.equipment_raw,
            site_id,
            site_label,
            planned_start_raw: raw.start_raw.map(|d| d.format("%d.%m.%Y").to_string()).unwrap_or_default(),
            planned_end_raw: raw.end_raw.map(|d| d.format("%d.%m.%Y").to_string()).unwrap_or_default(),
            planned_count: planned,
            arrived_count: next_stage,
            in_training_count: in_training,
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
