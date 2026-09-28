use leptos::prelude::*;

use crate::hooks::use_actor::use_actor;
use crate::services::orgs::search_orgs;
use crate::widgets::ActorNotice;

/// Нечіткий пошук організацій (02 §3): стійкий до опечаток/розкладки/скорочень
/// ("152НЦ", "а4896", "польша" — усі знаходять канонічну організацію). Результат звужений до
/// видимого поточному актору піддерева (backend::policy) — без обраного актора показуємо
/// `<ActorNotice/>`, а не мовчазне "нічого не знайдено".
#[component]
pub fn OrgSearch() -> impl IntoView {
    let actor = use_actor();
    let query = RwSignal::new(String::new());
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
        <div class="eyebrow">"Пошук організацій"</div>
        <div class="card">
            <input
                type="text"
                class="org-search__input"
                placeholder="152НЦ, а4896, польша…"
                prop:value=move || query.get()
                on:input=move |ev| query.set(event_target_value(&ev))
            />
            <Suspense fallback=|| view! { <p>"…"</p> }>
                {move || {
                    if actor.get().is_none() {
                        return Some(view! { <ActorNotice/> }.into_any());
                    }
                    results
                        .get()
                        .map(|res| match res {
                            Ok(_) if query.get().trim().is_empty() => {
                                view! { <p class="card__desc">"Почніть вводити номер, назву або синонім."</p> }
                                    .into_any()
                            }
                            Ok(list) if list.is_empty() => {
                                view! { <p class="card__desc">"Нічого не знайдено."</p> }.into_any()
                            }
                            Ok(list) => {
                                view! {
                                    <ul class="org-search__results">
                                        {list
                                            .into_iter()
                                            .map(|r| {
                                                view! {
                                                    <li class="org-search__result">
                                                        <span class="org-search__label">{r.label}</span>
                                                        <span class="org-search__matched">
                                                            "збіг: \""{r.matched_raw}"\""
                                                        </span>
                                                        <a href=format!("/org/{}", r.org_id) class="org-search__link">
                                                            "картка →"
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
