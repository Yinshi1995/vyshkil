//! Екстрактор архіву ВЧ (Етап 6, `source_files/Дельта/…/ОблікФаховоїПідготовкиВЧ.xlsx`/
//! `…АК.xlsx`, аркуш "Записи") — **лише реальні дані** (рішення користувача, сесія Етапу 6):
//! `docs/source-analysis.md` §1 позначає більшість `Дельта/` як синтетично згенеровану (ВОС/ОВТ/
//! спеціальність не узгоджені) — придатний для переносу лише файл з живими даними
//! (`20 АК/ОблікФаховоїПідготовкиАК.xlsx`, 17 овмбр). Формат — однорівневий заголовок, той самий
//! стиль, що й "Фах" (03 §2-3), АЛЕ дати вже справжні `datetime` (не текст, без потреби в `as_of`)
//! і "Місце проведення" — вільний географічний текст (населений пункт), не назва частини/майданчика
//! — резолюція `training_site` тут ЧАСТО не спрацює, і це очікувано (нерозпізнане — на ручне
//! підтвердження в сітці, той самий принцип 03 §5).

use calamine::{Data, DataType, Reader, Xlsx};
use chrono::NaiveDate;
use std::io::Cursor;

use crate::domain::normalize::normalize;

#[derive(Debug, Clone)]
pub struct RawVchArchiveRow {
    pub row_number: u32,
    pub org_raw: String,
    pub place_raw: String,
    pub specialty_raw: String,
    pub vos_raw: String,
    pub equipment_raw: String,
    pub start_raw: Option<NaiveDate>,
    pub end_raw: Option<NaiveDate>,
    pub planned_raw: String,
    pub actual_raw: String,
    pub note_raw: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Col {
    Org,
    Place,
    Specialty,
    Vos,
    Equipment,
    Start,
    End,
    Planned,
    Actual,
    Note,
}

const COL_SYNONYMS: &[(Col, &[&str])] = &[
    (Col::Org, &["військова частина"]),
    (Col::Place, &["місце проведення"]),
    (Col::Specialty, &["спеціальність"]),
    (Col::Vos, &["вос"]),
    (Col::Equipment, &["овт"]),
    (Col::Start, &["термін з"]),
    (Col::End, &["термін по"]),
    (Col::Planned, &["план"]),
    (Col::Actual, &["фактично навчається"]),
    (Col::Note, &["примітка"]),
];

fn match_col(header_cell: &str) -> Option<Col> {
    let norm = normalize(header_cell);
    if norm.is_empty() {
        return None;
    }
    COL_SYNONYMS.iter().find(|(_, syns)| syns.iter().any(|s| norm.contains(s))).map(|(c, _)| *c)
}

fn detect_header(rows: &[Vec<Data>]) -> Option<(usize, Vec<(usize, Col)>)> {
    for (r, row) in rows.iter().take(10).enumerate() {
        let mapping: Vec<(usize, Col)> = row
            .iter()
            .enumerate()
            .filter_map(|(c, cell)| cell.get_string().and_then(match_col).map(|col| (c, col)))
            .collect();
        if mapping.len() >= 7 {
            return Some((r, mapping));
        }
    }
    None
}

fn cell_text(row: &[Data], idx: usize) -> String {
    row.get(idx).map(|c| c.to_string()).unwrap_or_default().trim().to_string()
}

fn cell_date(row: &[Data], idx: usize) -> Option<NaiveDate> {
    row.get(idx)?.as_date()
}

fn parse_sheet(rows: Vec<Vec<Data>>) -> Vec<RawVchArchiveRow> {
    let Some((header_idx, mapping)) = detect_header(&rows) else { return Vec::new() };
    let col = |c: Col| mapping.iter().find(|(_, mc)| *mc == c).map(|(i, _)| *i);
    let Some(org_i) = col(Col::Org) else { return Vec::new() };
    let place_i = col(Col::Place);
    let specialty_i = col(Col::Specialty);
    let vos_i = col(Col::Vos);
    let equipment_i = col(Col::Equipment);
    let start_i = col(Col::Start);
    let end_i = col(Col::End);
    let planned_i = col(Col::Planned);
    let actual_i = col(Col::Actual);
    let note_i = col(Col::Note);

    let mut out = Vec::new();
    for (offset, row) in rows.iter().enumerate().skip(header_idx + 1) {
        let org_raw = cell_text(row, org_i);
        if org_raw.is_empty() {
            continue;
        }
        out.push(RawVchArchiveRow {
            row_number: (offset + 1) as u32,
            org_raw,
            place_raw: place_i.map(|i| cell_text(row, i)).unwrap_or_default(),
            specialty_raw: specialty_i.map(|i| cell_text(row, i)).unwrap_or_default(),
            vos_raw: vos_i.map(|i| cell_text(row, i)).unwrap_or_default(),
            equipment_raw: equipment_i.map(|i| cell_text(row, i)).unwrap_or_default(),
            start_raw: start_i.and_then(|i| cell_date(row, i)),
            end_raw: end_i.and_then(|i| cell_date(row, i)),
            planned_raw: planned_i.map(|i| cell_text(row, i)).unwrap_or_default(),
            actual_raw: actual_i.map(|i| cell_text(row, i)).unwrap_or_default(),
            note_raw: note_i.map(|i| cell_text(row, i)).unwrap_or_default(),
        });
    }
    out
}

/// Розбирає аркуш "Записи" (службові "Довідник"/"Інструкція"/"Superset" без розпізнаного
/// заголовка — пропускаються мовчки, той самий підхід, що й порожні аркуші в БпС).
pub fn extract(bytes: &[u8]) -> Result<Vec<RawVchArchiveRow>, ParseError> {
    let mut workbook: Xlsx<_> = Xlsx::new(Cursor::new(bytes.to_vec()))
        .map_err(|e| ParseError(format!("не вдалось відкрити xlsx: {e}")))?;

    let sheet_names = workbook.sheet_names().to_vec();
    let mut all_rows = Vec::new();
    for name in sheet_names {
        let range = workbook
            .worksheet_range(&name)
            .map_err(|e| ParseError(format!("аркуш \"{name}\": {e}")))?;
        let rows: Vec<Vec<Data>> = range.rows().map(|r| r.to_vec()).collect();
        all_rows.extend(parse_sheet(rows));
    }
    Ok(all_rows)
}
