//! Мікро-прототип стильової системи (Фаза 0, `docs/spec/08-style-system.md`). Без залежностей
//! (окрім `leptos` у dev-залежностях для прототипу кнопки) — токени й граматика атомів як чисті
//! Rust-дані, генератор CSS з них. `style_macros::cx!` валідує рядки атомів проти `ATOMS` тут.
//!
//! Обсяг навмисно малий (10 токенів, 20 атомів) — Фаза 0 лише перевіряє архітектуру, повна
//! граматика й теми — Фаза 1.

/// Примітивний токен: ім'я (без `--`), CSS-значення. `--space-*` — відступи, `--color-*` — палітра
/// (виміряно зі striy.pp.ua, `.claude/decisions/ui-visual-style-source.md` — НЕ вигадано заново).
pub struct Token {
    pub name: &'static str,
    pub value: &'static str,
}

pub const TOKENS: &[Token] = &[
    Token { name: "space-0", value: "0" },
    Token { name: "space-1", value: "4px" },
    Token { name: "space-2", value: "8px" },
    Token { name: "space-3", value: "12px" },
    Token { name: "space-4", value: "16px" },
    Token { name: "color-bg-darkest", value: "#0d0f0a" },
    Token { name: "color-panel", value: "#171912" },
    Token { name: "color-text-main", value: "#e8e4d8" },
    Token { name: "color-text-muted", value: "#b9b4a6" },
    Token { name: "color-accent", value: "#c9a84c" },
    Token { name: "radius-1", value: "4px" },
];

/// Один атом: ім'я (те, що пишуть у `cx!(...)`) → тіло CSS-декларації (без селектора). Значення
/// атома — ЛИШЕ через `var(--токен)`, ніколи літерал (07-code-structure §-подібне правило для
/// цієї системи: перевіряється архітектурним тестом у Фазі 4, тут — просто дотримано вручну).
pub struct Atom {
    pub name: &'static str,
    pub decl: &'static str,
}

pub const ATOMS: &[Atom] = &[
    Atom { name: "p0", decl: "padding: var(--space-0)" },
    Atom { name: "p1", decl: "padding: var(--space-1)" },
    Atom { name: "p2", decl: "padding: var(--space-2)" },
    Atom { name: "p3", decl: "padding: var(--space-3)" },
    Atom { name: "p4", decl: "padding: var(--space-4)" },
    Atom { name: "gap0", decl: "gap: var(--space-0)" },
    Atom { name: "gap1", decl: "gap: var(--space-1)" },
    Atom { name: "gap2", decl: "gap: var(--space-2)" },
    Atom { name: "gap3", decl: "gap: var(--space-3)" },
    Atom { name: "gap4", decl: "gap: var(--space-4)" },
    Atom { name: "flex", decl: "display: flex" },
    Atom { name: "col", decl: "flex-direction: column" },
    Atom { name: "items-c", decl: "align-items: center" },
    Atom { name: "justify-b", decl: "justify-content: space-between" },
    Atom { name: "fg-main", decl: "color: var(--color-text-main)" },
    Atom { name: "fg-muted", decl: "color: var(--color-text-muted)" },
    Atom { name: "fg-accent", decl: "color: var(--color-accent)" },
    Atom { name: "bg-panel", decl: "background: var(--color-panel)" },
    Atom { name: "r0", decl: "border-radius: 0" },
    Atom { name: "r1", decl: "border-radius: var(--radius-1)" },
];

/// Найближчий атом за Левенштейном — для підказки в помилці `cx!` (style_macros дублює свою
/// власну копію `ATOMS`-імен для валідації в proc-macro-контексті, де `style` як залежність
/// теж доступна, але підказку рахує так само; тут — та сама функція для генератора/тестів).
pub fn closest_atom(unknown: &str) -> Option<&'static str> {
    ATOMS
        .iter()
        .map(|a| (a.name, levenshtein(unknown, a.name)))
        .min_by_key(|(_, d)| *d)
        .map(|(name, _)| name)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut prev = row[0];
        row[0] = i;
        for j in 1..=b.len() {
            let tmp = row[j];
            row[j] = if a[i - 1] == b[j - 1] {
                prev
            } else {
                1 + prev.min(row[j]).min(row[j - 1])
            };
            prev = tmp;
        }
    }
    row[b.len()]
}

/// Генерує повний CSS-артефакт: `@layer tokens` (custom properties в `:root`) + `@layer atoms`
/// (один клас на атом). Прототип не має `@layer reset/base/components/overrides` — Фаза 1.
pub fn generate_css() -> String {
    let mut out = String::new();
    out.push_str("@layer tokens, atoms;\n\n");

    out.push_str("@layer tokens {\n  :root {\n");
    for t in TOKENS {
        out.push_str(&format!("    --{}: {};\n", t.name, t.value));
    }
    out.push_str("  }\n}\n\n");

    out.push_str("@layer atoms {\n");
    for a in ATOMS {
        out.push_str(&format!("  .{} {{ {}; }}\n", escape_class(a.name), a.decl));
    }
    out.push_str("}\n");

    out
}

/// Екранує символи, недопустимі в голому CSS-селекторі класу (Фаза 1 матиме варіанти виду
/// `hover:bg-panel` → клас `.hover\:bg-panel:hover` — тут ще не потрібно, атоми без варіантів).
fn escape_class(name: &str) -> String {
    name.replace(':', "\\:")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_count_matches_prototype_scope() {
        // 11, не рівно 10: перша версія мала "r1" з буквальним 4px (без токена) — власний тест
        // "усе через var()" це впіймав, довелось додати radius-1. Лишаю як є, не підганяю назад
        // до 10 — доказ, що правило справді щось ловить, цінніший за круге число.
        assert_eq!(TOKENS.len(), 11, "Фаза 0: ~10 токенів у мікро-прототипі (11 після радіус-фіксу)");
    }

    #[test]
    fn atom_count_matches_prototype_scope() {
        assert_eq!(ATOMS.len(), 20, "Фаза 0: рівно 20 атомів у мікро-прототипі");
    }

    #[test]
    fn every_atom_decl_references_a_token_var_or_is_a_bare_keyword() {
        // Атоми без токена (display:flex тощо) — легітимно бо не мають "значення зі шкали";
        // "0" теж легітимний буквально (сам токен space-0 — теж "0", жодна шкала не токенізує
        // нуль окремо); атоми, що МАЮТЬ ненульове числове/кольорове значення, йдуть через var().
        let bare_keyword_atoms = ["flex", "col", "items-c", "justify-b", "r0"];
        for a in ATOMS {
            if bare_keyword_atoms.contains(&a.name) {
                continue;
            }
            assert!(
                a.decl.contains("var(--"),
                "атом «{}» має нетокенізоване значення: {}",
                a.name,
                a.decl
            );
        }
    }

    #[test]
    fn generated_css_contains_every_token_and_atom() {
        let css = generate_css();
        for t in TOKENS {
            assert!(css.contains(&format!("--{}: {}", t.name, t.value)), "токен {} відсутній у CSS", t.name);
        }
        for a in ATOMS {
            assert!(
                css.contains(&format!(".{} {{", escape_class(a.name))),
                "атом {} відсутній у CSS",
                a.name
            );
        }
    }

    #[test]
    fn closest_atom_suggests_p3_for_typo_p33() {
        assert_eq!(closest_atom("p33"), Some("p3"));
    }
}
