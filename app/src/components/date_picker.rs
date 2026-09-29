//! Календар-вибір дати (feedback користувача — HeroUI-референс). Самодостатній: рахує сітку
//! днів через `chrono` напряму (НЕ через `domain::dates` — `components/` не знає домену за
//! правилом теки, `.claude/... /components/CLAUDE.md`; невелике дублювання назв місяців тут
//! свідоме, не лінь). Той самий плаваючий-панель-патерн, що `Select` (`components::select`).

use chrono::{Datelike, Duration, Months, NaiveDate};
use leptos::ev;
use leptos::leptos_dom::helpers::window_event_listener;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

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
    let view_month = RwSignal::new(value.get_untracked().unwrap_or_else(|| today()));
    let root: NodeRef<leptos::html::Div> = NodeRef::new();

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
        on_change.run(Some(d));
        open.set(false);
    };

    let go_prev = move |_| view_month.update(|m| *m = m.checked_sub_months(Months::new(1)).unwrap_or(*m));
    let go_next = move |_| view_month.update(|m| *m = m.checked_add_months(Months::new(1)).unwrap_or(*m));

    let trigger_label = move || value.get().map(fmt_date).unwrap_or_else(|| placeholder.clone());

    view! {
        <div class="date-picker" node_ref=root>
            <button
                type="button"
                class="date-picker__trigger"
                aria-haspopup="dialog"
                aria-expanded=move || open.get().to_string()
                on:click=move |_| {
                    let was_open = open.get_untracked();
                    open.set(!was_open);
                    if !was_open {
                        view_month.set(value.get_untracked().unwrap_or_else(|| today()));
                    }
                }
            >
                <span class="date-picker__value">{trigger_label}</span>
                <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
                    <path d="M2 3h12v11H2zM2 6h12M5 1v3M11 1v3" stroke="currentColor" stroke-width="1.3" fill="none" stroke-linecap="round"/>
                </svg>
            </button>
            <Show when=move || open.get()>
                <div class="date-picker__panel" role="dialog">
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
