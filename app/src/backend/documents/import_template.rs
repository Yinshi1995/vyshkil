//! Генератор типового зразка для імпорту xlsx (02 §1 — колонки сітки).
//! Отримує довідникові дані (`ImportEnums`) з `repo::dictionaries::export_enums_for_template`,
//! будує xlsx із двома аркушами: "Дані" (основна форма з data validation) та прихований
//! "Довідники" (списки для випадаючих меню).

use crate::backend::repo::dictionaries::ImportEnums;
use rust_xlsxwriter::{
    DataValidation, DataValidationRule, Format, FormatBorder, Formula, Workbook, XlsxError,
};

use super::style;

fn instruction_format() -> Format {
    Format::new()
        .set_font_name(style::FONT_NAME)
        .set_font_size(9)
        .set_italic()
        .set_font_color(style::TEXT_MUTED)
}

struct RefCol {
    col: u16,
    count: usize,
}

pub fn generate(enums: ImportEnums) -> Result<Vec<u8>, XlsxError> {
    let mut workbook = Workbook::new();

    // ----- Sheet "Довідники" (hidden, holds enum lists for data validation) -----
    let ref_sheet = workbook.add_worksheet();
    ref_sheet.set_name("Довідники")?;

    let ref_header = style::header_format();

    let mut ref_cols: Vec<RefCol> = Vec::new();

    let lists: &[(&str, &[String])] = &[
        ("Вид підготовки", &enums.training_kinds),
        ("Програма БЗВП", &enums.bzvp_programs),
        ("ВОС", &enums.vos_codes),
        ("Посада", &enums.positions),
        ("Курс", &enums.courses),
        ("ОВТ", &enums.equipment),
        ("Причини вибуття", &enums.attrition_reasons),
    ];

    for (col_idx, (title, values)) in lists.iter().enumerate() {
        let col = col_idx as u16;
        ref_sheet.write_string_with_format(0, col, *title, &ref_header)?;
        ref_sheet.set_column_width(col, 30)?;
        for (row, val) in values.iter().enumerate() {
            ref_sheet.write_string((row + 1) as u32, col, val)?;
        }
        ref_cols.push(RefCol {
            col,
            count: values.len(),
        });
    }

    // Org codes only (name excluded — ДСК: номер і назва не на одному екрані)
    let org_col: u16 = lists.len() as u16;
    ref_sheet.write_string_with_format(0, org_col, "Код ВЧ", &ref_header)?;
    ref_sheet.set_column_width(org_col, 12)?;
    let org_count = enums.org_codes.len();
    for (row, code) in enums.org_codes.iter().enumerate() {
        ref_sheet.write_string((row + 1) as u32, org_col, code)?;
    }

    ref_sheet.set_hidden(true);

    // Helper: create formula reference to a column in the hidden sheet
    let make_ref = |col: u16, count: usize| -> String {
        let col_letter = col_to_letter(col);
        format!(
            "=Довідники!${col_letter}$2:${col_letter}${}",
            count + 1
        )
    };

    // ----- Main "Дані" sheet -----
    let data_sheet = workbook.add_worksheet();
    data_sheet.set_name("Дані")?;

    let header_fmt = style::header_format();
    let instr_fmt = instruction_format();

    let columns = [
        ("Військова частина\n(код, напр. А4076)", 18.0),
        ("Вид підготовки", 16.0),
        ("Програма БЗВП\n(якщо БЗВП)", 14.0),
        ("ВОС (код — назва)", 22.0),
        ("Посада", 22.0),
        ("Курс\n(для інструкторів)", 14.0),
        ("ОВТ", 18.0),
        ("Місце проведення\n(населений пункт)", 20.0),
        ("Термін з\n(дд.мм.рррр)", 14.0),
        ("Термін по\n(дд.мм.рррр)", 14.0),
        ("К-сть план", 10.0),
        ("Прибуло", 10.0),
        ("Навчаються", 10.0),
        ("Вибуло", 10.0),
        ("Причина вибуття", 16.0),
        ("Організатор", 18.0),
        ("Підстава (№ розпор.)", 18.0),
        ("Примітка", 20.0),
    ];

    for (col, (name, width)) in columns.iter().enumerate() {
        let col = col as u16;
        data_sheet.write_string_with_format(0, col, *name, &header_fmt)?;
        data_sheet.set_column_width(col, *width)?;
    }

    // Row 1: instructions
    let instructions = [
        "А4076 / Т1234",
        "обрати зі списку",
        "лише для БЗВП",
        "обрати або ввести код",
        "обрати зі списку",
        "КІБР / КПК / ...",
        "обрати зі списку",
        "м. Верхньодніпровськ",
        "01.10.2026",
        "30.11.2026",
        "число",
        "число",
        "число",
        "число",
        "обрати зі списку",
        "вільний текст",
        "№123 від 01.10.2026",
        "вільний текст",
    ];
    for (col, hint) in instructions.iter().enumerate() {
        data_sheet.write_string_with_format(1, col as u16, *hint, &instr_fmt)?;
    }

    data_sheet.set_freeze_panes(2, 0)?;

    let max_row = 500u32;

    // Data validations via formula references to hidden "Довідники" sheet

    // Col 0: Org code
    if org_count > 0 {
        let org_letter = col_to_letter(org_col);
        let formula_str = format!(
            "=Довідники!${org_letter}$2:${org_letter}${}",
            org_count + 1
        );
        let dv = DataValidation::new()
            .allow_list_formula(Formula::new(&formula_str));
        data_sheet.add_data_validation(2, 0, max_row, 0, &dv)?;
    }

    // (data_col, ref_col_index) pairs — which data column gets which reference list
    let validations: &[(u16, usize)] = &[
        (1, 0),  // Вид підготовки
        (2, 1),  // Програма БЗВП
        (3, 2),  // ВОС
        (4, 3),  // Посада
        (5, 4),  // Курс
        (6, 5),  // ОВТ
        (14, 6), // Причина вибуття
    ];

    for &(data_col, ref_idx) in validations {
        let rc = &ref_cols[ref_idx];
        if rc.count == 0 {
            continue;
        }
        let formula_str = make_ref(rc.col, rc.count);
        let dv = DataValidation::new()
            .allow_list_formula(Formula::new(&formula_str));
        data_sheet.add_data_validation(2, data_col, max_row, data_col, &dv)?;
    }

    // Cols 8-9: date format pre-fill
    let date_fmt = Format::new()
        .set_font_name(style::FONT_NAME)
        .set_font_size(10)
        .set_num_format("dd.mm.yyyy")
        .set_border(FormatBorder::Thin)
        .set_border_color(style::BORDER_THIN);
    for col in [8u16, 9] {
        for row in 2..=20u32 {
            data_sheet.write_blank(row, col, &date_fmt)?;
        }
    }

    // Cols 10-13: whole number validation
    for col in 10..=13u16 {
        let dv = DataValidation::new()
            .allow_whole_number(DataValidationRule::Between(0, 10000));
        data_sheet.add_data_validation(2, col, max_row, col, &dv)?;
    }

    data_sheet.set_tab_color(style::BG_DARK_OLIVE);

    workbook.save_to_buffer()
}

fn col_to_letter(col: u16) -> String {
    let mut result = String::new();
    let mut n = col as u32;
    loop {
        result.insert(0, (b'A' + (n % 26) as u8) as char);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    result
}
