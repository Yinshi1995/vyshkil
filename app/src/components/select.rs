//! Стилізована випадайка (07 §1: перший реальний споживач `components/` — не знає домену,
//! просто замінює нативний `<select>`, чий відкритий список не можна стилізувати CSS, тому
//! на темному тлі застосунку він завжди виглядає чужорідною системною панеллю). Той самий
//! візуальний патерн, що й `widgets::group_grid::autocomplete` (`.cell__dropdown`), тут — під
//! своїми класами `.select__*`, бо семантика інша (весь список одразу, не пошук за запитом).

use leptos::ev;
use leptos::leptos_dom::helpers::window_event_listener;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::hooks::use_floating_position::use_floating_position;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

impl SelectOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self { value: value.into(), label: label.into() }
    }
}

/// `value`/`on_change` як у нативного `<select>` (рядкове значення) — виклик сам мапить
/// у свій тип (`i32::parse`, `Role::parse` тощо), той самий підхід, що вже був із `<select
/// on:change=...>` у `ActorSwitcher`/`Grid`, лише інша розмітка під капотом.
///
/// `id`/`on_keydown` — опційні, для вбудовування в `widgets::group_grid::Grid` (замінює
/// `<select>` у `TrainingKindCell`/`SiteCell`): `id` — щоб `Grid::focus_cell` знаходив саме
/// кнопку-тригер (фокусований елемент), `on_keydown` — щоб сітка лишалась власником
/// Tab/Enter/стрілок для навігації МІЖ клітинками (той самий принцип явного передавання
/// необробленої події, що й `widgets::group_grid::autocomplete` — не покладання на bubbling).
/// Коли `on_keydown` заданий ("режим сітки"): стрілки/Enter, поки закрито, НЕ відкривають
/// список (натомість форвардяться в сітку) — відкриття лише кліком або `Alt+↓` (той самий
/// шорткат, що вже задокументований у шпаргалці для полів з підказками). Без `on_keydown`
/// (самостійне вживання, напр. `ActorSwitcher`) — поведінка як була: стрілки відкривають.
#[component]
pub fn Select(
    #[prop(into)] value: Signal<String>,
    #[prop(into)] options: Signal<Vec<SelectOption>>,
    #[prop(into)] on_change: Callback<String>,
    #[prop(optional, into)] placeholder: String,
    #[prop(optional)] id: Option<String>,
    #[prop(optional)] on_keydown: Option<Callback<web_sys::KeyboardEvent>>,
) -> impl IntoView {
    let grid_mode = on_keydown.is_some();
    let forward = move |ev: web_sys::KeyboardEvent| {
        if let Some(cb) = on_keydown {
            cb.run(ev);
        }
    };
    let open = RwSignal::new(false);
    let highlighted = RwSignal::new(0usize);
    let root: NodeRef<leptos::html::Div> = NodeRef::new();
    let (flip_up, align_end) = use_floating_position(root, open.into());

    let current_index = move || {
        let v = value.get();
        options.get().iter().position(|o| o.value == v)
    };

    let select_index = move |i: usize| {
        if let Some(opt) = options.get_untracked().get(i) {
            on_change.run(opt.value.clone());
        }
        open.set(false);
    };

    // Клік поза компонентом закриває випадайку — той самий патерн, що для меню/поповерів.
    let handle = window_event_listener(ev::mousedown, move |ev| {
        if !open.get_untracked() {
            return;
        }
        let Some(root_el) = root.get_untracked() else { return };
        if let Some(target) = ev.target() {
            if let Ok(node) = target.dyn_into::<web_sys::Node>() {
                if root_el.contains(Some(&node)) {
                    return;
                }
            }
        }
        open.set(false);
    });
    on_cleanup(move || handle.remove());

    let on_trigger_keydown = move |ev: web_sys::KeyboardEvent| {
        let len = options.get_untracked().len();
        if len == 0 {
            forward(ev);
            return;
        }

        // Alt+↓ відкриває незалежно від режиму — той самий шорткат, що вже задокументований у
        // шпаргалці для полів з підказками ("Alt+↓ відкрити підказки поточного поля"); голий
        // ArrowDown у режимі сітки цього НЕ робить (див. нижче), бо сітка сама володіє стрілками.
        if ev.key() == "ArrowDown" && ev.alt_key() && !open.get_untracked() {
            ev.prevent_default();
            open.set(true);
            highlighted.set(current_index().unwrap_or(0));
            return;
        }

        match ev.key().as_str() {
            "ArrowDown" => {
                if !open.get_untracked() {
                    if grid_mode {
                        forward(ev);
                        return;
                    }
                    ev.prevent_default();
                    open.set(true);
                    highlighted.set(current_index().unwrap_or(0));
                } else {
                    ev.prevent_default();
                    highlighted.update(|h| *h = (*h + 1).min(len - 1));
                }
            }
            "ArrowUp" => {
                if !open.get_untracked() {
                    if grid_mode {
                        forward(ev);
                        return;
                    }
                    ev.prevent_default();
                    open.set(true);
                    highlighted.set(current_index().unwrap_or(0));
                } else {
                    ev.prevent_default();
                    highlighted.update(|h| *h = h.saturating_sub(1));
                }
            }
            "Home" if open.get_untracked() => {
                ev.prevent_default();
                highlighted.set(0);
            }
            "End" if open.get_untracked() => {
                ev.prevent_default();
                highlighted.set(len - 1);
            }
            // Enter — у сітці зарезервований нею для переходу до наступної клітинки, поки список
            // закритий; Space нею ніде не зарезервований, тому завжди може відкривати.
            "Enter" => {
                if open.get_untracked() {
                    ev.prevent_default();
                    select_index(highlighted.get_untracked());
                    if grid_mode {
                        forward(ev);
                    }
                } else if grid_mode {
                    forward(ev);
                } else {
                    ev.prevent_default();
                    open.set(true);
                    highlighted.set(current_index().unwrap_or(0));
                }
            }
            " " => {
                ev.prevent_default();
                if open.get_untracked() {
                    select_index(highlighted.get_untracked());
                } else {
                    open.set(true);
                    highlighted.set(current_index().unwrap_or(0));
                }
            }
            "Escape" if open.get_untracked() => {
                ev.prevent_default();
                open.set(false);
            }
            "Tab" => {
                open.set(false);
                if grid_mode {
                    forward(ev);
                }
            }
            key if key.chars().count() == 1 && !ev.ctrl_key() && !ev.alt_key() && !ev.meta_key() => {
                // Друкований символ — той самий "стрибок до першого варіанту, що починається на
                // цю літеру" (циклічно), що й нативний `<select>` (02 §-незадокументована UX-звичка).
                let ch = key.chars().next().unwrap().to_lowercase().to_string();
                let opts = options.get_untracked();
                let start = if open.get_untracked() { highlighted.get_untracked() + 1 } else { 0 };
                let found = (0..opts.len())
                    .map(|i| (start + i) % opts.len())
                    .find(|&i| opts[i].label.to_lowercase().starts_with(&ch));
                if let Some(i) = found {
                    open.set(true);
                    highlighted.set(i);
                }
            }
            _ => {
                if grid_mode {
                    forward(ev);
                }
            }
        }
    };

    let current_label = move || {
        let v = value.get();
        options
            .get()
            .iter()
            .find(|o| o.value == v)
            .map(|o| o.label.clone())
            .unwrap_or_else(|| placeholder.clone())
    };

    view! {
        <div class="select" node_ref=root>
            <button
                type="button"
                id=id
                class="select__trigger"
                aria-haspopup="listbox"
                aria-expanded=move || open.get().to_string()
                on:click=move |_| {
                    let was_open = open.get_untracked();
                    open.set(!was_open);
                    if !was_open {
                        highlighted.set(current_index().unwrap_or(0));
                    }
                }
                on:keydown=on_trigger_keydown
            >
                <span class="select__value">{current_label}</span>
                <svg class="select__chevron" viewBox="0 0 12 8" width="12" height="8" aria-hidden="true">
                    <path d="M1 1.5 6 6.5 11 1.5" stroke="currentColor" stroke-width="1.6" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
            </button>
            <Show when=move || open.get()>
                <ul
                    class="select__dropdown"
                    class:select__dropdown--flip-up=flip_up
                    class:select__dropdown--align-end=align_end
                    role="listbox"
                >
                    <For
                        each=move || { options.get().into_iter().enumerate().collect::<Vec<_>>() }
                        key=|(i, o)| (*i, o.value.clone())
                        let:item
                    >
                        {
                            let (i, opt) = item;
                            let is_active = move || highlighted.get() == i;
                            let is_selected = move || value.get() == opt.value;
                            let label = opt.label.clone();
                            view! {
                                <li
                                    class="select__option"
                                    class:select__option--active=is_active
                                    class:select__option--selected=is_selected
                                    role="option"
                                    on:mousedown=move |ev| {
                                        ev.prevent_default();
                                        select_index(i);
                                    }
                                    on:mouseenter=move |_| highlighted.set(i)
                                >
                                    {label}
                                </li>
                            }
                        }
                    </For>
                </ul>
            </Show>
        </div>
    }
}
