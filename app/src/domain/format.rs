//! Форматування чисел для текстових звітів (05 §D3 "Говорілка", §D4): розділювач тисяч —
//! нерозривний пробіл, зміни — зі знаком, мінус — справжній "−" (U+2212), не дефіс.

const NBSP: char = '\u{00A0}';
const MINUS: char = '\u{2212}';

pub fn thousands(n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    if n < 0 {
        out.push(MINUS);
    }
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(NBSP);
        }
        out.push(ch);
    }
    out
}

/// `+6`, `−27`, `0` (нуль — без знака).
pub fn signed_delta(n: i64) -> String {
    match n {
        0 => "0".to_string(),
        n if n > 0 => format!("+{}", thousands(n)),
        n => thousands(n),
    }
}

/// Відсоток цілим, округлення half-up; `part`/`whole` — кількості. `whole <= 0` → `0%`.
pub fn percent(part: i64, whole: i64) -> String {
    if whole <= 0 {
        return "0%".to_string();
    }
    let p = (part * 200 + whole) / (whole * 2);
    format!("{p}%")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thousands_groups_with_nbsp() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1000), "1\u{A0}000");
        assert_eq!(thousands(1234567), "1\u{A0}234\u{A0}567");
        assert_eq!(thousands(-27), "\u{2212}27");
        assert_eq!(thousands(-1500), "\u{2212}1\u{A0}500");
    }

    #[test]
    fn signed_delta_uses_real_minus() {
        assert_eq!(signed_delta(6), "+6");
        assert_eq!(signed_delta(-27), "\u{2212}27");
        assert_eq!(signed_delta(74), "+74");
        assert_eq!(signed_delta(0), "0");
    }

    #[test]
    fn percent_rounds_half_up() {
        assert_eq!(percent(1, 3), "33%");
        assert_eq!(percent(1, 2), "50%");
        assert_eq!(percent(2, 3), "67%");
        assert_eq!(percent(1, 8), "13%");
        assert_eq!(percent(5, 0), "0%");
    }
}
