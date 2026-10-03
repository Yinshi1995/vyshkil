//! D2 — "Контролька", накопичувальний тиждень (05 §D2). Чиста генерація xlsx з готових
//! `repo::documents`-даних (жодних SQL-викликів тут) — той самий поділ, що `d1.rs`.
//!
//! **Перший вертикальний зріз — лише ОДИН тиждень** (не весь накопичувальний журнал за багато
//! тижнів — те чекає на рішення про фонову генерацію, 05 §"Технічні рекомендації").
//!
//! На відміну від D1: правило групування (01 §1) тут НЕ показується двома розділеними секціями —
//! емпірично підтверджено на реальному еталоні (`110 омбр` з'являється як звичайний рядок і під
//! 17 АК, і під 20 АК того самого дня, без позначки "поза смугою") — виклик боку сервера сам
//! об'єднує `rollup.main`+`rollup.out_of_zone` в один плаский список на корпус.

use crate::backend::documents::style;
use crate::backend::repo::documents::OrgRollupRow;
use chrono::NaiveDate;
use rust_xlsxwriter::{Format, Formula, Workbook, XlsxError};

const COL_ORG: u16 = 0;
const COL_NOTE: u16 = 13;
/// (Всього, Закінчують сьогодні, почали сьогодні, вибули з різних причин) — перша колонка
/// кожного виду підготовки (4 підколонки тут, не 3, як у D1 — 05 §D2 додає "вибули").
const KIND_COLS: [u16; 3] = [1, 5, 9];
const KIND_LABELS: [&str; 3] = ["БЗВП", "Фахова", "Адаптація"];

/// Один день тижня: дата + список (мітка_корпусу, плаский список підрозділів).
pub struct DayBlock {
    pub date: NaiveDate,
    pub corps: Vec<(String, Vec<OrgRollupRow>)>,
}

pub fn build_week_sheet(
    workbook: &mut Workbook,
    week_start: NaiveDate,
    days: &[DayBlock],
) -> Result<(), XlsxError> {
    let week_end = week_start + chrono::Duration::days(6);
    let sheet_name = format!("{}-{}", week_start.format("%d.%m"), week_end.format("%d.%m"));
    let worksheet = workbook.add_worksheet().set_name(sheet_name)?;

    let title_fmt = style::title_format();
    let hdr_fmt = style::header_format();
    let week_total_label_fmt = style::total_format();
    let week_total_num_fmt = style::total_number_format();
    let date_label_fmt = style::subheader_format();
    let date_num_fmt = style::subheader_format();
    let corps_label_fmt = style::accent_format();
    let corps_num_fmt = style::data_number_format();

    // Column widths
    worksheet.set_column_width(COL_ORG, 25)?;
    for col in 1..=12u16 {
        worksheet.set_column_width(col, 10)?;
    }
    worksheet.set_column_width(COL_NOTE, 18)?;
    worksheet.set_row_height(0, 30)?;
    worksheet.set_row_height(1, 25)?;
    worksheet.set_row_height(2, 25)?;

    worksheet.merge_range(
        0,
        COL_ORG,
        0,
        COL_NOTE,
        &format!(
            "Контролька — тиждень {} – {}",
            week_start.format("%d.%m.%Y"),
            week_end.format("%d.%m.%Y")
        ),
        &title_fmt,
    )?;

    worksheet.merge_range(1, COL_ORG, 2, COL_ORG, "Військові частини", &hdr_fmt)?;
    worksheet.merge_range(1, COL_NOTE, 2, COL_NOTE, "Примітка", &hdr_fmt)?;
    for (kind_col, label) in KIND_COLS.iter().zip(KIND_LABELS.iter()) {
        worksheet.merge_range(1, *kind_col, 1, kind_col + 3, label, &hdr_fmt)?;
        worksheet.write_string_with_format(2, *kind_col, "Всього", &hdr_fmt)?;
        worksheet.write_string_with_format(2, kind_col + 1, "Закінчують сьогодні", &hdr_fmt)?;
        worksheet.write_string_with_format(2, kind_col + 2, "почали сьогодні", &hdr_fmt)?;
        worksheet.write_string_with_format(
            2,
            kind_col + 3,
            "вибули з різних причин",
            &hdr_fmt,
        )?;
    }

    let week_total_row = 3;
    let mut row = week_total_row + 1;
    let mut date_total_rows = Vec::with_capacity(days.len());
    let mut data_row_idx: usize = 0;

    for day in days {
        let date_row = row;
        date_total_rows.push(date_row);
        row += 1;

        let mut corps_total_rows = Vec::with_capacity(day.corps.len());
        for (corps_label, orgs) in &day.corps {
            let corps_row = row;
            corps_total_rows.push(corps_row);
            row += 1;

            let first_org_row = row;
            for org in orgs {
                write_org_row(worksheet, row, org, data_row_idx.is_multiple_of(2))?;
                data_row_idx += 1;
                row += 1;
            }
            let last_org_row = row.saturating_sub(1);
            write_range_total(
                worksheet,
                corps_row,
                first_org_row,
                last_org_row,
                corps_label,
                &corps_label_fmt,
                &corps_num_fmt,
            )?;
        }

        write_cells_total(
            worksheet,
            date_row,
            &corps_total_rows,
            &day.date.format("%d.%m.%Y").to_string(),
            &date_label_fmt,
            &date_num_fmt,
        )?;
    }

    write_cells_total(worksheet, week_total_row, &date_total_rows, "Тиждень", &week_total_label_fmt, &week_total_num_fmt)?;

    worksheet.set_freeze_panes(3, 1)?;
    Ok(())
}

fn write_org_row(
    worksheet: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    org: &OrgRollupRow,
    even: bool,
) -> Result<(), XlsxError> {
    let (text_fmt, num_fmt) = if even {
        (style::data_format(), style::data_number_format())
    } else {
        (style::data_alt_format(), style::data_alt_number_format())
    };
    let note_fmt = if even { style::muted_format() } else { style::data_alt_format() };

    worksheet.write_string_with_format(row, COL_ORG, &org.org_label, &text_fmt)?;
    for (kind_col, counts) in
        KIND_COLS.iter().zip([&org.bzvp, &org.special, &org.adaptation])
    {
        worksheet.write_number_with_format(row, *kind_col, counts.total as f64, &num_fmt)?;
        worksheet.write_number_with_format(row, kind_col + 1, counts.finishing_today as f64, &num_fmt)?;
        worksheet.write_number_with_format(row, kind_col + 2, counts.started_today as f64, &num_fmt)?;
        worksheet.write_number_with_format(row, kind_col + 3, counts.left_today as f64, &num_fmt)?;
    }
    if let Some(note) = &org.note {
        worksheet.write_string_with_format(row, COL_NOTE, note, &note_fmt)?;
    } else {
        worksheet.write_string_with_format(row, COL_NOTE, "", &note_fmt)?;
    }
    Ok(())
}

/// Підсумок суміжного діапазону (підрозділи одного корпусу одного дня) — `SUM(range)`, як D1.
fn write_range_total(
    worksheet: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    first_data_row: u32,
    last_data_row: u32,
    label: &str,
    label_fmt: &Format,
    num_fmt: &Format,
) -> Result<(), XlsxError> {
    worksheet.write_string_with_format(row, COL_ORG, label, label_fmt)?;
    if first_data_row > last_data_row {
        for col in 1..=(COL_NOTE - 1) {
            worksheet.write_number_with_format(row, col, 0.0, num_fmt)?;
        }
        worksheet.write_string_with_format(row, COL_NOTE, "", num_fmt)?;
        return Ok(());
    }
    for col in 1..=(COL_NOTE - 1) {
        let col_letter = column_letter(col);
        let formula = format!("=SUM({col_letter}{}:{col_letter}{})", first_data_row + 1, last_data_row + 1);
        worksheet.write_formula_with_format(row, col, Formula::new(formula), num_fmt)?;
    }
    worksheet.write_string_with_format(row, COL_NOTE, "", num_fmt)?;
    Ok(())
}

/// Підсумок НЕСУМІЖНИХ рядків (підсумки дня по корпусах не йдуть підряд — між ними сидять рядки
/// підрозділів; те саме для підсумку тижня по днях) — конкретні клітинки через "+", не `SUM(range)`.
fn write_cells_total(
    worksheet: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    source_rows: &[u32],
    label: &str,
    label_fmt: &Format,
    num_fmt: &Format,
) -> Result<(), XlsxError> {
    worksheet.write_string_with_format(row, COL_ORG, label, label_fmt)?;
    for col in 1..=(COL_NOTE - 1) {
        let col_letter = column_letter(col);
        if source_rows.is_empty() {
            worksheet.write_number_with_format(row, col, 0.0, num_fmt)?;
            continue;
        }
        let formula = source_rows
            .iter()
            .map(|r| format!("{col_letter}{}", r + 1))
            .collect::<Vec<_>>()
            .join("+");
        worksheet.write_formula_with_format(row, col, Formula::new(format!("={formula}")), num_fmt)?;
    }
    worksheet.write_string_with_format(row, COL_NOTE, "", num_fmt)?;
    Ok(())
}

/// A=0..N=13 вистачає (колонок у D2 лише 14) — без загальної base-26 конвертації.
fn column_letter(col: u16) -> char {
    (b'A' + col as u8) as char
}
