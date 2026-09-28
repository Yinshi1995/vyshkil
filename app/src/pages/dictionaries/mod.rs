mod server;

use leptos::prelude::*;

use crate::hooks::use_actor::use_actor;
use crate::services::dictionaries::get_dictionaries_overview;
use crate::types::dictionaries::DictionaryEntry;
use crate::widgets::ActorNotice;
use server::{confirm_learned_alias, get_learned_aliases, reject_learned_alias};

/// Сторінка `/dictionaries`: перегляд усіх "простих" довідників Етапу 2 (01 §2) + для
/// адміністратора — черга learned-синонімів на підтвердження/відхилення (01 §"Навчання").
#[component]
pub fn DictionariesPage() -> impl IntoView {
    let actor = use_actor();
    let overview = Resource::new(|| (), |_| get_dictionaries_overview());

    view! {
        <h1>"Довідники підготовки"</h1>
        <p>"Словники Етапу 2: види й напрямки підготовки, ВОС, посади, ОВТ, курси, причини убуття."</p>

        <Suspense fallback=|| view! { <p>"…"</p> }>
            {move || {
                overview
                    .get()
                    .map(|res| match res {
                        Ok(o) => {
                            view! {
                                <DictionarySection title="Види підготовки".to_string() entries=o.training_kinds/>
                                <DictionarySection title="Напрямки підготовки".to_string() entries=o.training_directions/>
                                <DictionarySection title="Програми БЗВП".to_string() entries=o.bzvp_programs/>
                                <DictionarySection title=format!("ВОС ({})", o.vos.len()) entries=o.vos/>
                                <DictionarySection title=format!("Посади ({})", o.positions.len()) entries=o.positions/>
                                <DictionarySection title="ОВТ".to_string() entries=o.equipment/>
                                <DictionarySection title="Курси".to_string() entries=o.courses/>
                                <DictionarySection title="Причини убуття".to_string() entries=o.attrition_reasons/>
                            }
                                .into_any()
                        }
                        Err(e) => view! { <p class="status-error">{e.to_string()}</p> }.into_any(),
                    })
            }}
        </Suspense>

        <div class="eyebrow">"Learned-синоніми на підтвердження"</div>
        {move || {
            if actor.get().is_none() {
                return view! { <ActorNotice/> }.into_any();
            }
            view! { <LearnedAliasQueue/> }.into_any()
        }}
    }
}

/// Список довідника у вигляді картки-таблиці. Порожній список — легітимний стан (описано текстом).
#[component]
fn DictionarySection(title: String, entries: Vec<DictionaryEntry>) -> impl IntoView {
    view! {
        <div class="eyebrow">{title}</div>
        <div class="card">
            {if entries.is_empty() {
                view! { <p class="card__desc">"Порожньо."</p> }.into_any()
            } else {
                view! {
                    <table>
                        <tbody>
                            {entries
                                .into_iter()
                                .map(|e| {
                                    view! {
                                        <tr>
                                            <td>{e.label}</td>
                                            <td class="status-ok">{e.extra.unwrap_or_default()}</td>
                                        </tr>
                                    }
                                })
                                .collect_view()}
                        </tbody>
                    </table>
                }
                    .into_any()
            }}
        </div>
    }
}

/// Черга learned-синонімів: видима лише адміну (сервер сам це перевіряє — тут просто показуємо
/// помилку доступу, якщо роль не адмін, а не ховаємо кнопку — 01 §6, права лише через `policy`).
#[component]
fn LearnedAliasQueue() -> impl IntoView {
    let actor = use_actor();
    let refresh = RwSignal::new(0u32);
    let queue = Resource::new(
        move || (actor.get(), refresh.get()),
        |(actor, _)| async move { get_learned_aliases(actor).await },
    );

    let on_confirm = move |id: i32| {
        leptos::task::spawn_local(async move {
            if confirm_learned_alias(actor.get(), id).await.is_ok() {
                refresh.update(|n| *n += 1);
            }
        });
    };
    let on_reject = move |id: i32| {
        leptos::task::spawn_local(async move {
            if reject_learned_alias(actor.get(), id).await.is_ok() {
                refresh.update(|n| *n += 1);
            }
        });
    };

    view! {
        <div class="card">
            <Suspense fallback=|| view! { <p>"…"</p> }>
                {move || {
                    queue
                        .get()
                        .map(|res| match res {
                            Ok(list) if list.is_empty() => {
                                view! { <p class="card__desc">"Немає learned-синонімів на підтвердження."</p> }
                                    .into_any()
                            }
                            Ok(list) => {
                                view! {
                                    <table>
                                        <thead>
                                            <tr>
                                                <th>"Тип"</th>
                                                <th>"Сире значення"</th>
                                                <th>"Використань"</th>
                                                <th>"Створено"</th>
                                                <th></th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {list
                                                .into_iter()
                                                .map(|a| {
                                                    let id = a.id;
                                                    view! {
                                                        <tr>
                                                            <td>{a.target_type}</td>
                                                            <td>{a.raw}</td>
                                                            <td>{a.uses_count}</td>
                                                            <td>{a.created_at}</td>
                                                            <td>
                                                                <button
                                                                    class="btn btn--primary"
                                                                    on:click=move |_| on_confirm(id)
                                                                >
                                                                    "Підтвердити"
                                                                </button>
                                                                " "
                                                                <button
                                                                    class="btn btn--outline"
                                                                    on:click=move |_| on_reject(id)
                                                                >
                                                                    "Відхилити"
                                                                </button>
                                                            </td>
                                                        </tr>
                                                    }
                                                })
                                                .collect_view()}
                                        </tbody>
                                    </table>
                                }
                                    .into_any()
                            }
                            Err(e) => view! { <p class="card__desc status-error">{e.to_string()}</p> }.into_any(),
                        })
                }}
            </Suspense>
        </div>
    }
}
