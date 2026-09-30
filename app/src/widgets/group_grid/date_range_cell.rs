//! "З"/"По" пов'язані як діапазон (`docs/spec/components/grid.md` §4, розділ Б) — домен-обізнаний
//! компонент (НЕ `components/`, `app/src/CLAUDE.md` §Дерево рішень): гнучкий розбір ("18.08" без
//! року, діапазон в одне поле, висновок року) — це `domain::dates`, а `components/` домену не
//! знає. `components::DatePicker` лишається незмінним для "Станом на" (суворий формат).
//!
//! **Жива маска дд.мм.рррр** (grid-interaction.md §3, дефект 5) — автокрапки й посимвольна
//! валідація, АЛЕ лише коли людина набирає ГОЛІ ЦИФРИ (`InputEvent.data()` — саме вставлений
//! символ, не весь вміст поля): перевірка через `web_sys::InputEvent`, не жорсткий 8-цифровий
//! `digits_only`-mask, як у `DatePicker` — той не підтримав би ні рік-less дату, ні діапазон в
//! одну клітинку. Якщо вставлений символ НЕ цифра (людина сама надрукувала крапку/тире, вставила
//! з буфера, "18.08-09.10" одним рухом) — пропускаємо форматування, лишаємо як є: `domain::dates::
//! {split_range, parse_maybe_range}` і так розбирають довільний текст (03 §5), посимвольна маска
//! в цьому разі лише заважала б (боролась би з уже проставленими людиною крапками).
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

use crate::domain::dates::{
    format_date_mask, parse_date, parse_maybe_range, split_range, split_token, validate_period,
};

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

/// Лишає тільки цифри, максимум 8 (ддммрррр) — той самий підхід, що `components::DatePicker`,
/// відфільтровує решту (крапки, тире діапазону вже розібране окремо вище по стеку виклику).
fn digits_only(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_digit()).take(8).collect()
}

/// Чи щойно вставлений символ (не весь вміст поля!) — ГОЛА цифра. `InputEvent.data()` дає САМЕ
/// те, що людина набрала цим натисканням (`None` для видалення/composition-подій); порожній рядок
/// (е.g. drop) теж НЕ рахуємо цифрою. Розрізняє "людина друкує підряд цифри" (жива маска
/// застосовується) від "людина сама набрала крапку/тире, або вставила готовий текст" (маска НЕ
/// втручається — лишає як написано).
fn is_plain_digit_insertion(ev: &web_sys::Event) -> bool {
    ev.dyn_ref::<web_sys::InputEvent>()
        .and_then(|e| e.data())
        .is_some_and(|d| !d.is_empty() && d.chars().all(|c| c.is_ascii_digit()))
}

/// Backspace на дд.мм.рррр-масці має стирати цифру РАЗОМ із зайвою автопроставленою крапкою
/// (grid-interaction.md §3) — звичайний Backspace прибрав би лише саму крапку (символ перед
/// курсором), переформатування одразу відновило б її назад, і людині здавалось би, що Backspace
/// узагалі нічого не робить. `value`/`cursor` — ASCII (цифри+крапки), байтові індекси = символьні.
fn backspace_skip_dot(value: &str, cursor: usize) -> Option<String> {
    if cursor == 0 || cursor > value.len() {
        return None;
    }
    let before = &value[..cursor];
    if !before.ends_with('.') {
        return None;
    }
    let remove_from = cursor.saturating_sub(2);
    Some(format!("{}{}", &value[..remove_from], &value[cursor..]))
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
    let start_input_ref: NodeRef<leptos::html::Input> = NodeRef::new();
    let end_input_ref: NodeRef<leptos::html::Input> = NodeRef::new();
    let position = use_popover_position(root, open.into());
    let view_month = RwSignal::new(today());
    let focused_day = RwSignal::new(today());

    // Реальний час (03 §5) — не посимвольне блокування, лише візуальний стан помилки; лінива
    // перевірка (не `parse_date`/`parse_end_date` напряму) — див. `lenient_parse`. Використовується
    // для календаря (якір/підсвітка діапазону) — `format_date_mask` (нижче) не знає про ІНШЕ поле,
    // тому перевірку "по" не раніше "з" лишає тут.
    let parsed_start = Signal::derive(move || lenient_parse(&start_raw.get(), as_of.get()));
    let parsed_end = Signal::derive(move || lenient_parse(&end_raw.get(), as_of.get()));
    // Живе форматування (grid-interaction.md §3) саме встановлює ці помилки на кожен keystroke --
    // точніше за `lenient_parse` для НЕЗАВЕРШЕНОГО вводу (ловить "31.04" одразу, без року).
    let start_mask_error = RwSignal::new(None::<String>);
    let end_mask_error = RwSignal::new(None::<String>);
    let start_invalid = Signal::derive(move || start_mask_error.get().is_some());
    let end_invalid = Signal::derive(move || {
        if end_mask_error.get().is_some() {
            return true;
        }
        match (parsed_start.get(), parsed_end.get()) {
            (Some(s), Some(e)) => validate_period(s, e).is_err(),
            _ => false,
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
                node_ref=start_input_ref
                prop:value=move || start_raw.get()
                on:input=move |ev| {
                    let typed = event_target_value(&ev);
                    // Жива маска -- ЛИШЕ коли щойно вставлений символ сам є голою цифрою
                    // (`is_plain_digit_insertion`): людина сама надрукувала крапку/тире
                    // (діапазон "18.08-09.10" одним рухом) чи вставила текст -- тоді формат вже
                    // її власний, маска (яка рахує ЦИФРИ з УСЬОГО поля) лише зіпсувала б його,
                    // зливши цифри обох половин діапазону в одну (напр. "18.08-09" прочиталось б
                    // як день"18"місяць"08"рік"09"→2009). Немаскований шлях — той самий, що був
                    // до живої маски: `split_range`/`parse_maybe_range` розбирають довільний текст.
                    if !is_plain_digit_insertion(&ev) {
                        match (split_range(&typed), parse_maybe_range(&typed, as_of.get_untracked())) {
                            (Some((start_part, _)), Ok((_, Some(end)))) => {
                                on_start_change.run(start_part.to_string());
                                on_end_change.run(fmt_date(end));
                            }
                            _ => on_start_change.run(typed),
                        }
                        return;
                    }
                    match split_range(&typed) {
                        Some(_) => {
                            match parse_maybe_range(&typed, as_of.get_untracked()) {
                                Ok((_, Some(end))) => {
                                    let (start_part, _) = split_range(&typed).unwrap();
                                    let masked = format_date_mask(&digits_only(start_part));
                                    start_mask_error.set(masked.error);
                                    end_mask_error.set(None);
                                    on_start_change.run(masked.display);
                                    on_end_change.run(fmt_date(end));
                                }
                                // Тире вже є, але "по"-частина ще не дописана до повної дати --
                                // не форматувати нічого, лишити як набрано.
                                _ => on_start_change.run(typed),
                            }
                        }
                        None => {
                            let masked = format_date_mask(&digits_only(&typed));
                            start_mask_error.set(masked.error);
                            on_start_change.run(masked.display);
                        }
                    }
                }
                on:keydown=move |ev| {
                    if ev.key() == "ArrowDown" && ev.alt_key() {
                        ev.prevent_default();
                        open_for(ActiveField::Start);
                        return;
                    }
                    if ev.key() == "Backspace" && !ev.shift_key() && !ev.ctrl_key() && !ev.alt_key() {
                        if let Some(input) = start_input_ref.get_untracked() {
                            let value = input.value();
                            let cursor = input
                                .selection_start()
                                .ok()
                                .flatten()
                                .map(|n| n as usize)
                                .unwrap_or(value.len());
                            if let Some(new_value) = backspace_skip_dot(&value, cursor) {
                                ev.prevent_default();
                                let masked = format_date_mask(&digits_only(&new_value));
                                start_mask_error.set(masked.error);
                                on_start_change.run(masked.display);
                                return;
                            }
                        }
                    }
                    on_start_keydown.run(ev);
                }
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
                node_ref=end_input_ref
                prop:value=move || end_raw.get()
                on:input=move |ev| {
                    let typed = event_target_value(&ev);
                    if !is_plain_digit_insertion(&ev) {
                        end_mask_error.set(None);
                        on_end_change.run(typed);
                        return;
                    }
                    let masked = format_date_mask(&digits_only(&typed));
                    end_mask_error.set(masked.error);
                    on_end_change.run(masked.display);
                }
                on:keydown=move |ev| {
                    if ev.key() == "ArrowDown" && ev.alt_key() {
                        ev.prevent_default();
                        open_for(ActiveField::End);
                        return;
                    }
                    if ev.key() == "Backspace" && !ev.shift_key() && !ev.ctrl_key() && !ev.alt_key() {
                        if let Some(input) = end_input_ref.get_untracked() {
                            let value = input.value();
                            let cursor = input
                                .selection_start()
                                .ok()
                                .flatten()
                                .map(|n| n as usize)
                                .unwrap_or(value.len());
                            if let Some(new_value) = backspace_skip_dot(&value, cursor) {
                                ev.prevent_default();
                                let masked = format_date_mask(&digits_only(&new_value));
                                end_mask_error.set(masked.error);
                                on_end_change.run(masked.display);
                                return;
                            }
                        }
                    }
                    on_end_keydown.run(ev);
                }
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
