mod server;

use std::time::Duration;

use leptos::ev;
use leptos::leptos_dom::helpers::{set_interval_with_handle, window_event_listener};
use leptos::prelude::*;

use crate::components::{read_file_bytes, DatePicker, FileDropzone, Select, SelectOption};
use crate::hooks::use_actor::use_actor;
use crate::types::submission::{CommitOutcome, DraftPayload, GroupFormRow};
use crate::widgets::group_grid::{snapshot_rows, wrap_rows, EditableRow, Grid};
use crate::widgets::ActorNotice;
use server::{commit_grid, get_draft, parse_bps_file, parse_fah_file, parse_terminy_file, save_draft};

/// Тип файлу, яким можна ДОПОВНИТИ сітку — Фах/БпС/Терміни, чисті виробники `GroupFormRow`
/// (03, Етап 5, перенесено з `pages::import` при об'єднанні з ручним вводом —
/// [[unified-training-form-source-type]]). КВід/ІВС/Архів ВЧ лишились на `/import`: інша форма
/// даних або окремий `source_type`, об'єднання їм не підходить (`pages/import/mod.rs` пояснює чому).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileKind {
    Fah,
    Bps,
    Terminy,
}

impl FileKind {
    const ALL: [FileKind; 3] = [FileKind::Fah, FileKind::Bps, FileKind::Terminy];

    fn label(self) -> &'static str {
        match self {
            FileKind::Fah => "Фах (Пройшли/Проходять)",
            FileKind::Bps => "БпС (Завершилась/Навчаються)",
            FileKind::Terminy => "Терміни (БЗВП/Фахова/Адаптація)",
        }
    }

    fn key(self) -> &'static str {
        match self {
            FileKind::Fah => "fah",
            FileKind::Bps => "bps",
            FileKind::Terminy => "terminy",
        }
    }

    fn from_key(key: &str) -> Self {
        match key {
            "bps" => FileKind::Bps,
            "terminy" => FileKind::Terminy,
            _ => FileKind::Fah,
        }
    }
}

/// Сітка введення (02): весь рядок вноситься без миші, автозбереження чернетки, `Ctrl+Enter`
/// фіксує все-або-нічого. Критерій готовності Етапу 4 — `docs/spec/06-roadmap.md`. Доповнюється
/// файлом (Фах/БпС/Терміни) в тому самому редагуванні — `FileDropzone` нижче
/// ([[unified-training-form-source-type]]).
#[component]
pub fn TrainingFormPage() -> impl IntoView {
    let actor = use_actor();

    view! {
        <h1>"Внесення груп на навчанні"</h1>
        {move || {
            if actor.get().is_none() {
                view! { <ActorNotice/> }.into_any()
            } else {
                view! { <FormBody/> }.into_any()
            }
        }}
    }
}

#[component]
fn FormBody() -> impl IntoView {
    let actor = use_actor();

    let submission_id = RwSignal::new(None::<i32>);
    // "Станом на" -- користувач заповнює першим полем; без системного часу навмисно (`domain::
    // dates` без chrono "clock", [[chrono-in-domain]] -- та сама заборона стосується й цього боку).
    let as_of_date = RwSignal::new(String::new());

    let next_id = StoredValue::new(0u32);
    let editable = RwSignal::new(wrap_rows(vec![GroupFormRow::default()], next_id));
    let active_cell = RwSignal::new((0usize, 0usize));

    // Undo/redo (02 §2, Ctrl+Z/Ctrl+Shift+Z) -- знімок усієї сітки перед кожною мутуючою дією
    // (не по клітинці: простіше, і "в межах сесії редагування" не вимагає точнішої гранулярності).
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
        let prev = undo_stack.try_update_value(|s| s.pop()).flatten();
        if let Some(prev) = prev {
            redo_stack.update_value(|r| r.push(snapshot_rows(editable)));
            replace_rows(prev);
        }
    };
    let redo = move || {
        let next = redo_stack.try_update_value(|s| s.pop()).flatten();
        if let Some(next) = next {
            undo_stack.update_value(|u| u.push(snapshot_rows(editable)));
            replace_rows(next);
        }
    };

    let cheat_sheet_open = RwSignal::new(true);
    let command_palette_open = RwSignal::new(false);
    let save_status = RwSignal::new(String::new());
    let commit_error = RwSignal::new(None::<(usize, String, String)>);
    let file_kind = RwSignal::new(FileKind::Fah);
    let parsing = RwSignal::new(false);

    // Відновлення чернетки при відкритті форми (02 §6).
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
                }
                save_status.set(format!("чернетку відновлено ({})", state.updated_at));
            }
        });
    });

    // Автозбереження кожні кілька секунд (02 §6) — незалежно від активності, доки є хоч один
    // непорожній рядок.
    Effect::new(move |_| {
        let Ok(handle) = set_interval_with_handle(
            move || {
                let Some(actor) = actor.get_untracked() else { return };
                // "Станом на" -- NOT NULL у submission.as_of_date; без нього зберігати чернетку
                // ще нема сенсу (04-01 §5), а спроба зберегти '' у date-колонку дала б 500.
                let as_of = as_of_date.get_untracked();
                if as_of.trim().is_empty() {
                    return;
                }
                let current_rows = snapshot_rows(editable);
                if current_rows.iter().all(|r| r.sender_org_id.is_none() && r.note.is_empty()) {
                    return;
                }
                let payload = DraftPayload { as_of_date: as_of, rows: current_rows };
                let sid = submission_id.get_untracked();
                leptos::task::spawn_local(async move {
                    match save_draft(Some(actor), sid, payload).await {
                        Ok(id) => {
                            submission_id.set(Some(id));
                            save_status.set("збережено".to_string());
                        }
                        Err(e) => save_status.set(format!("не збереглось: {e}")),
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
        let payload =
            DraftPayload { as_of_date: as_of_date.get_untracked(), rows: snapshot_rows(editable) };
        let sid = submission_id.get_untracked();
        commit_error.set(None);
        leptos::task::spawn_local(async move {
            match commit_grid(Some(actor), sid, payload).await {
                Ok(CommitOutcome::Committed { group_ids }) => {
                    save_status.set(format!("збережено: {} груп(и)", group_ids.len()));
                    replace_rows(vec![GroupFormRow::default()]);
                    submission_id.set(None);
                    active_cell.set((0, 0));
                }
                Ok(CommitOutcome::ValidationFailed { row_index, field, message }) => {
                    save_status.set("є помилки — перевірте підсвічену клітинку".to_string());
                    commit_error.set(Some((row_index, field, message)));
                }
                Err(e) => save_status.set(format!("не вдалось зберегти: {e}")),
            }
        });
    };

    // Імпорт файлу в ту саму сітку (03, Етап 5, перенесено з `pages::import` —
    // [[unified-training-form-source-type]]): якщо в сітці вже є непорожні рядки (людина вручну
    // щось вносить), розібрані рядки ДОДАЮТЬСЯ в кінець (той самий `submission_id`/чернетка —
    // одна сесія редагування, не дві); якщо сітка порожня (типовий єдиний рядок-заглушка) —
    // замінюються, щоб не лишати зайвий порожній рядок зверху.
    let on_file_selected = move |file: web_sys::File| {
        parsing.set(true);
        save_status.set("розбираю файл…".to_string());
        let kind = file_kind.get_untracked();
        read_file_bytes(file, move |bytes| {
            let actor_val = actor.get_untracked();
            leptos::task::spawn_local(async move {
                let result = match kind {
                    FileKind::Fah => parse_fah_file(actor_val, bytes).await,
                    FileKind::Bps => parse_bps_file(actor_val, bytes).await,
                    FileKind::Terminy => parse_terminy_file(actor_val, bytes).await,
                };
                match result {
                    Ok(rows) => {
                        let n = rows.len();
                        let existing = snapshot_rows(editable);
                        let is_blank = existing
                            .iter()
                            .all(|r| r.sender_org_id.is_none() && r.note.is_empty());
                        snapshot();
                        if is_blank {
                            replace_rows(rows);
                        } else {
                            let mut merged = existing;
                            merged.extend(rows);
                            replace_rows(merged);
                        }
                        save_status.set(format!("розібрано {n} рядків — перевірте перед збереженням"));
                    }
                    Err(e) => save_status.set(format!("не вдалось розібрати: {e}")),
                }
                parsing.set(false);
            });
        });
    };

    // Глобальні гарячі клавіші, що не залежать від конкретної клітинки (02 §2).
    let handle = window_event_listener(ev::keydown, move |ev| {
        let key = ev.key();
        if key == "?" || key == "F1" {
            ev.prevent_default();
            cheat_sheet_open.update(|v| *v = !*v);
        } else if ev.ctrl_key() && key.to_lowercase() == "k" {
            ev.prevent_default();
            command_palette_open.update(|v| *v = !*v);
        } else if ev.ctrl_key() && key == "Enter" {
            ev.prevent_default();
            do_commit();
        } else if ev.ctrl_key() && !ev.shift_key() && key.to_lowercase() == "z" {
            ev.prevent_default();
            undo();
        } else if ev.ctrl_key() && ev.shift_key() && key.to_lowercase() == "z" {
            ev.prevent_default();
            redo();
        } else if key == "Escape" {
            if cheat_sheet_open.get_untracked() {
                cheat_sheet_open.set(false);
            }
            if command_palette_open.get_untracked() {
                command_palette_open.set(false);
            }
        }
    });
    on_cleanup(move || handle.remove());

    view! {
        <div class="training-form">
            <div class="training-form__header">
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
                <Select
                    value=Signal::derive(move || file_kind.get().key().to_string())
                    options=Signal::derive(|| {
                        FileKind::ALL.iter().map(|k| SelectOption::new(k.key(), k.label())).collect()
                    })
                    on_change=Callback::new(move |v: String| file_kind.set(FileKind::from_key(&v)))
                />
                <FileDropzone
                    accept=".xlsx".to_string()
                    disabled=Signal::derive(move || parsing.get())
                    on_file=Callback::new(on_file_selected)
                />
                <span class="training-form__status">{move || save_status.get()}</span>
                <button class="btn btn--primary" on:click=move |_| do_commit()>
                    "Зберегти все (Ctrl+Enter)"
                </button>
                <button class="btn btn--outline" on:click=move |_| cheat_sheet_open.update(|v| *v = !*v)>
                    "? Шпаргалка"
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

            <Grid editable=editable next_id=next_id active_cell=active_cell before_mutate=snapshot/>

            <Show when=move || cheat_sheet_open.get()>
                <CheatSheet on_close=move || cheat_sheet_open.set(false)/>
            </Show>
            <Show when=move || command_palette_open.get()>
                <CommandPalette
                    on_close=move || command_palette_open.set(false)
                    on_new_row=move || {
                        snapshot();
                        let id = next_id.get_value();
                        next_id.set_value(id + 1);
                        editable.update(|v| {
                            v.push(EditableRow { id, data: RwSignal::new(GroupFormRow::default()) })
                        });
                    }
                />
            </Show>
        </div>
    }
}

#[component]
fn CheatSheet(#[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let shortcuts: [(&str, &str); 14] = [
        ("Tab / Shift+Tab", "наступне / попереднє поле"),
        ("Enter", "підтвердити і перейти далі (як Tab)"),
        ("↑ ↓", "рядок вище/нижче в тій самій колонці"),
        ("Alt+↓", "відкрити підказки поточного поля"),
        ("Esc", "закрити випадайку / скасувати редагування"),
        ("Ctrl+D", "скопіювати значення з клітинки вище"),
        ("Ctrl+Shift+D", "дублювати рядок нижче"),
        ("Ctrl+Enter", "зберегти всі зміни"),
        ("Ctrl+Z / Ctrl+Shift+Z", "undo / redo"),
        ("Ctrl+Delete", "видалити рядок"),
        ("F2", "редагувати клітинку, не стираючи вміст"),
        ("? / F1", "ця шпаргалка"),
        ("Ctrl+K", "командна палітра"),
        ("Ctrl+V", "вставити блок з Excel"),
    ];

    view! {
        // Без "клік по фону -- закрити": фон і кнопка "Закрити" в тому самому дереві, а
        // `Show` синхронно демонтує панель ще до завершення спливання події, тож зовнішній
        // обробник встигав спрацювати на вже скинутому closure ("invoked after being dropped").
        // Закриття -- лише кнопкою або `Esc` (глобальний слухач у `FormBody`).
        <div class="modal__overlay">
            <div class="modal__panel">
                <h2>"Гарячі клавіші"</h2>
                <table>
                    <tbody>
                        {shortcuts
                            .into_iter()
                            .map(|(k, d)| {
                                view! {
                                    <tr>
                                        <td class="cheat-sheet__key">{k}</td>
                                        <td>{d}</td>
                                    </tr>
                                }
                            })
                            .collect_view()}
                    </tbody>
                </table>
                <button class="btn btn--outline" on:click=move |_| on_close.run(())>
                    "Закрити"
                </button>
            </div>
        </div>
    }
}

#[component]
fn CommandPalette(
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_new_row: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="modal__overlay">
            <div class="modal__panel">
                <h2>"Командна палітра"</h2>
                <ul class="command-palette__list">
                    <li>
                        <button
                            class="btn btn--outline"
                            on:click=move |_| {
                                on_new_row.run(());
                                on_close.run(());
                            }
                        >
                            "Новий рядок"
                        </button>
                    </li>
                    <li class="card__desc">
                        "«перейти до частини…» і «згенерувати звіт…» — інших етапів, ще не підключено."
                    </li>
                </ul>
            </div>
        </div>
    }
}
