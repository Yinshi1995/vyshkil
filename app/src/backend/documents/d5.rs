//! D5 — додатки корпусу (05 §D5): Фах/БпС/КВід/ІВС/Терміни, кожен окремим xlsx.
//! Вхід — готові дані з `repo::documents`, без SQL тут.

use rust_xlsxwriter::{Format, FormatAlign, Formula, Workbook, XlsxError};

use crate::backend::repo::documents::{GroupDetailRow, StaffingDetail};

fn header_fmt() -> Format {
    Format::new().set_bold().set_align(FormatAlign::Center)
}

fn bold_fmt() -> Format {
    Format::new().set_bold()
}

fn sum_formula(col: u16, first_row: u32, last_row: u32) -> Formula {
    let col_letter = (b'A' + col as u8) as char;
    Formula::new(format!("SUM({col_letter}{first_row}:{col_letter}{last_row})"))
}

// ─── D5 Фах ────────────────────────────────────────────────────────────────

pub fn build_fah(
    workbook: &mut Workbook,
    corps_label: &str,
    as_of: &str,
    groups: &[GroupDetailRow],
) -> Result<(), XlsxError> {
    let completed: Vec<&GroupDetailRow> = groups.iter().filter(|g| g.completed > 0).collect();
    let in_progress: Vec<&GroupDetailRow> = groups.iter().filter(|g| g.in_training > 0).collect();

    build_fah_sheet(workbook, "Пройшли", corps_label, as_of, &completed)?;
    build_fah_sheet(workbook, "Проходять", corps_label, as_of, &in_progress)?;
    Ok(())
}

fn build_fah_sheet(
    workbook: &mut Workbook,
    sheet_name: &str,
    corps_label: &str,
    as_of: &str,
    rows: &[&GroupDetailRow],
) -> Result<(), XlsxError> {
    let ws = workbook.add_worksheet();
    ws.set_name(sheet_name)?;

    let hdr = header_fmt();
    let title = format!("{corps_label}. Фахова підготовка — {sheet_name} (станом на {as_of})");
    ws.merge_range(0, 0, 0, 7, &title, &bold_fmt())?;

    for (c, h) in ["№", "Підрозділ", "Місце", "Посада/ВОС", "ОВТ", "Термін з", "Термін по", "Кількість"].iter().enumerate() {
        ws.write_string_with_format(2, c as u16, *h, &hdr)?;
    }

    for (i, g) in rows.iter().enumerate() {
        let r = (i + 3) as u32;
        ws.write_number(r, 0, (i + 1) as f64)?;
        ws.write_string(r, 1, &g.org_label)?;
        ws.write_string(r, 2, &g.site_label)?;
        let vos_pos = match (&g.vos_code, &g.position_label) {
            (Some(v), Some(p)) => format!("{v} / {p}"),
            (Some(v), None) => v.clone(),
            (None, Some(p)) => p.clone(),
            (None, None) => g.course_label.clone().unwrap_or_default(),
        };
        ws.write_string(r, 3, &vos_pos)?;
        ws.write_string(r, 4, g.equipment_text.as_deref().unwrap_or(""))?;
        ws.write_string(r, 5, &g.planned_start)?;
        ws.write_string(r, 6, &g.planned_end)?;
        let count = if g.in_training > 0 { g.in_training } else { g.completed };
        ws.write_number(r, 7, count as f64)?;
    }

    Ok(())
}

// ─── D5 БпС ────────────────────────────────────────────────────────────────

pub fn build_bps(
    workbook: &mut Workbook,
    corps_label: &str,
    as_of: &str,
    groups: &[GroupDetailRow],
) -> Result<(), XlsxError> {
    let completed: Vec<&GroupDetailRow> = groups.iter().filter(|g| g.completed > 0).collect();
    let in_progress: Vec<&GroupDetailRow> = groups.iter().filter(|g| g.in_training > 0).collect();

    build_bps_sheet(workbook, "Завершилась", corps_label, as_of, &completed)?;
    build_bps_sheet(workbook, "Навчаються", corps_label, as_of, &in_progress)?;
    Ok(())
}

fn build_bps_sheet(
    workbook: &mut Workbook,
    sheet_name: &str,
    corps_label: &str,
    as_of: &str,
    rows: &[&GroupDetailRow],
) -> Result<(), XlsxError> {
    let ws = workbook.add_worksheet();
    ws.set_name(sheet_name)?;

    let hdr = header_fmt();
    let title = format!("{corps_label}. БпС — {sheet_name} (станом на {as_of})");
    ws.merge_range(0, 0, 0, 7, &title, &bold_fmt())?;

    for (c, h) in ["№", "Тип БпАК", "ВОС", "ВЧ", "Місце", "Терміни", "Кількість", "Примітки"].iter().enumerate() {
        ws.write_string_with_format(2, c as u16, *h, &hdr)?;
    }

    for (i, g) in rows.iter().enumerate() {
        let r = (i + 3) as u32;
        ws.write_number(r, 0, (i + 1) as f64)?;
        ws.write_string(r, 1, g.equipment_text.as_deref().unwrap_or(""))?;
        ws.write_string(r, 2, g.vos_code.as_deref().unwrap_or(""))?;
        ws.write_string(r, 3, &g.org_label)?;
        ws.write_string(r, 4, &g.site_label)?;
        let terms = format!("{} — {}", g.planned_start, g.planned_end);
        ws.write_string(r, 5, &terms)?;
        let count = if g.in_training > 0 { g.in_training } else { g.completed };
        ws.write_number(r, 6, count as f64)?;
        ws.write_string(r, 7, "")?;
    }

    Ok(())
}

// ─── D5 КВід ───────────────────────────────────────────────────────────────

pub fn build_kvid(
    workbook: &mut Workbook,
    corps_label: &str,
    as_of: &str,
    rows: &[StaffingDetail],
) -> Result<(), XlsxError> {
    let ws = workbook.add_worksheet();
    ws.set_name("КВід")?;

    let hdr = header_fmt();
    let title = format!("{corps_label}. Укомплектованість КВід (станом на {as_of})");
    ws.merge_range(0, 0, 0, 7, &title, &bold_fmt())?;

    for (c, h) in ["№", "Підрозділ", "За штатом", "За списком", "Наявність", "Мають підготовку", "Проходять", "Потребують"].iter().enumerate() {
        ws.write_string_with_format(2, c as u16, *h, &hdr)?;
    }

    for (i, s) in rows.iter().enumerate() {
        let r = (i + 3) as u32;
        ws.write_number(r, 0, (i + 1) as f64)?;
        ws.write_string(r, 1, &s.org_label)?;
        ws.write_number(r, 2, s.by_tos as f64)?;
        ws.write_number(r, 3, s.by_list as f64)?;
        ws.write_number(r, 4, s.present as f64)?;
        ws.write_number(r, 5, s.trained as f64)?;
        ws.write_number(r, 6, s.in_training as f64)?;
        ws.write_number(r, 7, s.need as f64)?;
    }

    let total_row = (rows.len() + 3) as u32;
    ws.write_string_with_format(total_row, 1, "ВСЬОГО", &bold_fmt())?;
    for c in 2..=7u16 {
        ws.write_formula(total_row, c, sum_formula(c, 4, total_row))?;
    }

    Ok(())
}

// ─── D5 ІВС ───────────────────────────────────────────────────────────────

pub fn build_ivs(
    workbook: &mut Workbook,
    corps_label: &str,
    as_of: &str,
    rows: &[StaffingDetail],
) -> Result<(), XlsxError> {
    let ws = workbook.add_worksheet();
    ws.set_name("ІВС")?;

    let hdr = header_fmt();
    let title = format!("{corps_label}. Укомплектованість ІВС (станом на {as_of})");
    ws.merge_range(0, 0, 0, 5, &title, &bold_fmt())?;

    for (c, h) in ["№", "Підрозділ", "За штатом", "За списком", "Мають підготовку", "Пройшли КІБР"].iter().enumerate() {
        ws.write_string_with_format(2, c as u16, *h, &hdr)?;
    }

    for (i, s) in rows.iter().enumerate() {
        let r = (i + 3) as u32;
        ws.write_number(r, 0, (i + 1) as f64)?;
        ws.write_string(r, 1, &s.org_label)?;
        ws.write_number(r, 2, s.by_tos as f64)?;
        ws.write_number(r, 3, s.by_list as f64)?;
        ws.write_number(r, 4, s.trained as f64)?;
        ws.write_number(r, 5, s.need as f64)?;
    }

    let total_row = (rows.len() + 3) as u32;
    ws.write_string_with_format(total_row, 1, "ВСЬОГО", &bold_fmt())?;
    for c in 2..=5u16 {
        ws.write_formula(total_row, c, sum_formula(c, 4, total_row))?;
    }

    Ok(())
}

// ─── D5 Терміни ────────────────────────────────────────────────────────────

pub fn build_terminy(
    workbook: &mut Workbook,
    corps_label: &str,
    as_of: &str,
    bzvp: &[GroupDetailRow],
    special: &[GroupDetailRow],
    adaptation: &[GroupDetailRow],
) -> Result<(), XlsxError> {
    build_terminy_sheet(workbook, "БЗВП", corps_label, as_of, bzvp)?;
    build_terminy_sheet(workbook, "Фахова", corps_label, as_of, special)?;
    build_terminy_sheet(workbook, "Адаптація", corps_label, as_of, adaptation)?;
    Ok(())
}

fn build_terminy_sheet(
    workbook: &mut Workbook,
    kind_name: &str,
    corps_label: &str,
    as_of: &str,
    groups: &[GroupDetailRow],
) -> Result<(), XlsxError> {
    let ws = workbook.add_worksheet();
    ws.set_name(kind_name)?;

    let hdr = header_fmt();
    let title = format!("{corps_label}. Терміни — {kind_name} (станом на {as_of})");
    ws.merge_range(0, 0, 0, 6, &title, &bold_fmt())?;

    for (c, h) in ["№", "Підрозділ", "Кількість", "Термін з", "Термін по", "Місце", "Спеціальність"].iter().enumerate() {
        ws.write_string_with_format(2, c as u16, *h, &hdr)?;
    }

    for (i, g) in groups.iter().enumerate() {
        let r = (i + 3) as u32;
        ws.write_number(r, 0, (i + 1) as f64)?;
        ws.write_string(r, 1, &g.org_label)?;
        ws.write_number(r, 2, g.in_training as f64)?;
        ws.write_string(r, 3, &g.planned_start)?;
        ws.write_string(r, 4, &g.planned_end)?;
        ws.write_string(r, 5, &g.site_label)?;
        let spec = match (&g.vos_code, &g.course_label) {
            (Some(v), Some(c)) => format!("{v} ({c})"),
            (Some(v), None) => v.clone(),
            (None, Some(c)) => c.clone(),
            (None, None) => String::new(),
        };
        ws.write_string(r, 6, &spec)?;
    }

    Ok(())
}
