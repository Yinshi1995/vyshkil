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
    let highlighted = RwSignal::new(0usize);
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

    let handle_keydown = move |ev: web_sys::KeyboardEvent| {
        let list = if open.get_untracked() {
            results.get_untracked().and_then(|r| r.ok()).unwrap_or_default()
        } else {
            Vec::new()
        };
        match ev.key().as_str() {
            "ArrowDown" if !list.is_empty() => {
                ev.prevent_default();
                highlighted.update(|h| *h = (*h + 1).min(list.len() - 1));
            }
            "ArrowUp" if !list.is_empty() => {
                ev.prevent_default();
                highlighted.update(|h| *h = h.saturating_sub(1));
            }
            "Escape" if open.get_untracked() => {
                ev.prevent_default();
                ev.stop_propagation();
                open.set(false);
            }
            // Без `!ev.ctrl_key()`: Ctrl+Enter (02 §2, "зберегти всі зміни") теж key=="Enter" --
            // без цієї перевірки клітинка ОДНОЧАСНО й підтверджувала підказку, поки глобальний
            // слухач паралельно викликав коміт.
            "Enter" if !ev.ctrl_key() && !list.is_empty() => {
                let chosen = &list[highlighted.get_untracked().min(list.len() - 1)];
                on_select.run((chosen.org_id, chosen.label.clone()));
                open.set(false);
                on_keydown.run(ev);
            }
            "Tab" if !list.is_empty() => {
                let chosen = &list[highlighted.get_untracked().min(list.len() - 1)];
                on_select.run((chosen.org_id, chosen.label.clone()));
                open.set(false);
                on_keydown.run(ev);
            }
            _ => on_keydown.run(ev),
        }
    };

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
                    highlighted.set(0);
                }
                on:focus=move |_| open.set(true)
                on:keydown=handle_keydown
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
                                        .enumerate()
                                        .map(|(i, r)| {
                                            let label_val = r.label.clone();
                                            let org_id = r.org_id;
                                            let is_active = move || highlighted.get() == i;
                                            view! {
                                                <li
                                                    class="cell__dropdown-item"
                                                    class:cell__dropdown-item--active=is_active
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
    let highlighted = RwSignal::new(0usize);
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

    let handle_keydown = move |ev: web_sys::KeyboardEvent| {
        let list = if open.get_untracked() {
            results.get_untracked().and_then(|r| r.ok()).unwrap_or_default()
        } else {
            Vec::new()
        };
        match ev.key().as_str() {
            "ArrowDown" if !list.is_empty() => {
                ev.prevent_default();
                highlighted.update(|h| *h = (*h + 1).min(list.len() - 1));
            }
            "ArrowUp" if !list.is_empty() => {
                ev.prevent_default();
                highlighted.update(|h| *h = h.saturating_sub(1));
            }
            "Escape" if open.get_untracked() => {
                ev.prevent_default();
                ev.stop_propagation();
                open.set(false);
            }
            "Enter" if !ev.ctrl_key() && !list.is_empty() => {
                let chosen = list[highlighted.get_untracked().min(list.len() - 1)].clone();
                on_select.run(chosen);
                open.set(false);
                on_keydown.run(ev);
            }
            "Tab" if !list.is_empty() => {
                let chosen = list[highlighted.get_untracked().min(list.len() - 1)].clone();
                on_select.run(chosen);
                open.set(false);
                on_keydown.run(ev);
            }
            _ => on_keydown.run(ev),
        }
    };

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
                    highlighted.set(0);
                }
                on:focus=move |_| open.set(true)
                on:keydown=handle_keydown
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
                                        .enumerate()
                                        .map(|(i, h)| {
                                            let hint = h.clone();
                                            let kind_label = match h.kind {
                                                VosPositionCourseKind::Vos => "ВОС",
                                                VosPositionCourseKind::Position => "посада",
                                                VosPositionCourseKind::Course => "курс",
                                            };
                                            let is_active = move || highlighted.get() == i;
                                            view! {
                                                <li
                                                    class="cell__dropdown-item"
                                                    class:cell__dropdown-item--active=is_active
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
