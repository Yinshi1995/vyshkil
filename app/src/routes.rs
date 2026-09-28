//! Таблиця маршрутів → pages::*. Нова сторінка = новий `<Route>` тут + рядок у `pages/CLAUDE.md`
//! (07 §3.10).

use leptos::prelude::*;
use leptos_router::{
    components::{Route, Routes},
    ParamSegment, StaticSegment,
};

use crate::pages::{home::HomePage, org_detail::OrgDetailPage};

#[component]
pub fn AppRoutes() -> impl IntoView {
    view! {
        <Routes fallback=|| view! { <p>"Сторінку не знайдено."</p> }>
            <Route path=StaticSegment("") view=HomePage/>
            <Route path=(StaticSegment("org"), ParamSegment("id")) view=OrgDetailPage/>
        </Routes>
    }
}
