//! Сторінка `/styleguide` (Фаза 2, docs/spec/08-style-system.md) — живий довідник атомів/токенів/
//! тем стильової системи, лише для `admin` (внутрішній інструмент розробки, не частина домену).
//! Перемикач тем міняє `data-theme` на `<html>` (те, що реально таргетить згенерований CSS —
//! `:root[data-theme="..."]`), скидається при виході зі сторінки, щоб не "протікати" в решту
//! застосунку (тут нема повноцінного per-користувач теми-перемикача — поза обсягом Фази 2).

use leptos::prelude::*;

use crate::components::{
    Accordion, AccordionItem, Checkbox, Combobox, ComboboxItem, ComboboxVariant, DatePicker,
    Dialog, Modal,
};
use crate::hooks::use_actor::use_actor;
use crate::layout::{ContentWidth, PageContent, PageHeader};
use crate::types::actor::Role;
use style_macros::cx;

/// Лише клієнт: на сервері `document()` торкається wasm-bindgen JS-статиків, яких нема на
/// нативному таргеті (паніка "cannot access imported statics on non-wasm targets") — `Effect::new`
/// виконується і при SSR-рендері, тому цю перевірку не можна пропустити.
fn set_theme(name: &str) {
    if !cfg!(target_arch = "wasm32") {
        return;
    }
    if let Some(el) = document().document_element() {
        let _ = el.set_attribute("data-theme", name);
    }
}

#[component]
pub fn StyleguidePage() -> impl IntoView {
    let actor = use_actor();
    let is_admin = move || actor.get().map(|a| a.role == Role::Admin).unwrap_or(false);

    let current_theme = RwSignal::new("night");
    Effect::new(move |_| set_theme(current_theme.get()));
    on_cleanup(|| set_theme("night"));

    let checkbox_checked = RwSignal::new(false);
    let modal_open = RwSignal::new(false);
    let dialog_open = RwSignal::new(false);
    let picked_date = RwSignal::new(None::<chrono::NaiveDate>);

    // --- Combobox (docs/spec/components/select.md) ---
    let kind_items = Signal::derive(|| {
        vec![
            ComboboxItem::new("bzvp", "БЗВП"),
            ComboboxItem::new("special", "Фахова"),
            ComboboxItem::new("adaptation", "Адаптація"),
        ]
    });
    let cb_closed = RwSignal::new(String::new());
    let cb_selected = RwSignal::new("special".to_string());
    let cb_invalid = RwSignal::new(String::new());

    // in-cell — той самий набір станів, що field (07 §7 вимагає обидва режими), окремі сигнали,
    // щоб перемикання в одному ряду не смикало інший.
    let cb_cell_closed = RwSignal::new(String::new());
    let cb_cell_selected = RwSignal::new("special".to_string());
    let cb_cell_invalid = RwSignal::new(String::new());
    let cb_cell_long = RwSignal::new("long".to_string());
    let cb_cell_vos = RwSignal::new(String::new());
    let cb_cell_many = RwSignal::new(String::new());

    // "Довгий текст" — перевірка ellipsis+tooltip на вузькому тригері.
    let long_items = Signal::derive(|| {
        vec![ComboboxItem::new(
            "long",
            "241 окрема бригада територіальної оборони (дуже довга назва для перевірки обрізання)",
        )]
    });
    let cb_long = RwSignal::new("long".to_string());

    // "З пошуком + групи + пояснення" — той самий патерн, що підказка ВОС "вамп → 218, бо Vampire".
    let vos_items = Signal::derive(|| {
        vec![
            ComboboxItem::new("218", "ВОС 218 — зовнішній пілот (оператор) БпЛА")
                .with_description("бо \"Vampire\" → 218")
                .with_group("БпЛА"),
            ComboboxItem::new("217", "ВОС 217 — оператор БпЛА (Mavic/Matrice)")
                .with_description("бо \"Mavic\" → 217")
                .with_group("БпЛА"),
            ComboboxItem::new("219", "ВОС 219 — оператор FPV-дронів")
                .with_description("бо \"FPV\" → 219")
                .with_group("БпЛА"),
            ComboboxItem::new("117", "ВОС 117 — навідник танка").with_group("Бронетехніка"),
            ComboboxItem::new("121", "ВОС 121 — механік-водій БМП").with_group("Бронетехніка"),
        ]
    });
    let cb_vos = RwSignal::new(String::new());

    // "300 елементів" — перевірка продуктивності рендеру довгого списку (не гальмує typeahead).
    let many_items = Signal::derive(|| {
        (1..=300).map(|i| ComboboxItem::new(i.to_string(), format!("Пункт №{i}"))).collect::<Vec<_>>()
    });
    let cb_many = RwSignal::new(String::new());

    view! {
        <PageHeader title="Стильова система — довідник".to_string()/>
        <PageContent width=ContentWidth::Detail>
        <Show when=is_admin fallback=|| view! { <p>"Сторінка лише для адміністратора."</p> }>
            <p>"Повний опис — docs/spec/08-style-system.md. Шпаргалка для агента — style/ATOMS.md."</p>

            <div class="eyebrow">"Тема"</div>
            <div class=cx!("flex gap2")>
                {style::theme_names()
                    .into_iter()
                    .map(|name| {
                        view! {
                            <button
                                class=cx!("p2 bd bg-panel fg-main hover:bg-raised")
                                on:click=move |_| current_theme.set(name)
                            >
                                {name}
                            </button>
                        }
                    })
                    .collect_view()}
            </div>

            <div class="eyebrow">"Приклад"</div>
            <div class=cx!("flex gap3 items-c")>
                <button class="btn btn--primary">"Зафіксувати →"</button>
                <button class="btn btn--outline">"Скасувати →"</button>
                <div class=cx!("bracket p3 bg-raised fg-main")>"Кутові скоби (тактичний мотив)"</div>
            </div>
            <p class=cx!("fg-muted")>"(кнопка — рецепт .btn/.btn--primary, не набір атомів: градієнт і 6-точковий clip-path не зводяться до однієї CSS-властивості на атом; кольори всередині — токени, форма — виміряна з striy.pp.ua)"</p>

            <div class="eyebrow">"Семантичні кольори"</div>
            <ul class=cx!("flex col gap1")>
                {[
                    ("surface-base", cx!("bg-base p1")),
                    ("surface-panel", cx!("bg-panel p1")),
                    ("surface-raised", cx!("bg-raised p1")),
                    ("fg-main", cx!("fg-main")),
                    ("fg-muted", cx!("fg-muted")),
                    ("fg-accent", cx!("fg-accent")),
                    ("ok", cx!("fg-ok")),
                    ("warn", cx!("fg-warn")),
                    ("danger", cx!("fg-danger")),
                    ("info", cx!("fg-info")),
                ]
                    .into_iter()
                    .map(|(label, class)| view! { <li class=class>{label}</li> })
                    .collect_view()}
            </ul>

            <div class="eyebrow">"Варіанти стану (наведи/фокус/клік)"</div>
            <div class=cx!("flex gap2")>
                <button class=cx!("p2 bd bg-panel fg-main hover:bg-raised")>"hover:bg-raised"</button>
                <button class=cx!("p2 bd bg-panel fg-main focus-visible:bd-accent")>"focus-visible:bd-accent"</button>
                <button class=cx!("p2 bd bg-panel fg-main active:bg-raised")>"active:bg-raised"</button>
            </div>

            <div class="eyebrow">"Відступи (шкала 0..8)"</div>
            <div class=cx!("flex gap2 items-e")>
                {(0..=8)
                    .map(|i| {
                        view! {
                            <div
                                class=format!("p{i} bg-panel bd")
                                title=format!("p{i}")
                            >
                                {i.to_string()}
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
            <p class=cx!("fg-muted")>"(шкала показана прямими class-рядками, не cx! — цикл генерує ім'я атома в рантаймі, макро-валідація працює лише на літералах)"</p>

            <div class="eyebrow">"Компоненти (HeroUI-референс, feedback користувача)"</div>
            <div class=cx!("flex gap4 items-c")>
                <Checkbox checked=checkbox_checked on_change=Callback::new(move |v| checkbox_checked.set(v)) label="Прийняти умови".to_string()/>
                <DatePicker value=picked_date on_change=Callback::new(move |v| picked_date.set(v)) placeholder="дд.мм.рррр".to_string()/>
            </div>
            <div class=cx!("mt3 maxw-prose")>
                <Accordion>
                    <AccordionItem title="Як додати новий атом?".to_string()>
                        <p class=cx!("fg-muted")>"Рядок у таблиці граматики style/grammar_data.rs — style/ATOMS.md."</p>
                    </AccordionItem>
                    <AccordionItem title="Чому власна система, не Tailwind?".to_string()>
                        <p class=cx!("fg-muted")>".claude/decisions/style-system-architecture.md — без Node/CDN, compile-time валідація."</p>
                    </AccordionItem>
                </Accordion>
            </div>

            <div class=cx!("flex gap3 items-c mt3")>
                <button class="btn btn--outline" on:click=move |_| modal_open.set(true)>"Відкрити модалку"</button>
                <button class="btn btn--outline" on:click=move |_| dialog_open.set(true)>"Відкрити діалог"</button>
            </div>
            <Modal open=modal_open on_close=Callback::new(move |_| modal_open.set(false))>
                <h2 class="modal__title">"Довільний вміст"</h2>
                <p class=cx!("fg-muted")>"Modal — оверлей+панель, Escape закриває, клік-поза НЕ закриває (навмисно)."</p>
            </Modal>
            <Dialog
                open=dialog_open
                on_close=Callback::new(move |_| dialog_open.set(false))
                title="Підтвердити дію?".to_string()
                description="Dialog — Modal + заголовок/опис/рядок дій.".to_string()
            >
                <button class="btn btn--outline" on:click=move |_| dialog_open.set(false)>"Скасувати"</button>
                <button class="btn btn--primary" on:click=move |_| dialog_open.set(false)>"Підтвердити"</button>
            </Dialog>

            <div class="eyebrow">"Combobox (переробка Select з нуля — docs/spec/components/select.md)"</div>
            <p class=cx!("fg-muted")>
                "Портал + hooks::use_popover_position — перевірити наживо: розгорни список і поскрол/зменш вікно, "
                "щоб побачити flip/align; hover/focus на тригерах — CSS-стани самі підхоплюються (:hover/:focus-visible)."
            </p>
            <div class=cx!("flex gap4 items-s wrap")>
                <div class=cx!("flex col gap1")>
                    <span class=cx!("t-xs fg-muted")>"field, закритий"</span>
                    <Combobox
                        value=cb_closed
                        items=kind_items
                        on_change=Callback::new(move |v| cb_closed.set(v))
                        placeholder="Оберіть вид".to_string()
                        searchable=false
                    />
                </div>
                <div class=cx!("flex col gap1")>
                    <span class=cx!("t-xs fg-muted")>"field, вибрано"</span>
                    <Combobox
                        value=cb_selected
                        items=kind_items
                        on_change=Callback::new(move |v| cb_selected.set(v))
                        placeholder="Оберіть вид".to_string()
                    />
                </div>
                <div class=cx!("flex col gap1")>
                    <span class=cx!("t-xs fg-muted")>"field, disabled"</span>
                    <Combobox
                        value=cb_selected
                        items=kind_items
                        on_change=Callback::new(|_| {})
                        placeholder="Оберіть вид".to_string()
                        disabled=true
                    />
                </div>
                <div class=cx!("flex col gap1")>
                    <span class=cx!("t-xs fg-muted")>"field, invalid"</span>
                    <Combobox
                        value=cb_invalid
                        items=kind_items
                        on_change=Callback::new(move |v| cb_invalid.set(v))
                        placeholder="Оберіть вид".to_string()
                        invalid=true
                    />
                </div>
                <div class=cx!("flex col gap1")>
                    <span class=cx!("t-xs fg-muted")>"field, довгий текст (обрізання+tooltip)"</span>
                    <Combobox
                        value=cb_long
                        items=long_items
                        on_change=Callback::new(move |v| cb_long.set(v))
                        placeholder="—".to_string()
                    />
                </div>
            </div>

            <div class=cx!("mt3")>
                <span class=cx!("t-xs fg-muted")>
                    "in-cell — заповнює клітинку рівно (як у Grid), без власної рамки; той самий набір станів, що field (07 §7)"
                </span>
                <table class=cx!("mt1 w-full bd")>
                    <thead>
                        <tr>
                            <th class=cx!("p2 bd t-xs fg-muted")>"закритий"</th>
                            <th class=cx!("p2 bd t-xs fg-muted")>"вибрано"</th>
                            <th class=cx!("p2 bd t-xs fg-muted")>"disabled"</th>
                            <th class=cx!("p2 bd t-xs fg-muted")>"invalid"</th>
                            <th class=cx!("p2 bd t-xs fg-muted")>"довгий текст"</th>
                            <th class=cx!("p2 bd t-xs fg-muted")>"пошук+групи (ВОС)"</th>
                            <th class=cx!("p2 bd t-xs fg-muted")>"300 елементів"</th>
                            <th class=cx!("p2 bd fg-muted")>"сусідня клітинка"</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr>
                            <td class=cx!("p0 bd")>
                                <Combobox
                                    value=cb_cell_closed
                                    items=kind_items
                                    on_change=Callback::new(move |v| cb_cell_closed.set(v))
                                    placeholder="—".to_string()
                                    variant=ComboboxVariant::InCell
                                />
                            </td>
                            <td class=cx!("p0 bd")>
                                <Combobox
                                    value=cb_cell_selected
                                    items=kind_items
                                    on_change=Callback::new(move |v| cb_cell_selected.set(v))
                                    placeholder="—".to_string()
                                    variant=ComboboxVariant::InCell
                                />
                            </td>
                            <td class=cx!("p0 bd")>
                                <Combobox
                                    value=cb_cell_selected
                                    items=kind_items
                                    on_change=Callback::new(|_| {})
                                    placeholder="—".to_string()
                                    variant=ComboboxVariant::InCell
                                    disabled=true
                                />
                            </td>
                            <td class=cx!("p0 bd")>
                                <Combobox
                                    value=cb_cell_invalid
                                    items=kind_items
                                    on_change=Callback::new(move |v| cb_cell_invalid.set(v))
                                    placeholder="—".to_string()
                                    variant=ComboboxVariant::InCell
                                    invalid=true
                                />
                            </td>
                            <td class=cx!("p0 bd")>
                                <Combobox
                                    value=cb_cell_long
                                    items=long_items
                                    on_change=Callback::new(move |v| cb_cell_long.set(v))
                                    placeholder="—".to_string()
                                    variant=ComboboxVariant::InCell
                                />
                            </td>
                            <td class=cx!("p0 bd")>
                                <Combobox
                                    value=cb_cell_vos
                                    items=vos_items
                                    on_change=Callback::new(move |v| cb_cell_vos.set(v))
                                    placeholder="Пошук ВОС…".to_string()
                                    variant=ComboboxVariant::InCell
                                    searchable=true
                                    empty_message="Нічого не знайдено".to_string()
                                />
                            </td>
                            <td class=cx!("p0 bd")>
                                <Combobox
                                    value=cb_cell_many
                                    items=many_items
                                    on_change=Callback::new(move |v| cb_cell_many.set(v))
                                    placeholder="Пошук…".to_string()
                                    variant=ComboboxVariant::InCell
                                    searchable=true
                                />
                            </td>
                            <td class=cx!("p2 bd fg-muted")>"тригер не налазить"</td>
                        </tr>
                    </tbody>
                </table>
            </div>

            <div class=cx!("mt3 flex gap4 items-s wrap")>
                <div class=cx!("flex col gap1")>
                    <span class=cx!("t-xs fg-muted")>"з пошуком, групи, двострічкові пункти (підказка ВОС)"</span>
                    <Combobox
                        value=cb_vos
                        items=vos_items
                        on_change=Callback::new(move |v| cb_vos.set(v))
                        placeholder="Пошук ВОС…".to_string()
                        searchable=true
                        empty_message="Нічого не знайдено — спробуй \"вамп\" чи \"НРК\"".to_string()
                    />
                </div>
                <div class=cx!("flex col gap1")>
                    <span class=cx!("t-xs fg-muted")>"300 елементів (продуктивність), порожній результат — набери щось відсутнє"</span>
                    <Combobox
                        value=cb_many
                        items=many_items
                        on_change=Callback::new(move |v| cb_many.set(v))
                        placeholder="Пошук…".to_string()
                        searchable=true
                    />
                </div>
            </div>

            <div class="eyebrow">{format!("Усі атоми ({})", style::all_atoms().len())}</div>
            <p class=cx!("fg-muted")>"Повний перелік — для пошуку. Групування за категоріями — style/ATOMS.md."</p>
            <div class=cx!("flex wrap gap1")>
                {style::all_atoms()
                    .into_iter()
                    .map(|name| view! { <code class=cx!("t-xs mono")>{name}</code> })
                    .collect_view()}
            </div>
        </Show>
        </PageContent>
    }
}
