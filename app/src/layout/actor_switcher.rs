use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::{Select, SelectOption};
use crate::hooks::use_actor::use_actor;
use crate::services::auth::{auth_logout, get_user_roles, switch_actor};
use crate::services::orgs::list_orgs;
use crate::types::actor::{Actor, Role};
use crate::types::auth::AuthMode;

#[component]
pub fn ActorSwitcher() -> impl IntoView {
    let auth_mode = expect_context::<RwSignal<AuthMode>>();

    view! {
        {move || match auth_mode.get() {
            AuthMode::Dev => view! { <DevSwitcher/> }.into_any(),
            AuthMode::Auth => view! { <AuthSwitcher/> }.into_any(),
        }}
        <ModeToggle/>
    }
}

#[component]
fn ModeToggle() -> impl IntoView {
    let auth_mode = expect_context::<RwSignal<AuthMode>>();

    let toggle = move |_: web_sys::MouseEvent| {
        auth_mode.update(|m| {
            *m = match m {
                AuthMode::Dev => AuthMode::Auth,
                AuthMode::Auth => AuthMode::Dev,
            };
        });
    };

    let label = move || match auth_mode.get() {
        AuthMode::Dev => "DEV",
        AuthMode::Auth => "AUTH",
    };

    view! {
        <button
            class="btn btn--ghost btn--xs auth-mode-toggle"
            on:click=toggle
            title="Перемкнути між dev та auth режимами"
        >
            {label}
        </button>
    }
}

#[component]
fn DevSwitcher() -> impl IntoView {
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
                                view! { <span class="status-error">"немає організацій"</span> }
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

#[component]
fn AuthSwitcher() -> impl IntoView {
    let actor = use_actor();
    let auth_mode = expect_context::<RwSignal<AuthMode>>();
    let auth_display_name = expect_context::<RwSignal<Option<String>>>();
    let roles = Resource::new(|| (), |_| get_user_roles());
    let menu_open = RwSignal::new(false);

    let on_logout = move |_: web_sys::MouseEvent| {
        spawn_local(async move {
            let _ = auth_logout().await;
            actor.set(None);
            auth_display_name.set(None);
            auth_mode.set(AuthMode::Dev);
            if cfg!(target_arch = "wasm32") {
                let _ = window().location().set_href("/login");
            }
        });
    };

    view! {
        <div class="actor-switcher actor-switcher--auth">
            <Suspense fallback=|| view! { <span>"..."</span> }>
                {move || {
                    let name = auth_display_name.get();
                    let display = name.unwrap_or_else(|| "Користувач".to_string());
                    Some(view! {
                        <div class="auth-user">
                            <button
                                class="auth-user__trigger"
                                on:click=move |_| menu_open.update(|v| *v = !*v)
                            >
                                <span class="auth-user__avatar">
                                    {display.chars().next().unwrap_or('?').to_uppercase().to_string()}
                                </span>
                                <span class="auth-user__name">{display}</span>
                                <svg class="auth-user__chevron" width="10" height="10" viewBox="0 0 10 10">
                                    <path d="M2 4 L5 7 L8 4" fill="none" stroke="currentColor" stroke-width="1.5"/>
                                </svg>
                            </button>
                            <Show when=move || menu_open.get()>
                                <div class="auth-user__menu">
                                    {move || {
                                        roles.get().map(|res| match res {
                                            Ok(role_list) if !role_list.is_empty() => {
                                                let items: Vec<_> = role_list.iter().map(|r| {
                                                    let org_id = r.org_id;
                                                    let role_str = r.role.clone();
                                                    let role_label = Role::parse(&r.role).map(|rl| rl.label()).unwrap_or(&r.role);
                                                    let label = format!("{} — {}", r.org_label, role_label);
                                                    let is_active = actor.get().map(|a| a.org_id == org_id).unwrap_or(false);
                                                    let active_class = if is_active { " auth-user__role--active" } else { "" };
                                                    view! {
                                                        <button
                                                            class=format!("auth-user__role{active_class}")
                                                            on:click=move |_| {
                                                                let role = Role::parse(&role_str).unwrap_or(Role::Admin);
                                                                actor.set(Some(Actor { org_id, role }));
                                                                let rs = role_str.clone();
                                                                spawn_local(async move {
                                                                    let _ = switch_actor(org_id, rs).await;
                                                                });
                                                                menu_open.set(false);
                                                            }
                                                        >
                                                            {label}
                                                        </button>
                                                    }
                                                }).collect();
                                                view! {
                                                    <div class="auth-user__roles">
                                                        {items}
                                                    </div>
                                                }.into_any()
                                            }
                                            Ok(_) => view! {
                                                <p class="auth-user__no-roles">"Немає призначених ролей"</p>
                                            }.into_any(),
                                            Err(_) => view! {
                                                <p class="auth-user__no-roles">"Увійдіть для доступу"</p>
                                            }.into_any(),
                                        })
                                    }}
                                    <hr class="auth-user__divider"/>
                                    <a href="/settings" class="auth-user__menu-item" on:click=move |_| menu_open.set(false)>
                                        "Налаштування"
                                    </a>
                                    <button class="auth-user__menu-item auth-user__logout" on:click=on_logout>
                                        "Вийти"
                                    </button>
                                </div>
                            </Show>
                        </div>
                    })
                }}
            </Suspense>
        </div>
    }
}
