mod server;

use std::time::Duration;

use leptos::ev;
use leptos::leptos_dom::helpers::{set_interval_with_handle, window_event_listener};
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::hooks::use_actor::use_actor;
use crate::types::staffing::StaffingRow;
use crate::types::submission::{CommitOutcome, DraftPayload, GroupFormRow};
use crate::widgets::group_grid::{snapshot_rows, wrap_rows, EditableRow, Grid};
use crate::widgets::ActorNotice;
use server::{
    commit_grid, commit_staffing, get_draft, parse_bps_file, parse_fah_file, parse_kvid_file,
    save_draft,
};

/// Тип файлу, що імпортуємо (03, критерій готовності Етапу 5 вимагає всі 5 — тут поки три,
/// решта окремими кроками, `backend/import/CLAUDE.md` пояснює чому не один детектор). Fah/Bps —
/// group-подібні дані (та сама `widgets::group_grid::Grid`, що й форма); Kvid — укомплектованість
/// (01 §4), зовсім інша форма даних (`StaffingRow`), своя проста таблиця нижче.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileKind {
    Fah,
    Bps,
    Kvid,
}

impl FileKind {
    fn label(self) -> &'static str {
        match self {
            FileKind::Fah => "Фах (Пройшли/Проходять)",
            FileKind::Bps => "БпС (Завершилась/Навчаються)",
            FileKind::Kvid => "КВід (укомплектованість)",
        }
    }

    fn is_staffing(self) -> bool {
        matches!(self, FileKind::Kvid)
    }
}

/// Превʼю імпорту (03): файл → структурний розбір → резолюція → та сама сітка, що й ручне
/// введення (02) → фіксація.
#[component]
pub fn ImportPage() -> impl IntoView {
    let actor = use_actor();

    view! {
        <h1>"Імпорт"</h1>
        {move || {
            if actor.get().is_none() {
                view! { <ActorNotice/> }.into_any()
            } else {
                view! { <ImportBody/> }.into_any()
            }
        }}
    }
}

/// Читає обраний файл у байти клієнтським `File::array_buffer()` (Promise → `JsFuture`) — файли
/// цього типу малі (десятки КБ), тож простий `Vec<u8>`-аргумент server fn (без multipart) досить.
fn read_file_bytes(ev: leptos::ev::Event, on_bytes: impl FnOnce(Vec<u8>) + 'static) {
    let Some(input) = ev.target().and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
    else {
        return;
    };
    let Some(files) = input.files() else { return };
    let Some(file) = files.get(0) else { return };

    leptos::task::spawn_local(async move {
        let promise = file.array_buffer();
        let Ok(buf) = wasm_bindgen_futures::JsFuture::from(promise).await else { return };
        let bytes = js_sys::Uint8Array::new(&buf).to_vec();
        on_bytes(bytes);
    });
}

#[component]
fn ImportBody() -> impl IntoView {
    let actor = use_actor();

    let submission_id = RwSignal::new(None::<i32>);
    let as_of_date = RwSignal::new(String::new());
    let file_kind = RwSignal::new(FileKind::Fah);
    let next_id = StoredValue::new(0u32);
    let editable = RwSignal::new(Vec::<EditableRow>::new());
    let active_cell = RwSignal::new((0usize, 0usize));
    let status = RwSignal::new(String::new());
    let parsing = RwSignal::new(false);
    let commit_error = RwSignal::new(None::<(usize, String, String)>);
    let cheat_sheet_open = RwSignal::new(false);
    // Kvid-гілка: інша форма даних (01 §4), не `GroupFormRow` -- окремий сигнал, без undo/draft-
    // автозбереження (задокументоване спрощення, `pages/import/CLAUDE.md`).
    let staffing_rows = RwSignal::new(Vec::<StaffingRow>::new());

    let undo_stack = StoredValue::new(Vec::<Vec<GroupFormRow>>::new());
    let redo_stack = StoredValue::new(Vec::<Vec<GroupFormRow>>::new());
    let replace_rows = move |data: Vec<GroupFormRow>| {
        editable.set(wrap_rows(data, next_id));
    };
    let snapshot = move || {
        undo_stack.update_value(|s| s.push(snapshot_rows(editable)));
        redo_stack.update_value(|s| s.clear());
    };
    let undo = move || {
        if let Some(prev) = undo_stack.try_update_value(|s| s.pop()).flatten() {
            redo_stack.update_value(|r| r.push(snapshot_rows(editable)));
            replace_rows(prev);
        }
    };
    let redo = move || {
        if let Some(next) = redo_stack.try_update_value(|s| s.pop()).flatten() {
            undo_stack.update_value(|u| u.push(snapshot_rows(editable)));
            replace_rows(next);
        }
    };

    // Відновлення чернетки превʼю при відкритті (02 §6, та сама логіка, що й training_form).
    let draft_loaded = RwSignal::new(false);
    Effect::new(move |_| {
        if draft_loaded.get_untracked() {
            return;
        }
        let Some(actor) = actor.get() else { return };
        draft_loaded.set(true);
        leptos::task::spawn_local(async move {
            if let Ok(Some(state)) = get_draft(Some(actor)).await {
                submission_id.set(Some(state.submission_id));
                as_of_date.set(state.payload.as_of_date);
                if !state.payload.rows.is_empty() {
                    replace_rows(state.payload.rows);
                    status.set(format!("чернетку відновлено ({})", state.updated_at));
                }
            }
        });
    });

    let on_file_change = move |ev: leptos::ev::Event| {
        parsing.set(true);
        status.set("розбираю файл…".to_string());
        let kind = file_kind.get_untracked();
        read_file_bytes(ev, move |bytes| {
            let actor_val = actor.get_untracked();
            leptos::task::spawn_local(async move {
                if kind.is_staffing() {
                    match parse_kvid_file(actor_val, bytes).await {
                        Ok(rows) => {
                            let n = rows.len();
                            staffing_rows.set(rows);
                            status.set(format!("розібрано {n} рядків — перевірте перед фіксацією"));
                        }
                        Err(e) => status.set(format!("не вдалось розібрати: {e}")),
                    }
                    parsing.set(false);
                    return;
                }
                // Kvid уже повернувся вище (is_staffing) -- сюди доходять лише Fah/Bps.
                let result = match kind {
                    FileKind::Fah => parse_fah_file(actor_val, bytes).await,
                    FileKind::Bps => parse_bps_file(actor_val, bytes).await,
                    FileKind::Kvid => unreachable!("is_staffing() повертає раніше"),
                };
                match result {
                    Ok(rows) => {
                        let n = rows.len();
                        snapshot();
                        replace_rows(rows);
                        submission_id.set(None);
                        status.set(format!("розібрано {n} рядків — перевірте перед фіксацією"));
                    }
                    Err(e) => status.set(format!("не вдалось розібрати: {e}")),
                }
                parsing.set(false);
            });
        });
    };

    // Автозбереження чернетки превʼю кожні 5с (02 §6) — та сама логіка, що й training_form.
    Effect::new(move |_| {
        let Ok(handle) = set_interval_with_handle(
            move || {
                let Some(actor) = actor.get_untracked() else { return };
                let as_of = as_of_date.get_untracked();
                if as_of.trim().is_empty() {
                    return;
                }
                let current_rows = snapshot_rows(editable);
                if current_rows.is_empty() {
                    return;
                }
                let payload = DraftPayload { as_of_date: as_of, rows: current_rows };
                let sid = submission_id.get_untracked();
                leptos::task::spawn_local(async move {
                    if let Ok(id) = save_draft(Some(actor), sid, payload).await {
                        submission_id.set(Some(id));
                    }
                });
            },
            Duration::from_secs(5),
        ) else {
            return;
        };
        on_cleanup(move || handle.clear());
    });

    let do_commit = move || {
        let Some(actor) = actor.get_untracked() else { return };
        commit_error.set(None);
        if file_kind.get_untracked().is_staffing() {
            let rows = staffing_rows.get_untracked();
            let as_of = as_of_date.get_untracked();
            leptos::task::spawn_local(async move {
                match commit_staffing(Some(actor), as_of, rows).await {
                    Ok(n) => {
                        status.set(format!("зафіксовано: {n} рядків укомплектованості"));
                        staffing_rows.set(Vec::new());
                    }
                    Err(e) => status.set(format!("не вдалось зберегти: {e}")),
                }
            });
            return;
        }
        let payload =
            DraftPayload { as_of_date: as_of_date.get_untracked(), rows: snapshot_rows(editable) };
        let sid = submission_id.get_untracked();
        leptos::task::spawn_local(async move {
            match commit_grid(Some(actor), sid, payload).await {
                Ok(CommitOutcome::Committed { group_ids }) => {
                    status.set(format!("зафіксовано: {} груп(и)", group_ids.len()));
                    replace_rows(Vec::new());
                    submission_id.set(None);
                }
                Ok(CommitOutcome::ValidationFailed { row_index, field, message }) => {
                    status.set("є помилки — перевірте підсвічену клітинку".to_string());
                    commit_error.set(Some((row_index, field, message)));
                }
                Err(e) => status.set(format!("не вдалось зберегти: {e}")),
            }
        });
    };

    let handle = window_event_listener(ev::keydown, move |ev| {
        let key = ev.key();
        if key == "?" || key == "F1" {
            ev.prevent_default();
            cheat_sheet_open.update(|v| *v = !*v);
        } else if ev.ctrl_key() && key == "Enter" {
            ev.prevent_default();
            do_commit();
        } else if ev.ctrl_key() && !ev.shift_key() && key.to_lowercase() == "z" {
            ev.prevent_default();
            undo();
        } else if ev.ctrl_key() && ev.shift_key() && key.to_lowercase() == "z" {
            ev.prevent_default();
            redo();
        } else if key == "Escape" && cheat_sheet_open.get_untracked() {
            cheat_sheet_open.set(false);
        }
    });
    on_cleanup(move || handle.remove());

    view! {
        <div class="training-form">
            <p>
                "Оберіть тип файлу й завантажте xlsx. Розпізнані рядки з'являться в тій самій "
                "сітці, що й ручне введення — перевірте нерозпізнані клітинки (без вибраної "
                "частини/ВОС/місця) перед фіксацією."
            </p>
            <div class="training-form__header">
                <select
                    on:change=move |ev| {
                        let v = event_target_value(&ev);
                        file_kind.set(match v.as_str() {
                            "bps" => FileKind::Bps,
                            "kvid" => FileKind::Kvid,
                            _ => FileKind::Fah,
                        });
                    }
                >
                    <option value="fah" selected=move || file_kind.get() == FileKind::Fah>
                        {FileKind::Fah.label()}
                    </option>
                    <option value="bps" selected=move || file_kind.get() == FileKind::Bps>
                        {FileKind::Bps.label()}
                    </option>
                    <option value="kvid" selected=move || file_kind.get() == FileKind::Kvid>
                        {FileKind::Kvid.label()}
                    </option>
                </select>
                <label class="training-form__as-of">
                    "Станом на "
                    <input
                        type="date"
                        prop:value=move || as_of_date.get()
                        on:input=move |ev| as_of_date.set(event_target_value(&ev))
                    />
                </label>
                <input type="file" accept=".xlsx" disabled=move || parsing.get() on:change=on_file_change/>
                <span class="training-form__status">{move || status.get()}</span>
                <button class="btn btn--primary" on:click=move |_| do_commit()>
                    "Зафіксувати все (Ctrl+Enter)"
                </button>
            </div>

            {move || {
                commit_error
                    .get()
                    .map(|(row_index, field, message)| {
                        view! {
                            <p class="status-error">
                                "Рядок "{row_index + 1}", «"{field}"»: "{message}
                            </p>
                        }
                    })
            }}

            <Show when=move || !file_kind.get().is_staffing() && !editable.get().is_empty()>
                <Grid editable=editable next_id=next_id active_cell=active_cell before_mutate=snapshot/>
            </Show>
            <Show when=move || file_kind.get().is_staffing() && !staffing_rows.get().is_empty()>
                <StaffingTable rows=staffing_rows/>
            </Show>

            <Show when=move || cheat_sheet_open.get()>
                <div class="cheat-sheet__overlay">
                    <div class="cheat-sheet__panel">
                        <h2>"Гарячі клавіші"</h2>
                        <p>"Ті самі, що у формі введення — Tab/Enter/стрілки в сітці, Ctrl+Enter фіксує, "?"/F1 ця шпаргалка."</p>
                        <button class="btn btn--outline" on:click=move |_| cheat_sheet_open.set(false)>
                            "Закрити"
                        </button>
                    </div>
                </div>
            </Show>
        </div>
    }
}

/// Превʼю укомплектованості (01 §4) — проста редагована таблиця, НЕ `widgets::group_grid::Grid`
/// (інша форма даних: організація + сім чисел, без ВОС/дат/воронки). Нерозпізнана організація
/// (`org_id = None`) показується текстом як є — рядок блокує фіксацію на сервері (та сама логіка
/// "не розпізнано" в помилках, 03 §5), клієнт лише підсвічує колір.
#[derive(Debug, Clone, Copy)]
enum StaffingField {
    ByTos,
    ByList,
    Present,
    TrainedSergeant,
    InTraining,
    PlannedNextMonth,
    NeedTraining,
}

impl StaffingField {
    fn get(self, row: &StaffingRow) -> i64 {
        match self {
            StaffingField::ByTos => row.by_tos,
            StaffingField::ByList => row.by_list,
            StaffingField::Present => row.present,
            StaffingField::TrainedSergeant => row.trained_sergeant,
            StaffingField::InTraining => row.in_training,
            StaffingField::PlannedNextMonth => row.planned_next_month,
            StaffingField::NeedTraining => row.need_training,
        }
    }

    fn set(self, row: &mut StaffingRow, v: i64) {
        match self {
            StaffingField::ByTos => row.by_tos = v,
            StaffingField::ByList => row.by_list = v,
            StaffingField::Present => row.present = v,
            StaffingField::TrainedSergeant => row.trained_sergeant = v,
            StaffingField::InTraining => row.in_training = v,
            StaffingField::PlannedNextMonth => row.planned_next_month = v,
            StaffingField::NeedTraining => row.need_training = v,
        }
    }
}

#[component]
fn StaffingTable(rows: RwSignal<Vec<StaffingRow>>) -> impl IntoView {
    view! {
        <table>
            <thead>
                <tr>
                    <th>"Підрозділ"</th>
                    <th>"За штатом"</th>
                    <th>"За списком"</th>
                    <th>"В наявності"</th>
                    <th>"Мають підготовку"</th>
                    <th>"Проходять підготовку"</th>
                    <th>"Заплановано наст. місяць"</th>
                    <th>"Потребують підготовку"</th>
                </tr>
            </thead>
            <tbody>
                <For
                    each=move || {
                        rows.get().into_iter().enumerate().collect::<Vec<_>>()
                    }
                    key=|(i, _)| *i
                    let:item
                >
                    <StaffingTableRow index=item.0 row=item.1 rows=rows/>
                </For>
            </tbody>
        </table>
    }
}

#[component]
fn StaffingTableRow(index: usize, row: StaffingRow, rows: RwSignal<Vec<StaffingRow>>) -> impl IntoView {
    let unresolved = row.org_id.is_none();
    let org_label = row.org_label.clone();

    let field_input = move |field: StaffingField, value: i64| {
        view! {
            <td>
                <input
                    type="number"
                    min="0"
                    prop:value=value.to_string()
                    on:input=move |ev| {
                        let Ok(n) = event_target_value(&ev).parse::<i64>() else { return };
                        rows.update(|rs| {
                            if let Some(r) = rs.get_mut(index) {
                                field.set(r, n);
                            }
                        });
                    }
                />
            </td>
        }
    };

    view! {
        <tr>
            <td class=move || if unresolved { "status-error" } else { "" }>{org_label}</td>
            {field_input(StaffingField::ByTos, StaffingField::ByTos.get(&row))}
            {field_input(StaffingField::ByList, StaffingField::ByList.get(&row))}
            {field_input(StaffingField::Present, StaffingField::Present.get(&row))}
            {field_input(StaffingField::TrainedSergeant, StaffingField::TrainedSergeant.get(&row))}
            {field_input(StaffingField::InTraining, StaffingField::InTraining.get(&row))}
            {field_input(StaffingField::PlannedNextMonth, StaffingField::PlannedNextMonth.get(&row))}
            {field_input(StaffingField::NeedTraining, StaffingField::NeedTraining.get(&row))}
        </tr>
    }
}
