//! Вибір файлу (переробка з нуля — `docs/spec/components/file-input.md`,
//! `.claude/decisions/label-for-file-input.md`): реальний `<input type="file">`, обгорнутий
//! `<label>` — той самий "visually-hidden + native wrap" патерн, що `Checkbox`
//! (`components::checkbox`), а не приховування `display:none` + програмний `.click()` з
//! `stop_propagation()`-форвардингом (стара реалізація) — та зв'язка задокументовано ненадійна
//! крос-браузерно/крос-вебвʼю для відкриття системного діалогу. `<label>`-обгортка дає клік І
//! клавіатуру (Enter/Space на сфокусованому `<input>`) БЕЗПЛАТНО, без жодного JS-форвардингу.
//!
//! Перетягування файлу тепер ловиться на рівні `window` (не самого елемента) — "будь-де на
//! сторінці" показує повноекранний приймач-оверлей (portal), а не лише над маленькою кнопкою;
//! лічильник dragenter/dragleave (не булевий прапорець) — стандартний захист від фліку, коли
//! курсор перетинає дочірні елементи всередині зони.

use leptos::ev;
use leptos::leptos_dom::helpers::window_event_listener;
use leptos::portal::Portal;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// Читає обраний файл у байти (`File::array_buffer()` Promise → `JsFuture`) — спільний шлях для
/// click-обрання й drag-drop. `on_error` — обов'язковий (не мовчати): раніше відмова
/// `array_buffer()` просто повертала функцію, і виклик "завис" на `parsing=true` назавжди.
pub fn read_file_bytes(
    file: web_sys::File,
    on_bytes: impl FnOnce(Vec<u8>) + 'static,
    on_error: impl FnOnce(String) + 'static,
) {
    leptos::task::spawn_local(async move {
        let promise = file.array_buffer();
        match wasm_bindgen_futures::JsFuture::from(promise).await {
            Ok(buf) => on_bytes(js_sys::Uint8Array::new(&buf).to_vec()),
            Err(_) => on_error("не вдалось прочитати файл".to_string()),
        }
    });
}

/// Розмір людською мовою (КБ/МБ) — для чипа обраного файлу.
fn human_size(bytes: f64) -> String {
    if bytes < 1024.0 {
        format!("{bytes:.0} Б")
    } else if bytes < 1024.0 * 1024.0 {
        format!("{:.0} КБ", bytes / 1024.0)
    } else {
        format!("{:.1} МБ", bytes / (1024.0 * 1024.0))
    }
}

/// `accept=".xlsx,.xls"` → чи закінчується імʼя файлу на один з розширень (регістронезалежно).
/// Порожній `accept` = приймати будь-що (перевірка не застосовується).
fn extension_allowed(accept: &str, file_name: &str) -> bool {
    if accept.trim().is_empty() {
        return true;
    }
    let name_lower = file_name.to_lowercase();
    accept.split(',').map(str::trim).filter(|s| !s.is_empty()).any(|ext| name_lower.ends_with(&ext.to_lowercase()))
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum FileDropzoneVariant {
    /// Компакт-кнопка для тулбару таблиці (іконка + текст).
    #[default]
    Compact,
    /// Велика зона для сторінки, де файл — головна дія (`/import`).
    Zone,
}

/// Стадія обробки обраного файлу — ЦЕЙ компонент знає лише "обрано" (імʼя/розмір); стадії
/// читання-на-сервері/розбору знає лише виклик (async parse — за межами компонента), тож стадію
/// передає контрольованим пропом `stage`, компонент лише малює чип за нею.
#[derive(Clone, PartialEq, Eq, Default)]
pub enum FileStage {
    #[default]
    Idle,
    Reading,
    Parsing,
    Ready,
    Error(String),
}

#[component]
#[allow(unused_parens)] // `(drag_depth.get() > 0)` нижче -- дужки потрібні для rstml, не для rustc
pub fn FileDropzone(
    #[prop(into)] on_file: Callback<web_sys::File>,
    #[prop(optional, into)] accept: String,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional)] variant: FileDropzoneVariant,
    #[prop(optional, into)] label: String,
    #[prop(optional, into)] stage: Signal<FileStage>,
    #[prop(optional, into)] on_clear: Option<Callback<()>>,
    #[prop(optional, default = 20.0)] max_size_mb: f64,
) -> impl IntoView {
    let chosen: RwSignal<Option<(String, f64)>> = RwSignal::new(None);
    let local_error: RwSignal<Option<String>> = RwSignal::new(None);
    let drag_depth = RwSignal::new(0i32);

    // `StoredValue` (Copy handle), не голий `String` -- інакше замикання, що його бере, саме не
    // `Copy`, і не могло б повторно братись і в `window`-listener'и, і в `on:change` нижче.
    let accept_for_check = StoredValue::new(accept.clone());
    let try_accept_file = move |file: &web_sys::File| -> Result<(), String> {
        let name = file.name();
        let accept_for_check = accept_for_check.get_value();
        if !extension_allowed(&accept_for_check, &name) {
            return Err(format!("неправильний тип файлу — очікується {accept_for_check}"));
        }
        let size_mb = file.size() / (1024.0 * 1024.0);
        if size_mb > max_size_mb {
            return Err(format!("файл завеликий ({size_mb:.1} МБ, максимум {max_size_mb:.0} МБ)"));
        }
        Ok(())
    };

    let handle_file = move |file: web_sys::File| {
        if disabled.get_untracked() {
            return;
        }
        match try_accept_file(&file) {
            Ok(()) => {
                local_error.set(None);
                chosen.set(Some((file.name(), file.size())));
                on_file.run(file);
            }
            Err(e) => local_error.set(Some(e)),
        }
    };

    // Перетягування — на рівні `window`, не самого елемента (drag-anywhere, 04-user-brief);
    // лічильник, не булевий прапорець -- dragenter/dragleave фліпають на кожному переході між
    // дочірніми елементами під курсором, лічильник ловить лише "увійшли в документ"/"вийшли".
    let dragenter_handle = window_event_listener(ev::dragenter, move |ev: web_sys::DragEvent| {
        if disabled.get_untracked() {
            return;
        }
        let has_files = ev
            .data_transfer()
            .map(|dt| dt.types().to_vec().iter().any(|t| t.as_string().as_deref() == Some("Files")))
            .unwrap_or(false);
        if has_files {
            ev.prevent_default();
            drag_depth.update(|n| *n += 1);
        }
    });
    let dragover_handle = window_event_listener(ev::dragover, move |ev: web_sys::DragEvent| {
        if drag_depth.get_untracked() > 0 {
            ev.prevent_default();
        }
    });
    let dragleave_handle = window_event_listener(ev::dragleave, move |_ev| {
        drag_depth.update(|n| *n = (*n - 1).max(0));
    });
    let drop_handle = window_event_listener(ev::drop, move |ev: web_sys::DragEvent| {
        if drag_depth.get_untracked() == 0 {
            return;
        }
        ev.prevent_default();
        drag_depth.set(0);
        if let Some(file) = ev.data_transfer().and_then(|dt| dt.files()).and_then(|f| f.get(0)) {
            handle_file(file);
        }
    });
    on_cleanup(move || {
        dragenter_handle.remove();
        dragover_handle.remove();
        dragleave_handle.remove();
        drop_handle.remove();
    });

    let trigger_label = if label.is_empty() { "Імпорт".to_string() } else { label };
    let zone_hint = format!("Перетягніть файл сюди або натисніть, щоб обрати ({accept})");
    // `StoredValue` (Copy handle), не голий `String.clone()` -- `<Show>`'s children — `Fn`,
    // викликається на кожен реренедер; `move ||`-замикання, що рухає НЕ-Copy `String` усередину,
    // саме по собі стає `FnOnce` (переміщення можливе лише один раз), той самий клас багу, що вже
    // ловився в `combobox.rs` цієї сесії.
    let accept_for_overlay = StoredValue::new(accept.clone());

    view! {
        <div class="dropzone-wrap">
            <label
                class="dropzone"
                class:dropzone--compact=move || variant == FileDropzoneVariant::Compact
                class:dropzone--zone=move || variant == FileDropzoneVariant::Zone
                class:dropzone--disabled=move || disabled.get()
            >
                <input
                    type="file"
                    class="dropzone__input"
                    accept=accept
                    disabled=move || disabled.get()
                    on:change=move |ev| {
                        if let Some(input) = ev.target().and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok()) {
                            if let Some(file) = input.files().and_then(|f| f.get(0)) {
                                handle_file(file);
                            }
                            // Скинути value -- інакше повторний вибір ТОГО САМОГО файлу не шле `change`.
                            input.set_value("");
                        }
                    }
                />
                <svg class="dropzone__icon" viewBox="0 0 24 24" width=move || if variant == FileDropzoneVariant::Zone { "28" } else { "16" } height=move || if variant == FileDropzoneVariant::Zone { "28" } else { "16" } aria-hidden="true">
                    <path
                        d="M12 15V4M12 4 7 9M12 4l5 5M4 16v3a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-3"
                        stroke="currentColor"
                        stroke-width="1.6"
                        fill="none"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    />
                </svg>
                {
                    let trigger_label = trigger_label.clone();
                    let zone_hint = zone_hint.clone();
                    move || {
                        if variant == FileDropzoneVariant::Zone {
                            view! { <p class="dropzone__text">{zone_hint.clone()}</p> }.into_any()
                        } else {
                            view! { <span class="dropzone__text">{trigger_label.clone()}</span> }.into_any()
                        }
                    }
                }
            </label>

            {move || local_error.get().map(|e| view! { <p class="status-error t-xs">{e}</p> })}

            {move || {
                chosen.get().map(|(name, size)| {
                    let size_label = human_size(size);
                    view! {
                        <div class="file-chip">
                            <span class="file-chip__stage" aria-hidden="true">
                                {move || match stage.get() {
                                    FileStage::Idle | FileStage::Ready => "✓",
                                    FileStage::Reading | FileStage::Parsing => "…",
                                    FileStage::Error(_) => "!",
                                }}
                            </span>
                            <span class="file-chip__name">{name}</span>
                            <span class="file-chip__size">{size_label}</span>
                            {move || {
                                if let FileStage::Error(msg) = stage.get() {
                                    Some(view! { <span class="file-chip__error">{msg}</span> })
                                } else {
                                    None
                                }
                            }}
                            <button
                                type="button"
                                class="file-chip__clear"
                                aria-label="Прибрати файл"
                                on:click=move |_| {
                                    chosen.set(None);
                                    local_error.set(None);
                                    if let Some(cb) = on_clear {
                                        cb.run(());
                                    }
                                }
                            >
                                "✕"
                            </button>
                        </div>
                    }
                })
            }}

            <Portal>
                // Дужки навколо `>` навмисні -- без них `view!`-макрос (rstml) сприймає `>`
                // як закриття тегу, не як оператор порівняння, і падає з незрозумілою помилкою типів.
                <Show when=move || (drag_depth.get() > 0)>
                    <div class="dropzone-overlay">
                        <div class="dropzone-overlay__panel">
                            <svg class="dropzone__icon" viewBox="0 0 24 24" width="40" height="40" aria-hidden="true">
                                <path
                                    d="M12 15V4M12 4 7 9M12 4l5 5M4 16v3a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-3"
                                    stroke="currentColor"
                                    stroke-width="1.4"
                                    fill="none"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                />
                            </svg>
                            <p>"Відпустіть файл, щоб імпортувати"</p>
                            <p class="t-xs fg-muted">{move || accept_for_overlay.get_value()}</p>
                        </div>
                    </div>
                </Show>
            </Portal>
        </div>
    }
}
