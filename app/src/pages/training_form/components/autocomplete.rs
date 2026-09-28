//! Одне поле "з розумним пошуком" (02 §3) — текстовий інпут з випадайкою підказок під ним.
//! Дві конкретні реалізації (організація / ВОС-посада-курс), а не один generic-компонент:
//! результати мають різну форму (`OrgSearchResult` vs `VosPositionCourseHint`), а Leptos
//! `#[component]` погано дружить з generic-параметрами по типу відповіді ресурсу.

use leptos::prelude::*;

use crate::hooks::use_actor::use_actor;
use crate::services::orgs::search_orgs;
use crate::types::submission::{VosPositionCourseHint, VosPositionCourseKind};
use crate::pages::training_form::server::search_vos_position_course;

/// Пошук організації (02 §1 колонки 1 і 9) — звужений до видимого акторові піддерева (сервер
/// сам фільтрує через `policy::visible_org_ids`, як і `pages::home::components::OrgSearch`).
#[component]
pub fn OrgAutocomplete(
    id: String,
    #[prop(into)] label: Signal<String>,
    #[prop(into)] on_select: Callback<(i32, String)>,
    #[prop(into)] on_label_input: Callback<String>,
    #[prop(into)] on_keydown: Callback<web_sys::KeyboardEvent>,
) -> impl IntoView {
    let actor = use_actor();
    let query = RwSignal::new(String::new());
    let open = RwSignal::new(false);
    let results = Resource::new(
        move || (actor.get(), query.get()),
        |(actor, q)| async move {
            if q.trim().is_empty() {
                Ok(Vec::new())
            } else {
                search_orgs(actor, q).await
            }
        },
    );

    view! {
        <div class="cell cell--autocomplete">
            <input
                type="text"
                id=id
                class="cell__input"
                prop:value=move || label.get()
                on:input=move |ev| {
                    let v = event_target_value(&ev);
                    on_label_input.run(v.clone());
                    query.set(v);
                    open.set(true);
                }
                on:focus=move |_| open.set(true)
                on:keydown=move |ev| on_keydown.run(ev)
            />
            <Show when=move || open.get() && !query.get().trim().is_empty()>
                <ul class="cell__dropdown">
                    {move || {
                        results
                            .get()
                            .map(|res| match res {
                                Ok(list) if list.is_empty() => {
                                    view! { <li class="cell__dropdown-empty">"нічого не знайдено"</li> }
                                        .into_any()
                                }
                                Ok(list) => {
                                    list.into_iter()
                                        .map(|r| {
                                            let label_val = r.label.clone();
                                            let org_id = r.org_id;
                                            view! {
                                                <li
                                                    class="cell__dropdown-item"
                                                    on:mousedown=move |ev| {
                                                        ev.prevent_default();
                                                        on_select.run((org_id, label_val.clone()));
                                                        open.set(false);
                                                    }
                                                >
                                                    {r.label}
                                                    <span class="cell__dropdown-matched">"— \""{r.matched_raw}"\""</span>
                                                </li>
                                            }
                                        })
                                        .collect_view()
                                        .into_any()
                                }
                                Err(_) => ().into_any(),
                            })
                    }}
                </ul>
            </Show>
        </div>
    }
}

/// Поле "ВОС / посада / курс" (02 §3) — одне поле з поясненням "бо …" для непрямих (через ОВТ)
/// підказок.
#[component]
pub fn VosPositionCourseAutocomplete(
    id: String,
    #[prop(into)] label: Signal<String>,
    #[prop(into)] on_select: Callback<VosPositionCourseHint>,
    #[prop(into)] on_label_input: Callback<String>,
    #[prop(into)] on_keydown: Callback<web_sys::KeyboardEvent>,
) -> impl IntoView {
    let query = RwSignal::new(String::new());
    let open = RwSignal::new(false);
    let results = Resource::new(
        move || query.get(),
        |q| async move {
            if q.trim().is_empty() {
                Ok(Vec::new())
            } else {
                search_vos_position_course(q).await
            }
        },
    );

    view! {
        <div class="cell cell--autocomplete">
            <input
                type="text"
                id=id
                class="cell__input"
                prop:value=move || label.get()
                on:input=move |ev| {
                    let v = event_target_value(&ev);
                    on_label_input.run(v.clone());
                    query.set(v);
                    open.set(true);
                }
                on:focus=move |_| open.set(true)
                on:keydown=move |ev| on_keydown.run(ev)
            />
            <Show when=move || open.get() && !query.get().trim().is_empty()>
                <ul class="cell__dropdown">
                    {move || {
                        results
                            .get()
                            .map(|res| match res {
                                Ok(list) if list.is_empty() => {
                                    view! { <li class="cell__dropdown-empty">"нічого не знайдено"</li> }
                                        .into_any()
                                }
                                Ok(list) => {
                                    list.into_iter()
                                        .map(|h| {
                                            let hint = h.clone();
                                            let kind_label = match h.kind {
                                                VosPositionCourseKind::Vos => "ВОС",
                                                VosPositionCourseKind::Position => "посада",
                                                VosPositionCourseKind::Course => "курс",
                                            };
                                            view! {
                                                <li
                                                    class="cell__dropdown-item"
                                                    on:mousedown=move |ev| {
                                                        ev.prevent_default();
                                                        on_select.run(hint.clone());
                                                        open.set(false);
                                                    }
                                                >
                                                    <span class="cell__dropdown-kind">{kind_label}</span>
                                                    " "{h.label}
                                                    {h.why.map(|w| view! { <span class="cell__dropdown-why">" · "{w}</span> })}
                                                </li>
                                            }
                                        })
                                        .collect_view()
                                        .into_any()
                                }
                                Err(_) => ().into_any(),
                            })
                    }}
                </ul>
            </Show>
        </div>
    }
}
