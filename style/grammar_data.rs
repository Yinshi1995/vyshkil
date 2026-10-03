// Єдине джерело даних граматики — `include!()`-иться і з `style/src/lib.rs`, і з
// `style_macros/src/lib.rs` (Фаза-0-обмеження задокументоване в
// `.claude/decisions/style-system-architecture.md` тепер вирішене: спільний файл замість
// дубльованих констант, жодна з двох крейтів не залежить від іншої). Textual include — не
// crate-залежність, тому цикл `style` ⇄ `style_macros` не виникає.
//
// Комбінаторні сімейства (spacing/gap) генеруються з (префікс, CSS-властивість) × індекс шкали —
// не 100+ рядків "p0".."m8" вручну. Ключові слова (`flex`, `fg-main` тощо) — явний список,
// бо не мають шкали.

/// Крок шкали відступів: `SPACE_SCALE[i]` = CSS-значення токена `--space-{i}`.
const SPACE_SCALE: &[&str] = &["0", "4px", "8px", "12px", "16px", "24px", "32px", "48px", "64px"];

/// Крок шкали радіусів: `RADIUS_SCALE[i]` = CSS-значення токена `--radius-{i}`.
const RADIUS_SCALE: &[&str] = &["0", "4px", "8px", "999px"];

/// Крок шкали letter-spacing: `TRACK_SCALE[i]` = CSS-значення токена `--track-{i}`. 7 кроків — не
/// вигадано наперед, а зібрано з РЕАЛЬНИХ значень наявного `app/style/main.css` (Фаза 3, [[style-
/// migration-rounds-to-scale]]): 0.03/0.04/0.05/0.08/0.1/0.2em траплялись по кілька разів кожне —
/// генуїнна повторювана потреба, не legacy-хак під один компонент.
const TRACK_SCALE: &[&str] = &["normal", "0.03em", "0.04em", "0.05em", "0.08em", "0.1em", "0.2em"];

/// (ім'я токена без префіксу кольору, CSS-значення) — польові тони й нейтралі. Значення night-теми
/// (єдина виміряна зі striy.pp.ua — `.claude/decisions/ui-visual-style-source.md`); day/print —
/// перше наближення (не звірено з реальним джерелом, немає такого для нових тем), задокументовано
/// в 08 §2.2.
const COLOR_TOKENS: &[(&str, &str)] = &[
    // нейтралі (ink-0 найтемніший .. ink-4 найсвітліший, night-полярність)
    ("ink-0", "#0d0f0a"),
    ("ink-1", "#171912"),
    ("ink-2", "#1f2217"),
    ("ink-3", "#b9b4a6"),
    ("ink-4", "#e8e4d8"),
    // золото (акцент, не залежить від теми). gold-1 — темніший варіант спеціально для day-теми
    // (fg-accent на світлому фоні); #a88a3a не проходив WCAG AA (2.97:1, зловлено контраст-тестом
    // при першому запуску) — підібрано #8a7030 (4.26:1 на surface-base дня) розрахунком.
    ("gold-1", "#8a7030"),
    ("gold-2", "#c9a84c"),
    ("gold-3", "#ddb95a"),
    // Світлий кінець градієнта primary-кнопки — виміряно з живого striy.pp.ua
    // (`getComputedStyle(.btn-primary).backgroundImage`), не вигадано.
    ("gold-4", "#e8c96a"),
    // олива (ok)
    ("olive-1", "#4a5a2e"),
    ("olive-2", "#6b8043"),
    ("olive-3", "#8fa565"),
    // бурштин (warn)
    ("amber-1", "#8a6a1f"),
    ("amber-2", "#b8912c"),
    ("amber-3", "#d4ae4d"),
    // цегла (danger) — те саме значення, що поточний --error
    ("brick-1", "#a15a4f"),
    ("brick-2", "#c97a6e"),
    ("brick-3", "#d99a90"),
    // сталь (info)
    ("steel-1", "#3f5a6e"),
    ("steel-2", "#5c7d94"),
    ("steel-3", "#82a2b5"),
];

/// Типографічна шкала: (ім'я, font-size, line-height).
const TEXT_SCALE: &[(&str, &str, &str)] = &[
    ("xs", "11px", "1.4"),
    ("sm", "13px", "1.5"),
    ("md", "15px", "1.55"),
    ("lg", "19px", "1.4"),
    ("xl", "24px", "1.3"),
    ("display", "34px", "1.15"),
];

const WEIGHT_SCALE: &[(&str, &str)] = &[("4", "400"), ("5", "500"), ("7", "700")];

const SHADOW_SCALE: &[(&str, &str)] = &[
    ("0", "none"),
    ("1", "0 2px 12px rgba(0, 0, 0, 0.5)"),
    ("2", "0 4px 20px rgba(0, 0, 0, 0.5)"),
];

const Z_SCALE: &[(&str, &str)] = &[
    ("dropdown", "20"),
    ("overlay", "50"),
    ("modal", "100"),
    ("toast", "150"),
];

const DURATION_SCALE: &[(&str, &str)] = &[("fast", "120ms"), ("base", "200ms"), ("slow", "320ms")];

/// Сімейства з числовою шкалою відступів: (префікс класу, CSS-властивість). `{префікс}{0..8}` —
/// один атом на кожен індекс `SPACE_SCALE`.
const SPACING_FAMILIES: &[(&str, &str)] = &[
    ("p", "padding"),
    ("px", "padding-inline"),
    ("py", "padding-block"),
    ("pt", "padding-top"),
    ("pr", "padding-right"),
    ("pb", "padding-bottom"),
    ("pl", "padding-left"),
    ("m", "margin"),
    ("mx", "margin-inline"),
    ("my", "margin-block"),
    ("mt", "margin-top"),
    ("mr", "margin-right"),
    ("mb", "margin-bottom"),
    ("ml", "margin-left"),
    ("gap", "gap"),
];

/// Атоми без шкали — ключове слово → CSS-декларація (без селектора).
const KEYWORD_ATOMS: &[(&str, &str)] = &[
    ("flex", "display: flex"),
    ("col", "flex-direction: column"),
    ("row", "flex-direction: row"),
    ("wrap", "flex-wrap: wrap"),
    ("grid", "display: grid"),
    ("cols-1", "grid-template-columns: repeat(1, minmax(0, 1fr))"),
    ("cols-2", "grid-template-columns: repeat(2, minmax(0, 1fr))"),
    ("cols-3", "grid-template-columns: repeat(3, minmax(0, 1fr))"),
    ("cols-4", "grid-template-columns: repeat(4, minmax(0, 1fr))"),
    ("cols-6", "grid-template-columns: repeat(6, minmax(0, 1fr))"),
    ("items-s", "align-items: flex-start"),
    ("items-c", "align-items: center"),
    ("items-e", "align-items: flex-end"),
    ("justify-s", "justify-content: flex-start"),
    ("justify-c", "justify-content: center"),
    ("justify-e", "justify-content: flex-end"),
    ("justify-b", "justify-content: space-between"),
    ("w-full", "width: 100%"),
    ("w-auto", "width: auto"),
    ("maxw-prose", "max-width: 65ch"),
    ("maxw-full", "max-width: 100%"),
    ("fw4", "font-weight: var(--weight-4)"),
    ("fw5", "font-weight: var(--weight-5)"),
    ("fw7", "font-weight: var(--weight-7)"),
    ("up", "text-transform: uppercase"),
    ("num", "font-variant-numeric: tabular-nums"),
    ("mono", "font-family: ui-monospace, \"Cascadia Code\", monospace"),
    ("head", "font-family: var(--font-heading)"),
    ("fg-main", "color: var(--fg-main)"),
    ("fg-muted", "color: var(--fg-muted)"),
    ("fg-subtle", "color: var(--fg-subtle)"),
    ("fg-accent", "color: var(--fg-accent)"),
    ("fg-ok", "color: var(--ok)"),
    ("fg-warn", "color: var(--warn)"),
    ("fg-danger", "color: var(--danger)"),
    ("fg-info", "color: var(--info)"),
    ("bg-base", "background: var(--surface-base)"),
    ("bg-panel", "background: var(--surface-panel)"),
    ("bg-raised", "background: var(--surface-raised)"),
    // Заповнення суцільним акцентом (кнопка/бірка) + контрастний текст на ньому + м'який тінт
    // (hover) — Фаза 3: старі --accent-contrast/--accent-dim, реально вжиті 1/5 разів у main.css.
    ("bg-accent", "background: var(--fg-accent)"),
    ("fg-on-accent", "color: var(--fg-on-accent)"),
    ("bg-accent-dim", "background: var(--accent-dim)"),
    ("bd", "border: 1px solid var(--border)"),
    ("bd-accent", "border: 1px solid var(--border-strong)"),
    // Виміряно з живого striy.pp.ua (getComputedStyle(.btn).clipPath) — симетричний зріз ДВОХ
    // протилежних кутів (верхній правий + нижній лівий) на 15px, не один кут навскіс, як
    // припускав Фаза-0/1 здогад (14px, один кут) — виправлено феідбеком користувача (Фаза 3).
    ("chamfer", "clip-path: polygon(0 0, calc(100% - 15px) 0, 100% 15px, 100% 100%, 15px 100%, 0 calc(100% - 15px))"),
];

/// Дозволені префікси-варіанти (`hover:bg-panel` тощо) — атом застосовується лише в цьому стані/
/// брейкпоінті. `md:`/`lg:` — min-width медіа-запит; `@md:` — container query (панелі незалежно
/// від viewport).
const VARIANTS: &[&str] = &["hover", "focus-visible", "active", "disabled", "invalid", "open", "md", "lg", "@md"];

/// (ім'я семантичного токена, CSS-значення для ЦІЄЇ теми). `night` — єдина виміряна зі
/// striy.pp.ua; `day` — перше наближення (нема реального джерела, задокументовано в 08 §2.2).
/// fg/bg-пари лишаються ПРЯМИМИ кольорами (не `color-mix()`) навмисно — щоб контраст-тест міг
/// порахувати WCAG без реалізації CSS color-mix-алгоритму; `color-mix()` використовується для
/// `border` (декоративне, не потребує точної AA-перевірки).
struct Theme {
    pub name: &'static str,
    pub pairs: &'static [(&'static str, &'static str)],
}

const THEMES: &[Theme] = &[
    Theme {
        name: "night",
        pairs: &[
            ("surface-base", "var(--ink-0)"),
            ("surface-panel", "var(--ink-1)"),
            ("surface-raised", "var(--ink-2)"),
            ("fg-main", "var(--ink-4)"),
            ("fg-muted", "var(--ink-3)"),
            // Третій, найтихіший текстовий тон — старий --text-muted (#8a8577), реально вжитий
            // 10 разів у наявному main.css (Фаза 3) — не збігається з жодним із двох існуючих
            // семантичних тонів (fg-main/fg-muted), тому окремий токен, не компресія до двох.
            ("fg-subtle", "#8a8577"),
            ("fg-accent", "var(--gold-2)"),
            // Текст на суцільному золотому фоні (кнопка/бірка) — старий --accent-contrast, той
            // самий на всіх темах (темний, читається на будь-якому відтінку золота).
            ("fg-on-accent", "#14140c"),
            ("accent-dim", "color-mix(in srgb, var(--gold-2) 12%, transparent)"),
            ("border", "color-mix(in srgb, var(--gold-2) 16%, transparent)"),
            ("border-strong", "var(--gold-2)"),
            ("focus-ring", "var(--gold-2)"),
            ("ok", "var(--olive-3)"),
            ("warn", "var(--amber-3)"),
            ("danger", "var(--brick-3)"),
            ("info", "var(--steel-3)"),
        ],
    },
    Theme {
        name: "day",
        pairs: &[
            ("surface-base", "#f5f3ec"),
            ("surface-panel", "#ffffff"),
            ("surface-raised", "#ebe7da"),
            ("fg-main", "#1a1a14"),
            ("fg-muted", "#4a4638"),
            ("fg-subtle", "#6b6656"),
            // gold-1 навмисно темний (для тексту НА світлому фоні дня) — саме тому як ФОН під
            // .btn--primary потребує світлого, не темного, тексту (на відміну від night/print).
            ("fg-accent", "var(--gold-1)"),
            ("fg-on-accent", "#ffffff"),
            ("accent-dim", "color-mix(in srgb, var(--gold-1) 12%, transparent)"),
            ("border", "color-mix(in srgb, var(--gold-1) 30%, transparent)"),
            ("border-strong", "var(--gold-1)"),
            ("focus-ring", "var(--gold-1)"),
            ("ok", "var(--olive-1)"),
            ("warn", "var(--amber-1)"),
            ("danger", "var(--brick-1)"),
            ("info", "var(--steel-1)"),
        ],
    },
];

/// Чи існує базовий атом (без варіант-префіксів) у граматиці — комбінаторні сімейства (spacing/
/// gap/radius/shadow/text/z/duration) перевіряються за індексом шкали, не списком.
fn is_valid_atom(base: &str) -> bool {
    if KEYWORD_ATOMS.iter().any(|(k, _)| *k == base) || base == "bracket" {
        return true;
    }
    for (prefix, _) in SPACING_FAMILIES {
        for i in 0..SPACE_SCALE.len() {
            if base == format!("{prefix}{i}") {
                return true;
            }
        }
    }
    for i in 0..RADIUS_SCALE.len() {
        if base == format!("r{i}") {
            return true;
        }
    }
    for i in 0..TRACK_SCALE.len() {
        if base == format!("track{i}") {
            return true;
        }
    }
    for (name, _) in SHADOW_SCALE {
        if base == format!("shadow{name}") {
            return true;
        }
    }
    for (name, _, _) in TEXT_SCALE {
        if base == format!("t-{name}") {
            return true;
        }
    }
    for (name, _) in Z_SCALE {
        if base == format!("z-{name}") {
            return true;
        }
    }
    for (name, _) in DURATION_SCALE {
        if base == format!("duration-{name}") {
            return true;
        }
    }
    false
}

/// CSS-декларація(ї) для базового атома (без селектора, `;`-розділені якщо декілька) — `None` для
/// невідомого імені. `bracket` повертає лише `position: relative`; самі кутові скоби —
/// псевдоелементи, додаються окремо в `generate_css` (не влазять у "одна властивість на атом").
fn atom_declaration(base: &str) -> Option<String> {
    if let Some((_, decl)) = KEYWORD_ATOMS.iter().find(|(k, _)| *k == base) {
        return Some((*decl).to_string());
    }
    if base == "bracket" {
        return Some("position: relative".to_string());
    }
    for (prefix, prop) in SPACING_FAMILIES {
        for i in 0..SPACE_SCALE.len() {
            if base == format!("{prefix}{i}") {
                return Some(format!("{prop}: var(--space-{i})"));
            }
        }
    }
    for i in 0..RADIUS_SCALE.len() {
        if base == format!("r{i}") {
            return Some(format!("border-radius: var(--radius-{i})"));
        }
    }
    for i in 0..TRACK_SCALE.len() {
        if base == format!("track{i}") {
            return Some(format!("letter-spacing: var(--track-{i})"));
        }
    }
    for (name, _) in SHADOW_SCALE {
        if base == format!("shadow{name}") {
            return Some(format!("box-shadow: var(--shadow-{name})"));
        }
    }
    for (name, _, _) in TEXT_SCALE {
        if base == format!("t-{name}") {
            return Some(format!(
                "font-size: var(--text-{name}-size); line-height: var(--text-{name}-line)"
            ));
        }
    }
    for (name, _) in Z_SCALE {
        if base == format!("z-{name}") {
            return Some(format!("z-index: var(--z-{name})"));
        }
    }
    for (name, _) in DURATION_SCALE {
        if base == format!("duration-{name}") {
            return Some(format!("transition-duration: var(--duration-{name})"));
        }
    }
    None
}

/// Усі базові імена атомів (для генератора CSS і для `style/ATOMS.md`-подібних перерахувань) —
/// та сама комбінаторика, що й `is_valid_atom`, але як перелік, не перевірка.
fn all_atom_names() -> Vec<String> {
    let mut out = Vec::new();
    for (k, _) in KEYWORD_ATOMS {
        out.push((*k).to_string());
    }
    out.push("bracket".to_string());
    for (prefix, _) in SPACING_FAMILIES {
        for i in 0..SPACE_SCALE.len() {
            out.push(format!("{prefix}{i}"));
        }
    }
    for i in 0..RADIUS_SCALE.len() {
        out.push(format!("r{i}"));
    }
    for i in 0..TRACK_SCALE.len() {
        out.push(format!("track{i}"));
    }
    for (name, _) in SHADOW_SCALE {
        out.push(format!("shadow{name}"));
    }
    for (name, _, _) in TEXT_SCALE {
        out.push(format!("t-{name}"));
    }
    for (name, _) in Z_SCALE {
        out.push(format!("z-{name}"));
    }
    for (name, _) in DURATION_SCALE {
        out.push(format!("duration-{name}"));
    }
    out
}

/// Левенштейн — для "може, ви мали на увазі" (typo в `cx!` чи в `closest_atom`-довідці).
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut prev = row[0];
        row[0] = i;
        for j in 1..=b.len() {
            let tmp = row[j];
            row[j] =
                if a[i - 1] == b[j - 1] { prev } else { 1 + prev.min(row[j]).min(row[j - 1]) };
            prev = tmp;
        }
    }
    row[b.len()]
}
