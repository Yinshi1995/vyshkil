//! Екстрактор "БпС" (03 §2-3) — другий тип файлу, ІНША структура за Фах (backend/import/CLAUDE.md
//! пояснює чому не один спільний детектор): дворівневий заголовок (група колонок у рядку N,
//! підколонки в рядку N+1 -- частина/місце розкладені на "номер"+"найменування" ОКРЕМИМИ
//! колонками, не одним текстом), рядок "Всього:" одразу під заголовком (не в кінці, як у Фах), і
//! funnel-воронка замість однієї "Кількість": "Завершилась" — Викликали→Прибуло до НЦ→Успішно
//! завершило; "Навчаються" — Викликали→Проходять (менше стадій).

use calamine::{Data, DataType, Reader, Xlsx};
use chrono::NaiveDate;
use std::io::Cursor;

use crate::domain::normalize::normalize;

#[derive(Debug, Clone)]
pub struct RawBpsRow {
    pub sheet: String,
    pub row_number: u32,
    pub org_number_raw: String,
    pub org_name_raw: String,
    pub site_number_raw: String,
    pub site_name_raw: String,
    pub vos_raw: String,
    pub equipment_raw: String,
    pub start_raw: Option<NaiveDate>,
    pub end_raw: Option<NaiveDate>,
    /// Викликали (завжди є, обидва аркуші).
    pub called_raw: String,
    /// "Прибуло до НЦ" (лише "Завершилась") або "Проходять" (лише "Навчаються") — та сама роль
    /// (наступна стадія воронки після "Викликали"), просто інша назва колонки.
    pub next_stage_raw: String,
    /// "Успішно завершило навчання" — лише "Завершилась"; на "Навчаються" немає третьої стадії.
    pub completed_raw: Option<String>,
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
    Equipment,
    Vos,
    OrgNumber,
    OrgName,
    SiteNumber,
    SiteName,
    Start,
    End,
    Called,
    NextStage,
    Completed,
}

fn cell_text(row: &[Data], idx: usize) -> String {
    row.get(idx).map(|c| c.to_string()).unwrap_or_default().trim().to_string()
}

fn cell_date(row: &[Data], idx: usize) -> Option<NaiveDate> {
    let cell = row.get(idx)?;
    cell.as_date().or_else(|| {
        let text = cell.get_string()?;
        NaiveDate::parse_from_str(text.trim(), "%Y-%m-%d").ok()
    })
}

/// Заголовок дворівневий: рядок N — групи колонок (деякі клітинки об'єднані по горизонталі,
/// у "сирому" вигляді текст лише в лівій клітинці групи — переносимо праворуч forward-fill'ом),
/// рядок N+1 — підколонки. `Col` визначаємо з ПАРИ (група, підколонка), бо "Військова частина"
/// зустрічається двічі (у групі "Військовослужбовці" — номер частини; у групі "Місце навчання" —
/// номер майданчика) і сама підколонка неоднозначна без групи.
fn detect_header(rows: &[Vec<Data>]) -> Option<(usize, Vec<(usize, Col)>)> {
    for (r, row) in rows.iter().take(25).enumerate() {
        let first = row.first().and_then(|c| c.get_string()).map(normalize).unwrap_or_default();
        if !(first.contains("з/п") || first == "№") {
            continue;
        }
        let Some(sub_row) = rows.get(r + 1) else { continue };

        // forward-fill групового рядка ліворуч->праворуч.
        let width = row.len().max(sub_row.len());
        let mut group_ff = vec![String::new(); width];
        let mut last = String::new();
        for (c, cell) in row.iter().enumerate() {
            let text = cell.get_string().map(normalize).unwrap_or_default();
            if !text.is_empty() {
                last = text;
            }
            group_ff[c] = last.clone();
        }

        let mapping: Vec<(usize, Col)> = (0..width)
            .filter_map(|c| {
                let group = group_ff.get(c).cloned().unwrap_or_default();
                let sub = sub_row.get(c).and_then(|c| c.get_string()).map(normalize).unwrap_or_default();
                classify(&group, &sub).map(|col| (c, col))
            })
            .collect();

        if mapping.len() >= 7 {
            return Some((r + 1, mapping)); // +1: дані йдуть після ОБОХ рядків заголовка.
        }
    }
    None
}

/// `group` — форвард-заповнений рядок групових заголовків (об'єднані по горизонталі клітинки
/// дають текст лише зліва); `sub` — підзаголовок під ним. Деякі колонки НЕ розділені на два
/// рівні (напр. "Тип БпАК"/"ВОС"/"Примітки") — тоді весь текст лежить у `group`, а `sub` порожній.
/// Тому перевіряємо ОБИДВА поля незалежно, не лише `sub`.
fn classify(group: &str, sub: &str) -> Option<Col> {
    if group == "тип бпак" || sub == "тип бпак" {
        return Some(Col::Equipment);
    }
    if group == "вос" || sub == "вос" {
        return Some(Col::Vos);
    }
    if sub.contains("дата початку") {
        return Some(Col::Start);
    }
    if sub.contains("дата завершення") {
        return Some(Col::End);
    }
    if sub.contains("викликали") {
        return Some(Col::Called);
    }
    if sub.contains("прибуло") || sub.contains("проходять") {
        return Some(Col::NextStage);
    }
    if sub.contains("завершило") {
        return Some(Col::Completed);
    }
    if sub.contains("військова частина") {
        return if group.contains("місце") { Some(Col::SiteNumber) } else { Some(Col::OrgNumber) };
    }
    if sub.contains("найменування") {
        return if group.contains("місце") { Some(Col::SiteName) } else { Some(Col::OrgName) };
    }
    None
}

/// "Всього:" — рядок-підсумок одразу під заголовком (03 §3), не "Кількість/План" у себе.
fn looks_like_summary(row: &[Data]) -> bool {
    row.iter().take(3).any(|c| {
        c.get_string().map(normalize).map(|n| n.contains("всього")).unwrap_or(false)
    })
}

fn parse_sheet(sheet_name: &str, rows: Vec<Vec<Data>>) -> Vec<RawBpsRow> {
    let Some((header_idx, mapping)) = detect_header(&rows) else { return Vec::new() };
    let col = |c: Col| mapping.iter().find(|(_, mc)| *mc == c).map(|(i, _)| *i);
    let (Some(org_number_i), Some(called_i)) = (col(Col::OrgNumber), col(Col::Called)) else {
        return Vec::new();
    };
    let org_name_i = col(Col::OrgName);
    let site_number_i = col(Col::SiteNumber);
    let site_name_i = col(Col::SiteName);
    let vos_i = col(Col::Vos);
    let equipment_i = col(Col::Equipment);
    let start_i = col(Col::Start);
    let end_i = col(Col::End);
    let next_stage_i = col(Col::NextStage);
    let completed_i = col(Col::Completed);

    let mut out = Vec::new();
    for (offset, row) in rows.iter().enumerate().skip(header_idx + 1) {
        if looks_like_summary(row) {
            continue;
        }
        let called_raw = cell_text(row, called_i);
        let org_number_raw = cell_text(row, org_number_i);
        if org_number_raw.is_empty() || called_raw.trim().parse::<i64>().map(|n| n <= 0).unwrap_or(true) {
            continue;
        }
        out.push(RawBpsRow {
            sheet: sheet_name.to_string(),
            row_number: (offset + 1) as u32,
            org_number_raw,
            org_name_raw: org_name_i.map(|i| cell_text(row, i)).unwrap_or_default(),
            site_number_raw: site_number_i.map(|i| cell_text(row, i)).unwrap_or_default(),
            site_name_raw: site_name_i.map(|i| cell_text(row, i)).unwrap_or_default(),
            vos_raw: vos_i.map(|i| cell_text(row, i)).unwrap_or_default(),
            equipment_raw: equipment_i.map(|i| cell_text(row, i)).unwrap_or_default(),
            start_raw: start_i.and_then(|i| cell_date(row, i)),
            end_raw: end_i.and_then(|i| cell_date(row, i)),
            called_raw,
            next_stage_raw: next_stage_i.map(|i| cell_text(row, i)).unwrap_or_default(),
            completed_raw: completed_i.map(|i| cell_text(row, i)),
        });
    }
    out
}

/// Розбирає аркуші "Завершилась"/"Навчаються" (службовий "Аркуш1" без розпізнаного заголовка —
/// пропускається мовчки).
pub fn extract(bytes: &[u8]) -> Result<Vec<RawBpsRow>, ParseError> {
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
