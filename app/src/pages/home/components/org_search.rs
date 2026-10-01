use leptos::prelude::*;

use crate::hooks::use_actor::use_actor;
use crate::services::orgs::search_orgs;
use crate::widgets::ActorNotice;

#[component]
pub fn OrgSearch() -> impl IntoView {
    let actor = use_actor();
    let query = RwSignal::new(String::new());
    let show_list = RwSignal::new(false);

    let results = Resource::new(
        move || (actor.get(), query.get()),
        |(actor, q)| async move { search_orgs(actor, q).await },
    );

    view! {
        <div class="eyebrow">"Пошук частин"</div>
        <div class="card">
            <input
                type="text"
                class="org-search__input"
                placeholder="Номер, назва або синонім частини…"
                prop:value=move || query.get()
                on:input=move |ev| {
                    query.set(event_target_value(&ev));
                    show_list.set(true);
                }
                on:focus=move |_| show_list.set(true)
            />
            <Suspense fallback=|| view! { <p class="card__desc">"…"</p> }>
                {move || {
                    if actor.get().is_none() {
                        return Some(view! { <ActorNotice/> }.into_any());
                    }
                    if !show_list.get() {
                        return Some(().into_any());
                    }
                    results
                        .get()
                        .map(|res| match res {
                            Ok(list) if list.is_empty() && !query.get().trim().is_empty() => {
                                view! { <p class="card__desc">"Нічого не знайдено."</p> }.into_any()
                            }
                            Ok(list) if list.is_empty() => {
                                ().into_any()
                            }
                            Ok(list) => {
                                let has_query = !query.get().trim().is_empty();
                                view! {
                                    <ul class="org-search__results">
                                        {list
                                            .into_iter()
                                            .map(|r| {
                                                let matched = r.matched_raw.clone();
                                                view! {
                                                    <li class="org-search__result">
                                                        <a
                                                            href=format!("/org/{}", r.org_id)
                                                            class="org-search__result-link"
                                                        >
                                                            <span class="org-search__label">{r.label}</span>
                                                            {if has_query {
                                                                Some(
                                                                    view! {
                                                                        <span class="org-search__matched">{matched}</span>
                                                                    },
                                                                )
                                                            } else {
                                                                None
                                                            }}
                                                        </a>
                                                    </li>
                                                }
                                            })
                                            .collect_view()}
                                    </ul>
                                }
                                    .into_any()
                            }
                            Err(e) => {
                                view! { <p class="card__desc status-error">{e.to_string()}</p> }.into_any()
                            }
                        })
                }}
            </Suspense>
        </div>
    }
}
