use leptos::prelude::*;

use crate::components::{Select, SelectOption};
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
                                let org_options: Vec<SelectOption> = list
                                    .iter()
                                    .map(|(id, name)| SelectOption::new(id.to_string(), name.clone()))
                                    .collect();
                                let role_options: Vec<SelectOption> = Role::ALL
                                    .iter()
                                    .map(|r| SelectOption::new(r.as_str(), r.label()))
                                    .collect();
                                let org_value = Signal::derive(move || {
                                    actor.get().map(|a| a.org_id.to_string()).unwrap_or_default()
                                });
                                let role_value = Signal::derive(move || {
                                    actor.get().map(|a| a.role.as_str().to_string()).unwrap_or_default()
                                });
                                let on_org_change = Callback::new(move |v: String| {
                                    let org_id: i32 = v.parse().unwrap_or_default();
                                    let role = actor.get_untracked().map(|a| a.role).unwrap_or(Role::Admin);
                                    actor.set(Some(Actor { org_id, role }));
                                });
                                let on_role_change = Callback::new(move |v: String| {
                                    let role = Role::parse(&v).unwrap_or(Role::Admin);
                                    if let Some(a) = actor.get_untracked() {
                                        actor.set(Some(Actor { org_id: a.org_id, role }));
                                    }
                                });
                                view! {
                                    <Select
                                        value=org_value
                                        options=org_options
                                        on_change=on_org_change
                                        placeholder="Оберіть частину"
                                    />
                                    <Select
                                        value=role_value
                                        options=role_options
                                        on_change=on_role_change
                                        placeholder="Роль"
                                    />
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
