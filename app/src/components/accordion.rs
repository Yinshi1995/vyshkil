//! Акордеон (feedback користувача — HeroUI-референс). Композиція розміткою (`<Accordion><
//! AccordionItem title="...">...</AccordionItem></Accordion>`), не даними-в-Vec — простіше для
//! викликача, коли вміст пункту — довільна розмітка, не рядок. Кожен пункт розгортається
//! незалежно (не "лише один відкритий" — найпростіший варіант, той самий, що HeroUI "Multiple
//! Expanded"). Плавне розгортання — CSS grid-template-rows 0fr→1fr, без вимірювання висоти в JS.

use leptos::prelude::*;

#[component]
pub fn Accordion(children: Children) -> impl IntoView {
    view! { <div class="accordion">{children()}</div> }
}

#[component]
pub fn AccordionItem(#[prop(into)] title: String, children: Children) -> impl IntoView {
    let open = RwSignal::new(false);
    let content = children();

    view! {
        <div class="accordion-item">
            <button
                type="button"
                class="accordion-item__trigger"
                aria-expanded=move || open.get().to_string()
                on:click=move |_| open.update(|o| *o = !*o)
            >
                <span class="accordion-item__title">{title}</span>
                <svg class="accordion-item__chevron" viewBox="0 0 12 8" width="12" height="8" aria-hidden="true">
                    <path d="M1 1.5 6 6.5 11 1.5" stroke="currentColor" stroke-width="1.6" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
            </button>
            <div class="accordion-item__panel" class:accordion-item__panel--open=move || open.get()>
                <div class="accordion-item__panel-inner">{content}</div>
            </div>
        </div>
    }
}
