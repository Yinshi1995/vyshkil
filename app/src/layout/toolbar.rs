//! Тулбар над даними (Етап 7.5, `docs/spec/components/layout.md` §4) — `display:flex;
//! justify-content:space-between`, БЕЗ окремих `left`/`right`-пропів (Leptos-компонент з двома
//! іменованими слотами дітей — зайва складність заради того, що `justify-content:space-between`
//! робить сам, якщо викликач дає РІВНО дві обгортки-групи як дітей). Усі контроли всередині —
//! однакова висота (`--control-height`, §4).

use leptos::prelude::*;

#[component]
pub fn Toolbar(children: Children) -> impl IntoView {
    view! { <div class="toolbar">{children()}</div> }
}
