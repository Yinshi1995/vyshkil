//! Екстрактор "КВід" (укомплектованість командирами відділень, 01 §4, 03 §2-3) — ТРЕТІЙ тип
//! файлу, знову інша структура (backend/import/CLAUDE.md): однорівневий заголовок, але це не
//! `training_group` узагалі — це `staffing_snapshot`/`staffing_metric` (напрям, якого не було в
//! жодному попередньому етапі). Секційні рядки ("Військові частини безпосереднього/оперативного
//! підпорядкування") — контекст, не дані (03 §3); підсумкові рядки ("ВСЬОГО:") мають формули й
//! пропускаються так само.

use calamine::{Data, DataType, Reader, Xlsx};
use std::io::Cursor;

use crate::domain::normalize::normalize;

#[derive(Debug, Clone)]
pub struct RawKvidRow {
    pub sheet: String,
    pub row_number: u32,
    pub org_raw: String,
    pub by_tos_raw: String,
    pub by_list_raw: String,
    pub present_raw: String,
    pub trained_sergeant_raw: String,
    pub in_training_raw: String,
    pub planned_next_month_raw: String,
    pub need_training_raw: String,
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
    ByTos,
    ByList,
    Present,
    TrainedSergeant,
    InTraining,
    PlannedNextMonth,
    NeedTraining,
}

const COL_SYNONYMS: &[(Col, &[&str])] = &[
    (Col::Org, &["підрозділ"]),
    (Col::ByTos, &["за штатом"]),
    (Col::ByList, &["за списком"]),
    (Col::Present, &["в наявності"]),
    (Col::TrainedSergeant, &["мають підготовку"]),
    // "Відсоток укомплектованості" свідомо не мапиться на жоден Col -- не зберігаємо (01 §4).
    (Col::InTraining, &["проходять підготовку"]),
    (Col::PlannedNextMonth, &["заплановано"]),
    (Col::NeedTraining, &["потребують підготовку"]),
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
        if mapping.len() >= 6 {
            return Some((r, mapping));
        }
    }
    None
}

fn cell_text(row: &[Data], idx: usize) -> String {
    row.get(idx).map(|c| c.to_string()).unwrap_or_default().trim().to_string()
}

/// "№ з/п" (колонка 0) — секційні/підсумкові рядки її не мають; звичайні дані завжди мають
/// додатне ціле число (03 §3: "рядки-підсумки… пропускати"). Текстовий парсинг, не `get_int()`:
/// Excel часто зберігає малі цілі як float, і `get_int()` на `Data::Float` мовчки дає `None`.
fn row_number_cell(row: &[Data]) -> Option<i64> {
    row.first()?.to_string().trim().parse::<i64>().ok().filter(|n| *n > 0)
}

fn parse_sheet(sheet_name: &str, rows: Vec<Vec<Data>>) -> Vec<RawKvidRow> {
    let Some((header_idx, mapping)) = detect_header(&rows) else { return Vec::new() };
    let col = |c: Col| mapping.iter().find(|(_, mc)| *mc == c).map(|(i, _)| *i);
    let Some(org_i) = col(Col::Org) else { return Vec::new() };
    let by_tos_i = col(Col::ByTos);
    let by_list_i = col(Col::ByList);
    let present_i = col(Col::Present);
    let trained_i = col(Col::TrainedSergeant);
    let in_training_i = col(Col::InTraining);
    let planned_i = col(Col::PlannedNextMonth);
    let need_i = col(Col::NeedTraining);

    let mut out = Vec::new();
    for (offset, row) in rows.iter().enumerate().skip(header_idx + 1) {
        if row_number_cell(row).is_none() {
            continue;
        }
        let org_raw = cell_text(row, org_i);
        if org_raw.is_empty() {
            continue;
        }
        out.push(RawKvidRow {
            sheet: sheet_name.to_string(),
            row_number: (offset + 1) as u32,
            org_raw,
            by_tos_raw: by_tos_i.map(|i| cell_text(row, i)).unwrap_or_default(),
            by_list_raw: by_list_i.map(|i| cell_text(row, i)).unwrap_or_default(),
            present_raw: present_i.map(|i| cell_text(row, i)).unwrap_or_default(),
            trained_sergeant_raw: trained_i.map(|i| cell_text(row, i)).unwrap_or_default(),
            in_training_raw: in_training_i.map(|i| cell_text(row, i)).unwrap_or_default(),
            planned_next_month_raw: planned_i.map(|i| cell_text(row, i)).unwrap_or_default(),
            need_training_raw: need_i.map(|i| cell_text(row, i)).unwrap_or_default(),
        });
    }
    out
}

pub fn extract(bytes: &[u8]) -> Result<Vec<RawKvidRow>, ParseError> {
    let mut workbook: Xlsx<_> = Xlsx::new(Cursor::new(bytes.to_vec()))
        .map_err(|e| ParseError(format!("не вдалось відкрити xlsx: {e}")))?;

    let sheet_names = workbook.sheet_names().to_vec();
    let mut all_rows = Vec::new();
    for name in sheet_names {
        let range = workbook
            .worksheet_range(&name)
            .map_err(|e| ParseError(format!("аркуш \"{name}\": {e}")))?;
        let rows: Vec<Vec<Data>> = range.rows().map(|r| r.to_vec()).collect();
        all_rows.extend(parse_sheet(&name, rows));
    }
    Ok(all_rows)
}
