//! Екстрактор "Фах" (03 §2-3) — перший і поки єдиний тип файлу (див. `pages/import/CLAUDE.md`
//! чому інші 4 файли з критерію Етапу 5 йдуть окремими кроками, не одним "універсальним детектором").
//! Аркуші "Пройшли"/"Проходять" (завершили/проходять фахову підготовку) — та сама плоска таблиця:
//! `Підрозділ | Місце проведення | Посада | ВОС | ОВТ | Термін з | Термін по | Кількість`.
//!
//! Лише СТРУКТУРНИЙ розбір (байти → рядки з сирими значеннями) — без БД: резолюція
//! org_id/vos_id/site_id і валідація — `backend::repo::imports_fah` (потребує `&DatabaseConnection`).

use calamine::{Data, DataType, Reader, Xlsx};
use chrono::NaiveDate;
use std::io::Cursor;

use crate::domain::normalize::normalize;

#[derive(Debug, Clone)]
pub struct RawFahRow {
    pub sheet: String,
    pub row_number: u32,
    pub org_raw: String,
    pub site_raw: String,
    pub position_raw: String,
    pub vos_raw: String,
    pub equipment_raw: String,
    pub start_raw: Option<NaiveDate>,
    pub end_raw: Option<NaiveDate>,
    pub count_raw: String,
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
    Site,
    Position,
    Vos,
    Equipment,
    Start,
    End,
    Count,
}

/// (поле, нормалізовані фрагменти заголовка, що на нього вказують) -- `contains`, не рівність:
/// реальні заголовки бувають багаторядковими ("Кількість \nлюдей") чи з уточненням у дужках.
const COL_SYNONYMS: &[(Col, &[&str])] = &[
    (Col::Org, &["підрозділ", "військова частина", "вч"]),
    (Col::Site, &["місце проведення", "місце навчання"]),
    (Col::Position, &["посада", "спеціальність"]),
    (Col::Vos, &["вос"]),
    (Col::Equipment, &["овт"]),
    (Col::Start, &["термін з", "дата початку"]),
    (Col::End, &["термін по", "дата завершення"]),
    // "Проходять" використовує "План"/"Факт" замість "Кількість" -- беремо "План" як базову
    // кількість (та сама спрощена рівність план=прибуло=навчаються, що й у ручному введенні,
    // 02 §4); "Факт" (фактично на сьогодні) поки не читаємо -- задокументоване спрощення.
    (Col::Count, &["кількість", "план"]),
];

fn match_col(header_cell: &str) -> Option<Col> {
    let norm = normalize(header_cell);
    if norm.is_empty() {
        return None;
    }
    COL_SYNONYMS.iter().find(|(_, syns)| syns.iter().any(|s| norm.contains(s))).map(|(c, _)| *c)
}

/// Знаходить рядок заголовка серед перших 20 рядків аркуша: той, де найбільше клітинок
/// збіглось із відомими назвами колонок (03 §3). Повертає (індекс рядка, мапа колонка→поле).
fn detect_header(rows: &[Vec<Data>]) -> Option<(usize, Vec<(usize, Col)>)> {
    let mut best: Option<(usize, Vec<(usize, Col)>)> = None;
    for (r, row) in rows.iter().take(20).enumerate() {
        let mapping: Vec<(usize, Col)> = row
            .iter()
            .enumerate()
            .filter_map(|(c, cell)| {
                let text = cell.get_string()?;
                match_col(text).map(|col| (c, col))
            })
            .collect();
        let is_better = best.as_ref().map(|(_, m)| mapping.len() > m.len()).unwrap_or(true);
        if is_better && mapping.len() >= 5 {
            best = Some((r, mapping));
        }
    }
    best
}

fn cell_text(row: &[Data], idx: usize) -> String {
    row.get(idx).map(|c| c.to_string()).unwrap_or_default().trim().to_string()
}

/// Реальна вада джерела (не гіпотетична — рядок Пройшли:431 живого файлу): деякі клітинки дати
/// набрані як ТЕКСТ ISO-формату ("2026-08-03"), а не як справжня Excel-дата, серед сусідніх
/// клітинок того самого стовпця, що є справжніми датами. `as_date()` таке не бачить — пробуємо
/// розібрати як ISO текст окремим фолбеком (не через `domain::dates`, той формат — dd.mm[.yy[yy]],
/// зовсім інший випадок).
fn cell_date(row: &[Data], idx: usize) -> Option<NaiveDate> {
    let cell = row.get(idx)?;
    cell.as_date().or_else(|| {
        let text = cell.get_string()?;
        NaiveDate::parse_from_str(text.trim(), "%Y-%m-%d").ok()
    })
}

/// Рядок-підсумок ("Всього за…", "…Підсумок", "ВСЬОГО", "РАЗОМ") — сам МАЄ додатну "Кількість"
/// (це сума!), тож перевірки "число > 0" замало (03 §3): виключаємо за текстом органу окремо.
const SUMMARY_MARKERS: &[&str] = &["підсумок", "всього", "разом"];

fn looks_like_summary(org_raw: &str) -> bool {
    let norm = normalize(org_raw);
    SUMMARY_MARKERS.iter().any(|m| norm.contains(m))
}

/// Рядок вважаємо даними, лише якщо колонка "Кількість"/"План" містить додатне ціле число і
/// органи-колонка не схожа на рядок-підсумок (03 §3). Підписні/порожні рядки відсіюються самим
/// числовим критерієм — "Кількість" у них завжди порожня.
fn is_data_row(org_raw: &str, count_raw: &str) -> bool {
    if looks_like_summary(org_raw) {
        return false;
    }
    count_raw.trim().parse::<i64>().map(|n| n > 0).unwrap_or(false)
}

/// Розбирає один аркуш (calamine `Range` вже прочитаний викликачем у `Vec<Vec<Data>>`, щоб не
/// плутатись із запозиченнями `Xlsx<Cursor<_>>` під час ітерації кількох аркушів).
fn parse_sheet(sheet_name: &str, rows: Vec<Vec<Data>>) -> Vec<RawFahRow> {
    let Some((header_idx, mapping)) = detect_header(&rows) else { return Vec::new() };

    let col = |c: Col| mapping.iter().find(|(_, mc)| *mc == c).map(|(i, _)| *i);
    let (Some(org_i), Some(count_i)) = (col(Col::Org), col(Col::Count)) else {
        return Vec::new();
    };
    let site_i = col(Col::Site);
    let position_i = col(Col::Position);
    let vos_i = col(Col::Vos);
    let equipment_i = col(Col::Equipment);
    let start_i = col(Col::Start);
    let end_i = col(Col::End);

    let mut out = Vec::new();
    for (offset, row) in rows.iter().enumerate().skip(header_idx + 1) {
        let org_raw = cell_text(row, org_i);
        let count_raw = cell_text(row, count_i);
        if !is_data_row(&org_raw, &count_raw) {
            continue;
        }
        out.push(RawFahRow {
            sheet: sheet_name.to_string(),
            row_number: (offset + 1) as u32,
            org_raw,
            site_raw: site_i.map(|i| cell_text(row, i)).unwrap_or_default(),
            position_raw: position_i.map(|i| cell_text(row, i)).unwrap_or_default(),
            vos_raw: vos_i.map(|i| cell_text(row, i)).unwrap_or_default(),
            equipment_raw: equipment_i.map(|i| cell_text(row, i)).unwrap_or_default(),
            start_raw: start_i.and_then(|i| cell_date(row, i)),
            end_raw: end_i.and_then(|i| cell_date(row, i)),
            count_raw,
        });
    }
    out
}

/// Розбирає весь файл (усі аркуші з розпізнаним заголовком; аркуші без нього -- пропускаються
/// мовчки, напр. службовий "Аркуш1", який трапляється в цих файлах порожнім).
pub fn extract(bytes: &[u8]) -> Result<Vec<RawFahRow>, ParseError> {
    let mut workbook: Xlsx<_> = Xlsx::new(Cursor::new(bytes.to_vec()))
        .map_err(|e| ParseError(format!("не вдалось відкрити xlsx: {e}")))?;

    let sheet_names = workbook.sheet_names().to_vec();
    let mut all_rows = Vec::new();
    for name in sheet_names {
        let range = workbook
            .worksheet_range(&name)
            .map_err(|e| ParseError(format!("аркуш «{name}»: {e}")))?;
        let rows: Vec<Vec<Data>> = range.rows().map(|r| r.to_vec()).collect();
        all_rows.extend(parse_sheet(&name, rows));
    }
    Ok(all_rows)
}
