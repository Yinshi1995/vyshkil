//! Розбір дат у довільних форматах (02 §4, 03 §4) — спільний код для форми введення (реальний
//! час, WASM, без запиту до сервера) і превʼю імпорту (03). Формати: `dd.mm`, `dd.mm.yy`,
//! `dd.mm.yyyy`, з крапкою в кінці (`16.08.`), діапазони в одній клітинці (`18.08-09.10`,
//! `18.08 – 09.10`, `31.08 - 23.10\n`). Рік без явного вказання — з контексту `as_of_date`.
//!
//! Повідомлення про помилку (`DateError::message`) навмисно розлогі — не просто "неможлива
//! дата", а що саме не так, чому, і який формат очікується (користувач бачить сирий рядок і має
//! зрозуміти, що виправити, без інструкції поруч).

use chrono::{Datelike, NaiveDate};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DateError {
    /// Не існує такого дня в місяці/році ("31.09" — вересень має 30 днів).
    Invalid { raw: String, day: u32, month: u32, year: i32 },
    /// Рік не з 2 чи 4 цифр ("18.09.20216" — 5 цифр).
    MalformedYear { raw: String, year_token: String },
    /// Рік явно вказаний, але не збігається з роком подання/початку ("11.07.2027" у звіті за 2026).
    YearMismatch { raw: String, given_year: i32, expected_year: i32 },
    /// Не вдалось розібрати структуру рядка взагалі (не dd.mm[.yy[yy]]).
    Unparseable { raw: String },
    /// "по" раніше "з".
    EndBeforeStart { start: NaiveDate, end: NaiveDate },
}

fn month_name(month: u32) -> &'static str {
    match month {
        1 => "січень",
        2 => "лютий",
        3 => "березень",
        4 => "квітень",
        5 => "травень",
        6 => "червень",
        7 => "липень",
        8 => "серпень",
        9 => "вересень",
        10 => "жовтень",
        11 => "листопад",
        12 => "грудень",
        _ => "місяць",
    }
}

/// Українське число-узгодження іменника після числівника: 1→форма однини, 2-4→форма "кілька",
/// 5+ (і 11-14)→форма родового відмінка множини. Тут лише два іменники (день/цифра), обидва за
/// тим самим правилом.
fn plural_form(n: u32, one: &'static str, few: &'static str, many: &'static str) -> &'static str {
    let last_two = n % 100;
    let last = n % 10;
    if (11..=14).contains(&last_two) {
        many
    } else if last == 1 {
        one
    } else if (2..=4).contains(&last) {
        few
    } else {
        many
    }
}

/// "28 днів, 29 днів, 30 днів, 31 день".
fn day_word(n: u32) -> &'static str {
    plural_form(n, "день", "дні", "днів")
}

/// "1 цифра, 2 цифри, 5 цифр".
fn digit_word(n: u32) -> &'static str {
    plural_form(n, "цифра", "цифри", "цифр")
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let first = NaiveDate::from_ymd_opt(year, month, 1).expect("місяць 1-12");
    let next_first = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .expect("наступний місяць завжди існує");
    (next_first - first).num_days() as u32
}

fn fmt_date(d: NaiveDate) -> String {
    d.format("%d.%m.%Y").to_string()
}

impl DateError {
    /// Людське пояснення (03 §5: "Рядок 4, «Термін по»: 31.09 — вересень має 30 днів, а
    /// вказано 31-е число") — що саме не так, чому, і який формат очікується.
    pub fn message(&self) -> String {
        match self {
            DateError::Invalid { raw, day, month, year } => {
                let days = days_in_month(*year, *month);
                format!(
                    "«{raw}» — такої дати нема: {} {year} року має {days} {}, а вказано {day}-е число",
                    month_name(*month),
                    day_word(days)
                )
            }
            DateError::MalformedYear { raw, year_token } => {
                let len = year_token.len() as u32;
                format!(
                    "«{raw}» — рік «{year_token}» має бути 2 або 4 цифри (наприклад, 26 або \
                     2026), а тут {len} {}",
                    digit_word(len)
                )
            }
            DateError::YearMismatch { raw, given_year, expected_year } => format!(
                "«{raw}» — рік {given_year} не збігається з роком подання ({expected_year}); \
                 якщо дата справді за {given_year} рік, перевірте поле «Станом на» вгорі"
            ),
            DateError::Unparseable { raw } => format!(
                "«{raw}» — не розпізнано як дату; очікую формат дд.мм або дд.мм.рррр \
                 (наприклад, 15.09 або 15.09.2026)"
            ),
            DateError::EndBeforeStart { start, end } => format!(
                "«по» ({}) раніше «з» ({}) — дати переплутані місцями, або в одній з них \
                 помилка в місяці/році",
                fmt_date(*end),
                fmt_date(*start)
            ),
        }
    }
}

fn parse_u32(s: &str) -> Option<u32> {
    let s = s.trim();
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// 2 цифри → 2000+YY (весь домен — 2020-і/2030-і); 4 цифри → як є; будь-яка інша довжина —
/// свідомо неправильні дані (`18.09.20216`), не вгадуємо.
fn resolve_year_token(y: &str) -> Option<i32> {
    match y.len() {
        2 => parse_u32(y).map(|yy| 2000 + yy as i32),
        4 => parse_u32(y).map(|yyyy| yyyy as i32),
        _ => None,
    }
}

/// Розкладає `raw` на (день, місяць, рік_якщо_є) без прив'язки до конкретного року. `pub` —
/// потрібен `widgets::group_grid::date_range_cell` для ЛЕГКОЇ перевірки "чи це взагалі реальна
/// дата" (день/місяць існують), БЕЗ правила "явний рік = рік `as_of`" (`parse_date`) — те правило
/// існує проти людських друкарських помилок при наборі, але саме цей компонент сам іноді пише
/// назад повністю розв'язану дату (з явним роком) через вибір у календарі чи авторозклад
/// діапазону, і рік там може ЗАКОНОМІРНО відрізнятись від `as_of.year()` (перехід через Новий
/// рік, ще не заповнене "Станом на") — повторна сувора перевірка тим самим правилом дала б
/// фальшиву помилку на щойно правильно вирішеній даті.
pub fn split_token(raw: &str) -> Result<(u32, u32, Option<i32>), DateError> {
    let trimmed = raw.trim().trim_end_matches('.');
    let parts: Vec<&str> = trimmed.split('.').collect();
    match parts.as_slice() {
        [d, m] => {
            let (Some(day), Some(month)) = (parse_u32(d), parse_u32(m)) else {
                return Err(DateError::Unparseable { raw: raw.to_string() });
            };
            Ok((day, month, None))
        }
        [d, m, y] => {
            let (Some(day), Some(month)) = (parse_u32(d), parse_u32(m)) else {
                return Err(DateError::Unparseable { raw: raw.to_string() });
            };
            let Some(year) = resolve_year_token(y) else {
                return Err(DateError::MalformedYear { raw: raw.to_string(), year_token: y.to_string() });
            };
            Ok((day, month, Some(year)))
        }
        _ => Err(DateError::Unparseable { raw: raw.to_string() }),
    }
}

fn make_date(raw: &str, year: i32, month: u32, day: u32) -> Result<NaiveDate, DateError> {
    NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| DateError::Invalid { raw: raw.to_string(), day, month, year })
}

/// Рік без явного вказання — з `as_of` (02 §4): якщо дата з поточним роком виходить більше ніж
/// на ~6 місяців уперед від `as_of`, це насправді минулий рік.
fn infer_year_from_as_of(raw: &str, day: u32, month: u32, as_of: NaiveDate) -> Result<NaiveDate, DateError> {
    let this_year = make_date(raw, as_of.year(), month, day)?;
    let six_months_ahead = as_of + chrono::Months::new(6);
    if this_year > six_months_ahead {
        make_date(raw, as_of.year() - 1, month, day)
    } else {
        Ok(this_year)
    }
}

/// Одна дата (напр. окреме поле "Термін з", без пари) — рік з `as_of_date` подання.
pub fn parse_date(raw: &str, as_of: NaiveDate) -> Result<NaiveDate, DateError> {
    let (day, month, year) = split_token(raw)?;
    match year {
        Some(y) if y == as_of.year() => make_date(raw, y, month, day),
        Some(y) => Err(DateError::YearMismatch {
            raw: raw.to_string(),
            given_year: y,
            expected_year: as_of.year(),
        }),
        None => infer_year_from_as_of(raw, day, month, as_of),
    }
}

/// Дата "по" щодо вже відомої дати "з" (02 §4: "перехід через Новий рік: 15.12-20.01") — якщо
/// місяць кінця менший за місяць початку, рік автоматично зсувається на наступний.
pub fn parse_end_date(raw: &str, start: NaiveDate) -> Result<NaiveDate, DateError> {
    let (day, month, year) = split_token(raw)?;
    match year {
        Some(y) if y == start.year() || y == start.year() + 1 => make_date(raw, y, month, day),
        Some(y) => Err(DateError::YearMismatch {
            raw: raw.to_string(),
            given_year: y,
            expected_year: start.year(),
        }),
        None => {
            let year = if month < start.month() { start.year() + 1 } else { start.year() };
            make_date(raw, year, month, day)
        }
    }
}

/// Перевіряє, що період не перевернутий ("по" не раніше "з").
pub fn validate_period(start: NaiveDate, end: NaiveDate) -> Result<(), DateError> {
    if end < start {
        Err(DateError::EndBeforeStart { start, end })
    } else {
        Ok(())
    }
}

/// Один рядок-роздільник діапазону в одній клітинці (`18.08-09.10`, `18.08 – 09.10`,
/// `31.08 - 23.10\n`) — дефіс/тире різних видів, довільні пробіли навколо. `pub` — потрібен
/// `widgets::group_grid::date_range_cell`, щоб розкласти введене "З" саме на дві клітинки, не
/// переписуючи "З" повністю розв'язаною датою (яка потім НЕ пройшла б повторно свою ж перевірку
/// `parse_date`: явний рік, що відрізняється від `as_of.year()`, — завжди `YearMismatch`, навіть
/// якщо цей рік щойно сам вивів `infer_year_from_as_of`).
pub fn split_range(raw: &str) -> Option<(&str, &str)> {
    let trimmed = raw.trim();
    for sep in ['-', '–', '—'] {
        if let Some(idx) = trimmed.find(sep) {
            let (left, right) = trimmed.split_at(idx);
            let right = &right[sep.len_utf8()..];
            if !left.trim().is_empty() && !right.trim().is_empty() {
                return Some((left.trim(), right.trim()));
            }
        }
    }
    None
}

/// Поле "з" розуміє і одну дату, і повний діапазон в одній клітинці (02 §4) — тоді "по"
/// заповнюється автоматично. Повертає `(з, Some(по))`, якщо це був діапазон, інакше `(дата, None)`.
pub fn parse_maybe_range(
    raw: &str,
    as_of: NaiveDate,
) -> Result<(NaiveDate, Option<NaiveDate>), DateError> {
    if let Some((start_raw, end_raw)) = split_range(raw) {
        let start = parse_date(start_raw, as_of)?;
        let end = parse_end_date(end_raw, start)?;
        validate_period(start, end)?;
        Ok((start, Some(end)))
    } else {
        Ok((parse_date(raw, as_of)?, None))
    }
}

/// Результат живого форматування ОДНОГО поля дати під час набору (клавіатурний контракт сітки,
/// `docs/spec/components/grid-interaction.md` §3) — `display` завжди показується (навіть
/// незакінчений ввід), `error` — лише коли вже досить цифр, щоб напевно сказати "така дата не
/// існує" (не для незакінченого вводу).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveDateMask {
    pub display: String,
    pub error: Option<String>,
}

/// Автоматичні крапки день.місяць.рік у міру набору цифр — рік НЕОБОВ'ЯЗКОВИЙ (02 §4: дата без
/// року лишається легальною), перша цифра дня/місяця, що виключає другу цифру (день `>3`, місяць
/// `>1`), одразу автодоповнюється нулем (набрати "9" в день одразу дає "09", бо жоден день
/// 40-99 неможливий). `digits` — вже відфільтровані цифри (викликач сам вирізає нецифрові
/// символи з сирого вводу поля, той самий підхід, що `components::DatePicker::digits_only`) —
/// ця функція лише про ФОРМУ дд.мм.рррр, не про DOM/події.
pub fn format_date_mask(digits: &str) -> LiveDateMask {
    let digits: Vec<char> = digits.chars().filter(|c| c.is_ascii_digit()).take(8).collect();
    if digits.is_empty() {
        return LiveDateMask { display: String::new(), error: None };
    }

    let mut idx = 0;
    let day_str = {
        let d0 = digits[idx];
        if d0 > '3' {
            idx += 1;
            format!("0{d0}")
        } else if idx + 1 < digits.len() {
            let s = format!("{}{}", digits[idx], digits[idx + 1]);
            idx += 2;
            s
        } else {
            // Одна цифра дня, ще не завершено -- показати як є, без помилки.
            return LiveDateMask { display: digits[idx].to_string(), error: None };
        }
    };
    if idx >= digits.len() {
        return LiveDateMask { display: day_str, error: None };
    }

    let month_str = {
        let m0 = digits[idx];
        if m0 > '1' {
            idx += 1;
            format!("0{m0}")
        } else if idx + 1 < digits.len() {
            let s = format!("{}{}", digits[idx], digits[idx + 1]);
            idx += 2;
            s
        } else {
            return LiveDateMask { display: format!("{day_str}.{}", digits[idx]), error: None };
        }
    };

    let year_digits: String = digits[idx..].iter().collect();
    let display = if year_digits.is_empty() {
        format!("{day_str}.{month_str}.")
    } else if year_digits.len() == 2 {
        format!("{day_str}.{month_str}.20{year_digits}")
    } else {
        format!("{day_str}.{month_str}.{year_digits}")
    };

    let month_num: u32 = month_str.parse().unwrap_or(0);
    if !(1..=12).contains(&month_num) {
        return LiveDateMask {
            display,
            error: Some(format!("місяць «{month_str}» неможливий (01–12)")),
        };
    }
    let day_num: u32 = day_str.parse().unwrap_or(0);
    // Толерантно до року (лютий до 29) — точна високосність перевіряється нижче, щойно рік відомий.
    let max_day_before_year = match month_num {
        4 | 6 | 9 | 11 => 30,
        2 => 29,
        _ => 31,
    };
    if day_num == 0 || day_num > max_day_before_year {
        return LiveDateMask {
            display,
            error: Some(format!(
                "«{day_str}.{month_str}» — {} {} не існує",
                day_num,
                month_name(month_num)
            )),
        };
    }
    let year = match year_digits.len() {
        4 => year_digits.parse::<i32>().ok(),
        2 => year_digits.parse::<i32>().ok().map(|y| 2000 + y),
        _ => None,
    };
    if let Some(year) = year {
        if NaiveDate::from_ymd_opt(year, month_num, day_num).is_none() {
            return LiveDateMask {
                display,
                error: Some(format!("«{day_str}.{month_str}.{year}» — такої дати не існує")),
            };
        }
    }
    LiveDateMask { display, error: None }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    // as_of "станом на" — 2026-09-28, той самий, що в наскрізних сценаріях цієї сесії.
    fn as_of() -> NaiveDate {
        d(2026, 9, 28)
    }

    #[test]
    fn plain_day_month_infers_current_year() {
        assert_eq!(parse_date("18.08", as_of()).unwrap(), d(2026, 8, 18));
    }

    #[test]
    fn two_digit_year() {
        assert_eq!(parse_date("18.08.26", as_of()).unwrap(), d(2026, 8, 18));
    }

    #[test]
    fn trailing_dot_tolerated() {
        assert_eq!(parse_date("16.08.", as_of()).unwrap(), d(2026, 8, 16));
    }

    #[test]
    fn far_future_without_year_means_last_year() {
        // as_of = 2026-09-28; "15.02" без року дав би 2026-02-15 — це в МИНУЛОМУ відносно as_of,
        // тож рік лишається поточним (не "вилітає" на 6+ міс. уперед).
        assert_eq!(parse_date("15.02", as_of()).unwrap(), d(2026, 2, 15));
        // А дата, що явно "вилітає" більш ніж на ~6 міс. уперед від as_of у поточному році,
        // трактується як минулий рік: 28.09 + 6міс = 2027-03-28; "01.05" наступного циклу
        // при as_of=2026-09-28 дає кандидата 2026-05-01, що вже В МИНУЛОМУ (не спрацьовує тут) —
        // перевіримо межовий приклад явно з іншим as_of.
        let as_of_early = d(2026, 1, 10);
        // "15.08" з as_of=2026-01-10: кандидат 2026-08-15, це +7 міс. уперед — більше 6 міс.,
        // отже трактуємо як 2025-08-15 (минулий рік).
        assert_eq!(parse_date("15.08", as_of_early).unwrap(), d(2025, 8, 15));
    }

    #[test]
    fn invalid_day_in_month() {
        assert_eq!(
            parse_date("31.09", as_of()),
            Err(DateError::Invalid { raw: "31.09".to_string(), day: 31, month: 9, year: 2026 })
        );
        assert_eq!(
            parse_date("31.09", as_of()).unwrap_err().message(),
            "«31.09» — такої дати нема: вересень 2026 року має 30 днів, а вказано 31-е число"
        );
    }

    #[test]
    fn invalid_day_message_uses_correct_day_word_form() {
        // жовтень 2026 має 31 день (не "днів") — перевіряємо узгодження на межовому випадку.
        assert_eq!(
            parse_date("32.10", as_of()).unwrap_err().message(),
            "«32.10» — такої дати нема: жовтень 2026 року має 31 день, а вказано 32-е число"
        );
    }

    #[test]
    fn implausible_explicit_year() {
        assert_eq!(
            parse_date("11.07.2027", as_of()),
            Err(DateError::YearMismatch {
                raw: "11.07.2027".to_string(),
                given_year: 2027,
                expected_year: 2026
            })
        );
        assert_eq!(
            parse_date("11.07.2027", as_of()).unwrap_err().message(),
            "«11.07.2027» — рік 2027 не збігається з роком подання (2026); якщо дата справді за \
             2027 рік, перевірте поле «Станом на» вгорі"
        );
    }

    #[test]
    fn malformed_year_digit_count() {
        assert_eq!(
            parse_date("18.09.20216", as_of()),
            Err(DateError::MalformedYear {
                raw: "18.09.20216".to_string(),
                year_token: "20216".to_string()
            })
        );
        assert_eq!(
            parse_date("18.09.20216", as_of()).unwrap_err().message(),
            "«18.09.20216» — рік «20216» має бути 2 або 4 цифри (наприклад, 26 або 2026), а тут \
             5 цифр"
        );
    }

    #[test]
    fn unparseable_message_shows_expected_format() {
        assert_eq!(
            parse_date("абракадабра", as_of()).unwrap_err().message(),
            "«абракадабра» — не розпізнано як дату; очікую формат дд.мм або дд.мм.рррр \
             (наприклад, 15.09 або 15.09.2026)"
        );
    }

    #[test]
    fn range_same_year_no_rollover() {
        assert_eq!(
            parse_maybe_range("18.08-09.10", as_of()).unwrap(),
            (d(2026, 8, 18), Some(d(2026, 10, 9)))
        );
    }

    #[test]
    fn range_with_en_dash_and_spaces() {
        assert_eq!(
            parse_maybe_range("18.08 – 09.10", as_of()).unwrap(),
            (d(2026, 8, 18), Some(d(2026, 10, 9)))
        );
    }

    #[test]
    fn range_with_spaced_hyphen_and_trailing_newline() {
        assert_eq!(
            parse_maybe_range("31.08 - 23.10\n", as_of()).unwrap(),
            (d(2026, 8, 31), Some(d(2026, 10, 23)))
        );
    }

    #[test]
    fn range_crosses_new_year() {
        // 02 §4: "перехід через Новий рік: 15.12-20.01" — місяць кінця (01) < місяць початку (12).
        assert_eq!(
            parse_maybe_range("15.12-20.01", as_of()).unwrap(),
            (d(2026, 12, 15), Some(d(2027, 1, 20)))
        );
    }

    #[test]
    fn single_date_has_no_range_end() {
        assert_eq!(parse_maybe_range("18.08", as_of()).unwrap(), (d(2026, 8, 18), None));
    }

    #[test]
    fn end_before_start_is_rejected() {
        // Той самий місяць — рік-ролловер не спрацьовує (місяць кінця не менший за початок),
        // тож 05.08 раніше 18.08 — реальна помилка "по" раніше "з".
        assert_eq!(
            parse_maybe_range("18.08-05.08", as_of()),
            Err(DateError::EndBeforeStart { start: d(2026, 8, 18), end: d(2026, 8, 5) })
        );
        assert_eq!(
            parse_maybe_range("18.08-05.08", as_of()).unwrap_err().message(),
            "«по» (05.08.2026) раніше «з» (18.08.2026) — дати переплутані місцями, або в одній з \
             них помилка в місяці/році"
        );
    }

    // format_date_mask — живе форматування (grid-interaction.md §3), сценарії з брифу користувача.

    #[test]
    fn mask_single_digit_day_waits_for_second() {
        assert_eq!(format_date_mask("1"), LiveDateMask { display: "1".to_string(), error: None });
    }

    #[test]
    fn mask_day_gt3_first_digit_auto_pads() {
        // "9" як перша цифра дня -- жоден день 90-99 неможливий, одразу "09"; "1" як перша цифра
        // місяця (не >1) чекає другу цифру -- показуємо як набрано.
        assert_eq!(format_date_mask("91"), LiveDateMask { display: "09.1".to_string(), error: None });
    }

    #[test]
    fn mask_month_gt1_first_digit_auto_pads() {
        assert_eq!(format_date_mask("189"), LiveDateMask { display: "18.09.".to_string(), error: None });
    }

    #[test]
    fn mask_day_month_complete_no_year_is_valid_partial() {
        assert_eq!(format_date_mask("1808"), LiveDateMask { display: "18.08.".to_string(), error: None });
    }

    #[test]
    fn mask_two_digit_year_gets_20_prefix() {
        // "180826" -- день 18, місяць 08, рік-цифри "26" -> "2026".
        assert_eq!(
            format_date_mask("180826"),
            LiveDateMask { display: "18.08.2026".to_string(), error: None }
        );
    }

    #[test]
    fn mask_four_digit_year_passthrough() {
        assert_eq!(
            format_date_mask("18082026"),
            LiveDateMask { display: "18.08.2026".to_string(), error: None }
        );
    }

    #[test]
    fn mask_day_31_in_30_day_month_is_error_even_without_year() {
        // "3104" -- 31 квітня, квітень має 30 днів -- помилка одразу, рік не потрібен.
        let result = format_date_mask("3104");
        assert_eq!(result.display, "31.04.");
        assert!(result.error.is_some(), "31.04 має бути помилкою: {result:?}");
    }

    #[test]
    fn mask_feb_29_non_leap_year_is_error() {
        let result = format_date_mask("290227"); // 29.02.2027 -- 2027 не високосний
        assert!(result.error.is_some(), "29.02.2027 має бути помилкою: {result:?}");
    }

    #[test]
    fn mask_feb_29_leap_year_is_ok() {
        let result = format_date_mask("290228"); // 29.02.2028 -- високосний
        assert_eq!(result.error, None, "29.02.2028 має бути коректною: {result:?}");
        assert_eq!(result.display, "29.02.2028");
    }

    #[test]
    fn mask_month_13_is_impossible() {
        // "0113" -- день 01, місяць 13 -- неможливо.
        let result = format_date_mask("0113");
        assert!(result.error.is_some(), "місяць 13 має бути помилкою: {result:?}");
    }

    #[test]
    fn mask_empty_digits_is_empty_display_no_error() {
        assert_eq!(format_date_mask(""), LiveDateMask { display: String::new(), error: None });
    }
}
