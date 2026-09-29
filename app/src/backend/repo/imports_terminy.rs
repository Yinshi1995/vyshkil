//! Резолюція сирих рядків "Терміни" (`backend::import::terminy::TerminyExtract`) — ТРИ незалежні
//! списки (БЗВП/Фахова/Адаптація) РАЗОМ у ТОЙ САМИЙ `GroupFormRow`/`Grid`, що й усі попередні
//! імпорти цієї сесії (Фах/БпС/ІВС): різні `training_kind` замість різних форм даних.

use sea_orm::DatabaseConnection;

use crate::backend::import::terminy::{extract_vos_code, RawAdaptRow, RawBzvpRow, RawSpecialRow, TerminyExtract};
use crate::backend::repo::{dictionaries, groups, orgs};
use crate::types::submission::GroupFormRow;

const BZVP_KIND_CODE: &str = "bzvp";
const SPECIAL_KIND_CODE: &str = "special";
const ADAPTATION_KIND_CODE: &str = "adaptation";

async fn resolve_org_pair(
    db: &DatabaseConnection,
    org_raw: &str,
) -> Result<(Option<i32>, String), sea_orm::DbErr> {
    Ok(match orgs::resolve_org(db, org_raw).await? {
        Some(r) => (Some(r.org_id), r.label),
        None => (None, org_raw.to_string()),
    })
}

async fn resolve_site(
    db: &DatabaseConnection,
    place_raw: &str,
) -> Result<(Option<i32>, String), sea_orm::DbErr> {
    if place_raw.is_empty() {
        return Ok((None, String::new()));
    }
    Ok(match orgs::resolve_org(db, place_raw).await? {
        Some(r) => {
            let site_id = groups::find_or_create_training_site(db, r.org_id).await?;
            (Some(site_id), r.label)
        }
        None => (None, place_raw.to_string()),
    })
}

async fn resolve_bzvp(
    db: &DatabaseConnection,
    rows: Vec<RawBzvpRow>,
) -> Result<Vec<GroupFormRow>, sea_orm::DbErr> {
    let training_kind_id = dictionaries::training_kind_id_by_code(db, BZVP_KIND_CODE).await?;
    let mut out = Vec::with_capacity(rows.len());
    for raw in rows {
        let (sender_org_id, sender_org_label) = resolve_org_pair(db, &raw.org_raw).await?;
        let (site_id, site_label) = resolve_site(db, &raw.place_raw).await?;
        let count: i64 = raw.count_raw.trim().parse().unwrap_or(0);
        out.push(GroupFormRow {
            sender_org_id,
            sender_org_label,
            training_kind_id,
            training_kind_label: "БЗВП".to_string(),
            site_id,
            site_label,
            planned_start_raw: raw.term_raw,
            planned_count: count,
            arrived_count: count,
            in_training_count: count,
            note: format!("Імпорт: Терміни, БЗВП. Розподіл: {}", raw.distribution_raw),
            ..Default::default()
        });
    }
    Ok(out)
}

async fn resolve_special(
    db: &DatabaseConnection,
    rows: Vec<RawSpecialRow>,
) -> Result<Vec<GroupFormRow>, sea_orm::DbErr> {
    let training_kind_id = dictionaries::training_kind_id_by_code(db, SPECIAL_KIND_CODE).await?;
    let mut out = Vec::with_capacity(rows.len());
    for raw in rows {
        let (sender_org_id, sender_org_label) = resolve_org_pair(db, &raw.org_raw).await?;
        let (site_id, site_label) = resolve_site(db, &raw.place_raw).await?;
        let (vos_id, vos_position_course_label) = match extract_vos_code(&raw.specialty_raw) {
            Some(code) => match dictionaries::resolve_vos_by_code(db, &code).await? {
                Some((id, label)) => (Some(id), label),
                None => (None, raw.specialty_raw.clone()),
            },
            None => (None, raw.specialty_raw.clone()),
        };
        let count: i64 = raw.count_raw.trim().parse().unwrap_or(0);
        out.push(GroupFormRow {
            sender_org_id,
            sender_org_label,
            training_kind_id,
            training_kind_label: "Фахова".to_string(),
            vos_id,
            vos_position_course_label,
            site_id,
            site_label,
            planned_start_raw: raw.term_raw,
            planned_count: count,
            arrived_count: count,
            in_training_count: count,
            note: "Імпорт: Терміни, фахова підготовка".to_string(),
            ..Default::default()
        });
    }
    Ok(out)
}

async fn resolve_adapt(
    db: &DatabaseConnection,
    rows: Vec<RawAdaptRow>,
) -> Result<Vec<GroupFormRow>, sea_orm::DbErr> {
    let training_kind_id = dictionaries::training_kind_id_by_code(db, ADAPTATION_KIND_CODE).await?;
    let mut out = Vec::with_capacity(rows.len());
    for raw in rows {
        let (sender_org_id, sender_org_label) = resolve_org_pair(db, &raw.org_raw).await?;
        let count: i64 = raw.count_raw.trim().parse().unwrap_or(0);
        out.push(GroupFormRow {
            sender_org_id,
            sender_org_label,
            training_kind_id,
            training_kind_label: "Адаптація".to_string(),
            planned_start_raw: raw.term_raw,
            planned_count: count,
            arrived_count: count,
            in_training_count: count,
            note: format!("Імпорт: Терміни, адаптація. Розподіл: {}", raw.distribution_raw),
            ..Default::default()
        });
    }
    Ok(out)
}

pub async fn resolve_rows(
    db: &DatabaseConnection,
    extract: TerminyExtract,
) -> Result<Vec<GroupFormRow>, sea_orm::DbErr> {
    let mut out = resolve_bzvp(db, extract.bzvp).await?;
    out.extend(resolve_special(db, extract.special).await?);
    out.extend(resolve_adapt(db, extract.adapt).await?);
    Ok(out)
}
