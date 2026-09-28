//! Нормалізація сирих значень (частини, місця, ВОС, посади, ОВТ, курси) для порівняння
//! й нечіткого пошуку. Компілюється і для SSR, і для WASM (без sea-orm/tokio/fs) —
//! та сама функція для валідації в формі введення (02) і для превʼю імпорту (03).

/// Латинські гліфи, які клавіатура/OCR плутають з кириличними в номерах і назвах частин
/// (рішення: [[normalize-direction-one-way]] — лише цей напрямок, не навпаки).
const LATIN_TO_CYRILLIC: &[(char, char)] = &[
    ('a', 'а'),
    ('t', 'т'),
    ('o', 'о'),
    ('p', 'р'),
    ('c', 'с'),
    ('e', 'е'),
    ('k', 'к'),
    ('m', 'м'),
    ('h', 'н'),
    ('b', 'в'),
    ('x', 'х'),
];

fn latin_to_cyrillic(ch: char) -> char {
    let lower = ch.to_ascii_lowercase();
    LATIN_TO_CYRILLIC
        .iter()
        .find(|(l, _)| *l == lower)
        .map(|(_, c)| *c)
        .unwrap_or(ch)
}

/// Нормальна форма сирого значення: нижній регістр, латинські двійники → кирилиця,
/// прибрані лапки/дужки, уніфікований дефіс/тире, згорнуті пробіли, "в/с"≈"вс"≈"в/сл".
pub fn normalize(raw: &str) -> String {
    let lower = raw.to_lowercase();

    // "в/с" / "в/сл" — уніфікуємо до "вс" ДО видалення інших спецсимволів,
    // щоб не втратити межу слова після прибирання "/".
    let unified_vs = lower.replace("в/сл", "вс").replace("в/с", "вс");

    let mut out = String::with_capacity(unified_vs.len());
    let mut last_was_space = true; // щоб не почати результат з пробілу
    for ch in unified_vs.chars() {
        let mapped = match ch {
            // лапки всіх видів, дужки — прибираємо повністю
            '«' | '»' | '"' | '“' | '”' | '\'' | '’' | '(' | ')' => continue,
            // дефіс/тире (-, –, —) уніфікуємо в звичайний дефіс
            '-' | '–' | '—' => '-',
            _ => latin_to_cyrillic(ch),
        };

        if mapped.is_whitespace() {
            if last_was_space {
                continue;
            }
            out.push(' ');
            last_was_space = true;
        } else {
            out.push(mapped);
            last_was_space = false;
        }
    }

    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::normalize;

    // Табличні тести на реальних граблях із docs/source-analysis.md / docs/spec/01-domain-model.md.
    #[test]
    fn real_world_footguns() {
        let cases: &[(&str, &str)] = &[
            // латинські двійники в номері частини (OCR/клавіатура)
            ("A4076", "а4076"),
            ("а4076", "а4076"),
            ("A1556", "а1556"),
            // лапки/дужки різних видів навколо назви
            ("«ОТУ Одеса»", "оту одеса"),
            ("ОТУ «Одеса»", "оту одеса"),
            ("\"ОТУ Одеса\"", "оту одеса"),
            ("241 обр ТрО\n(А4076)", "241 обр тро а4076"),
            // дефіс/тире уніфікуються
            ("18.08–09.10", "18.08-09.10"),
            ("18.08—09.10", "18.08-09.10"),
            // "в/с" варіанти
            ("в/сл", "вс"),
            ("в/с", "вс"),
            ("22 в/с", "22 вс"),
            // зайві пробіли й перенос рядка згортаються в один пробіл
            ("155   нц", "155 нц"),
            ("179 онтц\n (179 онтц)", "179 онтц 179 онтц"),
        ];

        for (input, expected) in cases {
            assert_eq!(&normalize(input), expected, "normalize({input:?})");
        }
    }

    #[test]
    fn idempotent_on_already_normalized() {
        assert_eq!(normalize("а4076"), "а4076");
    }
}
