//! Сторінка `/styleguide` (Фаза 2, docs/spec/08-style-system.md) — живий довідник атомів/токенів/
//! тем стильової системи, лише для `admin` (внутрішній інструмент розробки, не частина домену).
//! Перемикач тем міняє `data-theme` на `<html>` (те, що реально таргетить згенерований CSS —
//! `:root[data-theme="..."]`), скидається при виході зі сторінки, щоб не "протікати" в решту
//! застосунку (тут нема повноцінного per-користувач теми-перемикача — поза обсягом Фази 2).

use leptos::prelude::*;

use crate::components::{Accordion, AccordionItem, Checkbox};
use crate::hooks::use_actor::use_actor;
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

    view! {
        <Show when=is_admin fallback=|| view! { <p>"Сторінка лише для адміністратора."</p> }>
            <h1>"Стильова система — довідник"</h1>
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

            <div class="eyebrow">{format!("Усі атоми ({})", style::all_atoms().len())}</div>
            <p class=cx!("fg-muted")>"Повний перелік — для пошуку. Групування за категоріями — style/ATOMS.md."</p>
            <div class=cx!("flex wrap gap1")>
                {style::all_atoms()
                    .into_iter()
                    .map(|name| view! { <code class=cx!("t-xs mono")>{name}</code> })
                    .collect_view()}
            </div>
        </Show>
    }
}
