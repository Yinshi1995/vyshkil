//! Таблиця маршрутів → pages::*. Нова сторінка = новий `<Route>` тут + рядок у `pages/CLAUDE.md`
//! (07 §3.10).

use leptos::prelude::*;
use leptos_router::{
    components::{Route, Routes},
    ParamSegment, StaticSegment,
};

use crate::pages::{
    discrepancies::DiscrepanciesPage, documents::DocumentsPage, home::HomePage, import::ImportPage,
    org_detail::OrgDetailPage, settings::SettingsPage, training_form::TrainingFormPage,
};

#[component]
pub fn AppRoutes() -> impl IntoView {
    view! {
        <Routes fallback=|| view! { <p>"Сторінку не знайдено."</p> }>
            <Route path=StaticSegment("") view=HomePage/>
            <Route path=(StaticSegment("org"), ParamSegment("id")) view=OrgDetailPage/>
            <Route path=StaticSegment("training-form") view=TrainingFormPage/>
            <Route path=StaticSegment("import") view=ImportPage/>
            <Route path=StaticSegment("documents") view=DocumentsPage/>
            <Route path=StaticSegment("discrepancies") view=DiscrepanciesPage/>
            <Route path=StaticSegment("settings") view=SettingsPage/>
        </Routes>
    }
}
