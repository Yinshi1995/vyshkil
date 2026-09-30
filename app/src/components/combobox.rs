//! `Combobox` — переробка `Select` з нуля (feedback користувача, 2026-09-30; анатомія/стани/
//! клавіатура — `docs/spec/components/select.md`, дослідження референсів там же). Портал
//! (`leptos::portal::Portal`) + `hooks::use_popover_position` — панель ніколи не обрізається
//! `overflow` таблиці й сама перевертається/зсувається, щоб лишитись у в'юпорті.
//!
//! **`ComboboxMode`** (розділ Б, `docs/spec/components/grid.md` §2): `Trigger` — кнопка
//! відкриває панель, пошук (якщо `searchable`) ВСЕРЕДИНІ панелі, компонент сам фільтрує `items`.
//! `Input` — видимий елемент сам `<input>` (Headless UI Combobox-патерн, заміна
//! `OrgAutocomplete`/`VosPositionCourseAutocomplete`): типізація НЕ фільтрується компонентом
//! (лишається domain-agnostic, не знає про `search_orgs`) — викликач сам тримає
//! `Resource`/`Signal<Vec<ComboboxItem>>` ззовні через `on_query_change`, той самий патерн, що
//! вже був у `OrgAutocomplete`, лише перенесений сюди.

use leptos::ev;
use leptos::leptos_dom::helpers::window_event_listener;
use leptos::portal::Portal;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::hooks::use_popover_position::use_popover_position;

#[derive(Debug, Clone, PartialEq)]
pub struct ComboboxItem {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
    pub group: Option<String>,
}

impl ComboboxItem {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self { value: value.into(), label: label.into(), description: None, group: None }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }
}

/// `InCell` — заповнює клітинку Grid рівно по ширині/висоті рядка, без власної рамки/підкладки
/// (§3 спеки); `Field` — самостійне поле з рамкою, як і форми/тулбари.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ComboboxVariant {
    #[default]
    Field,
    InCell,
}

/// Хто є видимим тригером — див. документацію модуля. `Trigger` — кнопка; `Input` — текстове
/// поле само собою (асинхронний пошук, список фільтрується ЗЗОВНІ).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ComboboxMode {
    #[default]
    Trigger,
    Input,
}

/// Плаский список у порядку `items`, з опційним заголовком групи ПЕРЕД першим елементом кожної
/// групи (групи не пересортовуються — виклик сам розкладає `items` у бажаному порядку).
fn grouped(items: &[ComboboxItem]) -> Vec<(Option<String>, ComboboxItem)> {
    let mut out = Vec::with_capacity(items.len());
    let mut last_group: Option<&str> = None;
    for item in items {
        let group_header = match (&item.group, last_group) {
            (Some(g), Some(last)) if g == last => None,
            (Some(g), _) => Some(g.clone()),
            (None, _) => None,
        };
        out.push((group_header, item.clone()));
        last_group = item.group.as_deref();
    }
    out
}

#[component]
pub fn Combobox(
    #[prop(into)] value: Signal<String>,
    #[prop(into)] items: Signal<Vec<ComboboxItem>>,
    #[prop(into)] on_change: Callback<String>,
    #[prop(optional, into)] placeholder: String,
    #[prop(optional)] variant: ComboboxVariant,
    #[prop(optional)] mode: ComboboxMode,
    #[prop(optional)] searchable: bool,
    #[prop(optional, into)] empty_message: String,
    #[prop(optional)] id: Option<String>,
    #[prop(optional)] on_keydown: Option<Callback<web_sys::KeyboardEvent>>,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional, into)] invalid: Signal<bool>,
    // Лише `mode=Input`: видимий текст поля — ОКРЕМИЙ від `value` (обраний id), бо під час
    // пошуку текст інпута й обране значення розходяться ("вамп" на екрані, поки не обрано
    // "ВОС 218") — той самий поділ, що вже був у `OrgAutocomplete`.
    #[prop(optional, into)] label: Option<Signal<String>>,
    #[prop(optional)] on_label_input: Option<Callback<String>>,
    /// Кожна зміна тексту поля в `mode=Input` — викликач сам перезапускає свій `Resource`
    /// (компонент не знає про `search_orgs`/подібне, лишається domain-agnostic).
    #[prop(optional)] on_query_change: Option<Callback<String>>,
) -> impl IntoView {
    let grid_mode = on_keydown.is_some();
    let input_mode = mode == ComboboxMode::Input;
    let forward = move |ev: web_sys::KeyboardEvent| {
        if let Some(cb) = on_keydown {
            cb.run(ev);
        }
    };

    let open = RwSignal::new(false);
    let highlighted = RwSignal::new(0usize);
    // Чи людина СВІДОМО взаємодіяла зі списком цього відкриття (стрілка/друк) — grid-interaction.md
    // §2: "просто проходить Tab-ом... значення НЕ змінюється". Без цього прапорця будь-який Tab
    // при відкритій панелі (тепер відкривається одразу на фокусі) вибирав би підсвічений пункт 0
    // навіть коли людина лише йшла далі по рядку.
    let touched = RwSignal::new(false);
    let query = RwSignal::new(String::new());
    let root: NodeRef<leptos::html::Div> = NodeRef::new();
    let search_input: NodeRef<leptos::html::Input> = NodeRef::new();
    let field_input: NodeRef<leptos::html::Input> = NodeRef::new();
    let position = use_popover_position(root, open.into());

    let empty_message = Signal::derive(move || {
        if empty_message.is_empty() { "Нічого не знайдено".to_string() } else { empty_message.clone() }
    });

    // `mode=Input`: список приходить УЖЕ відфільтрованим ззовні (сервер-пошук) — компонент не
    // фільтрує вдруге. `mode=Trigger`: фільтрація за `query` лише коли `searchable`.
    let visible_items = Signal::derive(move || {
        let all = items.get();
        if input_mode || !searchable {
            return all;
        }
        let q = query.get().to_lowercase();
        if q.is_empty() {
            return all;
        }
        all.into_iter().filter(|it| it.label.to_lowercase().contains(&q)).collect()
    });

    let current_index = move || {
        let v = value.get();
        visible_items.get().iter().position(|o| o.value == v)
    };

    let close = move || {
        open.set(false);
        if !input_mode {
            query.set(String::new());
        }
    };

    let select_index = move |i: usize| {
        if let Some(item) = visible_items.get_untracked().get(i) {
            on_change.run(item.value.clone());
        }
        close();
    };

    let do_open = move || {
        open.set(true);
        touched.set(false);
        highlighted.set(current_index().unwrap_or(0));
        if searchable && !input_mode {
            request_animation_frame(move || {
                if let Some(el) = search_input.get_untracked() {
                    // `preventScroll` — інакше браузер прокручує сторінку, щоб довести щойно
                    // сфокусований інпут у видиму зону, а це саме той `scroll`, на який панель
                    // закривається (§4 "close on scroll") — і панель закривалась одразу після
                    // відкриття (feedback користувача, 2026-09-30, знайдено через Playwright).
                    let opts = web_sys::FocusOptions::new();
                    opts.set_prevent_scroll(true);
                    let _ = el.focus_with_options(&opts);
                }
            });
        }
    };

    // Клік поза (тригер+портал) закриває — портал не є нащадком `root` у DOM, тому перевіряємо
    // ОБИДВА вузли (той самий патерн, що `Select`/`DatePicker`, лише з другим вузлом порталу).
    let panel_ref: NodeRef<leptos::html::Div> = NodeRef::new();
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

    // Закрити на скрол сторінки (§4 "close on scroll") — `position:fixed` не стежить за
    // прокруткою сама, найпростіший робастний вибір без повного resize/scroll-спостерігача.
    let scroll_handle = window_event_listener(ev::scroll, move |_| {
        if open.get_untracked() {
            close();
        }
    });
    on_cleanup(move || scroll_handle.remove());

    let move_highlight = move |delta: i64| {
        let len = visible_items.get_untracked().len();
        if len == 0 {
            return;
        }
        touched.set(true);
        let cur = highlighted.get_untracked() as i64;
        highlighted.set((cur + delta).clamp(0, len as i64 - 1) as usize);
    };

    let on_trigger_keydown = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "ArrowDown" && ev.alt_key() && !open.get_untracked() {
            ev.prevent_default();
            do_open();
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
                    do_open();
                } else {
                    ev.prevent_default();
                    move_highlight(1);
                }
            }
            "ArrowUp" => {
                if !open.get_untracked() {
                    if grid_mode {
                        forward(ev);
                        return;
                    }
                    ev.prevent_default();
                    do_open();
                } else {
                    ev.prevent_default();
                    move_highlight(-1);
                }
            }
            "Home" if open.get_untracked() => {
                ev.prevent_default();
                highlighted.set(0);
            }
            "End" if open.get_untracked() => {
                ev.prevent_default();
                highlighted.set(visible_items.get_untracked().len().saturating_sub(1));
            }
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
                    do_open();
                }
            }
            " " if !searchable => {
                ev.prevent_default();
                if open.get_untracked() {
                    select_index(highlighted.get_untracked());
                } else {
                    do_open();
                }
            }
            "Escape" if open.get_untracked() => {
                ev.prevent_default();
                close();
            }
            "Tab" => {
                // Shift+Tab — завжди назад БЕЗ вибору; звичайний Tab вибирає підсвічене лише
                // якщо людина СВІДОМО торкнулась списку цього відкриття (стрілка/друк) —
                // grid-interaction.md §2.
                if open.get_untracked() && !ev.shift_key() && touched.get_untracked() {
                    select_index(highlighted.get_untracked());
                } else if open.get_untracked() {
                    close();
                }
                if grid_mode {
                    forward(ev);
                }
            }
            key if !searchable
                && key.chars().count() == 1
                && !ev.ctrl_key()
                && !ev.alt_key()
                && !ev.meta_key() =>
            {
                let ch = key.chars().next().unwrap().to_lowercase().to_string();
                let opts = visible_items.get_untracked();
                if opts.is_empty() {
                    return;
                }
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

    let on_search_keydown = move |ev: web_sys::KeyboardEvent| match ev.key().as_str() {
        "ArrowDown" => {
            ev.prevent_default();
            move_highlight(1);
        }
        "ArrowUp" => {
            ev.prevent_default();
            move_highlight(-1);
        }
        "Home" => {
            ev.prevent_default();
            highlighted.set(0);
        }
        "End" => {
            ev.prevent_default();
            highlighted.set(visible_items.get_untracked().len().saturating_sub(1));
        }
        "Enter" => {
            ev.prevent_default();
            select_index(highlighted.get_untracked());
        }
        "Escape" => {
            ev.prevent_default();
            close();
        }
        _ => {}
    };

    // `mode=Input`: клавіатура НЕ перевикористовує `on_trigger_keydown` — той тримає гілки
    // (typeahead на одну літеру, Space-select), розраховані на кнопку без власного тексту; поле
    // вводу приймає текст напряму. Контракт тут — той самий, що вже мав `OrgAutocomplete`.
    let on_field_keydown = move |ev: web_sys::KeyboardEvent| {
        let is_open = open.get_untracked();
        let len = visible_items.get_untracked().len();
        match ev.key().as_str() {
            "ArrowDown" if is_open && len > 0 => {
                ev.prevent_default();
                move_highlight(1);
            }
            "ArrowUp" if is_open && len > 0 => {
                ev.prevent_default();
                move_highlight(-1);
            }
            "Escape" if is_open => {
                ev.prevent_default();
                ev.stop_propagation();
                close();
            }
            "Enter" if !ev.ctrl_key() && is_open && len > 0 => {
                select_index(highlighted.get_untracked());
                forward(ev);
            }
            "Tab" if is_open && len > 0 && !ev.shift_key() && touched.get_untracked() => {
                select_index(highlighted.get_untracked());
                forward(ev);
            }
            _ => forward(ev),
        }
    };

    let placeholder_for_field = placeholder.clone();
    let current_label = Signal::derive(move || {
        let v = value.get();
        items
            .get()
            .iter()
            .find(|o| o.value == v)
            .map(|o| o.label.clone())
            .unwrap_or_else(|| placeholder.clone())
    });

    let field_text = Signal::derive(move || label.map(|l| l.get()).unwrap_or_default());

    let trigger_class = move || match variant {
        ComboboxVariant::Field => "combobox__trigger combobox__trigger--field",
        ComboboxVariant::InCell => "combobox__trigger combobox__trigger--in-cell",
    };
    let field_class = move || match variant {
        ComboboxVariant::Field => "combobox__field combobox__field--field",
        ComboboxVariant::InCell => "combobox__field combobox__field--in-cell",
    };

    // Панель відкрита разом з `open` у ВСІХ режимах, незалежно від тексту (grid-interaction.md
    // §2: "Фокус... → одразу відкритий список: недавні... далі весь довідник" — раніше `mode=
    // Input` показувала панель лише коли поле вже непорожнє, тому автокомпліт мовчав на самому
    // фокусі; викликач сам відповідає за те, щоб `items` містив і недавні, і довідник навіть для
    // порожнього запиту).
    let panel_open = Signal::derive(move || open.get());

    let id_for_button = id.clone();

    view! {
        <div class="combobox" node_ref=root>
            <Show
                when=move || input_mode
                fallback=move || {
                    view! {
                        <button
                            type="button"
                            id=id_for_button.clone()
                            class=trigger_class
                            class:combobox__trigger--empty=move || current_label.get().trim().is_empty()
                            disabled=move || disabled.get()
                            aria-haspopup="listbox"
                            aria-expanded=move || open.get().to_string()
                            aria-invalid=move || invalid.get().to_string()
                            title=current_label
                            on:click=move |_| {
                                // Мишачий клік по кнопці шле `focus` ПЕРЕД `click` (браузер сам
                                // фокусує ціль на mousedown) -- `on:focus` нижче вже відкриває
                                // панель до того, як цей обробник встигає спрацювати. Тож "клік
                                // по вже відкритій" тут означає "щойно відкрито тим самим кліком",
                                // не "користувач хоче закрити" -- toggle-закриття кліком
                                // самознищувалось б щоразу. Закриття лишається за Escape/вибором
                                // /кліком поза.
                                if disabled.get_untracked() {
                                    return;
                                }
                                do_open();
                            }
                            on:focus=move |_| {
                                if !disabled.get_untracked() && !open.get_untracked() {
                                    do_open();
                                }
                            }
                            on:keydown=on_trigger_keydown
                        >
                            <span class="combobox__value">{current_label}</span>
                            // Порожня клітинка в спокої -- порожньо (не "—"); підказка "Порожньо"
                            // лише на hover/фокус (grid-interaction.md §5, дефект 10) -- CSS-гейт
                            // через .combobox__trigger--empty, не завжди-видимий текст.
                            <span class="combobox__empty-hint" aria-hidden="true">"Порожньо"</span>
                            <svg class="combobox__chevron" viewBox="0 0 12 8" width="12" height="8" aria-hidden="true">
                                <path d="M1 1.5 6 6.5 11 1.5" stroke="currentColor" stroke-width="1.6" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
                            </svg>
                        </button>
                    }
                }
            >
                <input
                    type="text"
                    id=id.clone()
                    class=field_class
                    node_ref=field_input
                    placeholder=placeholder_for_field.clone()
                    disabled=move || disabled.get()
                    aria-haspopup="listbox"
                    aria-expanded=move || open.get().to_string()
                    aria-invalid=move || invalid.get().to_string()
                    prop:value=move || field_text.get()
                    on:input=move |ev| {
                        let v = event_target_value(&ev);
                        if let Some(cb) = on_label_input {
                            cb.run(v.clone());
                        }
                        if let Some(cb) = on_query_change {
                            cb.run(v.clone());
                        }
                        open.set(true);
                        touched.set(true);
                        highlighted.set(0);
                    }
                    on:focus=move |_| {
                        if !disabled.get_untracked() && !open.get_untracked() {
                            do_open();
                        }
                    }
                    on:keydown=on_field_keydown
                />
            </Show>
            <Show when=move || panel_open.get()>
                <Portal>
                    <div
                        class="combobox__panel"
                        node_ref=panel_ref
                        role="listbox"
                        style:top=move || position.get().and_then(|p| p.top).map(|v| format!("{v}px")).unwrap_or_default()
                        style:bottom=move || position.get().and_then(|p| p.bottom).map(|v| format!("{v}px")).unwrap_or_default()
                        style:left=move || position.get().and_then(|p| p.left).map(|v| format!("{v}px")).unwrap_or_default()
                        style:right=move || position.get().and_then(|p| p.right).map(|v| format!("{v}px")).unwrap_or_default()
                        style:min-width=move || position.get().map(|p| format!("{}px", p.min_width)).unwrap_or_default()
                        style:max-height=move || position.get().map(|p| format!("{}px", p.max_height)).unwrap_or_default()
                    >
                        <Show when=move || searchable && !input_mode>
                            <input
                                type="text"
                                class="combobox__search"
                                node_ref=search_input
                                placeholder="Пошук…"
                                prop:value=move || query.get()
                                on:input=move |ev| {
                                    query.set(event_target_value(&ev));
                                    highlighted.set(0);
                                }
                                on:keydown=on_search_keydown
                            />
                        </Show>
                        <div class="combobox__list">
                            {move || {
                                let list = visible_items.get();
                                if list.is_empty() {
                                    return view! {
                                        <div class="combobox__empty">{empty_message.get()}</div>
                                    }
                                        .into_any();
                                }
                                grouped(&list)
                                    .into_iter()
                                    .enumerate()
                                    .map(|(i, (group_header, item))| {
                                        let is_active = move || highlighted.get() == i;
                                        let is_selected = {
                                            let item_value = item.value.clone();
                                            Signal::derive(move || value.get() == item_value)
                                        };
                                        let label = item.label.clone();
                                        let description = item.description.clone();
                                        view! {
                                            <>
                                                {group_header
                                                    .map(|g| view! { <div class="combobox__group">{g}</div> })}
                                                <div
                                                    class="combobox__option"
                                                    class:combobox__option--active=is_active
                                                    class:combobox__option--selected=is_selected
                                                    role="option"
                                                    on:mousedown=move |ev| {
                                                        ev.prevent_default();
                                                        select_index(i);
                                                    }
                                                    on:mouseenter=move |_| highlighted.set(i)
                                                >
                                                    <div class="combobox__option-text">
                                                        <span class="combobox__option-label">{label}</span>
                                                        {description
                                                            .map(|d| {
                                                                view! {
                                                                    <span class="combobox__option-desc">{d}</span>
                                                                }
                                                            })}
                                                    </div>
                                                    <Show when=move || is_selected.get()>
                                                        <svg class="combobox__check" viewBox="0 0 14 11" width="14" height="11" aria-hidden="true">
                                                            <path d="M1 5.5 5 9.5 13 1.5" stroke="currentColor" stroke-width="1.8" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
                                                        </svg>
                                                    </Show>
                                                </div>
                                            </>
                                        }
                                    })
                                    .collect_view()
                                    .into_any()
                            }}
                        </div>
                    </div>
                </Portal>
            </Show>
        </div>
    }
}
