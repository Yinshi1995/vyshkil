//! Ширина контенту сторінки (Етап 7.5, `docs/spec/components/layout.md` §3) — `<main>` (app.rs)
//! більше не має власного `max-width`; кожна сторінка сама оголошує тип через цю обгортку.

use leptos::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContentWidth {
    /// Повна ширина, без обмеження — сітки/таблиці (`/training-form`, `/import`).
    #[default]
    Data,
    /// `max-width:1200px`, готова під дві колонки на широких екранах — картки з деталями.
    Detail,
    /// `max-width:720px` — довгий текст без таблиць.
    Reading,
}

#[component]
pub fn PageContent(#[prop(optional)] width: ContentWidth, children: Children) -> impl IntoView {
    let class = match width {
        ContentWidth::Data => "page-content page-content--data",
        ContentWidth::Detail => "page-content page-content--detail",
        ContentWidth::Reading => "page-content page-content--reading",
    };
    view! { <div class=class>{children()}</div> }
}
