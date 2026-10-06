//! Екстрактор "Терміни" (03 §2-3) — П'ЯТИЙ і останній тип файлу, найважча структура за
//! `backend/import/CLAUDE.md`: ОДИН заголовок (рядки 6-7, дворівневий — як у БпС), під ним ТРИ
//! ПАРАЛЕЛЬНІ незалежні списки в СПІЛЬНОМУ діапазоні рядків одного підрозділу — БЗВП (C-F),
//! Фахова підготовка (G-J), Адаптація (K-M). У реальному файлі підрозділ "241 обр ТрО" має 8
//! рядків БЗВП і 25 рядків Фахової в тому самому діапазоні рядків (10-34) — списки РІЗНОЇ
//! довжини, вирівняні по рядку лише випадково, не за змістом. Кожен список перевіряється
//! НЕЗАЛЕЖНО (є число в "Кількість людей" цього блоку — є рядок для НЬОГО), а не пара-в-пару.
//!
//! Назва частини (колонка B) вертикально злита на всі рядки блоку одного підрозділу (як номер А)
//! — заповнена лише в ПЕРШОМУ рядку блоку, далі порожня (`None`) до кінця блоку; носимо вперед
//! (той самий патерн, що й `ivs::parse_courses` для колонки "Підрозділ" другої таблиці). Рядок
//! "Всього за <частину>:" (і "Всього за 17 АК:" на самому початку) — підсумок з РЕАЛЬНИМИ числами
//! в тих самих колонках "Кількість людей" (не порожніми, як типові рядки-підсумки в інших типах!)
//! — тому явна перевірка на "всього" в колонці A ОБОВ'ЯЗКОВА, інакше підсумок потрапить як фантомний
//! рядок групи.
//!
//! Термін в одній клітинці ("18.08-09.10", "07.08 - 26.09") — той самий формат, що вже вміє
//! `domain::dates::parse_maybe_range` (02 §4) через `GroupFormRow.planned_start_raw` у сітці;
//! екстрактор віддає сирий текст як є, дату не парсить сам (те саме, що й "стажування" в ІВС).

use calamine::{Data, DataType, Reader, Xlsx};
use std::io::Cursor;

use crate::domain::normalize::normalize;

#[derive(Debug, Clone)]
pub struct RawBzvpRow {
    pub row_number: u32,
    pub org_raw: String,
    pub count_raw: String,
    pub term_raw: String,
    pub place_raw: String,
    pub distribution_raw: String,
}

#[derive(Debug, Clone)]
pub struct RawSpecialRow {
    pub row_number: u32,
    pub org_raw: String,
    pub count_raw: String,
    pub term_raw: String,
    pub place_raw: String,
    /// "ВОС 878\nБойовий медик взводу" — код виймається пізніше в `repo::imports_terminy`
    /// (`extract_vos_code`), тут лишається сирим текстом (той самий принцип, що й усі "raw"-поля).
    pub specialty_raw: String,
}

#[derive(Debug, Clone)]
pub struct RawAdaptRow {
    pub row_number: u32,
    pub org_raw: String,
    pub count_raw: String,
    pub term_raw: String,
    pub distribution_raw: String,
}

#[derive(Debug, Clone, Default)]
pub struct TerminyExtract {
    pub bzvp: Vec<RawBzvpRow>,
    pub special: Vec<RawSpecialRow>,
    pub adapt: Vec<RawAdaptRow>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn cell_text(row: &[Data], idx: usize) -> String {
    row.get(idx).map(|c| c.to_string()).unwrap_or_default().trim().to_string()
}

fn positive_count(row: &[Data], idx: usize) -> Option<String> {
    let raw = cell_text(row, idx);
    let n: i64 = raw.trim().parse().ok()?;
    (n > 0).then_some(raw)
}

/// Рядок-заголовок фіксований (03 §2: один реальний файл, жорстка структура, на відміну від
/// БпС де довелось шукати колонки за назвою) — шукаємо лише РЯДОК заголовка ("№ з/п" у колонці A),
/// самі колонки (0-12: А-М) далі беремо за фіксованою позицією.
fn detect_header_row(rows: &[Vec<Data>]) -> Option<usize> {
    rows.iter().take(10).position(|row| {
        let first = row.first().and_then(|c| c.get_string()).map(normalize).unwrap_or_default();
        first.contains("з/п")
    })
}

fn parse_sheet(rows: &[Vec<Data>]) -> TerminyExtract {
    let Some(header_row) = detect_header_row(rows) else { return TerminyExtract::default() };
    let data_start = header_row + 2; // +2: два рядки заголовка (група + підгрупа).

    let mut out = TerminyExtract::default();
    let mut current_org = String::new();

    for (idx, row) in rows.iter().enumerate().skip(data_start) {
        let a_text = cell_text(row, 0);
        if normalize(&a_text).contains("всього") {
            continue; // "Всього за …:" -- реальні числа в тих самих колонках, не дані.
        }
        let b_text = cell_text(row, 1);
        if !b_text.is_empty() {
            current_org = b_text; // перший рядок блоку підрозділу -- носимо далі для решти блоку.
        }
        if current_org.is_empty() {
            continue; // до першого підрозділу (не мало б статись після detect_header_row).
        }

        let excel_row = (idx + 1) as u32;
        if let Some(count_raw) = positive_count(row, 2) {
            out.bzvp.push(RawBzvpRow {
                row_number: excel_row,
                org_raw: current_org.clone(),
                count_raw,
                term_raw: cell_text(row, 3),
                place_raw: cell_text(row, 4),
                distribution_raw: cell_text(row, 5),
            });
        }
        if let Some(count_raw) = positive_count(row, 6) {
            out.special.push(RawSpecialRow {
                row_number: excel_row,
                org_raw: current_org.clone(),
                count_raw,
                term_raw: cell_text(row, 7),
                place_raw: cell_text(row, 8),
                specialty_raw: cell_text(row, 9),
            });
        }
        if let Some(count_raw) = positive_count(row, 10) {
            out.adapt.push(RawAdaptRow {
                row_number: excel_row,
                org_raw: current_org.clone(),
                count_raw,
                term_raw: cell_text(row, 11),
                distribution_raw: cell_text(row, 12),
            });
        }
    }
    out
}

/// "ВОС 878\nБойовий медик взводу" → "878". Формат нежорсткий (без "ВОС" або без цифр — `None`,
/// не падає всім імпортом).
pub fn extract_vos_code(specialty_raw: &str) -> Option<String> {
    let norm = specialty_raw.to_lowercase();
    let idx = norm.find("вос")?;
    let after = &specialty_raw[idx + "вос".len()..];
    let digits: String = after.chars().skip_while(|c| c.is_whitespace()).take_while(|c| c.is_ascii_digit()).collect();
    (!digits.is_empty()).then_some(digits)
}

pub fn extract(bytes: &[u8]) -> Result<TerminyExtract, ParseError> {
    let mut workbook: Xlsx<_> = Xlsx::new(Cursor::new(bytes.to_vec()))
        .map_err(|e| ParseError(format!("не вдалось відкрити xlsx: {e}")))?;

    let mut out = TerminyExtract::default();
    for name in workbook.sheet_names().to_vec() {
        let range = workbook
            .worksheet_range(&name)
            .map_err(|e| ParseError(format!("аркуш \"{name}\": {e}")))?;
        let rows: Vec<Vec<Data>> = range.rows().map(|r| r.to_vec()).collect();
        let sheet_extract = parse_sheet(&rows);
        out.bzvp.extend(sheet_extract.bzvp);
        out.special.extend(sheet_extract.special);
        out.adapt.extend(sheet_extract.adapt);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_vos_code_with_newline_title() {
        assert_eq!(extract_vos_code("ВОС 878\nБойовий медик взводу"), Some("878".to_string()));
    }

    #[test]
    fn extracts_vos_code_with_space() {
        assert_eq!(extract_vos_code("ВОС 216 Зовнішній пілот"), Some("216".to_string()));
    }

    #[test]
    fn no_vos_marker_returns_none() {
        assert_eq!(extract_vos_code("Кухар"), None);
    }
}
