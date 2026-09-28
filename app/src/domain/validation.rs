//! Валідація в реальному часі (03 §5, 02 §5) — та сама функція для форми введення (WASM, без
//! запиту до сервера) і превʼю імпорту. Тут — лише перевірки, що не потребують БД (кількості,
//! ПІБ-подібний текст); "ВОС не з довідника", "частина не розпізнана" тощо — перевіряються в
//! `backend::repo` (потребують запиту), не тут.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountError {
    /// Кількість не ціле невід'ємне число (03 §5).
    Negative,
    /// "Навчаються" > "Прибуло" > "План" — порядок воронки порушено (03 §5): на кожному кроці
    /// людей може лише убувати, не прибувати.
    FunnelOrderViolated,
}

/// Одна кількість (План/Прибуло/Навчаються) — ціле, невід'ємне.
pub fn validate_count(value: i64) -> Result<(), CountError> {
    if value < 0 {
        Err(CountError::Negative)
    } else {
        Ok(())
    }
}

/// Порядок воронки при первинному внесенні групи (02 §1, колонка 7: "Кількість (план) → Прибуло
/// → Навчаються") — `план ≥ прибуло ≥ навчаються`, кожен крок лише зменшує кількість.
pub fn validate_funnel_order(planned: i64, arrived: i64, in_training: i64) -> Result<(), CountError> {
    validate_count(planned)?;
    validate_count(arrived)?;
    validate_count(in_training)?;
    if arrived > planned || in_training > arrived {
        Err(CountError::FunnelOrderViolated)
    } else {
        Ok(())
    }
}

/// ПІБ-подібний текст (03 §5, 01 §1: людей поіменно не зберігаємо) — типовий патерн у поіменних
/// джерелах: "ПРІЗВИЩЕ Імʼя" (перше слово — великими літерами, друге — з великої, решта малі).
/// Евристика, не гарантія: мета — попередити, не мовчки пропустити, а не ідеально розпізнати.
pub fn looks_like_personal_name(text: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    for pair in words.windows(2) {
        let [surname, given] = pair else { continue };
        if is_all_uppercase_word(surname) && is_capitalized_word(given) {
            return true;
        }
    }
    false
}

fn is_all_uppercase_word(word: &str) -> bool {
    let letters: Vec<char> = word.chars().filter(|c| c.is_alphabetic()).collect();
    letters.len() >= 3 && letters.iter().all(|c| c.is_uppercase())
}

fn is_capitalized_word(word: &str) -> bool {
    let mut chars = word.chars().filter(|c| c.is_alphabetic());
    let Some(first) = chars.next() else { return false };
    first.is_uppercase() && chars.all(|c| c.is_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_rejects_negative() {
        assert_eq!(validate_count(-1), Err(CountError::Negative));
        assert_eq!(validate_count(0), Ok(()));
        assert_eq!(validate_count(12), Ok(()));
    }

    #[test]
    fn funnel_order_accepts_non_increasing() {
        assert_eq!(validate_funnel_order(20, 20, 20), Ok(()));
        assert_eq!(validate_funnel_order(20, 18, 17), Ok(()));
    }

    #[test]
    fn funnel_order_rejects_arrived_over_planned() {
        assert_eq!(validate_funnel_order(10, 12, 5), Err(CountError::FunnelOrderViolated));
    }

    #[test]
    fn funnel_order_rejects_in_training_over_arrived() {
        assert_eq!(validate_funnel_order(20, 15, 18), Err(CountError::FunnelOrderViolated));
    }

    #[test]
    fn funnel_order_rejects_negative_component() {
        assert_eq!(validate_funnel_order(-1, 0, 0), Err(CountError::Negative));
    }

    #[test]
    fn detects_surname_caps_plus_given_name() {
        assert!(looks_like_personal_name("ІВАНЕНКО Петро проходить курс"));
        assert!(looks_like_personal_name("сержант ПЕТРЕНКО Олег"));
    }

    #[test]
    fn plain_unit_text_is_not_flagged() {
        assert!(!looks_like_personal_name("241 обр ТрО (А4076)"));
        assert!(!looks_like_personal_name("Зовнішній пілот (оператор) БпЛА"));
        assert!(!looks_like_personal_name("КІБР"));
    }
}
