//! Стильова система, Фаза 1 (`docs/spec/08-style-system.md`) — повна граматика, токени, теми,
//! генератор CSS. Без залежностей у рантаймі (компілюється нативно й у wasm32); `leptos`/
//! `style_macros` — лише `[dev-dependencies]` для тестів/прикладів.
//!
//! Дані й комбінаторна логіка граматики — у `grammar_data.rs` (`include!`, СПІЛЬНИЙ з
//! `style_macros`, textual include не crate-залежність — вирішує Фаза-0-обмеження "дубльована
//! копія ATOMS", `.claude/decisions/style-system-architecture.md`).
//!
//! `is_valid_atom` використовується лише в `#[cfg(test)]` (генератор довіряє `all_atom_names()`,
//! яка сама будує список правильно) — не мертвий код по суті, лише не в non-test збірці.
#![allow(dead_code)]

include!("../grammar_data.rs");

mod contrast;

/// Медіа-брейкпоінти для `md:`/`lg:` (min-width). `@md:` — container query, значення нижче.
const BREAKPOINT_MD: &str = "768px";
const BREAKPOINT_LG: &str = "1024px";
const CONTAINER_MD: &str = "480px";

/// Найближчий атом за Левенштейном — підказка для документації/довідки поза компілятором
/// (сам `cx!` рахує підказку незалежно, у proc-macro-контексті, той самий алгоритм).
pub fn closest_atom(unknown: &str) -> Option<String> {
    all_atom_names()
        .into_iter()
        .min_by_key(|name| levenshtein(unknown, name))
}

/// Усі базові імена атомів — для `/styleguide` (Фаза 2), який показує весь перелік динамічно
/// (не літералами `cx!`, тому без макро-валідації — саме тому existence-тест на неї не спирається).
pub fn all_atoms() -> Vec<String> {
    all_atom_names()
}

/// Імена тем (`"night"`, `"day"`, `"print"`) — для перемикача тем на `/styleguide`.
pub fn theme_names() -> Vec<&'static str> {
    THEMES.iter().map(|t| t.name).collect()
}

/// Шпаргалка атомів (≤80 рядків, токен-економія для агента — див. другий pasted_content брифу)
/// — будується З ТИХ САМИХ таблиць `grammar_data.rs`, тому додавання нового сімейства/ключового
/// слова автоматично зʼявляється тут (і ламає `atoms_md_matches_committed_file`, якщо забули
/// перегенерувати `ATOMS.md`).
pub fn generate_atoms_md() -> String {
    let mut out = String::new();
    out.push_str("# ATOMS.md — шпаргалка атомів\n\n");
    out.push_str("АВТО-ЗГЕНЕРОВАНО з `grammar_data.rs` — НЕ РЕДАГУВАТИ ВРУЧНУ, `cargo run -p style --bin gen`.\n");
    out.push_str("Повний опис/приклади/do-don't — `docs/spec/08-style-system.md`.\n\n");
    out.push_str("Клас = `cx!(\"атом атом ...\")` — компілятор валідує кожен, підказує typo.\n\n");

    out.push_str("## Відступи (шкала 0..8 = 0/4/8/12/16/24/32/48/64px)\n\n");
    let families: Vec<&str> = SPACING_FAMILIES.iter().map(|(p, _)| *p).collect();
    out.push_str(&format!(
        "`{}` — кожен як `{{префікс}}0`..`{{префікс}}8`, напр. `p2` = padding 8px\n\n",
        families.join("` `")
    ));

    out.push_str("## Шкали без варіацій властивості\n\n");
    out.push_str(&format!(
        "- Радіус: {}\n",
        (0..RADIUS_SCALE.len())
            .map(|i| format!("`r{i}`"))
            .collect::<Vec<_>>()
            .join(" ")
    ));
    out.push_str(&format!(
        "- Letter-spacing: {}\n",
        (0..TRACK_SCALE.len())
            .map(|i| format!("`track{i}`"))
            .collect::<Vec<_>>()
            .join(" ")
    ));
    out.push_str(&format!(
        "- Текст: {}\n",
        TEXT_SCALE
            .iter()
            .map(|(n, _, _)| format!("`t-{n}`"))
            .collect::<Vec<_>>()
            .join(" ")
    ));
    out.push_str(&format!(
        "- Тінь: {}\n",
        SHADOW_SCALE
            .iter()
            .map(|(n, _)| format!("`shadow{n}`"))
            .collect::<Vec<_>>()
            .join(" ")
    ));
    out.push_str(&format!(
        "- Z-індекс: {}\n",
        Z_SCALE
            .iter()
            .map(|(n, _)| format!("`z-{n}`"))
            .collect::<Vec<_>>()
            .join(" ")
    ));
    out.push_str(&format!(
        "- Тривалість: {}\n\n",
        DURATION_SCALE
            .iter()
            .map(|(n, _)| format!("`duration-{n}`"))
            .collect::<Vec<_>>()
            .join(" ")
    ));

    out.push_str("## Ключові слова (без шкали)\n\n");
    let keywords: Vec<String> = KEYWORD_ATOMS
        .iter()
        .map(|(k, _)| format!("`{k}`"))
        .collect();
    for chunk in keywords.chunks(7) {
        out.push_str(&chunk.join(" "));
        out.push('\n');
    }
    out.push_str("`bracket` (кутові скоби, тактичний мотив)\n\n");

    out.push_str("## Варіанти (префікс перед `:`)\n\n");
    out.push_str(&format!(
        "{}\n",
        VARIANTS
            .iter()
            .map(|v| format!("`{v}:`"))
            .collect::<Vec<_>>()
            .join(" ")
    ));
    out.push_str(
        "Приклад: `hover:bg-raised`. `md:`/`lg:` — min-width медіа; `@md:` — container query.\n\n",
    );

    out.push_str("## Теми\n\n");
    out.push_str(&format!(
        "{} — `<html data-theme=\"...\">`, SSR виставляє атрибут (без блимання).\n\n",
        THEMES
            .iter()
            .map(|t| t.name)
            .collect::<Vec<_>>()
            .join(" · ")
    ));

    out.push_str("## Примітивні кольори (НЕ вживати напряму — лише через `fg-`/`bg-`/`bd-`)\n\n");
    let mut families_seen = Vec::new();
    for (name, _) in COLOR_TOKENS {
        let family = name
            .trim_end_matches(|c: char| c.is_ascii_digit())
            .trim_end_matches('-');
        if !families_seen.contains(&family) {
            families_seen.push(family);
        }
    }
    out.push_str(
        &families_seen
            .iter()
            .map(|f| format!("`{f}-N`"))
            .collect::<Vec<_>>()
            .join(" "),
    );
    out.push('\n');

    out
}

fn escape_class(name: &str) -> String {
    name.replace(':', "\\:").replace('@', "\\@")
}

/// Екранує селектор атома з варіант-префіксом і повертає (селектор-хвіст, обгортка). Обгортка —
/// `None` для псевдокласів (просто дописується до селектора), `Some(media/container query)` для
/// `md`/`lg`/`@md`.
fn variant_wrap(variant: &str, class_selector: &str) -> (String, Option<String>) {
    match variant {
        "hover" => (format!("{class_selector}:hover"), None),
        "focus-visible" => (format!("{class_selector}:focus-visible"), None),
        "active" => (format!("{class_selector}:active"), None),
        "disabled" => (format!("{class_selector}:disabled"), None),
        "invalid" => (format!("{class_selector}[aria-invalid=\"true\"]"), None),
        "open" => (format!("{class_selector}[data-state=\"open\"]"), None),
        "md" => (
            class_selector.to_string(),
            Some(format!("@media (min-width: {BREAKPOINT_MD})")),
        ),
        "lg" => (
            class_selector.to_string(),
            Some(format!("@media (min-width: {BREAKPOINT_LG})")),
        ),
        "@md" => (
            class_selector.to_string(),
            Some(format!("@container (min-width: {CONTAINER_MD})")),
        ),
        _ => (class_selector.to_string(), None),
    }
}

/// Генерує повний CSS-артефакт: `@layer reset, tokens, base, components, atoms, overrides;` +
/// вміст кожного шару. `components` лишається порожнім тут (рецепти пишуть свій CSS, Фаза 3) —
/// шар оголошений заради порядку каскаду (атоми ГАРАНТОВАНО переможуть стилі рецептів).
pub fn generate_css() -> String {
    let mut out = String::new();
    out.push_str("@layer reset, tokens, base, components, atoms, overrides;\n\n");

    // --- reset: мінімальний, не Pico-рівня (Фаза 3 вирішить, чи потрібно більше) ---
    out.push_str("@layer reset {\n");
    out.push_str("  *, *::before, *::after { box-sizing: border-box; }\n");
    out.push_str("  body { margin: 0; }\n");
    out.push_str("}\n\n");

    // --- tokens: примітивні (незалежні від теми) + семантичні (per-тема, [data-theme]) ---
    out.push_str("@layer tokens {\n");
    out.push_str("  :root {\n");
    for (i, v) in SPACE_SCALE.iter().enumerate() {
        out.push_str(&format!("    --space-{i}: {v};\n"));
    }
    for (i, v) in RADIUS_SCALE.iter().enumerate() {
        out.push_str(&format!("    --radius-{i}: {v};\n"));
    }
    for (i, v) in TRACK_SCALE.iter().enumerate() {
        out.push_str(&format!("    --track-{i}: {v};\n"));
    }
    for (name, v) in COLOR_TOKENS {
        out.push_str(&format!("    --{name}: {v};\n"));
    }
    for (name, size, line) in TEXT_SCALE {
        out.push_str(&format!("    --text-{name}-size: {size};\n"));
        out.push_str(&format!("    --text-{name}-line: {line};\n"));
    }
    for (name, v) in WEIGHT_SCALE {
        out.push_str(&format!("    --weight-{name}: {v};\n"));
    }
    for (name, v) in SHADOW_SCALE {
        out.push_str(&format!("    --shadow-{name}: {v};\n"));
    }
    for (name, v) in Z_SCALE {
        out.push_str(&format!("    --z-{name}: {v};\n"));
    }
    for (name, v) in DURATION_SCALE {
        out.push_str(&format!("    --duration-{name}: {v};\n"));
    }
    out.push_str("    --font-heading: \"Oswald\", \"Arial Narrow\", sans-serif;\n");
    out.push_str("    --font-body: \"Roboto\", \"Segoe UI\", sans-serif;\n");
    out.push_str("  }\n\n");

    for theme in THEMES {
        // "night" — типова тема, і на :root напряму (без атрибута), і на [data-theme="night"]
        // (SSR завжди виставляє атрибут — 08 §4 "жодного блимання" — але :root-фолбек рятує,
        // якщо атрибут ще не встиг застосуватись до першого фарбування).
        let selector = if theme.name == "night" {
            ":root, :root[data-theme=\"night\"]".to_string()
        } else {
            format!(":root[data-theme=\"{}\"]", theme.name)
        };
        out.push_str(&format!("  {selector} {{\n"));
        for (name, v) in theme.pairs {
            out.push_str(&format!("    --{name}: {v};\n"));
        }
        out.push_str("  }\n\n");
    }
    out.push_str("}\n\n");

    // --- base: типографіка за замовчуванням для голого HTML (Pico-подібний мінімум) ---
    out.push_str("@layer base {\n");
    out.push_str(
        "  body { background: var(--surface-base); color: var(--fg-main); font-family: var(--font-body); font-size: var(--text-md-size); line-height: var(--text-md-line); }\n",
    );
    out.push_str(
        "  h1, h2, h3 { font-family: var(--font-heading); font-weight: var(--weight-7); }\n",
    );
    out.push_str("  @media (prefers-reduced-motion: reduce) {\n");
    for (name, _) in DURATION_SCALE {
        out.push_str(&format!("    :root {{ --duration-{name}: 0ms; }}\n"));
    }
    out.push_str("  }\n");
    out.push_str("}\n\n");

    // --- components: порожньо тут навмисно (рецепти — Фаза 3), лише оголошено в @layer вище ---

    // --- atoms: базові + варіант-префіксовані правила ---
    out.push_str("@layer atoms {\n");
    for name in all_atom_names() {
        let Some(decl) = atom_declaration(&name) else {
            continue;
        };
        out.push_str(&format!("  .{} {{ {decl}; }}\n", escape_class(&name)));
        if name == "bracket" {
            // Кутові скоби — тактичний мотив (08 §5): псевдоелементи в кутах, золота лінія.
            out.push_str(&format!(
                "  .{cls}::before, .{cls}::after {{ content: \"\"; position: absolute; width: 10px; height: 10px; border: 2px solid var(--border-strong); }}\n",
                cls = escape_class(&name)
            ));
            out.push_str(&format!(
                "  .{cls}::before {{ top: -1px; left: -1px; border-right: none; border-bottom: none; }}\n",
                cls = escape_class(&name)
            ));
            out.push_str(&format!(
                "  .{cls}::after {{ bottom: -1px; right: -1px; border-left: none; border-top: none; }}\n",
                cls = escape_class(&name)
            ));
        }
    }
    for variant in VARIANTS {
        let (mut media_rules, mut plain_rules) = (String::new(), String::new());
        for name in all_atom_names() {
            let Some(decl) = atom_declaration(&name) else {
                continue;
            };
            let base_selector = format!(".{}\\:{}", escape_class(variant), escape_class(&name));
            let (selector, wrapper) = variant_wrap(variant, &base_selector);
            let rule = format!("  {selector} {{ {decl}; }}\n");
            match wrapper {
                Some(at_rule) => {
                    media_rules.push_str(&format!("  {at_rule} {{\n  {rule}  }}\n"));
                }
                None => plain_rules.push_str(&rule),
            }
        }
        out.push_str(&plain_rules);
        out.push_str(&media_rules);
    }
    out.push_str("}\n\n");

    // --- overrides: порожньо (свідомо останній шар каскаду для екстрених винятків) ---
    out.push_str("@layer overrides {\n}\n");

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_keyword_atom_or_scale_atom_resolves_to_a_declaration() {
        for name in all_atom_names() {
            assert!(
                atom_declaration(&name).is_some(),
                "атом «{name}» без декларації"
            );
        }
    }

    #[test]
    fn is_valid_atom_agrees_with_all_atom_names() {
        for name in all_atom_names() {
            assert!(
                is_valid_atom(&name),
                "«{name}» є в переліку, але is_valid_atom каже ні"
            );
        }
        assert!(
            !is_valid_atom("p99"),
            "p99 поза шкалою space (0..8) не має бути валідним"
        );
        assert!(
            !is_valid_atom("totally-made-up"),
            "вигадане ім'я не має бути валідним"
        );
    }

    #[test]
    fn every_atom_decl_references_a_token_var_or_is_a_documented_bare_value() {
        // "0" (нуль) і сирі числа для z-index — легітимні буквали (шкала сама токенізована в
        // :root, але Z_SCALE-значення "20"/"50" тощо навмисно НЕ через var() у власному
        // визначенні — вони ВИЗНАЧАЮТЬ токен, не споживають його). Атоми (не токени) мають
        // споживати ЛИШЕ var(--...) або бути чистим keyword без шкали.
        let scaleless_keywords: Vec<&str> = KEYWORD_ATOMS
            .iter()
            .map(|(k, _)| *k)
            .filter(|k| !k.starts_with("fg-") && !k.starts_with("bg-") && !k.starts_with("fw"))
            .collect();
        for name in all_atom_names() {
            let decl = atom_declaration(&name).unwrap();
            if scaleless_keywords.contains(&name.as_str()) || name == "bracket" {
                continue;
            }
            assert!(
                decl.contains("var(--"),
                "атом «{name}» має нетокенізоване значення: {decl}"
            );
        }
    }

    #[test]
    fn generate_css_contains_every_theme_and_a_reasonable_atom_sample() {
        let css = generate_css();
        for theme in THEMES {
            assert!(
                css.contains(&format!("data-theme=\"{}\"", theme.name))
                    || (theme.name == "night"
                        && css.contains(":root, :root[data-theme=\"night\"]")),
                "тема «{}» відсутня в CSS",
                theme.name
            );
        }
        for atom in [
            "p4",
            "gap2",
            "t-lg",
            "r1",
            "shadow1",
            "z-modal",
            "duration-base",
            "bracket",
        ] {
            assert!(
                css.contains(&format!(".{atom} {{")),
                "атом «{atom}» відсутній у generate_css()"
            );
        }
        assert!(
            css.contains(".hover\\:bg-panel:hover"),
            "hover-варіант bg-panel відсутній"
        );
        assert!(
            css.contains("@media (min-width: 768px)"),
            "md-брейкпоінт відсутній"
        );
    }

    #[test]
    fn closest_atom_suggests_p3_for_typo_p33() {
        assert_eq!(closest_atom("p33"), Some("p3".to_string()));
    }

    #[test]
    fn atoms_md_matches_committed_file() {
        let committed = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/ATOMS.md"))
            .expect("style/ATOMS.md має існувати");
        assert_eq!(
            committed,
            generate_atoms_md(),
            "style/ATOMS.md застарів — перегенеруй: `cargo run -p style --bin gen`"
        );
    }

    #[test]
    fn atoms_md_is_at_most_80_lines() {
        let lines = generate_atoms_md().lines().count();
        assert!(
            lines <= 80,
            "ATOMS.md {lines} рядків — понад бюджет 80 (токен-економія для агента)"
        );
    }
}
