//! Календар-вибір дати (feedback користувача — HeroUI-референс). Самодостатній: рахує сітку
//! днів через `chrono` напряму (НЕ через `domain::dates` — `components/` не знає домену за
//! правилом теки, `.claude/... /components/CLAUDE.md`; невелике дублювання назв місяців тут
//! свідоме, не лінь). Той самий плаваючий-панель-патерн, що `Select` (`components::select`).
//!
//! **Ввід з клавіатури** (feedback користувача — "лише кальцем клацати незручно"): текстове поле
//! з маскою "тільки цифри" — набираєш `20072026`, крапки `20.07.2026` з'являються самі (позиції
//! 2 і 4 в 8-цифровому `ддммрррр`), крапки не набираються вручну. Коміт (`on_change`) — лише
//! коли всі 8 цифр утворюють РЕАЛЬНУ дату (`NaiveDate::from_ymd_opt` сам відкидає `31.02` тощо);
//! доти показуємо набране як є, з видимою помилкою, не втрачаючи попереднє валідне значення.

use chrono::{Datelike, Duration, Months, NaiveDate};
use leptos::ev;
use leptos::leptos_dom::helpers::window_event_listener;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::hooks::use_floating_position::use_floating_position;

const MONTH_NAMES: [&str; 12] = [
    "Січень", "Лютий", "Березень", "Квітень", "Травень", "Червень", "Липень", "Серпень",
    "Вересень", "Жовтень", "Листопад", "Грудень",
];
const WEEKDAY_LABELS: [&str; 7] = ["Пн", "Вт", "Ср", "Чт", "Пт", "Сб", "Нд"];

/// 42 дні (6 тижнів), починаючи з понеділка того тижня, куди потрапляє 1-е число `view_month`.
fn month_grid(view_month: NaiveDate) -> Vec<NaiveDate> {
    let first = view_month.with_day(1).unwrap();
    let lead = first.weekday().num_days_from_monday() as i64;
    let start = first - Duration::days(lead);
    (0..42).map(|i| start + Duration::days(i)).collect()
}

fn fmt_date(d: NaiveDate) -> String {
    format!("{:02}.{:02}.{}", d.day(), d.month(), d.year())
}

/// Лишає тільки цифри, максимум 8 (ддммрррр) — решту введеного (крапки, пробіли, вставлений
/// текст) ігноруємо мовчки, а не підсвічуємо помилкою: людина просто набирає числа підряд.
fn digits_only(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_digit()).take(8).collect()
}

/// Вставляє крапки на позиціях 2 і 4 в міру набору — "2" → "2", "200" → "20.0", "20072026" →
/// "20.07.2026". Не намагається зберігати позицію курсора при переформатуванні (для звичайного
/// набору зліва направо це непомітно; складніше редагування посередині — не цей зріз).
fn mask_digits(digits: &str) -> String {
    let mut out = String::with_capacity(10);
    for (i, c) in digits.chars().enumerate() {
        if i == 2 || i == 4 {
            out.push('.');
        }
        out.push(c);
    }
    out
}

/// `None`, якщо цифр не 8 РІВНО, або вони не складаються в реальну дату (`31.02` тощо —
/// `NaiveDate` сам це відкидає, окремого календарного парсера писати не довелось).
fn try_parse_ddmmyyyy(digits: &str) -> Option<NaiveDate> {
    if digits.len() != 8 {
        return None;
    }
    let day: u32 = digits[0..2].parse().ok()?;
    let month: u32 = digits[2..4].parse().ok()?;
    let year: i32 = digits[4..8].parse().ok()?;
    NaiveDate::from_ymd_opt(year, month, day)
}

/// `chrono` тут БЕЗ feature "clock" (проєктне рішення [[chrono-in-domain]] — компілюється в WASM
/// однаково скрізь, не читає системний час сам). День "сьогодні" — лише клієнт, через
/// `js_sys::Date` (той самий підхід, що `[[wasm-bindgen-in-pages]]`); на сервері (SSR) — заглушка,
/// виправляється миттєво при гідратації (не впливає на функціонал, лише на перший кадр).
fn today() -> NaiveDate {
    if !cfg!(target_arch = "wasm32") {
        return NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    }
    let d = js_sys::Date::new_0();
    NaiveDate::from_ymd_opt(d.get_full_year() as i32, d.get_month() + 1, d.get_date())
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(2026, 1, 1).unwrap())
}

#[component]
pub fn DatePicker(
    #[prop(into)] value: Signal<Option<NaiveDate>>,
    #[prop(into)] on_change: Callback<Option<NaiveDate>>,
    #[prop(optional, into)] placeholder: String,
) -> impl IntoView {
    let open = RwSignal::new(false);
    let view_month = RwSignal::new(value.get_untracked().unwrap_or_else(today));
    let root: NodeRef<leptos::html::Div> = NodeRef::new();
    let (flip_up, align_end) = use_floating_position(root, open.into());

    // Текст поля — окремий сигнал від `value`: під час набору "31" (ще не дата) `value` не має
    // куди комітитись, поле все одно мусить показувати те, що людина щойно набрала.
    let raw = RwSignal::new(value.get_untracked().map(fmt_date).unwrap_or_default());
    let invalid = RwSignal::new(false);

    // Синхронізація ЗЗОВНІ (вибір у календарі теж іде через `pick`, який сам оновлює `raw` —
    // цей ефект ловить решту випадків: батько скинув `value`, інше поле форми змінило дату).
    Effect::new(move |_| {
        let v = value.get();
        raw.set(v.map(fmt_date).unwrap_or_default());
        invalid.set(false);
    });

    let handle = window_event_listener(ev::mousedown, move |ev| {
        if !open.get_untracked() {
            return;
        }
        let Some(root_el) = root.get_untracked() else { return };
        if let Some(target) = ev.target() {
            if let Ok(node) = target.dyn_into::<web_sys::Node>() {
                if root_el.contains(Some(&node)) {
                    return;
                }
            }
        }
        open.set(false);
    });
    on_cleanup(move || handle.remove());

    let pick = move |d: NaiveDate| {
        raw.set(fmt_date(d));
        invalid.set(false);
        on_change.run(Some(d));
        open.set(false);
    };

    let on_input = move |ev: ev::Event| {
        let typed = event_target_value(&ev);
        let digits = digits_only(&typed);
        raw.set(mask_digits(&digits));
        if digits.is_empty() {
            invalid.set(false);
            on_change.run(None);
        } else if digits.len() == 8 {
            match try_parse_ddmmyyyy(&digits) {
                Some(d) => {
                    invalid.set(false);
                    view_month.set(d);
                    on_change.run(Some(d));
                }
                None => invalid.set(true),
            }
        } else {
            // Ще не всі 8 цифр — не помилка, людина просто не дописала.
            invalid.set(false);
        }
    };

    let go_prev = move |_| view_month.update(|m| *m = m.checked_sub_months(Months::new(1)).unwrap_or(*m));
    let go_next = move |_| view_month.update(|m| *m = m.checked_add_months(Months::new(1)).unwrap_or(*m));

    let toggle_open = move || {
        let was_open = open.get_untracked();
        open.set(!was_open);
        if !was_open {
            view_month.set(value.get_untracked().unwrap_or_else(today));
        }
    };

    view! {
        <div class="date-picker" node_ref=root>
            <div class="date-picker__field" class:date-picker__field--invalid=move || invalid.get()>
                <input
                    type="text"
                    inputmode="numeric"
                    autocomplete="off"
                    class="date-picker__input"
                    placeholder=placeholder
                    prop:value=move || raw.get()
                    on:input=on_input
                    on:keydown=move |ev| {
                        if ev.key() == "Escape" && open.get_untracked() {
                            ev.prevent_default();
                            open.set(false);
                        }
                    }
                />
                <button
                    type="button"
                    class="date-picker__icon-btn"
                    aria-haspopup="dialog"
                    aria-expanded=move || open.get().to_string()
                    aria-label="Відкрити календар"
                    on:click=move |_| toggle_open()
                >
                    <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
                        <path d="M2 3h12v11H2zM2 6h12M5 1v3M11 1v3" stroke="currentColor" stroke-width="1.3" fill="none" stroke-linecap="round"/>
                    </svg>
                </button>
            </div>
            <Show when=move || open.get()>
                <div
                    class="date-picker__panel"
                    class:date-picker__panel--flip-up=flip_up
                    class:date-picker__panel--align-end=align_end
                    role="dialog"
                >
                    <div class="date-picker__header">
                        <button type="button" class="date-picker__nav" on:click=go_prev aria-label="Попередній місяць">"‹"</button>
                        <span class="date-picker__month">
                            {move || {
                                let m = view_month.get();
                                format!("{} {}", MONTH_NAMES[m.month0() as usize], m.year())
                            }}
                        </span>
                        <button type="button" class="date-picker__nav" on:click=go_next aria-label="Наступний місяць">"›"</button>
                    </div>
                    <div class="date-picker__weekdays">
                        {WEEKDAY_LABELS.iter().map(|w| view! { <span>{*w}</span> }).collect_view()}
                    </div>
                    <div class="date-picker__grid">
                        {move || {
                            let m = view_month.get();
                            let selected = value.get();
                            let today = today();
                            month_grid(m)
                                .into_iter()
                                .map(|d| {
                                    let in_month = d.month() == m.month();
                                    let is_selected = selected == Some(d);
                                    let is_today = d == today;
                                    view! {
                                        <button
                                            type="button"
                                            class="date-picker__day"
                                            class:date-picker__day--muted=move || !in_month
                                            class:date-picker__day--selected=move || is_selected
                                            class:date-picker__day--today=move || is_today && !is_selected
                                            on:click=move |_| pick(d)
                                        >
                                            {d.day().to_string()}
                                        </button>
                                    }
                                })
                                .collect_view()
                        }}
                    </div>
                </div>
            </Show>
        </div>
    }
}
