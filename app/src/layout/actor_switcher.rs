use leptos::prelude::*;

use crate::hooks::use_actor::use_actor;
use crate::services::orgs::list_orgs;
use crate::types::actor::{Actor, Role};

/// Перемикач актора: список організацій із `org` + вибір ролі. Тільки для розробки —
/// пізніше цю пару (org, роль) віддаватиме мікросервіс автентифікації (01 §6).
#[component]
pub fn ActorSwitcher() -> impl IntoView {
    let actor = use_actor();
    let orgs = Resource::new(|| (), |_| list_orgs());

    view! {
        <div class="actor-switcher">
            <label>"Актор:"</label>
            <Suspense fallback=|| view! { <span>"..."</span> }>
                {move || {
                    orgs.get()
                        .map(|res| match res {
                            Ok(list) if list.is_empty() => {
                                view! { <span class="status-error">"немає організацій (сід ще не завантажено)"</span> }
                                    .into_any()
                            }
                            Ok(list) => {
                                let on_org_change = move |ev| {
                                    let org_id: i32 = event_target_value(&ev).parse().unwrap_or_default();
                                    let role = actor.get().map(|a| a.role).unwrap_or(Role::Admin);
                                    actor.set(Some(Actor { org_id, role }));
                                };
                                let on_role_change = move |ev| {
                                    let role = Role::parse(&event_target_value(&ev)).unwrap_or(Role::Admin);
                                    if let Some(a) = actor.get() {
                                        actor.set(Some(Actor { org_id: a.org_id, role }));
                                    }
                                };
                                let current_org = actor.get().map(|a| a.org_id);
                                view! {
                                    <select on:change=on_org_change>
                                        {list.iter()
                                            .map(|(id, name)| {
                                                let selected = current_org == Some(*id);
                                                view! {
                                                    <option value=id.to_string() selected=selected>
                                                        {name.clone()}
                                                    </option>
                                                }
                                            })
                                            .collect_view()}
                                    </select>
                                    <select on:change=on_role_change>
                                        {Role::ALL
                                            .iter()
                                            .map(|r| {
                                                view! {
                                                    <option value=r.as_str()>{r.label()}</option>
                                                }
                                            })
                                            .collect_view()}
                                    </select>
                                }
                                    .into_any()
                            }
                            Err(e) => {
                                view! { <span class="status-error">{format!("помилка: {e}")}</span> }
                                    .into_any()
                            }
                        })
                }}
            </Suspense>
        </div>
    }
}
