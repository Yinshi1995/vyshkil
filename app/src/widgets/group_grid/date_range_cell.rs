//! "З"/"По" пов'язані як діапазон (`docs/spec/components/grid.md` §4, розділ Б) — домен-обізнаний
//! компонент (НЕ `components/`, `app/src/CLAUDE.md` §Дерево рішень): гнучкий розбір ("18.08" без
//! року, діапазон в одне поле, висновок року) — це `domain::dates`, а `components/` домену не
//! знає. `components::DatePicker` лишається незмінним для "Станом на" (суворий формат).
//!
//! **"Маска дд.мм.рррр"** з брифу користувача тут — ПЛЕЙСХОЛДЕР-підказка, не посимвольна
//! трансформація вводу (як у `DatePicker`): поле мусить лишатись вільним текстом, бо приймає і
//! "18.08" (без року), і "18.08-09.10" (діапазон в одну клітинку) — жорсткий 8-цифровий мask
//! (`DatePicker`) робить обидва неможливими. Валідація — реальний час через `domain::dates`
//! (03 §5), не посимвольне блокування вводу.
//!
//! Спрощення позиціонування: ОБИДВА поля ("З" і "По") ділять ОДИН календар-поповер, який завжди
//! прив'язаний до обгортки поля "З" (не до того, з якого відкрито) — уникає необхідності
//! динамічно міняти `NodeRef` у `use_popover_position` (фіксований параметр на виклик); візуально
//! не заважає, бо обидва поля сусідні колонки.

use chrono::{Datelike, Duration, Months, NaiveDate};
use leptos::ev;
use leptos::leptos_dom::helpers::window_event_listener;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::domain::dates::{parse_date, parse_maybe_range, split_range, split_token, validate_period};

/// Легка перевірка "чи це взагалі реальна дата" для ВІЗУАЛЬНОЇ валідності поля — БЕЗ правила
/// "явний рік = рік `as_of`" (`parse_date` це правило має, і правильно: воно проти друкарських
/// помилок при наборі). Тут же явний рік у тексті МІГ прийти від самого компонента (вибір у
/// календарі, авторозклад діапазону) і законно відрізнятись від `as_of.year()` — сувора
/// повторна перевірка дала б фальшиву помилку на щойно правильно вирішеній даті.
pub(super) fn lenient_parse(raw: &str, as_of: NaiveDate) -> Option<NaiveDate> {
    let (day, month, year) = split_token(raw).ok()?;
    match year {
        Some(y) => NaiveDate::from_ymd_opt(y, month, day),
        None => parse_date(raw, as_of).ok(),
    }
}
use crate::hooks::use_popover_position::use_popover_position;

const MONTH_NAMES: [&str; 12] = [
    "Січень", "Лютий", "Березень", "Квітень", "Травень", "Червень", "Липень", "Серпень",
    "Вересень", "Жовтень", "Листопад", "Грудень",
];
const WEEKDAY_LABELS: [&str; 7] = ["Пн", "Вт", "Ср", "Чт", "Пт", "Сб", "Нд"];

fn month_grid(view_month: NaiveDate) -> Vec<NaiveDate> {
    let first = view_month.with_day(1).unwrap();
    let lead = first.weekday().num_days_from_monday() as i64;
    let start = first - Duration::days(lead);
    (0..42).map(|i| start + Duration::days(i)).collect()
}

fn fmt_date(d: NaiveDate) -> String {
    format!("{:02}.{:02}.{}", d.day(), d.month(), d.year())
}

/// `chrono` без feature "clock" (`[[chrono-in-domain]]`) — день "сьогодні" лише клієнт, через
/// `js_sys::Date` (той самий підхід, що `components::DatePicker`).
fn today() -> NaiveDate {
    if !cfg!(target_arch = "wasm32") {
        return NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    }
    let d = js_sys::Date::new_0();
    NaiveDate::from_ymd_opt(d.get_full_year() as i32, d.get_month() + 1, d.get_date())
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(2026, 1, 1).unwrap())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveField {
    Start,
    End,
}

#[component]
pub fn DateRangeCell(
    start_id: String,
    end_id: String,
    #[prop(into)] start_raw: Signal<String>,
    #[prop(into)] end_raw: Signal<String>,
    #[prop(into)] as_of: Signal<NaiveDate>,
    #[prop(into)] on_start_change: Callback<String>,
    #[prop(into)] on_end_change: Callback<String>,
    #[prop(into)] on_start_keydown: Callback<web_sys::KeyboardEvent>,
    #[prop(into)] on_end_keydown: Callback<web_sys::KeyboardEvent>,
) -> impl IntoView {
    let open = RwSignal::new(false);
    let active_field = RwSignal::new(ActiveField::Start);
    let root: NodeRef<leptos::html::Div> = NodeRef::new();
    let calendar_root: NodeRef<leptos::html::Div> = NodeRef::new();
    let position = use_popover_position(root, open.into());
    let view_month = RwSignal::new(today());
    let focused_day = RwSignal::new(today());

    // Реальний час (03 §5) — не посимвольне блокування, лише візуальний стан помилки; лінива
    // перевірка (не `parse_date`/`parse_end_date` напряму) — див. `lenient_parse`.
    let parsed_start = Signal::derive(move || lenient_parse(&start_raw.get(), as_of.get()));
    let parsed_end = Signal::derive(move || lenient_parse(&end_raw.get(), as_of.get()));
    let start_invalid = Signal::derive(move || {
        !start_raw.get().trim().is_empty() && parsed_start.get().is_none()
    });
    let end_invalid = Signal::derive(move || {
        let end = end_raw.get();
        if end.trim().is_empty() {
            return false;
        }
        match (parsed_start.get(), parsed_end.get()) {
            (Some(s), Some(e)) => validate_period(s, e).is_err(),
            _ => parsed_end.get().is_none(),
        }
    });

    let close = move || open.set(false);

    let open_for = move |field: ActiveField| {
        active_field.set(field);
        let anchor = match field {
            ActiveField::Start => parsed_start.get_untracked(),
            ActiveField::End => parsed_end.get_untracked(),
        }
        .unwrap_or_else(today);
        view_month.set(anchor);
        focused_day.set(anchor);
        open.set(true);
        request_animation_frame(move || {
            if let Some(el) = calendar_root.get_untracked() {
                let opts = web_sys::FocusOptions::new();
                opts.set_prevent_scroll(true);
                let _ = el.focus_with_options(&opts);
            }
        });
    };

    let pick = move |d: NaiveDate| {
        match active_field.get_untracked() {
            ActiveField::Start => on_start_change.run(fmt_date(d)),
            ActiveField::End => on_end_change.run(fmt_date(d)),
        }
        close();
    };

    let handle = window_event_listener(ev::mousedown, move |ev| {
        if !open.get_untracked() {
            return;
        }
        let Some(target) = ev.target().and_then(|t| t.dyn_into::<web_sys::Node>().ok()) else {
            return;
        };
        let inside_root = root.get_untracked().is_some_and(|r| r.contains(Some(&target)));
        let inside_calendar = calendar_root.get_untracked().is_some_and(|c| c.contains(Some(&target)));
        if !inside_root && !inside_calendar {
            close();
        }
    });
    on_cleanup(move || handle.remove());

    let on_calendar_keydown = move |ev: web_sys::KeyboardEvent| match ev.key().as_str() {
        "ArrowLeft" => {
            ev.prevent_default();
            focused_day.update(|d| *d -= Duration::days(1));
            view_month.set(focused_day.get_untracked());
        }
        "ArrowRight" => {
            ev.prevent_default();
            focused_day.update(|d| *d += Duration::days(1));
            view_month.set(focused_day.get_untracked());
        }
        "ArrowUp" => {
            ev.prevent_default();
            focused_day.update(|d| *d -= Duration::days(7));
            view_month.set(focused_day.get_untracked());
        }
        "ArrowDown" => {
            ev.prevent_default();
            focused_day.update(|d| *d += Duration::days(7));
            view_month.set(focused_day.get_untracked());
        }
        "PageUp" => {
            ev.prevent_default();
            focused_day.update(|d| *d = d.checked_sub_months(Months::new(1)).unwrap_or(*d));
            view_month.set(focused_day.get_untracked());
        }
        "PageDown" => {
            ev.prevent_default();
            focused_day.update(|d| *d = d.checked_add_months(Months::new(1)).unwrap_or(*d));
            view_month.set(focused_day.get_untracked());
        }
        "Enter" => {
            ev.prevent_default();
            pick(focused_day.get_untracked());
        }
        "Escape" => {
            ev.prevent_default();
            close();
        }
        _ => {}
    };

    view! {
        <div class="date-range-cell__start" node_ref=root>
            <input
                type="text"
                id=start_id
                class="cell__input"
                class:cell__input--invalid=move || start_invalid.get()
                placeholder="дд.мм.рррр"
                prop:value=move || start_raw.get()
                on:input=move |ev| {
                    let typed = event_target_value(&ev);
                    // Розкладає ЛИШЕ на дві клітинки — "З" лишається як набрано (без року, якщо
                    // без року й набрано), не переписується повністю розв'язаною датою: інакше
                    // явний рік (щойно сам виведений) не пройшов би повторно `parse_date`'s
                    // власну перевірку "явний рік = рік `as_of`" (`YearMismatch`).
                    match (split_range(&typed), parse_maybe_range(&typed, as_of.get_untracked())) {
                        (Some((start_part, _)), Ok((_, Some(end)))) => {
                            on_start_change.run(start_part.to_string());
                            on_end_change.run(fmt_date(end));
                        }
                        _ => on_start_change.run(typed),
                    }
                }
                on:keydown=move |ev| on_start_keydown.run(ev)
            />
            <button
                type="button"
                class="date-picker__icon-btn"
                aria-label="Відкрити календар"
                tabindex="-1"
                on:click=move |_| open_for(ActiveField::Start)
            >
                <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
                    <path d="M2 3h12v11H2zM2 6h12M5 1v3M11 1v3" stroke="currentColor" stroke-width="1.3" fill="none" stroke-linecap="round"/>
                </svg>
            </button>
            <Show when=move || open.get()>
                <leptos::portal::Portal>
                    <div
                        class="date-range-cell__panel"
                        role="dialog"
                        tabindex="-1"
                        node_ref=calendar_root
                        on:keydown=on_calendar_keydown
                        style:top=move || position.get().and_then(|p| p.top).map(|v| format!("{v}px")).unwrap_or_default()
                        style:bottom=move || position.get().and_then(|p| p.bottom).map(|v| format!("{v}px")).unwrap_or_default()
                        style:left=move || position.get().and_then(|p| p.left).map(|v| format!("{v}px")).unwrap_or_default()
                        style:right=move || position.get().and_then(|p| p.right).map(|v| format!("{v}px")).unwrap_or_default()
                    >
                        <div class="date-picker__header">
                            <button
                                type="button"
                                class="date-picker__nav"
                                aria-label="Попередній місяць"
                                on:click=move |_| view_month.update(|m| *m = m.checked_sub_months(Months::new(1)).unwrap_or(*m))
                            >
                                "‹"
                            </button>
                            <span class="date-picker__month">
                                {move || {
                                    let m = view_month.get();
                                    format!("{} {}", MONTH_NAMES[m.month0() as usize], m.year())
                                }}
                            </span>
                            <button
                                type="button"
                                class="date-picker__nav"
                                aria-label="Наступний місяць"
                                on:click=move |_| view_month.update(|m| *m = m.checked_add_months(Months::new(1)).unwrap_or(*m))
                            >
                                "›"
                            </button>
                        </div>
                        <div class="date-picker__weekdays">
                            {WEEKDAY_LABELS.iter().map(|w| view! { <span>{*w}</span> }).collect_view()}
                        </div>
                        <div class="date-picker__grid">
                            {move || {
                                let m = view_month.get();
                                let range_start = parsed_start.get();
                                let range_end = parsed_end.get();
                                let focused = focused_day.get();
                                let now = today();
                                month_grid(m)
                                    .into_iter()
                                    .map(|d| {
                                        let in_month = d.month() == m.month();
                                        let in_range = match (range_start, range_end) {
                                            (Some(s), Some(e)) => d >= s && d <= e,
                                            _ => false,
                                        };
                                        let is_endpoint = Some(d) == range_start || Some(d) == range_end;
                                        let is_focused = d == focused;
                                        let is_today = d == now;
                                        view! {
                                            <button
                                                type="button"
                                                class="date-picker__day"
                                                class:date-picker__day--muted=move || !in_month
                                                class:date-picker__day--selected=move || is_endpoint
                                                class:date-range-cell__day--in-range=move || in_range && !is_endpoint
                                                class:date-picker__day--today=move || is_today && !is_endpoint
                                                class:date-range-cell__day--focused=move || is_focused
                                                on:click=move |_| pick(d)
                                            >
                                                {d.day().to_string()}
                                            </button>
                                        }
                                    })
                                    .collect_view()
                            }}
                        </div>
                        <div class="date-picker__footer">
                            <button
                                type="button"
                                class="btn btn--outline"
                                on:click=move |_| pick(today())
                            >
                                "Сьогодні"
                            </button>
                        </div>
                    </div>
                </leptos::portal::Portal>
            </Show>
        </div>
        <div class="date-range-cell__end">
            <input
                type="text"
                id=end_id
                class="cell__input"
                class:cell__input--invalid=move || end_invalid.get()
                placeholder="дд.мм.рррр"
                prop:value=move || end_raw.get()
                on:input=move |ev| on_end_change.run(event_target_value(&ev))
                on:keydown=move |ev| on_end_keydown.run(ev)
            />
            <button
                type="button"
                class="date-picker__icon-btn"
                aria-label="Відкрити календар"
                tabindex="-1"
                on:click=move |_| open_for(ActiveField::End)
            >
                <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
                    <path d="M2 3h12v11H2zM2 6h12M5 1v3M11 1v3" stroke="currentColor" stroke-width="1.3" fill="none" stroke-linecap="round"/>
                </svg>
            </button>
        </div>
    }
}
