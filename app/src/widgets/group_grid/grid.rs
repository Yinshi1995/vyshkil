//! Сітка введення (02 §1-2, §4) — один рядок = одна група. Кожен рядок обгорнутий у власний
//! `RwSignal` (`EditableRow`), а не одним великим `RwSignal<Vec<GroupFormRow>>` — інакше кожне
//! натискання клавіші перерендерювало б усю таблицю й губило фокус посеред введення. Структурні
//! зміни (додати/видалити/дублювати рядок) чіпають зовнішній `RwSignal<Vec<EditableRow>>`
//! (володіє ним `pages/training_form/mod.rs`, не ця сітка — щоб чернетку/undo/коміт можна було
//! підмінювати весь список ззовні), зміна поля — лише внутрішній сигнал одного рядка.
//!
//! Віртуалізація (02 §1: "рендерити тільки видимі рядки") тут спрощена до прогресивного
//! дорендерювання: рядки, які вже потрапили в DOM, не демонтуються при прокрутці (стабільність
//! фокуса важливіша за точну економію на сотнях рядків) — рендериться дедалі більше рядків у міру
//! прокрутки вниз, а не ковзне вікно. Задокументовано як свідоме спрощення.

use leptos::prelude::*;
use wasm_bindgen::JsCast;

use super::autocomplete::{OrgAutocomplete, VosPositionCourseAutocomplete};
use crate::services::groups::get_training_sites;
use crate::types::submission::{GroupFormRow, VosPositionCourseHint, VosPositionCourseKind};

pub const N_COLS: usize = 14;
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
) -> impl IntoView {
    let visible_count = RwSignal::new(INITIAL_VISIBLE);

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
                before_mutate.run(());
                editable.update(|v| {
                    if v.len() > 1 && row < v.len() {
                        v.remove(row);
                    }
                });
                let last = editable.get_untracked().len().saturating_sub(1);
                move_to(row.min(last), col);
            }
            "d" | "D" if ev.ctrl_key() && ev.shift_key() => {
                ev.prevent_default();
                before_mutate.run(());
                let copy = editable.get_untracked().get(row).map(|r| r.data.get_untracked());
                if let Some(copy) = copy {
                    let id = next_id.get_value();
                    next_id.set_value(id + 1);
                    editable.update(|v| v.insert(row + 1, EditableRow { id, data: RwSignal::new(copy) }));
                }
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
        <div class="grid" role="table">
            <div class="grid__header" role="row">
                <span>"Частина"</span>
                <span>"Вид підготовки"</span>
                <span>"ВОС / посада / курс"</span>
                <span>"ОВТ"</span>
                <span>"Місце"</span>
                <span>"З"</span>
                <span>"По"</span>
                <span>"План"</span>
                <span>"Прибуло"</span>
                <span>"Навчаються"</span>
                <span>"Організатор"</span>
                <span>"№ розпорядження"</span>
                <span>"Дата розпорядження"</span>
                <span>"Примітка"</span>
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
                                on_keydown=move |col: usize, ev: web_sys::KeyboardEvent| {
                                    on_cell_keydown(row_index, col, ev)
                                }
                            />
                        }
                    }
                </For>
            </div>
            <button
                class="btn btn--outline grid__add-row"
                on:click=move |_| {
                    before_mutate.run(());
                    let len = editable.get_untracked().len();
                    ensure_row(len);
                }
            >
                "+ рядок"
            </button>
        </div>
    }
}

fn copy_column(dst: &mut GroupFormRow, src: &GroupFormRow, col: usize) {
    match col {
        0 => {
            dst.sender_org_id = src.sender_org_id;
            dst.sender_org_label = src.sender_org_label.clone();
        }
        1 => {
            dst.training_kind_id = src.training_kind_id;
            dst.training_kind_label = src.training_kind_label.clone();
        }
        2 => {
            dst.vos_id = src.vos_id;
            dst.position_id = src.position_id;
            dst.course_id = src.course_id;
            dst.vos_position_course_label = src.vos_position_course_label.clone();
        }
        3 => dst.equipment_text = src.equipment_text.clone(),
        4 => {
            dst.site_id = src.site_id;
            dst.site_label = src.site_label.clone();
        }
        5 => dst.planned_start_raw = src.planned_start_raw.clone(),
        6 => dst.planned_end_raw = src.planned_end_raw.clone(),
        7 => dst.planned_count = src.planned_count,
        8 => dst.arrived_count = src.arrived_count,
        9 => dst.in_training_count = src.in_training_count,
        10 => {
            dst.organizer_org_id = src.organizer_org_id;
            dst.organizer_org_label = src.organizer_org_label.clone();
        }
        11 => dst.basis_doc_number = src.basis_doc_number.clone(),
        12 => dst.basis_doc_date_raw = src.basis_doc_date_raw.clone(),
        13 => dst.note = src.note.clone(),
        _ => {}
    }
}

#[component]
fn Row(
    row_index: usize,
    data: RwSignal<GroupFormRow>,
    #[prop(into)] on_keydown: Callback<(usize, web_sys::KeyboardEvent)>,
) -> impl IntoView {
    view! {
        <div class="grid__row" role="row">
            <OrgAutocomplete
                id=cell_id(row_index, 0)
                label=Signal::derive(move || data.get().sender_org_label)
                on_select=move |id: i32, label: String| {
                    data.update(|d| {
                        d.sender_org_id = Some(id);
                        d.sender_org_label = label;
                        d.site_id = None;
                        d.site_label.clear();
                    });
                }
                on_label_input=Callback::new(move |v: String| {
                    data.update(|d| {
                        d.sender_org_label = v;
                        d.sender_org_id = None;
                    });
                })
                on_keydown=Callback::new(move |ev: web_sys::KeyboardEvent| on_keydown.run((0, ev)))
            />
            <TrainingKindCell row_index=row_index data=data on_keydown=on_keydown/>
            <VosPositionCourseAutocomplete
                id=cell_id(row_index, 2)
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
                on_keydown=Callback::new(move |ev: web_sys::KeyboardEvent| on_keydown.run((2, ev)))
            />
            <input
                type="text"
                id=cell_id(row_index, 3)
                class="cell__input"
                prop:value=move || data.get().equipment_text
                on:input=move |ev| data.update(|d| d.equipment_text = event_target_value(&ev))
                on:keydown=move |ev| on_keydown.run((3, ev))
            />
            <SiteCell row_index=row_index data=data on_keydown=on_keydown/>
            <input
                type="text"
                id=cell_id(row_index, 5)
                class="cell__input"
                placeholder="18.08"
                prop:value=move || data.get().planned_start_raw
                on:input=move |ev| data.update(|d| d.planned_start_raw = event_target_value(&ev))
                on:keydown=move |ev| on_keydown.run((5, ev))
            />
            <input
                type="text"
                id=cell_id(row_index, 6)
                class="cell__input"
                placeholder="09.10"
                prop:value=move || data.get().planned_end_raw
                on:input=move |ev| data.update(|d| d.planned_end_raw = event_target_value(&ev))
                on:keydown=move |ev| on_keydown.run((6, ev))
            />
            <input
                type="number"
                min="0"
                id=cell_id(row_index, 7)
                class="cell__input"
                prop:value=move || data.get().planned_count.to_string()
                on:input=move |ev| {
                    let v: i64 = event_target_value(&ev).parse().unwrap_or(0);
                    data.update(|d| d.planned_count = v);
                }
                on:keydown=move |ev| on_keydown.run((7, ev))
            />
            <input
                type="number"
                min="0"
                id=cell_id(row_index, 8)
                class="cell__input"
                prop:value=move || data.get().arrived_count.to_string()
                on:input=move |ev| {
                    let v: i64 = event_target_value(&ev).parse().unwrap_or(0);
                    data.update(|d| d.arrived_count = v);
                }
                on:keydown=move |ev| on_keydown.run((8, ev))
            />
            <input
                type="number"
                min="0"
                id=cell_id(row_index, 9)
                class="cell__input"
                prop:value=move || data.get().in_training_count.to_string()
                on:input=move |ev| {
                    let v: i64 = event_target_value(&ev).parse().unwrap_or(0);
                    data.update(|d| d.in_training_count = v);
                }
                on:keydown=move |ev| on_keydown.run((9, ev))
            />
            <OrgAutocomplete
                id=cell_id(row_index, 10)
                label=Signal::derive(move || data.get().organizer_org_label)
                on_select=move |id: i32, label: String| {
                    data.update(|d| {
                        d.organizer_org_id = Some(id);
                        d.organizer_org_label = label;
                    });
                }
                on_label_input=Callback::new(move |v: String| {
                    data.update(|d| {
                        d.organizer_org_label = v;
                        d.organizer_org_id = None;
                    });
                })
                on_keydown=Callback::new(move |ev: web_sys::KeyboardEvent| on_keydown.run((10, ev)))
            />
            <input
                type="text"
                id=cell_id(row_index, 11)
                class="cell__input"
                prop:value=move || data.get().basis_doc_number
                on:input=move |ev| data.update(|d| d.basis_doc_number = event_target_value(&ev))
                on:keydown=move |ev| on_keydown.run((11, ev))
            />
            <input
                type="text"
                id=cell_id(row_index, 12)
                class="cell__input"
                placeholder="25.02.26"
                prop:value=move || data.get().basis_doc_date_raw
                on:input=move |ev| data.update(|d| d.basis_doc_date_raw = event_target_value(&ev))
                on:keydown=move |ev| on_keydown.run((12, ev))
            />
            <input
                type="text"
                id=cell_id(row_index, 13)
                class="cell__input"
                prop:value=move || data.get().note
                on:input=move |ev| data.update(|d| d.note = event_target_value(&ev))
                on:keydown=move |ev| on_keydown.run((13, ev))
            />
        </div>
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

    view! {
        <select
            id=cell_id(row_index, 1)
            class="cell__input"
            on:change=move |ev| {
                let v = event_target_value(&ev);
                let id: Option<i32> = v.parse().ok();
                data.update(|d| d.training_kind_id = id);
            }
            on:keydown=move |ev| on_keydown.run((1, ev))
        >
            <option value="">"—"</option>
            <Suspense fallback=|| ()>
                {move || {
                    kinds
                        .get()
                        .map(|res| match res {
                            Ok(o) => {
                                o.training_kinds
                                    .into_iter()
                                    .map(|k| {
                                        let selected = move || data.get().training_kind_id == Some(k.id);
                                        view! {
                                            <option value=k.id.to_string() selected=selected>
                                                {k.label}
                                            </option>
                                        }
                                    })
                                    .collect_view()
                                    .into_any()
                            }
                            Err(_) => ().into_any(),
                        })
                }}
            </Suspense>
        </select>
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

    view! {
        <select
            id=cell_id(row_index, 4)
            class="cell__input"
            on:change=move |ev| {
                let v = event_target_value(&ev);
                let id: Option<i32> = v.parse().ok();
                let sites_now = sites.get().and_then(|r| r.ok()).unwrap_or_default();
                let label = sites_now.into_iter().find(|s| Some(s.site_id) == id).map(|s| s.label);
                data.update(|d| {
                    d.site_id = id;
                    d.site_label = label.unwrap_or_default();
                });
            }
            on:keydown=move |ev| on_keydown.run((4, ev))
        >
            <option value="">"—"</option>
            <Suspense fallback=|| ()>
                {move || {
                    sites
                        .get()
                        .map(|res| match res {
                            Ok(list) => {
                                list.into_iter()
                                    .map(|s| {
                                        let selected = move || data.get().site_id == Some(s.site_id);
                                        view! {
                                            <option value=s.site_id.to_string() selected=selected>
                                                {s.label}
                                            </option>
                                        }
                                    })
                                    .collect_view()
                                    .into_any()
                            }
                            Err(_) => ().into_any(),
                        })
                }}
            </Suspense>
        </select>
    }
}
