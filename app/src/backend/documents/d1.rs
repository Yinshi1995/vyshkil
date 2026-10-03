//! D1 — щоденна зведена таблиця органу (05 §D1). Чиста генерація xlsx з готового
//! `repo::documents::DailyRollup` — той самий поділ, що `backend::import::*`.
//!
//! Повний тижневий файл: 7 денних аркушів + службові "початок"/"кінець" (приховані, маркери
//! для Excel 3D-формул) + "тижневий" аркуш зі `SUM(початок:кінець!C5)` по кожному рядку.
//! Усі денні аркуші мають ІДЕНТИЧНУ структуру рядків (ті самі органи, той самий порядок —
//! `repo::documents::grouped_org_ids` `ORDER BY o.short_name` гарантує це), тому 3D-формули
//! тижневого аркуша коректно підсумовують кожну клітинку через усі 7 днів.

use crate::backend::documents::style;
use crate::backend::repo::documents::{DailyRollup, OrgRollupRow};
use chrono::NaiveDate;
use rust_xlsxwriter::{Format, Formula, Worksheet, Workbook, XlsxError};

const COL_ORG: u16 = 0;
const COL_NOTE: u16 = 10;
/// (Всього, Закінчують, Почали) — перша колонка кожного виду підготовки в денному аркуші.
const KIND_COLS: [u16; 3] = [1, 4, 7];
const KIND_LABELS: [&str; 3] = ["БЗВП", "Фахова", "Адаптація"];
/// Колонки Закінчують/Почали в денному аркуші (KIND_COLS[i]+1 та KIND_COLS[i]+2) —
/// тижневий аркуш будується саме з цих координат через `SUM(початок:кінець!<col><row>)`.
const KIND_FINISHING_COLS: [u16; 3] = [2, 5, 8];
const KIND_STARTED_COLS: [u16; 3] = [3, 6, 9];

/// Розташування рядків одного денного аркуша — незмінне між днями (однакові органи в
/// однаковому порядку), тому тижневий аркуш будує формули один раз за цим макетом.
pub struct DayLayout {
    /// 0-indexed рядки органів "основної" секції (порядок = `DailyRollup::main`).
    pub main_org_rows: Vec<u32>,
    /// Рядок "ВСЬОГО" основної секції.
    pub main_total_row: u32,
    /// 0-indexed рядки органів "поза смугою" (порожній, якщо секції нема).
    pub ooz_org_rows: Vec<u32>,
    /// Рядок "РАЗОМ" секції "поза смугою" (`None`, якщо секції нема).
    pub ooz_total_row: Option<u32>,
}

fn set_column_widths(ws: &mut Worksheet) -> Result<(), XlsxError> {
    ws.set_column_width(COL_ORG, 25)?;
    for col in 1..=9u16 {
        ws.set_column_width(col, 12)?;
    }
    ws.set_column_width(COL_NOTE, 20)?;
    Ok(())
}

/// Будує один денний аркуш у `workbook`. Повертає [`DayLayout`] — макет рядків для
/// подальшого використання тижневим аркушем.
pub fn build_day_sheet(
    workbook: &mut Workbook,
    org_label: &str,
    date: NaiveDate,
    rollup: &DailyRollup,
) -> Result<DayLayout, XlsxError> {
    let sheet_name = date.format("%d.%m").to_string();
    let worksheet = workbook.add_worksheet().set_name(sheet_name)?;

    let title_fmt = style::title_format();
    let hdr_fmt = style::header_format();
    let total_label_fmt = style::total_format();
    let total_num_fmt = style::total_number_format();
    let section_fmt = style::section_format();

    set_column_widths(worksheet)?;
    worksheet.set_row_height(0, 30)?;
    worksheet.set_row_height(1, 25)?;
    worksheet.set_row_height(2, 25)?;

    worksheet.merge_range(
        0,
        COL_ORG,
        0,
        COL_NOTE,
        &format!(
            "{org_label}. Зведена таблиця підготовки станом на {}",
            date.format("%d.%m.%Y")
        ),
        &title_fmt,
    )?;

    worksheet.merge_range(1, COL_ORG, 2, COL_ORG, "Військові частини", &hdr_fmt)?;
    worksheet.merge_range(1, COL_NOTE, 2, COL_NOTE, "Примітка", &hdr_fmt)?;
    for (kind_col, label) in KIND_COLS.iter().zip(KIND_LABELS.iter()) {
        worksheet.merge_range(1, *kind_col, 1, kind_col + 2, label, &hdr_fmt)?;
        worksheet.write_string_with_format(2, *kind_col, "Всього", &hdr_fmt)?;
        worksheet.write_string_with_format(2, kind_col + 1, "Закінчують сьогодні", &hdr_fmt)?;
        worksheet.write_string_with_format(2, kind_col + 2, "Почали сьогодні", &hdr_fmt)?;
    }

    let total_row: u32 = 3;
    let mut row: u32 = total_row + 1;

    let first_main_row = row;
    let mut main_org_rows = Vec::with_capacity(rollup.main.len());
    for (idx, org) in rollup.main.iter().enumerate() {
        main_org_rows.push(row);
        write_org_row(worksheet, row, org, idx % 2 == 0)?;
        row += 1;
    }
    let last_main_row = row.saturating_sub(1);
    write_total_row(worksheet, total_row, first_main_row, last_main_row, "ВСЬОГО", &total_label_fmt, &total_num_fmt)?;

    let mut ooz_org_rows = Vec::new();
    let mut ooz_total_row = None;

    if !rollup.out_of_zone.is_empty() {
        worksheet.merge_range(
            row, COL_ORG, row, COL_NOTE,
            &format!(
                "Підрозділи, які знаходяться в штатному підпорядкуванні {org_label}, \
                 але не виконують бойові завдання в смузі оборони УВ (с) \"Південь\""
            ),
            &section_fmt,
        )?;
        row += 1;
        let first_ooz_row = row;
        for (idx, org) in rollup.out_of_zone.iter().enumerate() {
            ooz_org_rows.push(row);
            write_org_row(worksheet, row, org, idx % 2 == 0)?;
            row += 1;
        }
        let last_ooz_row = row.saturating_sub(1);
        let razom_row = row;
        write_total_row(worksheet, razom_row, first_ooz_row, last_ooz_row, "РАЗОМ", &total_label_fmt, &total_num_fmt)?;
        ooz_total_row = Some(razom_row);
    }

    worksheet.set_freeze_panes(3, 1)?;

    Ok(DayLayout { main_org_rows, main_total_row: total_row, ooz_org_rows, ooz_total_row })
}

/// Будує повний тижневий D1-файл:
/// 1. Прихований аркуш "початок" (маркер початку 3D-діапазону).
/// 2. 7 денних аркушів (`build_day_sheet`).
/// 3. Прихований аркуш "кінець" (маркер кінця 3D-діапазону).
/// 4. Аркуш "тижневий" — "Закінчили/Почали протягом тижня" з `SUM(початок:кінець!...)`.
///
/// `days`: пари (дата, rollup) для кожного дня тижня пн–нд.
/// Порядок і кількість органів МУСЯТЬ збігатися між днями — `repo::documents::grouped_org_ids`
/// з `ORDER BY o.short_name` гарантує це для стандартного виклику.
pub fn build_weekly_file(
    workbook: &mut Workbook,
    org_label: &str,
    week_start: NaiveDate,
    days: &[(NaiveDate, DailyRollup)],
) -> Result<(), XlsxError> {
    build_marker_sheet(workbook, "початок")?;

    let mut layout: Option<DayLayout> = None;
    for (date, rollup) in days {
        let l = build_day_sheet(workbook, org_label, *date, rollup)?;
        if layout.is_none() {
            layout = Some(l);
        }
    }

    build_marker_sheet(workbook, "кінець")?;

    // "початок" (index 0) is hidden; make the first day sheet active so Excel opens there
    if let Ok(ws) = workbook.worksheet_from_index(1) {
        ws.set_active(true);
    }

    if let Some(layout) = layout {
        let week_end = week_start + chrono::Duration::days(6);
        let first_rollup = days.first().map(|(_, r)| r);
        if let Some(rollup) = first_rollup {
            build_weekly_sheet(workbook, org_label, week_start, week_end, rollup, &layout)?;
        }
    }

    Ok(())
}

/// Порожній прихований аркуш-маркер для 3D-діапазону Excel.
fn build_marker_sheet(workbook: &mut Workbook, name: &str) -> Result<(), XlsxError> {
    let ws = workbook.add_worksheet().set_name(name)?;
    ws.set_hidden(true);
    Ok(())
}

/// "Тижневий" аркуш: Закінчили/Почали за кожний вид підготовки — формулами
/// `=SUM(початок:кінець!{col}{row+1})`, де `col`/`row` — координати відповідної клітинки
/// в ДЕННИХ аркушах (ті самі для кожного дня — `DayLayout` від першого дня вистачає).
fn build_weekly_sheet(
    workbook: &mut Workbook,
    org_label: &str,
    week_start: NaiveDate,
    week_end: NaiveDate,
    first_rollup: &DailyRollup,
    layout: &DayLayout,
) -> Result<(), XlsxError> {
    /// Перша колонка кожного виду в тижневому аркуші (2 підколонки: Закінчили, Почали).
    const WEEKLY_KIND_COLS: [u16; 3] = [1, 3, 5];
    const WEEKLY_COL_NOTE: u16 = 7;

    let ws = workbook.add_worksheet().set_name("тижневий")?;

    let title_fmt = style::title_format();
    let hdr_fmt = style::header_format();
    let total_label_fmt = style::total_format();
    let total_num_fmt = style::total_number_format();
    let section_fmt = style::section_format();

    ws.set_column_width(0, 25)?;
    for col in 1..=6u16 {
        ws.set_column_width(col, 14)?;
    }
    ws.set_column_width(WEEKLY_COL_NOTE, 20)?;
    ws.set_row_height(0, 30)?;
    ws.set_row_height(1, 25)?;
    ws.set_row_height(2, 25)?;

    ws.merge_range(
        0, 0, 0, WEEKLY_COL_NOTE,
        &format!(
            "{org_label}. Підготовка за тиждень {} – {}",
            week_start.format("%d.%m.%Y"),
            week_end.format("%d.%m.%Y")
        ),
        &title_fmt,
    )?;

    ws.merge_range(1, 0, 2, 0, "Військові частини", &hdr_fmt)?;
    ws.merge_range(1, WEEKLY_COL_NOTE, 2, WEEKLY_COL_NOTE, "Примітка", &hdr_fmt)?;
    for (wk_col, label) in WEEKLY_KIND_COLS.iter().zip(KIND_LABELS.iter()) {
        ws.merge_range(1, *wk_col, 1, wk_col + 1, label, &hdr_fmt)?;
        ws.write_string_with_format(2, *wk_col, "Закінчили за тиждень", &hdr_fmt)?;
        ws.write_string_with_format(2, wk_col + 1, "Почали за тиждень", &hdr_fmt)?;
    }

    // ВСЬОГО рядок (row 3).
    let total_row: u32 = 3;
    ws.write_string_with_format(total_row, 0, "ВСЬОГО", &total_label_fmt)?;
    for (i, wk_col) in WEEKLY_KIND_COLS.iter().enumerate() {
        write_3d_sum_fmt(ws, total_row, *wk_col, KIND_FINISHING_COLS[i], layout.main_total_row, &total_num_fmt)?;
        write_3d_sum_fmt(ws, total_row, wk_col + 1, KIND_STARTED_COLS[i], layout.main_total_row, &total_num_fmt)?;
    }

    // Рядки основних органів.
    for (idx, org) in first_rollup.main.iter().enumerate() {
        if let Some(&daily_row) = layout.main_org_rows.get(idx) {
            let wk_row = total_row + 1 + idx as u32;
            let (text_fmt, num_fmt) = row_formats(idx);
            ws.write_string_with_format(wk_row, 0, &org.org_label, &text_fmt)?;
            for (i, wk_col) in WEEKLY_KIND_COLS.iter().enumerate() {
                write_3d_sum_fmt(ws, wk_row, *wk_col, KIND_FINISHING_COLS[i], daily_row, &num_fmt)?;
                write_3d_sum_fmt(ws, wk_row, wk_col + 1, KIND_STARTED_COLS[i], daily_row, &num_fmt)?;
            }
        }
    }

    // Секція "поза смугою" (якщо є).
    if !first_rollup.out_of_zone.is_empty() {
        let ooz_header_row = total_row + 1 + first_rollup.main.len() as u32;
        ws.merge_range(
            ooz_header_row, 0, ooz_header_row, WEEKLY_COL_NOTE,
            &format!(
                "Підрозділи, які знаходяться в штатному підпорядкуванні {org_label}, \
                 але не виконують бойові завдання в смузі оборони УВ (с) \"Південь\""
            ),
            &section_fmt,
        )?;

        for (idx, org) in first_rollup.out_of_zone.iter().enumerate() {
            if let Some(&daily_row) = layout.ooz_org_rows.get(idx) {
                let wk_row = ooz_header_row + 1 + idx as u32;
                let (text_fmt, num_fmt) = row_formats(idx);
                ws.write_string_with_format(wk_row, 0, &org.org_label, &text_fmt)?;
                for (i, wk_col) in WEEKLY_KIND_COLS.iter().enumerate() {
                    write_3d_sum_fmt(ws, wk_row, *wk_col, KIND_FINISHING_COLS[i], daily_row, &num_fmt)?;
                    write_3d_sum_fmt(ws, wk_row, wk_col + 1, KIND_STARTED_COLS[i], daily_row, &num_fmt)?;
                }
            }
        }

        let ooz_total_row_wk = ooz_header_row + 1 + first_rollup.out_of_zone.len() as u32;
        if let Some(daily_ooz_total) = layout.ooz_total_row {
            ws.write_string_with_format(ooz_total_row_wk, 0, "РАЗОМ", &total_label_fmt)?;
            for (i, wk_col) in WEEKLY_KIND_COLS.iter().enumerate() {
                write_3d_sum_fmt(ws, ooz_total_row_wk, *wk_col, KIND_FINISHING_COLS[i], daily_ooz_total, &total_num_fmt)?;
                write_3d_sum_fmt(ws, ooz_total_row_wk, wk_col + 1, KIND_STARTED_COLS[i], daily_ooz_total, &total_num_fmt)?;
            }
        }
    }

    ws.set_freeze_panes(3, 1)?;
    Ok(())
}

fn row_formats(idx: usize) -> (Format, Format) {
    if idx.is_multiple_of(2) {
        (style::data_format(), style::data_number_format())
    } else {
        (style::data_alt_format(), style::data_alt_number_format())
    }
}

fn write_3d_sum_fmt(
    ws: &mut Worksheet,
    wk_row: u32,
    wk_col: u16,
    daily_col: u16,
    daily_row: u32,
    fmt: &Format,
) -> Result<(), XlsxError> {
    ws.write_formula_with_format(wk_row, wk_col, Formula::new(three_d_formula(daily_col, daily_row)), fmt)?;
    Ok(())
}

fn three_d_formula(daily_col: u16, daily_row: u32) -> String {
    format!("=SUM(початок:кінець!{}{})", column_letter(daily_col), daily_row + 1)
}

fn write_org_row(worksheet: &mut Worksheet, row: u32, org: &OrgRollupRow, even: bool) -> Result<(), XlsxError> {
    let (text_fmt, num_fmt) = if even {
        (style::data_format(), style::data_number_format())
    } else {
        (style::data_alt_format(), style::data_alt_number_format())
    };
    let note_fmt = if even { style::muted_format() } else {
        style::data_alt_format()
    };

    worksheet.write_string_with_format(row, COL_ORG, &org.org_label, &text_fmt)?;
    for (kind_col, counts) in
        KIND_COLS.iter().zip([&org.bzvp, &org.special, &org.adaptation])
    {
        worksheet.write_number_with_format(row, *kind_col, counts.total as f64, &num_fmt)?;
        worksheet.write_number_with_format(row, kind_col + 1, counts.finishing_today as f64, &num_fmt)?;
        worksheet.write_number_with_format(row, kind_col + 2, counts.started_today as f64, &num_fmt)?;
    }
    if let Some(note) = &org.note {
        worksheet.write_string_with_format(row, COL_NOTE, note, &note_fmt)?;
    } else {
        worksheet.write_string_with_format(row, COL_NOTE, "", &note_fmt)?;
    }
    Ok(())
}

fn write_total_row(
    worksheet: &mut Worksheet,
    row: u32,
    first_data_row: u32,
    last_data_row: u32,
    label: &str,
    label_fmt: &Format,
    num_fmt: &Format,
) -> Result<(), XlsxError> {
    worksheet.write_string_with_format(row, COL_ORG, label, label_fmt)?;
    if first_data_row > last_data_row {
        for col in 1..=9u16 {
            worksheet.write_number_with_format(row, col, 0.0, num_fmt)?;
        }
        worksheet.write_string_with_format(row, COL_NOTE, "", num_fmt)?;
        return Ok(());
    }
    for col in 1..=9u16 {
        let col_letter = column_letter(col);
        let formula =
            format!("=SUM({col_letter}{}:{col_letter}{})", first_data_row + 1, last_data_row + 1);
        worksheet.write_formula_with_format(row, col, Formula::new(formula), num_fmt)?;
    }
    worksheet.write_string_with_format(row, COL_NOTE, "", num_fmt)?;
    Ok(())
}

/// A=0..K=10 вистачає (колонок у D1 не більше 15).
fn column_letter(col: u16) -> char {
    (b'A' + col as u8) as char
}
