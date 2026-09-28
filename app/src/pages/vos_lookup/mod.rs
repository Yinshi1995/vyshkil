mod server;

use leptos::prelude::*;

use server::get_equipment_vos_hint;

/// Сторінка `/vos-lookup`: підказка "ОВТ/сленг → ВОС" (02 §3, критерій готовності Етапу 2:
/// "вамп" → 218, "mavic" → 217, "fpv" → 219, "нрк" → 129, "darts" → 216, з поясненням "бо …").
#[component]
pub fn VosLookupPage() -> impl IntoView {
    let query = RwSignal::new(String::new());
    let results = Resource::new(
        move || query.get(),
        |q| async move {
            if q.trim().is_empty() {
                Ok(Vec::new())
            } else {
                get_equipment_vos_hint(q).await
            }
        },
    );

    view! {
        <h1>"Підказка ВОС за ОВТ"</h1>
        <p>
            "Набери назву чи сленг обладнання (\"вамп\", \"mavic\", \"fpv\", \"нрк\", \"darts\"…) — "
            "покажемо ВОС, до якого веде ця підказка."
        </p>
        <div class="eyebrow">"Пошук"</div>
        <div class="card">
            <input
                type="text"
                class="hint-search__input"
                placeholder="вамп, mavic, fpv, нрк, darts…"
                prop:value=move || query.get()
                on:input=move |ev| query.set(event_target_value(&ev))
            />
            <Suspense fallback=|| view! { <p>"…"</p> }>
                {move || {
                    results
                        .get()
                        .map(|res| match res {
                            Ok(_) if query.get().trim().is_empty() => {
                                view! { <p class="card__desc">"Почніть вводити назву ОВТ або сленг."</p> }
                                    .into_any()
                            }
                            Ok(list) if list.is_empty() => {
                                view! { <p class="card__desc">"Нічого не знайдено."</p> }.into_any()
                            }
                            Ok(list) => {
                                view! {
                                    <ul class="hint-search__results">
                                        {list
                                            .into_iter()
                                            .map(|h| {
                                                view! {
                                                    <li class="hint-search__result">
                                                        <span class="hint-search__label">
                                                            "ВОС "{h.vos_code}" — "{h.vos_title}
                                                        </span>
                                                        <span class="hint-search__reason">
                                                            "бо \""{h.matched_raw}"\" → "{h.matched_equipment}
                                                        </span>
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
