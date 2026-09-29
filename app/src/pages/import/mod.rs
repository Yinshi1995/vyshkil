mod server;

use std::time::Duration;

use leptos::ev;
use leptos::leptos_dom::helpers::{set_interval_with_handle, window_event_listener};
use leptos::prelude::*;

use crate::components::{DatePicker, FileDropzone, Select, SelectOption};
use crate::hooks::use_actor::use_actor;
use crate::types::staffing::{InstructorStaffingRow, StaffingRow};
use crate::types::submission::{CommitOutcome, DraftPayload, GroupFormRow};
use crate::widgets::group_grid::{snapshot_rows, wrap_rows, EditableRow, Grid};
use crate::widgets::ActorNotice;
use server::{
    commit_archive_grid, commit_grid, commit_instructor_staffing, commit_staffing, get_draft,
    parse_bps_file, parse_fah_file, parse_ivs_file, parse_kvid_file, parse_terminy_file,
    parse_vch_archive_file, save_draft,
};

/// Тип файлу, що імпортуємо (03, критерій готовності Етапу 5 — усі п'ять,
/// `backend/import/CLAUDE.md` пояснює чому не один детектор; VchArchive — Етап 6, окремий
/// `source_type`, не критерій Етапу 5). Fah/Bps/Terminy/VchArchive —
/// group-подібні дані (та сама `widgets::group_grid::Grid`, що й форма); Kvid — укомплектованість
/// (01 §4), зовсім інша форма даних (`StaffingRow`), своя проста таблиця нижче; Ivs — ОБИДВІ форми
/// одразу з одного файлу (стажування+курси в `Grid`, укомплектованість інструкторів у своїй
/// таблиці) — `backend/import/ivs.rs` пояснює чому.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileKind {
    Fah,
    Bps,
    Kvid,
    Ivs,
    Terminy,
    VchArchive,
}

impl FileKind {
    const ALL: [FileKind; 6] = [
        FileKind::Fah,
        FileKind::Bps,
        FileKind::Kvid,
        FileKind::Ivs,
        FileKind::Terminy,
        FileKind::VchArchive,
    ];

    fn label(self) -> &'static str {
        match self {
            FileKind::Fah => "Фах (Пройшли/Проходять)",
            FileKind::Bps => "БпС (Завершилась/Навчаються)",
            FileKind::Kvid => "КВід (укомплектованість)",
            FileKind::Ivs => "ІВС (інструктори)",
            FileKind::Terminy => "Терміни (БЗВП/Фахова/Адаптація)",
            FileKind::VchArchive => "Архів ВЧ (одноразовий перенос, Етап 6)",
        }
    }

    /// Рядковий ключ для `components::Select` (`value`/`on_change` — рядки, як у нативного
    /// `<select>`) — той самий підхід, що `ActorSwitcher`.
    fn key(self) -> &'static str {
        match self {
            FileKind::Fah => "fah",
            FileKind::Bps => "bps",
            FileKind::Kvid => "kvid",
            FileKind::Ivs => "ivs",
            FileKind::Terminy => "terminy",
            FileKind::VchArchive => "vch_archive",
        }
    }

    fn from_key(key: &str) -> Self {
        match key {
            "bps" => FileKind::Bps,
            "kvid" => FileKind::Kvid,
            "ivs" => FileKind::Ivs,
            "terminy" => FileKind::Terminy,
            "vch_archive" => FileKind::VchArchive,
            _ => FileKind::Fah,
        }
    }

    /// Лише проста таблиця, БЕЗ сітки взагалі (Kvid). Ivs показує сітку (стажування+курси) ПОРЯД
    /// з таблицею укомплектованості — не сюди, власна гілка в `ImportBody`.
    fn is_staffing(self) -> bool {
        matches!(self, FileKind::Kvid)
    }

    /// Одноразовий перенос (Етап 6) — інший `submission.source_type` ("archive_seed" замість
    /// "table"), окреме тріо `get_archive_draft`/`save_archive_draft`/`commit_archive_grid` у
    /// `server.rs`. Без автозбереження чернетки (`ImportBody` пропускає інтервал для цього виду) —
    /// перенос робиться раз, чернетка-в-часі тут не потрібна (той самий принцип спрощення, що й
    /// Kvid без undo).
    fn is_archive(self) -> bool {
        matches!(self, FileKind::VchArchive)
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
/// Приймає `web_sys::File` напряму (не `Event`) — той самий шлях для click-обрання й drag-drop
/// (`components::FileDropzone` віддає файл уже здобутим з обох джерел, деталі різні лише в ньому).
fn read_file_bytes(file: web_sys::File, on_bytes: impl FnOnce(Vec<u8>) + 'static) {
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
    // Ivs-гілка: те саме спрощення, інший набір метрик (01 §4) -- `editable`/`staffing_rows`
    // заповнюються ОБИДВА одразу з одного файлу (courses+internships у Grid, укомплектованість тут).
    let instructor_staffing_rows = RwSignal::new(Vec::<InstructorStaffingRow>::new());

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

    let on_file_selected = move |file: web_sys::File| {
        parsing.set(true);
        status.set("розбираю файл…".to_string());
        let kind = file_kind.get_untracked();
        read_file_bytes(file, move |bytes| {
            let actor_val = actor.get_untracked();
            leptos::task::spawn_local(async move {
                if kind == FileKind::Ivs {
                    match parse_ivs_file(actor_val, bytes).await {
                        Ok((staffing, groups)) => {
                            let (n_staffing, n_groups) = (staffing.len(), groups.len());
                            instructor_staffing_rows.set(staffing);
                            snapshot();
                            replace_rows(groups);
                            submission_id.set(None);
                            status.set(format!(
                                "розібрано {n_staffing} рядків укомплектованості + \
                                 {n_groups} груп (стажування/курси) — перевірте перед фіксацією"
                            ));
                        }
                        Err(e) => status.set(format!("не вдалось розібрати: {e}")),
                    }
                    parsing.set(false);
                    return;
                }
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
                // Ivs/Kvid уже повернулись вище -- сюди доходять Fah/Bps/Terminy/VchArchive.
                let result = match kind {
                    FileKind::Fah => parse_fah_file(actor_val, bytes).await,
                    FileKind::Bps => parse_bps_file(actor_val, bytes).await,
                    FileKind::Terminy => parse_terminy_file(actor_val, bytes).await,
                    FileKind::VchArchive => parse_vch_archive_file(actor_val, bytes).await,
                    FileKind::Kvid | FileKind::Ivs => unreachable!("повертають раніше"),
                };
                match result {
                    Ok(rows) => {
                        let n = rows.len();
                        // Звіт переносу (Етап 6, роадмап: "скільки рядків, скільки відхилено") --
                        // лише для архіву: непізнана частина заздалегідь підказує обсяг ручної
                        // роботи ДО спроби фіксації (сама фіксація все одно все-або-нічого).
                        let unresolved =
                            kind.is_archive().then(|| rows.iter().filter(|r| r.sender_org_id.is_none()).count());
                        snapshot();
                        replace_rows(rows);
                        submission_id.set(None);
                        status.set(match unresolved {
                            Some(0) | None => format!("розібрано {n} рядків — перевірте перед фіксацією"),
                            Some(u) => format!(
                                "розібрано {n} рядків, {u} з нерозпізнаною частиною — перевірте перед фіксацією"
                            ),
                        });
                    }
                    Err(e) => status.set(format!("не вдалось розібрати: {e}")),
                }
                parsing.set(false);
            });
        });
    };

    // Автозбереження чернетки превʼю кожні 5с (02 §6) — та сама логіка, що й training_form.
    // VchArchive пропускає: одноразовий перенос, чернетка-в-часі не потрібна (`FileKind::
    // is_archive` doc-comment) — інакше зберігав би архівні рядки під ЧУЖИЙ source_type='table'.
    Effect::new(move |_| {
        let Ok(handle) = set_interval_with_handle(
            move || {
                if file_kind.get_untracked().is_archive() {
                    return;
                }
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
        if file_kind.get_untracked() == FileKind::Ivs {
            // Два незалежні записи з одного файлу (`backend/import/ivs.rs`) -- комітяться
            // окремими викликами; часткова невдача одного не відкочує інший (той самий рівень
            // атомарності, що й "спробуй ще раз" для решти імпортів цієї сторінки).
            status.set("фіксую…".to_string());
            let instructor_rows = instructor_staffing_rows.get_untracked();
            let as_of = as_of_date.get_untracked();
            if !instructor_rows.is_empty() {
                leptos::task::spawn_local(async move {
                    match commit_instructor_staffing(Some(actor), as_of, instructor_rows).await {
                        Ok(n) => {
                            status.update(|s| *s = format!("{s}; укомплектованість: {n} рядків"));
                            instructor_staffing_rows.set(Vec::new());
                        }
                        Err(e) => status.set(format!("не вдалось зберегти укомплектованість: {e}")),
                    }
                });
            }
            let payload =
                DraftPayload { as_of_date: as_of_date.get_untracked(), rows: snapshot_rows(editable) };
            let sid = submission_id.get_untracked();
            leptos::task::spawn_local(async move {
                match commit_grid(Some(actor), sid, payload).await {
                    Ok(CommitOutcome::Committed { group_ids }) => {
                        status.update(|s| *s = format!("{s}; групи: {}", group_ids.len()));
                        replace_rows(Vec::new());
                        submission_id.set(None);
                    }
                    Ok(CommitOutcome::ValidationFailed { row_index, field, message }) => {
                        status.set("є помилки — перевірте підсвічену клітинку".to_string());
                        commit_error.set(Some((row_index, field, message)));
                    }
                    Err(e) => status.set(format!("не вдалось зберегти групи: {e}")),
                }
            });
            return;
        }
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
        if file_kind.get_untracked().is_archive() {
            // Без submission_id -- одноразовий перенос завжди створює нове подання
            // (`is_archive()` doc-comment: чернетка-в-часі не ведеться).
            let payload =
                DraftPayload { as_of_date: as_of_date.get_untracked(), rows: snapshot_rows(editable) };
            leptos::task::spawn_local(async move {
                match commit_archive_grid(Some(actor), None, payload).await {
                    Ok(CommitOutcome::Committed { group_ids }) => {
                        status.set(format!("перенесено: {} груп(и)", group_ids.len()));
                        replace_rows(Vec::new());
                    }
                    Ok(CommitOutcome::ValidationFailed { row_index, field, message }) => {
                        status.set("є помилки — перевірте підсвічену клітинку".to_string());
                        commit_error.set(Some((row_index, field, message)));
                    }
                    Err(e) => status.set(format!("не вдалось перенести: {e}")),
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
                <Select
                    value=Signal::derive(move || file_kind.get().key().to_string())
                    options=Signal::derive(|| {
                        FileKind::ALL.iter().map(|k| SelectOption::new(k.key(), k.label())).collect()
                    })
                    on_change=Callback::new(move |v: String| file_kind.set(FileKind::from_key(&v)))
                />
                <label class="training-form__as-of">
                    "Станом на "
                    <DatePicker
                        value=Signal::derive(move || {
                            chrono::NaiveDate::parse_from_str(&as_of_date.get(), "%Y-%m-%d").ok()
                        })
                        on_change=Callback::new(move |d: Option<chrono::NaiveDate>| {
                            as_of_date.set(d.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default());
                        })
                        placeholder="дд.мм.рррр".to_string()
                    />
                </label>
                <FileDropzone
                    accept=".xlsx".to_string()
                    disabled=Signal::derive(move || parsing.get())
                    on_file=Callback::new(on_file_selected)
                />
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
            <Show when=move || {
                file_kind.get() == FileKind::Ivs && !instructor_staffing_rows.get().is_empty()
            }>
                <InstructorStaffingTable rows=instructor_staffing_rows/>
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

/// Превʼю укомплектованості ІВС (01 §4) — той самий патерн, що й `StaffingTable`, інший набір
/// метрик (`InstructorStaffingRow`, без `present`/`in_training`/…, з `trained_kibr`).
#[derive(Debug, Clone, Copy)]
enum InstructorField {
    ByTos,
    ByList,
    TrainedSergeant,
    TrainedKibr,
}

impl InstructorField {
    fn get(self, row: &InstructorStaffingRow) -> i64 {
        match self {
            InstructorField::ByTos => row.by_tos,
            InstructorField::ByList => row.by_list,
            InstructorField::TrainedSergeant => row.trained_sergeant,
            InstructorField::TrainedKibr => row.trained_kibr,
        }
    }

    fn set(self, row: &mut InstructorStaffingRow, v: i64) {
        match self {
            InstructorField::ByTos => row.by_tos = v,
            InstructorField::ByList => row.by_list = v,
            InstructorField::TrainedSergeant => row.trained_sergeant = v,
            InstructorField::TrainedKibr => row.trained_kibr = v,
        }
    }
}

#[component]
fn InstructorStaffingTable(rows: RwSignal<Vec<InstructorStaffingRow>>) -> impl IntoView {
    view! {
        <table>
            <thead>
                <tr>
                    <th>"Підрозділ"</th>
                    <th>"За штатом"</th>
                    <th>"За списком"</th>
                    <th>"Сержантська підготовка"</th>
                    <th>"КІБР"</th>
                </tr>
            </thead>
            <tbody>
                <For
                    each=move || { rows.get().into_iter().enumerate().collect::<Vec<_>>() }
                    key=|(i, _)| *i
                    let:item
                >
                    <InstructorStaffingTableRow index=item.0 row=item.1 rows=rows/>
                </For>
            </tbody>
        </table>
    }
}

#[component]
fn InstructorStaffingTableRow(
    index: usize,
    row: InstructorStaffingRow,
    rows: RwSignal<Vec<InstructorStaffingRow>>,
) -> impl IntoView {
    let unresolved = row.org_id.is_none();
    let org_label = row.org_label.clone();

    let field_input = move |field: InstructorField, value: i64| {
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
            {field_input(InstructorField::ByTos, InstructorField::ByTos.get(&row))}
            {field_input(InstructorField::ByList, InstructorField::ByList.get(&row))}
            {field_input(InstructorField::TrainedSergeant, InstructorField::TrainedSergeant.get(&row))}
            {field_input(InstructorField::TrainedKibr, InstructorField::TrainedKibr.get(&row))}
        </tr>
    }
}
