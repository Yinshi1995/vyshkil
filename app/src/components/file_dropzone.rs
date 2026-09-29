//! Інтерактивна зона вибору файлу (feedback користувача) — заміна голого `<input type="file">`
//! з системною кнопкою "Choose file": перетягнути файл АБО клікнути будь-де в зоні, щоб відкрити
//! системний діалог. Прихований `<input type="file">` лишається одним джерелом істини для обох
//! шляхів (клік і drag-drop дають той самий `web_sys::File`, обробка — в один callback).

use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[component]
pub fn FileDropzone(
    #[prop(into)] on_file: Callback<web_sys::File>,
    #[prop(optional, into)] accept: String,
    #[prop(optional, into)] disabled: Signal<bool>,
) -> impl IntoView {
    let input_ref: NodeRef<leptos::html::Input> = NodeRef::new();
    let dragging = RwSignal::new(false);
    let file_name = RwSignal::new(String::new());

    let handle_file = move |file: web_sys::File| {
        file_name.set(file.name());
        on_file.run(file);
    };

    let open_picker = move || {
        if !disabled.get_untracked() {
            if let Some(el) = input_ref.get_untracked() {
                el.click();
            }
        }
    };

    view! {
        <div
            class="dropzone"
            class:dropzone--dragging=move || dragging.get()
            class:dropzone--disabled=move || disabled.get()
            role="button"
            tabindex="0"
            on:click=move |_| open_picker()
            on:keydown=move |ev| {
                if ev.key() == "Enter" || ev.key() == " " {
                    ev.prevent_default();
                    open_picker();
                }
            }
            on:dragover=move |ev| {
                ev.prevent_default();
                if !disabled.get_untracked() {
                    dragging.set(true);
                }
            }
            on:dragleave=move |_| dragging.set(false)
            on:drop=move |ev| {
                ev.prevent_default();
                dragging.set(false);
                if disabled.get_untracked() {
                    return;
                }
                if let Some(file) = ev.data_transfer().and_then(|dt| dt.files()).and_then(|f| f.get(0)) {
                    handle_file(file);
                }
            }
        >
            <svg class="dropzone__icon" viewBox="0 0 24 24" width="28" height="28" aria-hidden="true">
                <path
                    d="M12 15V4M12 4 7 9M12 4l5 5M4 16v3a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-3"
                    stroke="currentColor"
                    stroke-width="1.6"
                    fill="none"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                />
            </svg>
            <p class="dropzone__text">
                {move || {
                    let name = file_name.get();
                    if name.is_empty() {
                        "Перетягніть файл сюди або натисніть, щоб обрати".to_string()
                    } else {
                        name
                    }
                }}
            </p>
            <input
                type="file"
                class="dropzone__input"
                accept=accept
                node_ref=input_ref
                on:click=move |ev| ev.stop_propagation()
                on:change=move |ev| {
                    if let Some(file) = ev
                        .target()
                        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                        .and_then(|input| input.files())
                        .and_then(|f| f.get(0))
                    {
                        handle_file(file);
                    }
                }
            />
        </div>
    }
}
