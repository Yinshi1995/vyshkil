//! Екстрактор "ІВС" (інструктори, 01 §4, 03 §2-3) — ЧЕТВЕРТИЙ тип файлу, і найскладніший з дотепер
//! зроблених: ОДИН аркуш містить ДВІ незалежні таблиці одна під одною.
//!
//! Таблиця 1 (рядки ~8-25) — укомплектованість груп інструкторів: дворівневий заголовок
//! (група/підгрупа, як у БпС), дві секції підпорядкування з "Всього"-рядками (як у КВід). Це знову
//! `staffing_snapshot`/`staffing_metric` (01 §4, `category='instructors'`), АЛЕ інший набір метрик
//! за КВід (тут `trained_kibr`, немає `present`/`in_training`/`planned_next_month`/`need_training`)
//! — тому окремий тип рядка, не переліт `types::staffing::StaffingRow`. Остання колонка секції 1
//! ("Проходять стажування") — вільний текст виду "2 (з 15.09 по 04.11) 1 нб 235 мцпвчп": за 01 §4
//! це НЕ метрика, а `training_group` виду "стажування" (`training_kind.code='internship'`) — тому
//! екстрактор повертає його ОКРЕМИМ рядком (`RawInternshipRow`), не текстом.
//!
//! Таблиця 2 (рядки ~27-35) — курси інструкторів (КІБР/КПК/СККВ/…): однорівневий заголовок,
//! Підрозділ (вертикально злитий на кілька курсів однієї частини — forward-fill униз) / Курс /
//! Кількість / Період навчання (дві дати) / Місце проведення. Структурно це "рядок групи" з
//! курсом замість ВОС/посади — той самий `GroupFormRow`, що й Фах/стажування, просто інший
//! `course_id` замість `vos_id`/`position_id` (`training_group.course_id` саме для цього і є).
//!
//! Обидві таблиці шукаються НЕЗАЛЕЖНО скануванням усього аркуша: рядки таблиці 2 не потрапляють у
//! таблицю 1, бо колонка "за штатом" (позиція B) у рядках таблиці 2 містить текст курсу, не число
//! — природно відсіюється тим самим тестом "колонка B — додатне ціле", без явного розмежування меж.

use calamine::{Data, DataType, Reader, Xlsx};
use chrono::NaiveDate;
use std::io::Cursor;

use crate::domain::normalize::normalize;

#[derive(Debug, Clone)]
pub struct RawIvsStaffingRow {
    pub row_number: u32,
    pub org_raw: String,
    pub by_tos_raw: String,
    pub by_list_raw: String,
    pub trained_sergeant_raw: String,
    pub trained_kibr_raw: String,
}

#[derive(Debug, Clone)]
pub struct RawInternshipRow {
    pub row_number: u32,
    pub org_raw: String,
    pub count_raw: String,
    /// "дд.мм" без року — рік визначається пізніше через `as_of` (той самий шлях, що й ручне
    /// введення: `domain::dates`, у `widgets::group_grid`, не тут).
    pub start_raw: String,
    pub end_raw: String,
    pub site_raw: String,
}

#[derive(Debug, Clone)]
pub struct RawIvsCourseRow {
    pub row_number: u32,
    pub org_raw: String,
    pub course_raw: String,
    pub count_raw: String,
    pub start_raw: Option<NaiveDate>,
    pub end_raw: Option<NaiveDate>,
    pub site_raw: String,
}

#[derive(Debug, Clone, Default)]
pub struct IvsExtract {
    pub staffing: Vec<RawIvsStaffingRow>,
    pub internships: Vec<RawInternshipRow>,
    pub courses: Vec<RawIvsCourseRow>,
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

fn cell_date(row: &[Data], idx: usize) -> Option<NaiveDate> {
    row.get(idx)?.as_date()
}

fn looks_like_summary(org: &str) -> bool {
    normalize(org).contains("всього")
}

// --- Таблиця 1: укомплектованість -------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StaffingCol {
    Org,
    ByTos,
    ByList,
    TrainedSergeant,
    TrainedKibr,
    Internship,
}

/// Підколонка (`sub`), якщо непорожня, переважає над групою — так позначені "за штатом"/"за
/// списком"/… (кожна підколонка стоїть у СВОЇЙ клітинці, без forward-fill, на відміну від групи:
/// відсоткові клітинки поруч навмисно ПОРОЖНІ в підрядку, тому не потрапляють у жодну `Col` —
/// 01 §4 "відсотки не зберігаються"). "Проходять стажування" й "Підрозділ" мають текст ЛИШЕ в
/// групі (вертикально злиті на обидва рядки заголовка, як "Тип БпАК"/"ВОС" у БпС) — перевіряємо
/// групу тільки коли підрядок порожній.
fn classify_staffing(group: &str, sub: &str) -> Option<StaffingCol> {
    if sub.contains("за штатом") {
        return Some(StaffingCol::ByTos);
    }
    if sub.contains("за списком") {
        return Some(StaffingCol::ByList);
    }
    if sub.contains("сержантська") {
        return Some(StaffingCol::TrainedSergeant);
    }
    if sub.contains("кібр") {
        return Some(StaffingCol::TrainedKibr);
    }
    if !sub.is_empty() {
        return None;
    }
    if group.contains("стажування") {
        return Some(StaffingCol::Internship);
    }
    // Точна рівність, не `contains` -- вище в аркуші є рядок-заголовок документа ("…груп
    // інструкторів ПІДРОЗДІЛІВ 17 АК станом на…", A7:I7 злитий на весь рядок), forward-fill
    // якого випадково містить підрядок "підрозділ" у КОЖНІЙ колонці й ламав пошук заголовка,
    // якби `detect_staffing_header` розглянув (рядок 6, рядок 7) як пару перш ніж дійти до
    // справжньої (рядок 7, рядок 8) -- зловлено тестом на реальному файлі.
    if group == "підрозділ" {
        return Some(StaffingCol::Org);
    }
    None
}

fn detect_staffing_header(rows: &[Vec<Data>]) -> Option<(usize, Vec<(usize, StaffingCol)>)> {
    for r in 0..rows.len().saturating_sub(1).min(15) {
        let group_row = &rows[r];
        let sub_row = &rows[r + 1];
        let width = group_row.len().max(sub_row.len());

        // forward-fill групового рядка ліворуч->праворуч (той самий патерн, що й БпС).
        let mut group_ff = vec![String::new(); width];
        let mut last = String::new();
        for (c, slot) in group_ff.iter_mut().enumerate().take(width) {
            let text = group_row.get(c).and_then(|c| c.get_string()).map(normalize).unwrap_or_default();
            if !text.is_empty() {
                last = text;
            }
            *slot = last.clone();
        }

        let mapping: Vec<(usize, StaffingCol)> = (0..width)
            .filter_map(|c| {
                let group = group_ff.get(c).cloned().unwrap_or_default();
                let sub = sub_row.get(c).and_then(|c| c.get_string()).map(normalize).unwrap_or_default();
                classify_staffing(&group, &sub).map(|col| (c, col))
            })
            .collect();

        if mapping.len() >= 5 {
            return Some((r + 1, mapping)); // +1: дані йдуть після ОБОХ рядків заголовка.
        }
    }
    None
}

fn parse_staffing_and_internships(
    rows: &[Vec<Data>],
) -> (Vec<RawIvsStaffingRow>, Vec<RawInternshipRow>) {
    let Some((header_idx, mapping)) = detect_staffing_header(rows) else {
        return (Vec::new(), Vec::new());
    };
    let col = |c: StaffingCol| mapping.iter().find(|(_, mc)| *mc == c).map(|(i, _)| *i);
    let Some(org_i) = col(StaffingCol::Org) else { return (Vec::new(), Vec::new()) };
    let Some(by_tos_i) = col(StaffingCol::ByTos) else { return (Vec::new(), Vec::new()) };
    let by_list_i = col(StaffingCol::ByList);
    let trained_i = col(StaffingCol::TrainedSergeant);
    let kibr_i = col(StaffingCol::TrainedKibr);
    let internship_i = col(StaffingCol::Internship);

    let mut staffing = Vec::new();
    let mut internships = Vec::new();
    for (idx, row) in rows.iter().enumerate().skip(header_idx + 1) {
        let excel_row = (idx + 1) as u32;
        let org_raw = cell_text(row, org_i);
        if org_raw.is_empty() || looks_like_summary(&org_raw) {
            continue;
        }
        let by_tos_raw = cell_text(row, by_tos_i);
        if by_tos_raw.parse::<i64>().map(|n| n <= 0).unwrap_or(true) {
            continue;
        }

        staffing.push(RawIvsStaffingRow {
            row_number: excel_row,
            org_raw: org_raw.clone(),
            by_tos_raw,
            by_list_raw: by_list_i.map(|i| cell_text(row, i)).unwrap_or_default(),
            trained_sergeant_raw: trained_i.map(|i| cell_text(row, i)).unwrap_or_default(),
            trained_kibr_raw: kibr_i.map(|i| cell_text(row, i)).unwrap_or_default(),
        });

        if let Some(i) = internship_i {
            let raw_text = cell_text(row, i);
            if let Some(parsed) = parse_internship_text(&raw_text) {
                internships.push(RawInternshipRow { org_raw, row_number: excel_row, ..parsed });
            }
        }
    }
    (staffing, internships)
}

/// "2 (з 15.09 по 04.11) 1 нб 235 мцпвчп" → кількість + період + місце. Роздільник періоду — "по"
/// або "до" (обидва трапляються в реальному файлі, 03 §2). Формат нежорсткий: рядок без дужок або
/// без числа на початку — не стажування (`None`), пропускається, а не падає всім імпортом.
fn parse_internship_text(text: &str) -> Option<RawInternshipRow> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let open = text.find('(')?;
    let close = text.find(')')?;
    if close <= open {
        return None;
    }
    let count_raw = text[..open].trim().to_string();
    if count_raw.is_empty() || count_raw.parse::<i64>().is_err() {
        return None;
    }
    let inner = text[open + 1..close].trim();
    let inner = inner.strip_prefix('з').unwrap_or(inner).trim();
    let (start_raw, end_raw) = if let Some(idx) = inner.find(" по ") {
        (inner[..idx].trim(), inner[idx + " по ".len()..].trim())
    } else {
        let idx = inner.find(" до ")?;
        (inner[..idx].trim(), inner[idx + " до ".len()..].trim())
    };
    let site_raw = text[close + 1..].trim().to_string();
    Some(RawInternshipRow {
        row_number: 0,
        org_raw: String::new(),
        count_raw,
        start_raw: start_raw.to_string(),
        end_raw: end_raw.to_string(),
        site_raw,
    })
}

// --- Таблиця 2: курси інструкторів --------------------------------------------------------------

fn detect_course_header(rows: &[Vec<Data>]) -> Option<usize> {
    rows.iter().take(35).position(|row| {
        let a = row.first().and_then(|c| c.get_string()).map(normalize).unwrap_or_default();
        let b = row.get(1).and_then(|c| c.get_string()).map(normalize).unwrap_or_default();
        a.contains("підрозділ") && b == "курс"
    })
}

fn parse_courses(rows: &[Vec<Data>]) -> Vec<RawIvsCourseRow> {
    let Some(header_idx) = detect_course_header(rows) else { return Vec::new() };

    let mut out = Vec::new();
    let mut last_org = String::new();
    for (idx, row) in rows.iter().enumerate().skip(header_idx + 1) {
        let excel_row = (idx + 1) as u32;
        let org_cell = cell_text(row, 0);
        if !org_cell.is_empty() {
            last_org = org_cell;
        }
        if last_org.is_empty() || looks_like_summary(&last_org) {
            continue;
        }
        let course_raw = cell_text(row, 1);
        let count_raw = cell_text(row, 2);
        if course_raw.is_empty() || count_raw.parse::<i64>().map(|n| n <= 0).unwrap_or(true) {
            continue;
        }
        out.push(RawIvsCourseRow {
            row_number: excel_row,
            org_raw: last_org.clone(),
            course_raw,
            count_raw,
            start_raw: cell_date(row, 3),
            end_raw: cell_date(row, 4),
            site_raw: cell_text(row, 5),
        });
    }
    out
}

/// Один аркуш, дві таблиці (див. заголовок модуля). "Аркуш1"/подібні службові назви без
/// розпізнаного заголовка жодної з таблиць — просто нуль рядків, не помилка (той самий підхід,
/// що й у БпС для порожніх аркушів).
pub fn extract(bytes: &[u8]) -> Result<IvsExtract, ParseError> {
    let mut workbook: Xlsx<_> = Xlsx::new(Cursor::new(bytes.to_vec()))
        .map_err(|e| ParseError(format!("не вдалось відкрити xlsx: {e}")))?;

    let mut out = IvsExtract::default();
    for name in workbook.sheet_names().to_vec() {
        let range = workbook
            .worksheet_range(&name)
            .map_err(|e| ParseError(format!("аркуш \"{name}\": {e}")))?;
        let rows: Vec<Vec<Data>> = range.rows().map(|r| r.to_vec()).collect();

        let (staffing, internships) = parse_staffing_and_internships(&rows);
        out.staffing.extend(staffing);
        out.internships.extend(internships);
        out.courses.extend(parse_courses(&rows));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_internship_with_po_separator() {
        let r = parse_internship_text("2 (з 15.09 по 04.11) 1 нб 235 мцпвчп").unwrap();
        assert_eq!(r.count_raw, "2");
        assert_eq!(r.start_raw, "15.09");
        assert_eq!(r.end_raw, "04.11");
        assert_eq!(r.site_raw, "1 нб 235 мцпвчп");
    }

    #[test]
    fn parses_internship_with_do_separator_and_trailing_spaces() {
        let r = parse_internship_text("3 (з 26.08 до 15.10) 1 нб 235 мцпвчп                     ").unwrap();
        assert_eq!(r.count_raw, "3");
        assert_eq!(r.start_raw, "26.08");
        assert_eq!(r.end_raw, "15.10");
        assert_eq!(r.site_raw, "1 нб 235 мцпвчп");
    }

    #[test]
    fn rejects_text_without_parens() {
        assert!(parse_internship_text("немає стажування").is_none());
    }

    #[test]
    fn rejects_empty() {
        assert!(parse_internship_text("").is_none());
    }
}
