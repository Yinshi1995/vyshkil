//! Меню рядка (дублювати/видалити) у колонці дій (`docs/spec/components/grid.md` §3, розділ Б).
//! НЕ `components::Combobox` — це вибір ДІЇ, не значення зі списку, інша семантика; власний
//! маленький popover (кнопка + `Portal` + 2 пункти), той самий `hooks::use_popover_position`.
//! "Видалити" лише СИГНАЛІЗУЄ намір (`on_delete_requested`) — підтвердження (`components::Dialog`)
//! лишається за викликачем (`grid.rs`), той самий діалог і для Ctrl+Delete, і для пункту меню.

use leptos::ev;
use leptos::leptos_dom::helpers::window_event_listener;
use leptos::portal::Portal;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::hooks::use_popover_position::use_popover_position;

#[component]
pub fn RowMenu(
    #[prop(into)] on_duplicate: Callback<()>,
    #[prop(into)] on_delete_requested: Callback<()>,
) -> impl IntoView {
    let open = RwSignal::new(false);
    let root: NodeRef<leptos::html::Div> = NodeRef::new();
    let panel_ref: NodeRef<leptos::html::Div> = NodeRef::new();
    let position = use_popover_position(root, open.into());

    let close = move || open.set(false);

    let handle = window_event_listener(ev::mousedown, move |ev| {
        if !open.get_untracked() {
            return;
        }
        let Some(target) = ev.target().and_then(|t| t.dyn_into::<web_sys::Node>().ok()) else {
            return;
        };
        let inside_root = root.get_untracked().is_some_and(|r| r.contains(Some(&target)));
        let inside_panel = panel_ref.get_untracked().is_some_and(|p| p.contains(Some(&target)));
        if !inside_root && !inside_panel {
            close();
        }
    });
    on_cleanup(move || handle.remove());

    view! {
        <div class="row-menu" node_ref=root>
            <button
                type="button"
                class="row-menu__trigger"
                tabindex="-1"
                aria-haspopup="menu"
                aria-expanded=move || open.get().to_string()
                aria-label="Меню рядка"
                on:click=move |_| open.update(|o| *o = !*o)
            >
                <svg viewBox="0 0 4 16" width="4" height="16" aria-hidden="true">
                    <circle cx="2" cy="2" r="1.6" fill="currentColor"/>
                    <circle cx="2" cy="8" r="1.6" fill="currentColor"/>
                    <circle cx="2" cy="14" r="1.6" fill="currentColor"/>
                </svg>
            </button>
            <Show when=move || open.get()>
                <Portal>
                    <div
                        class="row-menu__panel"
                        role="menu"
                        node_ref=panel_ref
                        style:top=move || position.get().and_then(|p| p.top).map(|v| format!("{v}px")).unwrap_or_default()
                        style:bottom=move || position.get().and_then(|p| p.bottom).map(|v| format!("{v}px")).unwrap_or_default()
                        style:left=move || position.get().and_then(|p| p.left).map(|v| format!("{v}px")).unwrap_or_default()
                        style:right=move || position.get().and_then(|p| p.right).map(|v| format!("{v}px")).unwrap_or_default()
                    >
                        <button
                            type="button"
                            role="menuitem"
                            class="row-menu__item"
                            on:click=move |_| {
                                close();
                                on_duplicate.run(());
                            }
                        >
                            "Дублювати"
                        </button>
                        <button
                            type="button"
                            role="menuitem"
                            class="row-menu__item row-menu__item--danger"
                            on:click=move |_| {
                                close();
                                on_delete_requested.run(());
                            }
                        >
                            "Видалити"
                        </button>
                    </div>
                </Portal>
            </Show>
        </div>
    }
}
