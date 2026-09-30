//! Сітка введення (02 §1-2, §4; розділ Б — `docs/spec/components/grid.md`) — один рядок = одна
//! група. Кожен рядок обгорнутий у власний `RwSignal` (`EditableRow`), а не одним великим
//! `RwSignal<Vec<GroupFormRow>>` — інакше кожне натискання клавіші перерендерювало б усю таблицю
//! й губило фокус посеред введення. Структурні зміни (додати/видалити/дублювати рядок) чіпають
//! зовнішній `RwSignal<Vec<EditableRow>>` (володіє ним `pages/training_form/mod.rs`, не ця
//! сітка — щоб чернетку/undo/коміт можна було підмінювати весь список ззовні), зміна поля —
//! лише внутрішній сигнал одного рядка.
//!
//! Віртуалізація (02 §1: "рендерити тільки видимі рядки") тут спрощена до прогресивного
//! дорендерювання: рядки, які вже потрапили в DOM, не демонтуються при прокрутці (стабільність
//! фокуса важливіша за точну економію на сотнях рядків) — рендериться дедалі більше рядків у міру
//! прокрутки вниз, а не ковзне вікно. Задокументовано як свідоме спрощення.
//!
//! **Розділ Б**: 10 живих колонок (було 14) — секундарні поля (організатор/підстава/примітка)
//! лишились лише в `RowEditor` (drawer). Колонка дій — номер/розгортання/індикатор/меню
//! (`row_menu.rs`). Дати "З"/"По" — `date_range_cell::DateRangeCell` (потребує `as_of` для
//! висновку року без явної вказівки — новий проп `Grid::as_of`, обидві сторінки-виклики оновлені).

use chrono::NaiveDate;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use super::autocomplete::VosPositionCourseAutocomplete;
use super::columns::{self, ColumnsState, DATA_COLUMNS};
use super::date_range_cell::DateRangeCell;
use super::row_editor::RowEditor;
use super::row_menu::RowMenu;
use crate::components::{Combobox, ComboboxItem, ComboboxVariant, Dialog, Drawer};
use crate::domain::dates::validate_period;
use super::date_range_cell::lenient_parse;
use crate::domain::validation::validate_funnel_order;
use crate::services::groups::get_training_sites;
use crate::types::submission::{GroupFormRow, VosPositionCourseHint, VosPositionCourseKind};

/// Частина-відправник більше НЕ колонка рядка (дефект 1) — 9 живих колонок (було 10): Вид
/// підготовки/ВОС/ОВТ/Місце/З/По/План/Прибуло/Навчаються.
pub const N_COLS: usize = 9;
const INITIAL_VISIBLE: usize = 30;
const GROW_STEP: usize = 30;

#[derive(Clone, Copy)]
pub struct EditableRow {
    pub id: u32,
    pub data: RwSignal<GroupFormRow>,
}

/// Обгортає сирі рядки (з чернетки/undo/коміту) у власні сигнали зі свіжими id. `StoredValue`,
/// не `&mut u32`: `.get_value()` повертає КОПІЮ, тож `&mut` на неї нічого не пише назад у
/// сховище -- лічильник мовчки не рухався, і повторні виклики (напр. відновлення чернетки після
/// початкового рендеру) видавали ті самі id, що й уже змонтовані рядки. `<For>` бачив однаковий
/// ключ і НЕ перемонтовував рядок — DOM далі показував старий (осиротілий) сигнал, поки
/// автозбереження/коміт читали новий (порожній) з `editable`: значення губилися мовчки.
pub fn wrap_rows(data: Vec<GroupFormRow>, next_id: StoredValue<u32>) -> Vec<EditableRow> {
    data.into_iter()
        .map(|d| {
            let id = next_id.get_value();
            next_id.set_value(id + 1);
            EditableRow { id, data: RwSignal::new(d) }
        })
        .collect()
}

/// Знімок поточного стану сітки як plain-даних (для автозбереження/коміту/undo).
pub fn snapshot_rows(editable: RwSignal<Vec<EditableRow>>) -> Vec<GroupFormRow> {
    editable.get_untracked().iter().map(|r| r.data.get_untracked()).collect()
}

fn cell_id(row: usize, col: usize) -> String {
    format!("cell-{row}-{col}")
}

fn focus_cell(row: usize, col: usize) {
    let id = cell_id(row, col);
    if let Some(el) = document().get_element_by_id(&id) {
        if let Ok(html_el) = el.dyn_into::<web_sys::HtmlElement>() {
            let _ = html_el.focus();
            html_el.scroll_into_view();
        }
    }
}

#[component]
pub fn Grid(
    editable: RwSignal<Vec<EditableRow>>,
    next_id: StoredValue<u32>,
    active_cell: RwSignal<(usize, usize)>,
    #[prop(into)] before_mutate: Callback<()>,
    #[prop(into)] as_of: Signal<NaiveDate>,
    /// Піднято до виклика (`FormBody`) — тулбар сторінки сам показує "Колонки"
    /// (`columns::ColumnsToggle`) в ОДНІЙ лінії з рештою кнопок (дефект 8), а не в окремому
    /// `.grid-toolbar` усередині `Grid`.
    columns: ColumnsState,
) -> impl IntoView {
    let visible_count = RwSignal::new(INITIAL_VISIBLE);
    // "Розгорнути рядок" (feedback користувача — "як Notion") — один спільний Drawer на всю
    // сітку, не по одному на рядок: тримає лише RwSignal<GroupFormRow> обраного рядка, переживає
    // видалення/перестановку рядків (це посилання на сигнал, не індекс).
    let editing_row = RwSignal::new(None::<RwSignal<GroupFormRow>>);
    // Видалення — завжди через підтвердження (02 §2 "Ctrl+Delete — з підтвердженням"), і з
    // гарячої клавіші, і з пункту меню рядка (§3 спеки) — той самий `Dialog`, той самий рядок.
    let pending_delete = RwSignal::new(None::<usize>);

    // Тіні країв (§6) — видимі, лише поки лишається горизонтальний скрол; перерахунок і на
    // `scroll`, і щоразу, як міняється розкладка колонок (ширина/видимість), бо контент може
    // почати/перестати вилазити за екран без жодного скролу від людини.
    let grid_root: NodeRef<leptos::html::Div> = NodeRef::new();
    let shadow_left = RwSignal::new(false);
    let shadow_right = RwSignal::new(false);
    let recompute_shadows = move || {
        let Some(el) = grid_root.get_untracked() else { return };
        let left = el.scroll_left();
        let max_left = el.scroll_width() - el.client_width();
        shadow_left.set(left > 0);
        shadow_right.set(left < max_left - 1);
    };
    Effect::new(move |_| {
        columns.widths.get();
        columns.visible.get();
        request_animation_frame(move || recompute_shadows());
    });

    let fresh_row = move || {
        let id = next_id.get_value();
        next_id.set_value(id + 1);
        EditableRow { id, data: RwSignal::new(GroupFormRow::default()) }
    };

    let ensure_row = move |row_index: usize| {
        editable.update(|v| {
            while v.len() <= row_index {
                let row = fresh_row();
                v.push(row);
            }
        });
    };

    let move_to = move |row: usize, col: usize| {
        active_cell.set((row, col));
        focus_cell(row, col);
        if row + 5 >= visible_count.get_untracked() {
            visible_count.update(|c| *c += GROW_STEP);
        }
    };

    let advance = move |row: usize, col: usize, forward: bool| {
        if forward {
            if col + 1 < N_COLS {
                move_to(row, col + 1);
            } else {
                before_mutate.run(());
                ensure_row(row + 1);
                move_to(row + 1, 0);
            }
        } else if col > 0 {
            move_to(row, col - 1);
        } else if row > 0 {
            move_to(row - 1, N_COLS - 1);
        }
    };

    let open_row_editor = move |row: usize| {
        active_cell.update(|c| c.0 = row);
        if let Some(data) = editable.get_untracked().get(row).map(|r| r.data) {
            editing_row.set(Some(data));
        }
    };

    let request_delete = move |row: usize| pending_delete.set(Some(row));

    let confirm_delete = move || {
        let Some(row) = pending_delete.get_untracked() else { return };
        pending_delete.set(None);
        before_mutate.run(());
        editable.update(|v| {
            if v.len() > 1 && row < v.len() {
                v.remove(row);
            }
        });
        let last = editable.get_untracked().len().saturating_sub(1);
        move_to(row.min(last), 0);
    };

    let duplicate_row = move |row: usize| {
        before_mutate.run(());
        let copy = editable.get_untracked().get(row).map(|r| r.data.get_untracked());
        if let Some(copy) = copy {
            let id = next_id.get_value();
            next_id.set_value(id + 1);
            editable.update(|v| v.insert(row + 1, EditableRow { id, data: RwSignal::new(copy) }));
        }
    };

    let on_cell_keydown = move |row: usize, col: usize, ev: web_sys::KeyboardEvent| {
        let key = ev.key();
        match key.as_str() {
            "Tab" => {
                ev.prevent_default();
                advance(row, col, !ev.shift_key());
            }
            // Без `!ev.ctrl_key()`: Ctrl+Enter (02 §2, "зберегти всі зміни") теж несе key=="Enter"
            // -- без цієї перевірки клітинка ОДНОЧАСНО й "переходила далі" (створюючи зайвий
            // рядок), поки глобальний слухач (mod.rs) паралельно викликав коміт із уже зіпсованим
            // станом сітки.
            "Enter" if !ev.ctrl_key() => {
                ev.prevent_default();
                advance(row, col, true);
            }
            "ArrowUp" => {
                ev.prevent_default();
                if row > 0 {
                    move_to(row - 1, col);
                }
            }
            "ArrowDown" => {
                ev.prevent_default();
                before_mutate.run(());
                ensure_row(row + 1);
                move_to(row + 1, col);
            }
            "Delete" if ev.ctrl_key() => {
                ev.prevent_default();
                request_delete(row);
            }
            "e" | "E" if ev.ctrl_key() => {
                ev.prevent_default();
                open_row_editor(row);
            }
            "d" | "D" if ev.ctrl_key() && ev.shift_key() => {
                ev.prevent_default();
                duplicate_row(row);
            }
            "d" | "D" if ev.ctrl_key() => {
                ev.prevent_default();
                if row > 0 {
                    before_mutate.run(());
                    let above = editable.get_untracked().get(row - 1).map(|r| r.data.get_untracked());
                    if let Some(above) = above {
                        if let Some(r) = editable.get_untracked().get(row) {
                            r.data.update(|d| copy_column(d, &above, col));
                        }
                    }
                }
            }
            "F2" => {
                ev.prevent_default();
            }
            _ => {}
        }
    };

    view! {
        <div
            class="grid"
            role="table"
            node_ref=grid_root
            class:grid--shadow-left=move || shadow_left.get()
            class:grid--shadow-right=move || shadow_right.get()
            on:scroll=move |_| recompute_shadows()
        >
            <div
                class="grid__header"
                role="row"
                style:grid-template-columns=move || columns.grid_template_columns()
            >
                <span class="grid__actions-header" title="Дії рядка"></span>
                {DATA_COLUMNS
                    .iter()
                    .enumerate()
                    .map(|(i, col)| {
                        view! {
                            <Show when=move || columns.is_visible(i)>
                                <span class="grid__header-cell">
                                    <span class="grid__header-label" title=col.label>{col.label}</span>
                                    <span
                                        class="grid__resize-handle"
                                        on:mousedown=move |ev| {
                                            ev.prevent_default();
                                            columns::start_resize(columns, i, &ev);
                                        }
                                        on:dblclick=move |_| columns.reset_width(i)
                                    ></span>
                                </span>
                            </Show>
                        }
                    })
                    .collect_view()}
            </div>
            <div class="grid__body">
                <For
                    each=move || {
                        editable
                            .get()
                            .into_iter()
                            .take(visible_count.get())
                            .enumerate()
                            .collect::<Vec<_>>()
                    }
                    key=|(_, r)| r.id
                    let:item
                >
                    {
                        let (row_index, row) = item;
                        view! {
                            <Row
                                row_index=row_index
                                data=row.data
                                as_of=as_of
                                columns=columns
                                on_keydown=move |col: usize, ev: web_sys::KeyboardEvent| {
                                    on_cell_keydown(row_index, col, ev)
                                }
                                on_expand=Callback::new(move |_| open_row_editor(row_index))
                                on_duplicate=Callback::new(move |_| duplicate_row(row_index))
                                on_delete_requested=Callback::new(move |_| request_delete(row_index))
                            />
                        }
                    }
                </For>
            </div>
        </div>
        <button
            type="button"
            class="btn btn--ghost grid__add-row"
            title="Новий рядок (Tab в останній клітинці робить те саме)"
            on:click=move |_| {
                before_mutate.run(());
                let len = editable.get_untracked().len();
                ensure_row(len);
            }
        >
            "+ Новий рядок"
        </button>
        <Drawer
            open=Signal::derive(move || editing_row.get().is_some())
            on_close=Callback::new(move |_| {
                editing_row.set(None);
                let (row, col) = active_cell.get_untracked();
                focus_cell(row, col);
            })
        >
            {move || editing_row.get().map(|d| view! { <RowEditor data=d/> })}
        </Drawer>
        <Dialog
            open=Signal::derive(move || pending_delete.get().is_some())
            on_close=Callback::new(move |_| pending_delete.set(None))
            title="Видалити рядок?".to_string()
            description="Дію не можна скасувати клавішею Ctrl+Z після збереження.".to_string()
        >
            <button type="button" class="btn btn--outline" on:click=move |_| pending_delete.set(None)>
                "Скасувати"
            </button>
            <button type="button" class="btn btn--primary" on:click=move |_| confirm_delete()>
                "Видалити"
            </button>
        </Dialog>
    }
}

fn copy_column(dst: &mut GroupFormRow, src: &GroupFormRow, col: usize) {
    match col {
        0 => {
            dst.training_kind_id = src.training_kind_id;
            dst.training_kind_label = src.training_kind_label.clone();
        }
        1 => {
            dst.vos_id = src.vos_id;
            dst.position_id = src.position_id;
            dst.course_id = src.course_id;
            dst.vos_position_course_label = src.vos_position_course_label.clone();
        }
        2 => dst.equipment_text = src.equipment_text.clone(),
        3 => {
            dst.site_id = src.site_id;
            dst.site_label = src.site_label.clone();
        }
        4 => dst.planned_start_raw = src.planned_start_raw.clone(),
        5 => dst.planned_end_raw = src.planned_end_raw.clone(),
        6 => dst.planned_count = src.planned_count,
        7 => dst.arrived_count = src.arrived_count,
        8 => dst.in_training_count = src.in_training_count,
        _ => {}
    }
}

/// Порушення порядку воронки цього рядка (02 §5, реальний час — `domain::validation`, WASM,
/// без запиту серверу; сервер той самий виклик робить ЗНОВУ при коміті, тут лише
/// швидший фідбек).
fn funnel_violated(row: &GroupFormRow) -> bool {
    validate_funnel_order(row.planned_count, row.arrived_count, row.in_training_count).is_err()
}

/// "З"/"По" — та сама перевірка, що `date_range_cell::DateRangeCell` показує на самих полях;
/// тут лише для індикатора в колонці дій (§3 спеки), невелике свідоме дублювання (той самий
/// принцип, що вже двічі в проєкті — `date_picker.rs`/`date_range_cell.rs`'s `month_grid`).
fn date_range_invalid(row: &GroupFormRow, as_of: NaiveDate) -> bool {
    let start_text = row.planned_start_raw.trim();
    let end_text = row.planned_end_raw.trim();
    if start_text.is_empty() && end_text.is_empty() {
        return false;
    }
    let start = lenient_parse(&row.planned_start_raw, as_of);
    if !start_text.is_empty() && start.is_none() {
        return true;
    }
    if end_text.is_empty() {
        return false;
    }
    let end = lenient_parse(&row.planned_end_raw, as_of);
    match (start, end) {
        (Some(s), Some(e)) => validate_period(s, e).is_err(),
        _ => end.is_none(),
    }
}

fn has_secondary_fields(row: &GroupFormRow) -> bool {
    !row.note.is_empty()
        || row.organizer_org_id.is_some()
        || !row.basis_doc_number.is_empty()
        || !row.basis_doc_date_raw.is_empty()
        || !row.composition.is_empty()
}

#[component]
fn Row(
    row_index: usize,
    data: RwSignal<GroupFormRow>,
    #[prop(into)] as_of: Signal<NaiveDate>,
    columns: ColumnsState,
    #[prop(into)] on_keydown: Callback<(usize, web_sys::KeyboardEvent)>,
    #[prop(into)] on_expand: Callback<()>,
    #[prop(into)] on_duplicate: Callback<()>,
    #[prop(into)] on_delete_requested: Callback<()>,
) -> impl IntoView {
    // `sender_org_id` більше не колонка рядка (дефект 1) -- на /training-form тулбар завжди
    // проставляє його в кожен рядок (нижче ніколи не спрацює); на /import (Ivs/VchArchive,
    // резолюція з файлу) нерозпізнана частина й досі можлива на рядок -- той самий червоний
    // індикатор у жолобі тепер сигналізує і про це (людина відкриває рядок, Ctrl+E, і бачить/
    // виправляє поле "Військова частина" в `RowEditor`, яке лишається).
    let has_error = Signal::derive(move || {
        let row = data.get();
        row.sender_org_id.is_none()
            || funnel_violated(&row)
            || date_range_invalid(&row, as_of.get())
    });
    let has_info = Signal::derive(move || !has_error.get() && has_secondary_fields(&data.get()));

    // Пропозиція "План → Прибуло/Навчаються" (grid-interaction.md §4): на виході з "План", якщо
    // сусідні поля ще порожні, підставляємо ТЕ САМЕ значення приглушеним кольором -- Tab далі
    // просто приймає його (значення вже реальне), ручний ввід перезаписує й знімає позначку.
    let arrived_suggested = RwSignal::new(false);
    let in_training_suggested = RwSignal::new(false);
    let on_plan_blur = move || {
        let row = data.get_untracked();
        if row.planned_count == 0 {
            return;
        }
        if row.arrived_count == 0 {
            data.update(|d| d.arrived_count = row.planned_count);
            arrived_suggested.set(true);
        }
        if row.in_training_count == 0 {
            data.update(|d| d.in_training_count = row.planned_count);
            in_training_suggested.set(true);
        }
    };

    view! {
        <div
            class="grid__row"
            role="row"
            style:grid-template-columns=move || columns.grid_template_columns()
        >
            <div class="grid__actions">
                <span class="grid__row-number">{(row_index + 1).to_string()}</span>
                <button
                    type="button"
                    class="grid__expand"
                    aria-label="Розгорнути рядок"
                    title="Деталі рядка (Ctrl+E)"
                    tabindex="-1"
                    on:click=move |_| on_expand.run(())
                >
                    <svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true">
                        <path d="M2 2h3M2 2v3M10 2H7M10 2v3M2 10h3M2 10V7M10 10H7M10 10V7" stroke="currentColor" stroke-width="1.3" fill="none" stroke-linecap="round"/>
                    </svg>
                    <Show when=move || has_error.get()>
                        <span class="grid__row-badge grid__row-badge--danger" title="Помилка в рядку"></span>
                    </Show>
                    <Show when=move || has_info.get()>
                        <span class="grid__row-badge grid__row-badge--info" title="Є заповнені деталі"></span>
                    </Show>
                </button>
                <RowMenu on_duplicate=on_duplicate on_delete_requested=on_delete_requested/>
            </div>
            <Show when=move || columns.is_visible(0)>
                <TrainingKindCell row_index=row_index data=data on_keydown=on_keydown/>
            </Show>
            <Show when=move || columns.is_visible(1)>
                <VosPositionCourseAutocomplete
                    id=cell_id(row_index, 1)
                    label=Signal::derive(move || data.get().vos_position_course_label)
                    on_select=Callback::new(move |hint: VosPositionCourseHint| {
                        data.update(|d| {
                            d.vos_id = None;
                            d.position_id = None;
                            d.course_id = None;
                            match hint.kind {
                                VosPositionCourseKind::Vos => d.vos_id = Some(hint.id),
                                VosPositionCourseKind::Position => d.position_id = Some(hint.id),
                                VosPositionCourseKind::Course => d.course_id = Some(hint.id),
                            }
                            d.vos_position_course_label = hint.label;
                        });
                    })
                    on_label_input=Callback::new(move |v: String| {
                        data.update(|d| d.vos_position_course_label = v);
                    })
                    on_keydown=Callback::new(move |ev: web_sys::KeyboardEvent| on_keydown.run((1, ev)))
                />
            </Show>
            <Show when=move || columns.is_visible(2)>
                <input
                    type="text"
                    id=cell_id(row_index, 2)
                    class="cell__input"
                    title=move || data.get().equipment_text
                    prop:value=move || data.get().equipment_text
                    on:input=move |ev| data.update(|d| d.equipment_text = event_target_value(&ev))
                    on:keydown=move |ev| on_keydown.run((2, ev))
                />
            </Show>
            <Show when=move || columns.is_visible(3)>
                <SiteCell row_index=row_index data=data on_keydown=on_keydown/>
            </Show>
            <Show when=move || columns.is_visible(4) || columns.is_visible(5)>
                <DateRangeCell
                    start_id=cell_id(row_index, 4)
                    end_id=cell_id(row_index, 5)
                    start_raw=Signal::derive(move || data.get().planned_start_raw)
                    end_raw=Signal::derive(move || data.get().planned_end_raw)
                    as_of=as_of
                    on_start_change=Callback::new(move |v: String| data.update(|d| d.planned_start_raw = v))
                    on_end_change=Callback::new(move |v: String| data.update(|d| d.planned_end_raw = v))
                    on_start_keydown=Callback::new(move |ev: web_sys::KeyboardEvent| on_keydown.run((4, ev)))
                    on_end_keydown=Callback::new(move |ev: web_sys::KeyboardEvent| on_keydown.run((5, ev)))
                />
            </Show>
            <Show when=move || columns.is_visible(6)>
                <NumberCell
                    id=cell_id(row_index, 6)
                    value=Signal::derive(move || data.get().planned_count)
                    on_change=Callback::new(move |v: i64| data.update(|d| d.planned_count = v))
                    on_blur=Callback::new(move |_| on_plan_blur())
                    on_keydown=Callback::new(move |ev: web_sys::KeyboardEvent| on_keydown.run((6, ev)))
                    invalid=Signal::derive(|| false)
                    suggested=Signal::derive(|| false)
                />
            </Show>
            <Show when=move || columns.is_visible(7)>
                <NumberCell
                    id=cell_id(row_index, 7)
                    value=Signal::derive(move || data.get().arrived_count)
                    on_change=Callback::new(move |v: i64| {
                        arrived_suggested.set(false);
                        data.update(|d| d.arrived_count = v);
                    })
                    on_keydown=Callback::new(move |ev: web_sys::KeyboardEvent| on_keydown.run((7, ev)))
                    invalid=Signal::derive(move || funnel_violated(&data.get()))
                    suggested=Signal::derive(move || arrived_suggested.get())
                />
            </Show>
            <Show when=move || columns.is_visible(8)>
                <NumberCell
                    id=cell_id(row_index, 8)
                    value=Signal::derive(move || data.get().in_training_count)
                    on_change=Callback::new(move |v: i64| {
                        in_training_suggested.set(false);
                        data.update(|d| d.in_training_count = v);
                    })
                    on_keydown=Callback::new(move |ev: web_sys::KeyboardEvent| on_keydown.run((8, ev)))
                    invalid=Signal::derive(move || funnel_violated(&data.get()))
                    suggested=Signal::derive(move || in_training_suggested.get())
                />
            </Show>
        </div>
    }
}

/// Число (План/Прибуло/Навчаються) — права юстиція + tabular-nums (`.cell__input--number`,
/// новий рецепт `main.css`), порожньо (0) показує `placeholder="—"` замість цифри "0" (02 §5:
/// "порожньо = порожньо") — не змінює тип поля на `Option<i64>` (03/04 модель лишається
/// незмінною), лише як `0` подається в UI. `suggested` — значення прийшло з підказки "План → ..."
/// (grid-interaction.md §4), не ручного вводу — показуємо приглушеним, доки людина сама не введе.
#[component]
fn NumberCell(
    id: String,
    #[prop(into)] value: Signal<i64>,
    #[prop(into)] on_change: Callback<i64>,
    #[prop(into)] on_keydown: Callback<web_sys::KeyboardEvent>,
    #[prop(into)] invalid: Signal<bool>,
    #[prop(into)] suggested: Signal<bool>,
    #[prop(optional, into)] on_blur: Option<Callback<()>>,
) -> impl IntoView {
    view! {
        <input
            type="number"
            min="0"
            id=id
            class="cell__input cell__input--number"
            class:cell__input--invalid=move || invalid.get()
            class:cell__input--suggested=move || suggested.get()
            placeholder="—"
            prop:value=move || {
                let v = value.get();
                if v == 0 { String::new() } else { v.to_string() }
            }
            on:input=move |ev| {
                let text = event_target_value(&ev);
                let v: i64 = if text.trim().is_empty() { 0 } else { text.parse().unwrap_or(0) };
                on_change.run(v);
            }
            on:blur=move |_| {
                if let Some(cb) = on_blur {
                    cb.run(());
                }
            }
            on:keydown=move |ev| on_keydown.run(ev)
        />
    }
}

#[component]
fn TrainingKindCell(
    row_index: usize,
    data: RwSignal<GroupFormRow>,
    #[prop(into)] on_keydown: Callback<(usize, web_sys::KeyboardEvent)>,
) -> impl IntoView {
    use crate::services::dictionaries::get_dictionaries_overview;

    let kinds = Resource::new(|| (), |_| get_dictionaries_overview());
    // Тригер має бути в DOM одразу (щоб Grid::focus_cell могла сфокусувати Tab'ом ще до відповіді
    // сервера) — тому читаємо ресурс через Effect у звичайний сигнал, не Signal::derive напряму.
    let items = RwSignal::new(Vec::<ComboboxItem>::new());
    Effect::new(move |_| {
        if let Some(Ok(o)) = kinds.get() {
            items.set(o.training_kinds.into_iter().map(|k| ComboboxItem::new(k.id.to_string(), k.label)).collect());
        }
    });

    view! {
        <Combobox
            id=cell_id(row_index, 0)
            variant=ComboboxVariant::InCell
            value=Signal::derive(move || data.get().training_kind_id.map(|id| id.to_string()).unwrap_or_default())
            items=items
            on_change=Callback::new(move |v: String| {
                data.update(|d| d.training_kind_id = v.parse().ok());
            })
            on_keydown=Callback::new(move |ev| on_keydown.run((0, ev)))
        />
    }
}

#[component]
fn SiteCell(
    row_index: usize,
    data: RwSignal<GroupFormRow>,
    #[prop(into)] on_keydown: Callback<(usize, web_sys::KeyboardEvent)>,
) -> impl IntoView {
    let sites = Resource::new(
        move || data.get().sender_org_id,
        |org_id| async move {
            match org_id {
                Some(id) => get_training_sites(id).await,
                None => Ok(Vec::new()),
            }
        },
    );
    // Той самий підхід, що TrainingKindCell: Effect у звичайний сигнал, не Suspense — тригер
    // лишається в DOM одразу, і коректно оновлюється щоразу, як міняється sender_org_id.
    let items = RwSignal::new(Vec::<ComboboxItem>::new());
    Effect::new(move |_| {
        let mut opts = Vec::new();
        if let Some(Ok(list)) = sites.get() {
            opts.extend(list.into_iter().map(|s| ComboboxItem::new(s.site_id.to_string(), s.label)));
        }
        items.set(opts);
    });

    view! {
        <Combobox
            id=cell_id(row_index, 3)
            variant=ComboboxVariant::InCell
            value=Signal::derive(move || data.get().site_id.map(|id| id.to_string()).unwrap_or_default())
            items=items
            on_change=Callback::new(move |v: String| {
                let id: Option<i32> = v.parse().ok();
                let sites_now = sites.get_untracked().and_then(|r| r.ok()).unwrap_or_default();
                let label = sites_now.into_iter().find(|s| Some(s.site_id) == id).map(|s| s.label);
                data.update(|d| {
                    d.site_id = id;
                    d.site_label = label.unwrap_or_default();
                });
            })
            on_keydown=Callback::new(move |ev| on_keydown.run((3, ev)))
        />
    }
}
