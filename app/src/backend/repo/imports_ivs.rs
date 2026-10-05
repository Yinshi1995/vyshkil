//! Резолюція сирих рядків "ІВС" (`backend::import::ivs::IvsExtract`) — ТРИ різних результати з
//! ОДНОГО файлу (модуль `ivs.rs` пояснює чому): укомплектованість (`InstructorStaffingRow`,
//! `category='instructors'`) окремо; стажування й курси РАЗОМ як `GroupFormRow` (та сама сітка,
//! що й Фах/БпС) — обидва "рядок групи" з іншим заповненим ідентифікатором (`training_kind`
//! 'internship' для стажування, 'special'+`course_id` для курсів), відмінність лише вихідних даних,
//! не форми.

use sea_orm::DatabaseConnection;

use crate::backend::import::ivs::{IvsExtract, RawInternshipRow, RawIvsCourseRow, RawIvsStaffingRow};
use crate::backend::repo::{dictionaries, groups, orgs};
use crate::types::staffing::InstructorStaffingRow;
use crate::types::submission::GroupFormRow;

const INTERNSHIP_KIND_CODE: &str = "internship";
const COURSE_KIND_CODE: &str = "special";

pub struct IvsResolved {
    pub staffing: Vec<InstructorStaffingRow>,
    pub groups: Vec<GroupFormRow>,
}

async fn resolve_staffing(
    db: &DatabaseConnection,
    rows: Vec<RawIvsStaffingRow>,
) -> Result<Vec<InstructorStaffingRow>, sea_orm::DbErr> {
    let mut out = Vec::with_capacity(rows.len());
    for raw in rows {
        let (org_id, org_label) = match orgs::resolve_org(db, &raw.org_raw).await? {
            Some(r) => (Some(r.org_id), r.label),
            None => (None, raw.org_raw.clone()),
        };
        let parse = |s: &str| s.trim().parse::<i64>().unwrap_or(0);
        out.push(InstructorStaffingRow {
            org_id,
            org_label,
            by_tos: parse(&raw.by_tos_raw),
            by_list: parse(&raw.by_list_raw),
            trained_sergeant: parse(&raw.trained_sergeant_raw),
            trained_kibr: parse(&raw.trained_kibr_raw),
        });
    }
    Ok(out)
}

async fn resolve_site(
    db: &DatabaseConnection,
    site_raw: &str,
) -> Result<(Option<i32>, String), sea_orm::DbErr> {
    Ok(match orgs::resolve_org(db, site_raw).await? {
        Some(r) => {
            let site_id = groups::find_or_create_training_site(db, r.org_id).await?;
            (Some(site_id), r.label)
        }
        None => (None, site_raw.to_string()),
    })
}

async fn resolve_internships(
    db: &DatabaseConnection,
    rows: Vec<RawInternshipRow>,
) -> Result<Vec<GroupFormRow>, sea_orm::DbErr> {
    let training_kind_id = dictionaries::training_kind_id_by_code(db, INTERNSHIP_KIND_CODE).await?;

    let mut out = Vec::with_capacity(rows.len());
    for raw in rows {
        let (sender_org_id, sender_org_label) = match orgs::resolve_org(db, &raw.org_raw).await? {
            Some(r) => (Some(r.org_id), r.label),
            None => (None, raw.org_raw.clone()),
        };
        let (site_id, site_label) = resolve_site(db, &raw.site_raw).await?;
        let count: i64 = raw.count_raw.trim().parse().unwrap_or(0);

        out.push(GroupFormRow {
            sender_org_id,
            sender_org_label,
            training_kind_id,
            training_kind_label: "Стажування".to_string(),
            bzvp_program_id: None,
            vos_id: None,
            position_id: None,
            course_id: None,
            vos_position_course_label: String::new(),
            equipment_text: String::new(),
            site_id,
            site_label,
            planned_start_raw: raw.start_raw,
            planned_end_raw: raw.end_raw,
            planned_count: count,
            arrived_count: count,
            in_training_count: count,
            composition: Vec::new(),
            organizer_org_id: None,
            organizer_org_label: String::new(),
            basis_doc_number: String::new(),
            basis_doc_date_raw: String::new(),
            note: format!("Імпорт: ІВС-Стажування {}", raw.row_number),
        });
    }
    Ok(out)
}

async fn resolve_courses(
    db: &DatabaseConnection,
    rows: Vec<RawIvsCourseRow>,
) -> Result<Vec<GroupFormRow>, sea_orm::DbErr> {
    let training_kind_id = dictionaries::training_kind_id_by_code(db, COURSE_KIND_CODE).await?;

    let mut out = Vec::with_capacity(rows.len());
    for raw in rows {
        let (sender_org_id, sender_org_label) = match orgs::resolve_org(db, &raw.org_raw).await? {
            Some(r) => (Some(r.org_id), r.label),
            None => (None, raw.org_raw.clone()),
        };
        let (site_id, site_label) = resolve_site(db, &raw.site_raw).await?;
        let (course_id, course_label) = match dictionaries::resolve_course(db, &raw.course_raw).await? {
            Some((id, label)) => (Some(id), label),
            None => (None, raw.course_raw.clone()),
        };
        let count: i64 = raw.count_raw.trim().parse().unwrap_or(0);

        out.push(GroupFormRow {
            sender_org_id,
            sender_org_label,
            training_kind_id,
            training_kind_label: "Фахова".to_string(),
            bzvp_program_id: None,
            vos_id: None,
            position_id: None,
            course_id,
            vos_position_course_label: course_label,
            equipment_text: String::new(),
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
            note: format!("Імпорт: ІВС-Курс {}", raw.row_number),
        });
    }
    Ok(out)
}

pub async fn resolve_rows(
    db: &DatabaseConnection,
    extract: IvsExtract,
) -> Result<IvsResolved, sea_orm::DbErr> {
    let staffing = resolve_staffing(db, extract.staffing).await?;
    let mut groups = resolve_internships(db, extract.internships).await?;
    groups.extend(resolve_courses(db, extract.courses).await?);
    Ok(IvsResolved { staffing, groups })
}
