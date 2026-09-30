mod components;
mod server;

use leptos::prelude::*;

use crate::layout::{ContentWidth, PageContent, PageHeader};
use crate::services::health::health_check;
use crate::services::orgs::list_orgs;
use components::{OrgSearch, SubordinationTree};

#[component]
pub fn HomePage() -> impl IntoView {
    let db_status = Resource::new(|| (), |_| health_check());
    let orgs = Resource::new(|| (), |_| list_orgs());

    view! {
        <PageHeader
            title="Taktoblik".to_string()
            subtitle="Облік заходів підготовки військових частин: збір даних, нормалізація, звірка між рівнями підпорядкування, звітні документи."
                .to_string()
        />
        <PageContent width=ContentWidth::Detail>
        <div class="card-row">
            <div class="card">
                <div class="icon-box">
                    <svg viewBox="0 0 24 24" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
                        <ellipse cx="12" cy="5" rx="8" ry="3"></ellipse>
                        <path d="M4 5v6c0 1.66 3.58 3 8 3s8-1.34 8-3V5"></path>
                        <path d="M4 11v6c0 1.66 3.58 3 8 3s8-1.34 8-3v-6"></path>
                    </svg>
                </div>
                <div class="card__label">"З'єднання з БД"</div>
                <Suspense fallback=|| view! { <div class="card__value">"…"</div> }>
                    {move || {
                        db_status
                            .get()
                            .map(|res| match res {
                                Ok(status) => {
                                    view! { <div class="card__value status-ok">{status}</div> }.into_any()
                                }
                                Err(e) => {
                                    view! {
                                        <div class="card__value status-error">"помилка"</div>
                                        <div class="card__desc">{e.to_string()}</div>
                                    }
                                        .into_any()
                                }
                            })
                    }}
                </Suspense>
            </div>
            <div class="card">
                <div class="icon-box">
                    <svg viewBox="0 0 24 24" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M3 21h18"></path>
                        <path d="M6 21V7l6-4 6 4v14"></path>
                        <path d="M10 21v-6h4v6"></path>
                        <path d="M10 11h.01M14 11h.01M10 15h.01M14 15h.01"></path>
                    </svg>
                </div>
                <div class="card__label">"Організацій у довіднику"</div>
                <Suspense fallback=|| view! { <div class="card__value">"…"</div> }>
                    {move || {
                        orgs.get()
                            .map(|res| match res {
                                Ok(list) => view! { <div class="card__value">{list.len()}</div> }.into_any(),
                                Err(e) => {
                                    view! {
                                        <div class="card__value status-error">"помилка"</div>
                                        <div class="card__desc">{e.to_string()}</div>
                                    }
                                        .into_any()
                                }
                            })
                    }}
                </Suspense>
                <div class="card__desc">"Dev-сід Етапу 1 (зі specи, не з source_files): органи + приклад переходу 17 АК → 7 КШР."</div>
            </div>
        </div>
        <OrgSearch/>
        <SubordinationTree/>
        </PageContent>
    }
}
