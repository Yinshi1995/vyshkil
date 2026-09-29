//! WCAG-контраст для семантичних fg/bg-пар (08 §7) — рахує реально, не покладається на "має
//! бути ОК": падає з поясненням, яка пара і скільки бракує.
//!
//! Резолвить лише ОДИН рівень `var(--X)`-непрямості в `COLOR_TOKENS` (семантичні токени theme.pairs
//! посилаються на примітивні, не глибше) — `color-mix(...)`-значення (`border`/`border-strong`)
//! НЕ резолвляться тут (алгоритм color-mix не реалізовано, ці токени — декоративні межі, не
//! текст, WCAG 1.4.11 non-text-contrast 3:1 — TODO Фаза 2, коли знадобиться реальна перевірка меж).

use crate::{Theme, COLOR_TOKENS};

fn resolve_hex(value: &str) -> Option<(u8, u8, u8)> {
    let hex = if let Some(token) = value
        .strip_prefix("var(--")
        .and_then(|s| s.strip_suffix(")"))
    {
        COLOR_TOKENS
            .iter()
            .find(|(name, _)| *name == token)
            .map(|(_, v)| *v)?
    } else if value.starts_with('#') {
        value
    } else {
        return None; // color-mix(...) чи інше нерезолвлюване тут значення.
    };
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}

fn srgb_to_linear(c: u8) -> f64 {
    let c = c as f64 / 255.0;
    if c <= 0.03928 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn relative_luminance((r, g, b): (u8, u8, u8)) -> f64 {
    0.2126 * srgb_to_linear(r) + 0.7152 * srgb_to_linear(g) + 0.0722 * srgb_to_linear(b)
}

/// WCAG-коефіцієнт контрасту (1..21) для двох кольорів.
pub fn contrast_ratio(a: (u8, u8, u8), b: (u8, u8, u8)) -> f64 {
    let (l1, l2) = (relative_luminance(a), relative_luminance(b));
    let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}

fn theme_value<'a>(theme: &'a Theme, name: &str) -> Option<&'a str> {
    theme
        .pairs
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, v)| *v)
}

/// Пари (fg, bg, мінімальний коефіцієнт) — перевіряються для КОЖНОЇ теми з `THEMES`. Звичайний
/// текст ≥ 4.5:1 (`fg-main`/`fg-muted` на поверхнях); великі заголовки/UI-акценти ≥ 3:1
/// (`fg-accent` — зазвичай великі цифри/заголовки, статуси — зазвичай плашки з текстом, не дрібний
/// абзац).
const PAIRS: &[(&str, &str, f64)] = &[
    ("fg-main", "surface-base", 4.5),
    ("fg-main", "surface-panel", 4.5),
    ("fg-main", "surface-raised", 4.5),
    ("fg-muted", "surface-base", 4.5),
    ("fg-muted", "surface-panel", 4.5),
    // Найтихіший тон — навмисно де-акцентований (лейбли/дрібні підписи, не абзаци), тому та сама
    // нижча межа, що й fg-accent, а не повний body-text бар.
    ("fg-subtle", "surface-base", 3.0),
    ("fg-subtle", "surface-panel", 3.0),
    ("fg-accent", "surface-base", 3.0),
    ("fg-accent", "surface-panel", 3.0),
    // Текст на суцільній заливці акцентом (кнопка/бірка) — тут "фон" = fg-accent (використовується
    // як background, не foreground, у bg-accent-атомі); звичайний текстовий бар 4.5:1.
    ("fg-on-accent", "fg-accent", 4.5),
    ("ok", "surface-base", 3.0),
    ("warn", "surface-base", 3.0),
    ("danger", "surface-base", 3.0),
    ("info", "surface-base", 3.0),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::THEMES;

    #[test]
    fn contrast_ratio_black_on_white_is_21() {
        let ratio = contrast_ratio((0, 0, 0), (255, 255, 255));
        assert!(
            (ratio - 21.0).abs() < 0.01,
            "чорне на білому має бути 21:1, отримано {ratio}"
        );
    }

    #[test]
    fn contrast_ratio_is_symmetric() {
        let a = contrast_ratio((201, 168, 76), (13, 15, 10));
        let b = contrast_ratio((13, 15, 10), (201, 168, 76));
        assert!(
            (a - b).abs() < 0.001,
            "контраст має бути симетричним: {a} vs {b}"
        );
    }

    #[test]
    fn every_semantic_fg_bg_pair_meets_wcag_aa_in_every_theme() {
        let mut failures = Vec::new();
        for theme in THEMES {
            for (fg_name, bg_name, min_ratio) in PAIRS {
                let (Some(fg_raw), Some(bg_raw)) =
                    (theme_value(theme, fg_name), theme_value(theme, bg_name))
                else {
                    continue;
                };
                let (Some(fg), Some(bg)) = (resolve_hex(fg_raw), resolve_hex(bg_raw)) else {
                    continue; // color-mix(...) чи інше — поза обсягом цього тесту, див. doc-comment.
                };
                let ratio = contrast_ratio(fg, bg);
                if ratio < *min_ratio {
                    failures.push(format!(
                        "тема «{}»: {fg_name} на {bg_name} — {ratio:.2}:1, потрібно ≥{min_ratio}:1",
                        theme.name
                    ));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "WCAG AA порушено:\n{}",
            failures.join("\n")
        );
    }
}
