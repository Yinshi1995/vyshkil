//! D1 — щоденна зведена таблиця органу (05 §D1). Чиста генерація xlsx з готового
//! `repo::documents::DailyRollup` (жодних SQL-викликів тут) — той самий поділ, що
//! `backend::import::*` (структура) / `repo::imports_*` (SQL): тут структура, дані вже прийшли.
//!
//! **Перший вертикальний зріз — лише ОДИН денний аркуш** (не весь тижневий файл зі службовими
//! аркушами "початок"/"кінець"/"тижневий" і прихованими колонками M-O для тижневих формул —
//! наступний крок, коли денний аркуш перевірений на реальному використанні).

use crate::backend::repo::documents::{DailyRollup, OrgRollupRow};
use chrono::NaiveDate;
use rust_xlsxwriter::{Format, FormatAlign, Formula, Workbook, XlsxError};

const COL_ORG: u16 = 0;
const COL_NOTE: u16 = 10;
/// (Всього, Закінчують сьогодні, почали сьогодні) — перша колонка кожного виду підготовки.
const KIND_COLS: [u16; 3] = [1, 4, 7];
const KIND_LABELS: [&str; 3] = ["БЗВП", "Фахова", "Адаптація"];

/// Будує один денний аркуш у вже створеному `workbook` (щоб згодом додавати інші дні того самого
/// тижня в один файл без переписування заголовків). Повертає індекс аркуша.
pub fn build_day_sheet(
    workbook: &mut Workbook,
    org_label: &str,
    date: NaiveDate,
    rollup: &DailyRollup,
) -> Result<(), XlsxError> {
    let sheet_name = date.format("%d.%m").to_string();
    let worksheet = workbook.add_worksheet().set_name(sheet_name)?;

    let title_format = Format::new().set_bold().set_align(FormatAlign::Center);
    let header_format = Format::new().set_bold().set_align(FormatAlign::Center).set_text_wrap();
    let total_format = Format::new().set_bold();

    worksheet.merge_range(
        0,
        COL_ORG,
        0,
        COL_NOTE,
        &format!(
            "{org_label}. Зведена таблиця підготовки станом на {}",
            date.format("%d.%m.%Y")
        ),
        &title_format,
    )?;

    worksheet.merge_range(1, COL_ORG, 2, COL_ORG, "Військові частини", &header_format)?;
    worksheet.merge_range(1, COL_NOTE, 2, COL_NOTE, "Примітка", &header_format)?;
    for (kind_col, label) in KIND_COLS.iter().zip(KIND_LABELS.iter()) {
        worksheet.merge_range(1, *kind_col, 1, kind_col + 2, label, &header_format)?;
        worksheet.write_string_with_format(2, *kind_col, "Всього", &header_format)?;
        worksheet.write_string_with_format(2, kind_col + 1, "Закінчують сьогодні", &header_format)?;
        worksheet.write_string_with_format(2, kind_col + 2, "почали сьогодні", &header_format)?;
    }

    let total_row = 3;
    let mut row = total_row + 1;

    let first_main_row = row;
    for org in &rollup.main {
        write_org_row(worksheet, row, org)?;
        row += 1;
    }
    let last_main_row = row.saturating_sub(1);
    write_total_row(worksheet, total_row, first_main_row, last_main_row, "ВСЬОГО", &total_format)?;

    if !rollup.out_of_zone.is_empty() {
        worksheet.merge_range(
            row,
            COL_ORG,
            row,
            COL_NOTE,
            &format!(
                "Підрозділи, які знаходяться в штатному підпорядкуванні {org_label}, \
                 але не виконують бойові завдання в смузі оборони УВ (с) \"Південь\""
            ),
            &header_format,
        )?;
        row += 1;
        let first_ooz_row = row;
        for org in &rollup.out_of_zone {
            write_org_row(worksheet, row, org)?;
            row += 1;
        }
        let last_ooz_row = row.saturating_sub(1);
        write_total_row(worksheet, row, first_ooz_row, last_ooz_row, "РАЗОМ", &total_format)?;
    }

    worksheet.set_freeze_panes(3, 1)?;
    Ok(())
}

fn write_org_row(
    worksheet: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    org: &OrgRollupRow,
) -> Result<(), XlsxError> {
    worksheet.write_string(row, COL_ORG, &org.org_label)?;
    for (kind_col, counts) in
        KIND_COLS.iter().zip([&org.bzvp, &org.special, &org.adaptation].into_iter())
    {
        worksheet.write_number(row, *kind_col, counts.total as f64)?;
        worksheet.write_number(row, kind_col + 1, counts.finishing_today as f64)?;
        worksheet.write_number(row, kind_col + 2, counts.started_today as f64)?;
    }
    if let Some(note) = &org.note {
        worksheet.write_string(row, COL_NOTE, note)?;
    }
    Ok(())
}

/// Підсумковий рядок формулами `SUM(...)` (05 §D1: "рядок ВСЬОГО з формулами SUM" — люди потім
/// правлять дані руками в Excel, формули мають перераховуватись самі, а не застигнути числом).
fn write_total_row(
    worksheet: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    first_data_row: u32,
    last_data_row: u32,
    label: &str,
    format: &Format,
) -> Result<(), XlsxError> {
    worksheet.write_string_with_format(row, COL_ORG, label, format)?;
    if first_data_row > last_data_row {
        // Немає жодного рядка в секції — SUM() від'ємного діапазону не побудувати, нулі напряму.
        for col in 1..=9u16 {
            worksheet.write_number_with_format(row, col, 0.0, format)?;
        }
        return Ok(());
    }
    for col in 1..=9u16 {
        let col_letter = column_letter(col);
        let formula = format!("=SUM({col_letter}{}:{col_letter}{})", first_data_row + 1, last_data_row + 1);
        worksheet.write_formula_with_format(row, col, Formula::new(formula), format)?;
    }
    Ok(())
}

/// A=0..Z=25 вистачає (колонок у D1 лише 11) — без загальної base-26 конвертації.
fn column_letter(col: u16) -> char {
    (b'A' + col as u8) as char
}
